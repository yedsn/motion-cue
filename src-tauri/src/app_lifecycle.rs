use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[cfg(target_os = "macos")]
const MACOS_REOPEN_DELAY: Duration = Duration::from_millis(250);
const MACOS_PROTOCOL_REOPEN_SUPPRESSION: Duration = Duration::from_millis(750);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManagementWindowEvent {
    GuiStartup,
    BackgroundStartup,
    OpenRequested,
    CloseRequested,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManagementWindowState {
    pub window_visible: bool,
    pub dock_visible: bool,
    pub focus_window: bool,
    pub regular_app: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayManagementAction {
    None,
    Open,
    OpenAndCheckUpdate,
}

pub struct MacosReopenState {
    generation: AtomicU64,
    suppress_until: Mutex<Option<Instant>>,
}

impl MacosReopenState {
    pub fn new() -> Self {
        Self {
            generation: AtomicU64::new(0),
            suppress_until: Mutex::new(None),
        }
    }

    pub fn suppress_for_protocol_playback(&self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        *self.suppress_until.lock().unwrap() =
            Some(Instant::now() + MACOS_PROTOCOL_REOPEN_SUPPRESSION);
    }

    pub fn begin_reopen(&self) -> u64 {
        self.generation.fetch_add(1, Ordering::AcqRel) + 1
    }

    pub fn explicit_open(&self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        *self.suppress_until.lock().unwrap() = None;
    }

    pub fn should_restore_window(&self, generation: u64) -> bool {
        if self.generation.load(Ordering::Acquire) != generation {
            return false;
        }
        !self
            .suppress_until
            .lock()
            .unwrap()
            .is_some_and(|deadline| deadline > Instant::now())
    }
}

#[cfg(target_os = "macos")]
pub const fn macos_reopen_delay() -> Duration {
    MACOS_REOPEN_DELAY
}

pub const fn desired_state(event: ManagementWindowEvent) -> ManagementWindowState {
    match event {
        ManagementWindowEvent::GuiStartup | ManagementWindowEvent::OpenRequested => {
            ManagementWindowState {
                window_visible: true,
                dock_visible: true,
                focus_window: true,
                regular_app: true,
            }
        }
        ManagementWindowEvent::BackgroundStartup | ManagementWindowEvent::CloseRequested => {
            ManagementWindowState {
                window_visible: false,
                dock_visible: false,
                focus_window: false,
                regular_app: false,
            }
        }
    }
}

pub fn tray_management_action(menu_id: &str) -> TrayManagementAction {
    match menu_id {
        "open" => TrayManagementAction::Open,
        "check_update" => TrayManagementAction::OpenAndCheckUpdate,
        _ => TrayManagementAction::None,
    }
}

pub fn handles_window_lifecycle(window_label: &str) -> bool {
    window_label == "main"
}

pub fn register_macos_link_receiver(app: &tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    {
        use tauri::Manager;
        let result = app.path().resource_dir().map_err(|error| error.to_string())
            .and_then(|directory| {
                let receiver = directory.join("MotionCueLink.app");
                if !receiver.is_dir() { return Ok(()); }
                let status = std::process::Command::new("/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister")
                    .arg("-f").arg(&receiver).status().map_err(|error| error.to_string())?;
                if !status.success() { return Err(format!("链接接收器注册失败: {status}")); }
                let launch = std::process::Command::new("/usr/bin/open").arg("-g").arg(receiver)
                    .status().map_err(|error| error.to_string())?;
                if launch.success() { Ok(()) } else { Err(format!("链接接收器启动失败: {launch}")) }
            });
        if let Err(error) = result {
            app.state::<crate::AppState>()
                .diagnostics
                .record("warning", "invocation", error, None);
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

/// Read native state on the UI thread; never include invocation parameters.
pub fn record_macos_event(app: &tauri::AppHandle, event: &'static str) {
    #[cfg(target_os = "macos")]
    {
        use tauri::Manager;
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            let Some(marker) = objc2::MainThreadMarker::new() else {
                return;
            };
            let native = objc2_app_kit::NSApplication::sharedApplication(marker);
            let visible = handle
                .get_webview_window("main")
                .and_then(|window| window.is_visible().ok());
            if let Some(state) = handle.try_state::<crate::AppState>() {
                state.diagnostics.record(
                    "info",
                    "macos-lifecycle",
                    format!(
                        "event={event} active={} policy={:?} main_visible={visible:?}",
                        native.isActive(),
                        native.activationPolicy()
                    ),
                    None,
                );
            }
        });
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (app, event);
}

#[cfg(target_os = "macos")]
pub fn apply_macos_app_state(
    app: &tauri::AppHandle,
    event: ManagementWindowEvent,
) -> Result<(), String> {
    use std::sync::atomic::AtomicBool;

    static DOCK_SHOULD_BE_VISIBLE: AtomicBool = AtomicBool::new(true);

    let desired = desired_state(event);
    let visible = desired.dock_visible;
    DOCK_SHOULD_BE_VISIBLE.store(visible, Ordering::Release);

    app.set_activation_policy(if desired.regular_app {
        tauri::ActivationPolicy::Regular
    } else {
        tauri::ActivationPolicy::Accessory
    })
    .map_err(|error| error.to_string())?;

    app.set_dock_visibility(visible)
        .map_err(|error| error.to_string())?;
    if !visible {
        let app = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(1100));
            if !DOCK_SHOULD_BE_VISIBLE.load(Ordering::Acquire) {
                let _ = app.set_dock_visibility(false);
            }
        });
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn apply_macos_app_state(
    _app: &tauri::AppHandle,
    _event: ManagementWindowEvent,
) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        desired_state, handles_window_lifecycle, tray_management_action, MacosReopenState,
        ManagementWindowEvent, ManagementWindowState, TrayManagementAction,
    };

    #[test]
    fn gui_startup_shows_and_focuses_management_window_with_dock() {
        assert_eq!(
            desired_state(ManagementWindowEvent::GuiStartup),
            ManagementWindowState {
                window_visible: true,
                dock_visible: true,
                focus_window: true,
                regular_app: true,
            }
        );
    }

    #[test]
    fn background_startup_keeps_management_window_and_dock_hidden() {
        assert_eq!(
            desired_state(ManagementWindowEvent::BackgroundStartup),
            ManagementWindowState {
                window_visible: false,
                dock_visible: false,
                focus_window: false,
                regular_app: false,
            }
        );
    }

    #[test]
    fn open_request_restores_window_focus_and_dock() {
        assert_eq!(
            desired_state(ManagementWindowEvent::OpenRequested),
            ManagementWindowState {
                window_visible: true,
                dock_visible: true,
                focus_window: true,
                regular_app: true,
            }
        );
    }

    #[test]
    fn close_request_hides_window_and_dock_without_focus() {
        assert_eq!(
            desired_state(ManagementWindowEvent::CloseRequested),
            ManagementWindowState {
                window_visible: false,
                dock_visible: false,
                focus_window: false,
                regular_app: false,
            }
        );
    }

    #[test]
    fn tray_open_actions_share_the_management_window_path() {
        assert_eq!(tray_management_action("open"), TrayManagementAction::Open);
        assert_eq!(
            tray_management_action("check_update"),
            TrayManagementAction::OpenAndCheckUpdate
        );
        assert_eq!(tray_management_action("stop"), TrayManagementAction::None);
    }

    #[test]
    fn only_main_window_controls_management_lifecycle() {
        assert!(handles_window_lifecycle("main"));
        assert!(!handles_window_lifecycle("overlay-monitor-1"));
        assert!(!handles_window_lifecycle("plugin-runtime"));
    }

    #[test]
    fn protocol_playback_suppresses_the_related_macos_reopen() {
        let state = MacosReopenState::new();
        let generation = state.begin_reopen();
        state.suppress_for_protocol_playback();
        assert!(!state.should_restore_window(generation));
    }

    #[test]
    fn recent_protocol_playback_suppresses_a_following_macos_reopen() {
        let state = MacosReopenState::new();
        state.suppress_for_protocol_playback();
        let generation = state.begin_reopen();
        assert!(!state.should_restore_window(generation));
    }

    #[test]
    fn standalone_macos_reopen_restores_the_management_window() {
        let state = MacosReopenState::new();
        let generation = state.begin_reopen();
        assert!(state.should_restore_window(generation));
    }

    #[test]
    fn explicit_open_cancels_pending_restore_and_clears_protocol_suppression() {
        let state = MacosReopenState::new();
        let pending = state.begin_reopen();
        state.suppress_for_protocol_playback();
        state.explicit_open();
        assert!(!state.should_restore_window(pending));
        assert!(state.should_restore_window(state.begin_reopen()));
    }

    #[test]
    fn repeated_playback_cancels_each_pending_reopen() {
        let state = MacosReopenState::new();
        for _ in 0..10 {
            let pending = state.begin_reopen();
            state.suppress_for_protocol_playback();
            assert!(!state.should_restore_window(pending));
        }
    }

    #[test]
    fn late_playback_invalidates_a_scheduled_gui_restore() {
        let state = MacosReopenState::new();
        let startup = state.begin_reopen();
        let reopen = state.begin_reopen();
        state.suppress_for_protocol_playback();
        assert!(!state.should_restore_window(startup));
        assert!(!state.should_restore_window(reopen));
    }
}

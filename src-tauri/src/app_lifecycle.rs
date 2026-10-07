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
}

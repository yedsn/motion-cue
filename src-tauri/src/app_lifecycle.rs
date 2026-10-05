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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayManagementAction {
    None,
    Open,
    OpenAndCheckUpdate,
}

pub const fn desired_state(event: ManagementWindowEvent) -> ManagementWindowState {
    match event {
        ManagementWindowEvent::GuiStartup | ManagementWindowEvent::OpenRequested => {
            ManagementWindowState {
                window_visible: true,
                dock_visible: true,
                focus_window: true,
            }
        }
        ManagementWindowEvent::BackgroundStartup | ManagementWindowEvent::CloseRequested => {
            ManagementWindowState {
                window_visible: false,
                dock_visible: false,
                focus_window: false,
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
pub fn apply_dock_visibility(
    app: &tauri::AppHandle,
    event: ManagementWindowEvent,
) -> Result<(), String> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    static DOCK_SHOULD_BE_VISIBLE: AtomicBool = AtomicBool::new(true);

    let visible = desired_state(event).dock_visible;
    DOCK_SHOULD_BE_VISIBLE.store(visible, Ordering::Release);
    if matches!(event, ManagementWindowEvent::GuiStartup) {
        return Ok(());
    }

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
pub fn apply_dock_visibility(
    _app: &tauri::AppHandle,
    _event: ManagementWindowEvent,
) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        desired_state, handles_window_lifecycle, tray_management_action, ManagementWindowEvent,
        ManagementWindowState, TrayManagementAction,
    };

    #[test]
    fn gui_startup_shows_and_focuses_management_window_with_dock() {
        assert_eq!(
            desired_state(ManagementWindowEvent::GuiStartup),
            ManagementWindowState {
                window_visible: true,
                dock_visible: true,
                focus_window: true,
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
}

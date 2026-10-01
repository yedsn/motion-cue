#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    hide_console_for_desktop_launch();
    motion_cue_lib::entry();
}

#[cfg(target_os = "windows")]
fn hide_console_for_desktop_launch() {
    let first = std::env::args().nth(1);
    let keep_console = matches!(
        first.as_deref(),
        Some(
            "play"
                | "list"
                | "stop"
                | "open"
                | "import-animation"
                | "install-plugin"
                | "enable-plugin"
                | "disable-plugin"
                | "uninstall-plugin"
                | "diagnostics"
        )
    ) || first
        .as_deref()
        .is_some_and(|value| value.starts_with('-') && value != "--background");
    if keep_console {
        return;
    }
    use windows_sys::Win32::System::Console::GetConsoleWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
    let window = unsafe { GetConsoleWindow() };
    if !window.is_null() {
        unsafe { ShowWindow(window, SW_HIDE) };
    }
}

#[cfg(not(target_os = "windows"))]
fn hide_console_for_desktop_launch() {}

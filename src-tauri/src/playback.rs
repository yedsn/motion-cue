use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use tauri::{
    utils::config::BackgroundThrottlingPolicy, AppHandle, Emitter, Manager, PhysicalPosition,
    PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};
use uuid::Uuid;

use crate::catalog::{resolve, validate_animation};
use crate::diagnostics::Diagnostics;
use crate::models::{now_ms, AnimationDefinition, MonitorTarget, PlaybackRequest};
use crate::settings::ConfigStore;

#[derive(Clone)]
struct ActiveSession {
    id: String,
    fingerprint: String,
    started_at: u64,
    request: PlaybackRequest,
    monitor_signature: Vec<String>,
    audio_owner_label: String,
}

#[derive(Clone, Debug, PartialEq)]
struct MonitorDescriptor {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    scale_factor: f64,
    primary: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct OverlayPlan {
    key: String,
    label: String,
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
}

pub struct PlaybackCoordinator {
    active: Mutex<Option<ActiveSession>>,
    overlays: Mutex<HashMap<String, String>>,
}

impl PlaybackCoordinator {
    pub fn new() -> Self {
        Self {
            active: Mutex::new(None),
            overlays: Mutex::new(HashMap::new()),
        }
    }

    pub fn play(
        &self,
        app: &AppHandle,
        store: &ConfigStore,
        diagnostics: &Diagnostics,
        command: &str,
        params: BTreeMap<String, String>,
    ) -> Result<(), String> {
        let config = store.get();
        let animation = resolve(&config, command)?;
        self.play_animation(app, diagnostics, &config, command, animation, params)
    }

    pub fn preview(
        &self,
        app: &AppHandle,
        store: &ConfigStore,
        diagnostics: &Diagnostics,
        animation: AnimationDefinition,
        params: BTreeMap<String, String>,
    ) -> Result<(), String> {
        validate_animation(&animation)?;
        let command = animation.command.clone();
        let config = store.get();
        self.play_animation(app, diagnostics, &config, &command, animation, params)
    }

    fn play_animation(
        &self,
        app: &AppHandle,
        diagnostics: &Diagnostics,
        config: &crate::models::AppConfig,
        command: &str,
        mut animation: AnimationDefinition,
        params: BTreeMap<String, String>,
    ) -> Result<(), String> {
        apply_overrides(&mut animation, &params)?;
        let fingerprint = serde_json::to_string(&(animation.id.as_str(), &params))
            .map_err(|error| error.to_string())?;
        {
            let active = self.active.lock().unwrap();
            if active.as_ref().is_some_and(|active| {
                active.fingerprint == fingerprint
                    && now_ms().saturating_sub(active.started_at) <= 300
            }) {
                diagnostics.record("info", "invocation", "重复调用已合并", Some(command));
                return Ok(());
            }
        }
        self.stop(app, diagnostics, "replaced")?;
        let session_id = Uuid::new_v4().to_string();
        let started_at = now_ms();
        let labels =
            self.prepare_overlays(app, &animation.target, config.settings.overlay_topmost)?;
        let initial_monitor_signature = monitor_signature(app)?;
        let audio_data_url = animation
            .audio
            .resource_id
            .as_ref()
            .and_then(|resource_id| load_audio_data_url(app, resource_id));
        let request = PlaybackRequest {
            session_id: session_id.clone(),
            animation: animation.clone(),
            params,
            started_at,
            play_audio: animation.audio.enabled && !config.settings.muted,
            target_frame_rate: config.settings.target_frame_rate,
            transition: animation
                .transition
                .clone()
                .unwrap_or_else(|| config.settings.default_transition.clone()),
            audio_data_url,
        };
        let audio_owner_label = labels.first().cloned().unwrap_or_default();
        *self.active.lock().unwrap() = Some(ActiveSession {
            id: session_id.clone(),
            fingerprint,
            started_at,
            request: request.clone(),
            monitor_signature: initial_monitor_signature,
            audio_owner_label,
        });
        for (index, label) in labels.into_iter().enumerate() {
            if let Some(window) = app.get_webview_window(&label) {
                window
                    .show()
                    .map_err(|error| format!("显示覆盖窗口失败: {error}"))?;
                apply_overlay_runtime_flags(&window, config.settings.overlay_topmost)?;
                let mut window_request = request.clone();
                window_request.play_audio = request.play_audio && index == 0;
                if index > 0 {
                    window_request.audio_data_url = None;
                }
                let _ = window.emit("motioncue://playback", &window_request);
                let retry_window = window.clone();
                let retry_request = window_request.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    for delay in [150, 450, 900, 1500] {
                        std::thread::sleep(std::time::Duration::from_millis(delay));
                        let _ = retry_window.emit("motioncue://playback", &retry_request);
                    }
                });
            }
        }
        diagnostics.record("info", "playback", "动画开始播放", Some(command));
        let app_handle = app.clone();
        let timeout_session = session_id.clone();
        let timeout = session_timeout_ms(animation.duration_ms);
        tauri::async_runtime::spawn_blocking(move || {
            std::thread::sleep(std::time::Duration::from_millis(timeout));
            if let Some(coordinator) = app_handle.try_state::<PlaybackCoordinator>() {
                let should_stop = coordinator
                    .active
                    .lock()
                    .unwrap()
                    .as_ref()
                    .is_some_and(|active| active.id == timeout_session);
                if should_stop {
                    let diagnostics = app_handle.state::<Diagnostics>();
                    let _ = coordinator.stop(&app_handle, &diagnostics, "timeout");
                }
            }
        });
        let app_handle = app.clone();
        let topology_session = session_id.clone();
        tauri::async_runtime::spawn_blocking(move || loop {
            std::thread::sleep(std::time::Duration::from_millis(750));
            let Some(coordinator) = app_handle.try_state::<PlaybackCoordinator>() else {
                break;
            };
            let expected = coordinator
                .active
                .lock()
                .unwrap()
                .as_ref()
                .and_then(|active| {
                    (active.id == topology_session).then(|| active.monitor_signature.clone())
                });
            let Some(expected) = expected else { break };
            let current = monitor_signature(&app_handle).unwrap_or_default();
            if current != expected {
                let diagnostics = app_handle.state::<Diagnostics>();
                diagnostics.record(
                    "warning",
                    "monitor",
                    "播放期间显示器配置变化，已安全结束当前动画",
                    None,
                );
                let _ = coordinator.stop(&app_handle, &diagnostics, "monitor-topology-changed");
                break;
            }
        });
        Ok(())
    }

    pub fn stop(
        &self,
        app: &AppHandle,
        diagnostics: &Diagnostics,
        reason: &str,
    ) -> Result<(), String> {
        let session = self.active.lock().unwrap().take();
        if let Some(session) = session {
            let labels = self
                .overlays
                .lock()
                .unwrap()
                .values()
                .cloned()
                .collect::<Vec<_>>();
            for label in labels {
                if let Some(window) = app.get_webview_window(&label) {
                    let _ = window.emit(
                        "motioncue://stop",
                        serde_json::json!({ "sessionId": session.id }),
                    );
                    let _ = window.hide();
                }
            }
            diagnostics.record(
                "info",
                "playback",
                format!("动画会话已停止: {reason}"),
                None,
            );
        }
        Ok(())
    }

    pub fn complete(
        &self,
        app: &AppHandle,
        diagnostics: &Diagnostics,
        session_id: &str,
    ) -> Result<(), String> {
        let is_current = self
            .active
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|active| active.id == session_id);
        if is_current {
            self.stop(app, diagnostics, "complete")?;
        }
        Ok(())
    }

    pub fn current(&self) -> Option<PlaybackRequest> {
        self.active
            .lock()
            .unwrap()
            .as_ref()
            .map(|active| active.request.clone())
    }

    pub fn current_for_window(&self, label: &str) -> Option<PlaybackRequest> {
        self.active.lock().unwrap().as_ref().map(|active| {
            let mut request = active.request.clone();
            request.play_audio = request.play_audio && active.audio_owner_label == label;
            if !request.play_audio {
                request.audio_data_url = None;
            }
            request
        })
    }

    fn prepare_overlays(
        &self,
        app: &AppHandle,
        target: &MonitorTarget,
        overlay_topmost: bool,
    ) -> Result<Vec<String>, String> {
        let monitors: Vec<MonitorDescriptor> = match target {
            MonitorTarget::Primary => app
                .primary_monitor()
                .map_err(|error| format!("读取主显示器失败: {error}"))?
                .into_iter()
                .map(|monitor| MonitorDescriptor {
                    x: monitor.position().x,
                    y: monitor.position().y,
                    width: monitor.size().width,
                    height: monitor.size().height,
                    scale_factor: monitor.scale_factor(),
                    primary: true,
                })
                .collect(),
            MonitorTarget::All => app
                .available_monitors()
                .map_err(|error| format!("读取显示器失败: {error}"))?
                .into_iter()
                .map(|monitor| MonitorDescriptor {
                    x: monitor.position().x,
                    y: monitor.position().y,
                    width: monitor.size().width,
                    height: monitor.size().height,
                    scale_factor: monitor.scale_factor(),
                    primary: false,
                })
                .collect(),
        };
        let existing = self.overlays.lock().unwrap().clone();
        let plans = plan_overlays(&monitors, target, &existing)?;
        let active_keys = plans
            .iter()
            .map(|plan| plan.key.clone())
            .collect::<Vec<_>>();
        let mut labels = Vec::new();
        for plan in plans {
            let window =
                get_or_build_overlay(app, &plan.label, plan.position, plan.size, overlay_topmost)?;
            window
                .set_position(plan.position)
                .map_err(|error| format!("设置覆盖位置失败: {error}"))?;
            window
                .set_size(plan.size)
                .map_err(|error| format!("设置覆盖尺寸失败: {error}"))?;
            let _ = window.set_always_on_top(overlay_topmost);
            let _ = window.set_skip_taskbar(true);
            window
                .set_focusable(false)
                .map_err(|error| format!("禁用覆盖窗口焦点失败: {error}"))?;
            if let Err(error) = apply_overlay_runtime_flags(&window, overlay_topmost) {
                let _ = window.hide();
                return Err(error);
            }
            self.overlays
                .lock()
                .unwrap()
                .insert(plan.key, plan.label.clone());
            labels.push(plan.label);
        }
        let stale = self
            .overlays
            .lock()
            .unwrap()
            .iter()
            .filter(|(key, _)| !active_keys.contains(key))
            .map(|(_, label)| label.clone())
            .collect::<Vec<_>>();
        for label in stale {
            if let Some(window) = app.get_webview_window(&label) {
                let _ = window.close();
            }
            self.overlays
                .lock()
                .unwrap()
                .retain(|_, value| value != &label);
        }
        Ok(labels)
    }
}

fn apply_overlay_runtime_flags(
    window: &WebviewWindow,
    overlay_topmost: bool,
) -> Result<(), String> {
    if let Err(error) = window.set_ignore_cursor_events(true) {
        return Err(format!("启用鼠标穿透失败，覆盖窗口已隐藏: {error}"));
    }
    let _ = window.set_always_on_top(overlay_topmost);
    let _ = window.set_skip_taskbar(true);
    let _ = window.set_focusable(false);
    #[cfg(target_os = "windows")]
    apply_native_click_through(window)
        .map_err(|error| format!("设置 Windows 鼠标穿透样式失败，覆盖窗口已隐藏: {error}"))?;
    #[cfg(target_os = "windows")]
    apply_native_topmost(window, overlay_topmost)
        .map_err(|error| format!("设置 Windows 覆盖窗口层级失败，覆盖窗口已隐藏: {error}"))?;
    Ok(())
}

fn get_or_build_overlay(
    app: &AppHandle,
    label: &str,
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    overlay_topmost: bool,
) -> Result<WebviewWindow, String> {
    for attempt in 0..8 {
        if let Some(window) = app.get_webview_window(label) {
            return Ok(window);
        }
        match build_overlay(app, label, position, size, overlay_topmost) {
            Ok(window) => return Ok(window),
            Err(error) if error.contains("already exists") && attempt < 7 => {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(error) => return Err(error),
        }
    }
    Err(format!("覆盖窗口未能及时就绪: {label}"))
}

fn plan_overlays(
    monitors: &[MonitorDescriptor],
    target: &MonitorTarget,
    existing: &HashMap<String, String>,
) -> Result<Vec<OverlayPlan>, String> {
    let selected = match target {
        MonitorTarget::All => monitors.iter().collect::<Vec<_>>(),
        MonitorTarget::Primary => monitors
            .iter()
            .find(|monitor| monitor.primary)
            .or_else(|| monitors.first())
            .into_iter()
            .collect::<Vec<_>>(),
    };
    if selected.is_empty() {
        return Err("未找到可用显示器".into());
    }
    Ok(selected
        .into_iter()
        .map(|monitor| {
            let key = monitor_key(monitor);
            let label = existing
                .get(&key)
                .cloned()
                .unwrap_or_else(|| format!("overlay-{}", sanitize_label(&key)));
            OverlayPlan {
                key,
                label,
                position: PhysicalPosition::new(monitor.x, monitor.y),
                size: PhysicalSize::new(monitor.width, monitor.height),
            }
        })
        .collect())
}

fn monitor_key(monitor: &MonitorDescriptor) -> String {
    format!(
        "{}:{}:{}:{}",
        monitor.x, monitor.y, monitor.width, monitor.height
    )
}

fn session_timeout_ms(duration_ms: u64) -> u64 {
    duration_ms.saturating_add(5_000).clamp(10_000, 60_000)
}

#[cfg(test)]
fn monitor_signature_from_descriptors(monitors: &[MonitorDescriptor]) -> Vec<String> {
    let mut signature = monitors
        .iter()
        .map(|monitor| {
            format!(
                "{}:{}:{}:{}:{}",
                monitor.x, monitor.y, monitor.width, monitor.height, monitor.scale_factor
            )
        })
        .collect::<Vec<_>>();
    signature.sort();
    signature
}

#[cfg(test)]
fn assign_audio_to_window(request: &mut PlaybackRequest, audio_owner_label: &str, label: &str) {
    request.play_audio = request.play_audio && audio_owner_label == label;
    if !request.play_audio {
        request.audio_data_url = None;
    }
}

fn monitor_signature(app: &AppHandle) -> Result<Vec<String>, String> {
    let mut signature = app
        .available_monitors()
        .map_err(|error| format!("读取显示器失败: {error}"))?
        .into_iter()
        .map(|monitor| {
            let position = monitor.position();
            let size = monitor.size();
            format!(
                "{}:{}:{}:{}:{}",
                position.x,
                position.y,
                size.width,
                size.height,
                monitor.scale_factor()
            )
        })
        .collect::<Vec<_>>();
    signature.sort();
    Ok(signature)
}

fn sanitize_label(key: &str) -> String {
    key.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn build_overlay(
    app: &AppHandle,
    label: &str,
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    overlay_topmost: bool,
) -> Result<WebviewWindow, String> {
    WebviewWindowBuilder::new(app, label, WebviewUrl::App("/?view=overlay".into()))
        .title("MotionCue Overlay")
        .background_throttling(BackgroundThrottlingPolicy::Disabled)
        .position(position.x as f64, position.y as f64)
        .inner_size(size.width as f64, size.height as f64)
        .decorations(false)
        .transparent(true)
        .always_on_top(overlay_topmost)
        .skip_taskbar(true)
        .focused(false)
        .visible(false)
        .shadow(false)
        .build()
        .map_err(|error| format!("创建覆盖窗口失败: {error}"))
}

#[cfg(target_os = "windows")]
fn apply_native_click_through(window: &WebviewWindow) -> Result<(), String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
        WS_EX_TRANSPARENT,
    };

    let hwnd = window.hwnd().map_err(|error| error.to_string())?;
    let style = unsafe { GetWindowLongPtrW(hwnd.0, GWL_EXSTYLE) };
    if style == 0 {
        return Err("读取覆盖窗口扩展样式失败".into());
    }
    let next =
        style | WS_EX_TRANSPARENT as isize | WS_EX_NOACTIVATE as isize | WS_EX_TOOLWINDOW as isize;
    unsafe { windows_sys::Win32::Foundation::SetLastError(0) };
    let applied = unsafe { SetWindowLongPtrW(hwnd.0, GWL_EXSTYLE, next) };
    if applied == 0 && unsafe { windows_sys::Win32::Foundation::GetLastError() } != 0 {
        return Err("写入覆盖窗口扩展样式失败".into());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn apply_native_topmost(window: &WebviewWindow, overlay_topmost: bool) -> Result<(), String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE, SWP_NOMOVE,
        SWP_NOSIZE,
    };

    let hwnd = window.hwnd().map_err(|error| error.to_string())?;
    let insert_after = if overlay_topmost {
        HWND_TOPMOST
    } else {
        HWND_NOTOPMOST
    };
    let ok = unsafe {
        SetWindowPos(
            hwnd.0,
            insert_after,
            0,
            0,
            0,
            0,
            SWP_ASYNCWINDOWPOS | SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
    };
    if ok == 0 {
        return Err("写入覆盖窗口层级失败".into());
    }
    Ok(())
}

fn apply_overrides(
    animation: &mut AnimationDefinition,
    params: &BTreeMap<String, String>,
) -> Result<(), String> {
    for key in params.keys() {
        if !matches!(key.as_str(), "text" | "color" | "duration") {
            return Err(format!("动画未声明参数: {key}"));
        }
    }
    if let Some(text) = params.get("text") {
        if text.chars().count() > 200 {
            return Err("文字参数超过 200 个字符".into());
        }
        animation.text = Some(text.clone());
    }
    if let Some(color) = params.get("color") {
        if color.len() != 7
            || !color.starts_with('#')
            || !color[1..]
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        {
            return Err("颜色参数必须为 #RRGGBB".into());
        }
        animation.colors = vec![color.clone()];
    }
    if let Some(duration) = params.get("duration") {
        let duration = duration
            .parse::<u64>()
            .map_err(|_| "duration 必须是毫秒整数".to_string())?;
        if !(100..=60_000).contains(&duration) {
            return Err("duration 必须在 100 到 60000 毫秒之间".into());
        }
        animation.duration_ms = duration;
    }
    if let Some(transition) = &animation.transition {
        crate::catalog::validate_transition(
            transition.enter_ms,
            transition.exit_ms,
            animation.duration_ms,
        )?;
    }
    Ok(())
}

fn load_audio_data_url(app: &AppHandle, resource_id: &str) -> Option<String> {
    crate::audio::resource_data_url(app, resource_id).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_playback_overrides() {
        let mut animation = crate::catalog::builtins().remove(0);
        apply_overrides(
            &mut animation,
            &BTreeMap::from([
                ("text".into(), "Done".into()),
                ("duration".into(), "5000".into()),
            ]),
        )
        .unwrap();
        assert_eq!(animation.text.as_deref(), Some("Done"));
        assert_eq!(animation.duration_ms, 5000);
    }

    #[test]
    fn rejects_unknown_and_invalid_overrides() {
        let mut animation = crate::catalog::builtins().remove(0);
        assert!(apply_overrides(
            &mut animation,
            &BTreeMap::from([("script".into(), "bad".into())])
        )
        .is_err());
        assert!(apply_overrides(
            &mut animation,
            &BTreeMap::from([("color".into(), "red".into())])
        )
        .is_err());
    }

    #[test]
    fn plans_all_mixed_dpi_monitors_with_physical_bounds() {
        let monitors = vec![
            MonitorDescriptor {
                x: 0,
                y: 0,
                width: 2560,
                height: 1440,
                scale_factor: 1.5,
                primary: true,
            },
            MonitorDescriptor {
                x: -1920,
                y: 120,
                width: 1920,
                height: 1080,
                scale_factor: 1.0,
                primary: false,
            },
        ];
        let plans = plan_overlays(&monitors, &MonitorTarget::All, &HashMap::new()).unwrap();
        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].position, PhysicalPosition::new(0, 0));
        assert_eq!(plans[0].size, PhysicalSize::new(2560, 1440));
        assert_eq!(plans[1].position, PhysicalPosition::new(-1920, 120));
        assert_eq!(plans[1].size, PhysicalSize::new(1920, 1080));
    }

    #[test]
    fn plans_primary_monitor_only_and_reuses_labels() {
        let monitors = vec![
            MonitorDescriptor {
                x: -1920,
                y: 0,
                width: 1920,
                height: 1080,
                scale_factor: 1.0,
                primary: false,
            },
            MonitorDescriptor {
                x: 0,
                y: 0,
                width: 2560,
                height: 1440,
                scale_factor: 1.25,
                primary: true,
            },
        ];
        let key = "0:0:2560:1440".to_string();
        let existing = HashMap::from([(key, "overlay-existing".to_string())]);
        let plans = plan_overlays(&monitors, &MonitorTarget::Primary, &existing).unwrap();
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].label, "overlay-existing");
        assert_eq!(plans[0].position, PhysicalPosition::new(0, 0));
    }

    #[test]
    fn monitor_plan_keys_identify_topology_changes() {
        let before = MonitorDescriptor {
            x: 0,
            y: 0,
            width: 2560,
            height: 1440,
            scale_factor: 1.0,
            primary: true,
        };
        let after = MonitorDescriptor {
            width: 1920,
            height: 1080,
            ..before.clone()
        };
        assert_ne!(monitor_key(&before), monitor_key(&after));
    }

    #[test]
    fn monitor_signatures_detect_hotplug_scale_and_position_changes() {
        let before = vec![
            MonitorDescriptor {
                x: 0,
                y: 0,
                width: 2560,
                height: 1440,
                scale_factor: 1.25,
                primary: true,
            },
            MonitorDescriptor {
                x: 2560,
                y: 0,
                width: 1920,
                height: 1080,
                scale_factor: 1.0,
                primary: false,
            },
        ];
        let mut hotplugged = before.clone();
        hotplugged.pop();
        let mut scaled = before.clone();
        scaled[0].scale_factor = 1.5;
        let mut moved = before.clone();
        moved[1].x = -1920;

        let signature = monitor_signature_from_descriptors(&before);
        assert_ne!(signature, monitor_signature_from_descriptors(&hotplugged));
        assert_ne!(signature, monitor_signature_from_descriptors(&scaled));
        assert_ne!(signature, monitor_signature_from_descriptors(&moved));
    }

    #[test]
    fn assigns_audio_to_one_overlay_only() {
        let mut owner = PlaybackRequest {
            session_id: "session".into(),
            animation: crate::catalog::builtins()
                .into_iter()
                .find(|animation| animation.id == "task-complete")
                .unwrap(),
            params: BTreeMap::new(),
            started_at: 1,
            play_audio: true,
            target_frame_rate: 60,
            transition: crate::models::default_transition(),
            audio_data_url: Some("data:audio/wav;base64,AA==".into()),
        };
        let mut secondary = owner.clone();

        assign_audio_to_window(&mut owner, "overlay-primary", "overlay-primary");
        assign_audio_to_window(&mut secondary, "overlay-primary", "overlay-secondary");

        assert!(owner.play_audio);
        assert!(owner.audio_data_url.is_some());
        assert!(!secondary.play_audio);
        assert!(secondary.audio_data_url.is_none());
    }

    #[test]
    fn session_timeout_has_safety_bounds() {
        assert_eq!(session_timeout_ms(100), 10_000);
        assert_eq!(session_timeout_ms(3_000), 10_000);
        assert_eq!(session_timeout_ms(30_000), 35_000);
        assert_eq!(session_timeout_ms(60_000), 60_000);
        assert_eq!(session_timeout_ms(u64::MAX), 60_000);
    }
}

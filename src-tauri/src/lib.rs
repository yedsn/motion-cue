use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::menu::{CheckMenuItem, Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_updater::UpdaterExt;

mod app_lifecycle;
mod audio;
mod catalog;
mod diagnostics;
mod invocation;
mod models;
mod playback;
mod plugins;
mod settings;

use app_lifecycle::ManagementWindowEvent;
use catalog::public_command_list;
use diagnostics::Diagnostics;
use invocation::{
    parse_deep_link, parse_launch_args, start_server, IpcCommand, IpcResponse, LaunchIntent,
};
use models::{AppConfig, PluginRuntime};
use playback::PlaybackCoordinator;
use settings::ConfigStore;

pub struct AppState {
    pub config: ConfigStore,
    pub diagnostics: Diagnostics,
    pub playback: PlaybackCoordinator,
    pub plugins_root: PathBuf,
    pub macos_reopen: app_lifecycle::MacosReopenState,
}

pub struct TrayState {
    pub mute_item: CheckMenuItem<tauri::Wry>,
}

const APP_UPDATE_EVENT: &str = "app-update-event";
const DEFAULT_UPDATER_PUBKEY: &str = "REPLACE_WITH_TAURI_UPDATER_PUBLIC_KEY";
static UPDATE_TASK_RUNNING: AtomicBool = AtomicBool::new(false);

struct UpdateTaskGuard;

impl UpdateTaskGuard {
    fn acquire() -> Result<Self, String> {
        UPDATE_TASK_RUNNING
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .map(|_| Self)
            .map_err(|_| "已有更新任务正在进行，请稍后再试".to_string())
    }
}

impl Drop for UpdateTaskGuard {
    fn drop(&mut self) {
        UPDATE_TASK_RUNNING.store(false, Ordering::Release);
    }
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AppUpdateSummary {
    version: String,
    current_version: String,
    notes: Option<String>,
    pub_date: Option<String>,
    target: String,
    download_url: String,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AppUpdateCheckResult {
    available: bool,
    current_version: String,
    update: Option<AppUpdateSummary>,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AppUpdateEventPayload {
    stage: String,
    downloaded_bytes: Option<u64>,
    chunk_length: Option<u64>,
    content_length: Option<u64>,
    message: Option<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdaterPluginRuntimeConfig {
    #[serde(default)]
    endpoints: Vec<String>,
    #[serde(default)]
    pubkey: String,
}

pub fn entry() {
    configure_webview_rendering();
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let intent = match parse_launch_args(&args) {
        Ok(intent) => intent,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    if let LaunchIntent::Forward(command) = intent.clone() {
        let endpoint = cli_endpoint_path();
        let response = forward_or_start(&endpoint, command);
        match response {
            Ok(response) => {
                if let Some(data) = response.data {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&data).unwrap_or_default()
                    );
                } else {
                    println!("{}", response.message);
                }
                if !response.ok {
                    std::process::exit(1);
                }
                return;
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
    }
    let _ = run(intent);
}

fn configure_webview_rendering() {
    #[cfg(target_os = "windows")]
    if std::env::var_os("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").is_none() {
        std::env::set_var(
            "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
            "--enable-gpu --ignore-gpu-blocklist --disable-background-timer-throttling --disable-renderer-backgrounding --disable-backgrounding-occluded-windows",
        );
    }
}

fn cli_endpoint_path() -> PathBuf {
    let app_data = cli_app_data_dir();
    invocation::endpoint_path(&app_data)
}

#[cfg(target_os = "windows")]
fn cli_app_data_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("com.motioncue.desktop")
}

#[cfg(not(target_os = "windows"))]
fn cli_app_data_dir() -> PathBuf {
    directories::ProjectDirs::from("", "", "com.motioncue.desktop")
        .map(|directories| directories.data_dir().to_path_buf())
        .unwrap_or_else(|| std::env::temp_dir().join("com.motioncue.desktop"))
}

fn forward_or_start(
    endpoint: &std::path::Path,
    command: IpcCommand,
) -> Result<IpcResponse, String> {
    if let Ok(response) =
        invocation::forward(endpoint, command.clone(), std::time::Duration::from_secs(8))
    {
        return Ok(response);
    }
    let executable = std::env::current_exe()
        .map_err(|error| format!("无法定位 MotionCue 可执行程序: {error}"))?;
    let mut process = Command::new(executable);
    process
        .arg("--background")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        process.creation_flags(0x08000000);
    }
    process
        .spawn()
        .map_err(|error| format!("启动 MotionCue 后台实例失败: {error}"))?;
    invocation::wait_and_forward(endpoint, command)
}

fn run(initial_intent: LaunchIntent) -> Result<(), String> {
    let initial_intent = Arc::new(std::sync::Mutex::new(Some(initial_intent)));
    let initial_intent_for_instance = initial_intent.clone();
    let builder = tauri::Builder::default();
    #[cfg(target_os = "macos")]
    let builder = builder.activate_ignoring_other_apps(false);
    let app = builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(
            move |app, args, _cwd| {
                let command = match parse_launch_args(&args[1..]) {
                    Ok(intent) => match command_for_secondary_launch(intent) {
                        Some(command) => command,
                        None => return,
                    },
                    Err(error) => {
                        app.state::<Diagnostics>()
                            .record("error", "invocation", error, None);
                        return;
                    }
                };
                let _ = handle_ipc(app, command);
                let _ = app.emit(
                    "motioncue://invocation",
                    serde_json::json!({ "source": "single-instance" }),
                );
            },
        ))
        .setup(move |app| {
            let initial_intent = initial_intent_for_instance.lock().unwrap().take();
            if matches!(initial_intent, Some(LaunchIntent::Background)) {
                let _ = app_lifecycle::apply_macos_app_state(
                    app.handle(),
                    ManagementWindowEvent::BackgroundStartup,
                );
            }
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| format!("读取应用数据目录失败: {error}"))?;
            audio::seed_library(app.handle())?;
            let config_path = data_dir.join("config.json");
            let (config, recovery) = ConfigStore::load(config_path)?;
            let diagnostics = Diagnostics::new(
                data_dir.join("diagnostics.jsonl"),
                config.get().settings.diagnostics_retention,
            );
            if let Some(message) = recovery {
                diagnostics.record("warning", "config", message, None);
            }
            let plugins_root = data_dir.join("plugins");
            std::fs::create_dir_all(&plugins_root)
                .map_err(|error| format!("创建插件目录失败: {error}"))?;
            app.manage(AppState {
                config,
                diagnostics,
                playback: PlaybackCoordinator::new(),
                plugins_root,
                macos_reopen: app_lifecycle::MacosReopenState::new(),
            });
            let endpoint = invocation::endpoint_path(&data_dir);
            let app_handle = app.handle().clone();
            start_server(app.handle().clone(), endpoint, handle_ipc)?;
            setup_tray(app)?;
            app_lifecycle::register_macos_link_receiver(app.handle());
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            if let Err(error) = app.deep_link().register_all() {
                app.state::<AppState>().diagnostics.record(
                    "warning",
                    "invocation",
                    format!("注册自定义链接失败: {error}"),
                    None,
                );
            }
            let deep_link_app = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                app_lifecycle::record_macos_event(&deep_link_app, "protocol-received");
                let requests = event
                    .urls()
                    .into_iter()
                    .map(|url| parse_deep_link(url.as_str(), "protocol"))
                    .collect::<Vec<_>>();
                if requests.iter().any(Result::is_ok) {
                    deep_link_app
                        .state::<AppState>()
                        .macos_reopen
                        .suppress_for_protocol_playback();
                }
                for request in requests {
                    match request {
                        Ok(request) => {
                            let playback_app = deep_link_app.clone();
                            tauri::async_runtime::spawn_blocking(move || {
                                let _ =
                                    play_request(&playback_app, request.command, request.params);
                            });
                        }
                        Err(error) => deep_link_app.state::<AppState>().diagnostics.record(
                            "error",
                            "invocation",
                            error,
                            None,
                        ),
                    }
                }
            });
            if let Some(intent) = initial_intent {
                match intent {
                    LaunchIntent::Forward(command) => {
                        let _ = handle_ipc(&app_handle, command);
                    }
                    LaunchIntent::Background => {
                        if let Err(error) = hide_management_window(
                            &app_handle,
                            ManagementWindowEvent::BackgroundStartup,
                        ) {
                            record_lifecycle_warning(&app_handle, error);
                        }
                    }
                    LaunchIntent::Gui => {
                        open_gui_startup(&app_handle);
                    }
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            config_get,
            settings_update,
            animation_save,
            animation_delete,
            animation_reset,
            animation_play,
            animation_preview,
            animation_stop_all,
            animation_export,
            animation_import,
            audio_import,
            audio_library_list,
            audio_resource,
            diagnostics_get,
            diagnostics_record,
            plugins_get,
            plugin_install,
            plugin_set_enabled,
            plugin_uninstall,
            plugin_runtime_get,
            plugin_sound,
            playback_current,
            playback_complete,
            playback_error,
            get_app_version,
            check_app_update,
            download_and_install_update,
            restart_app,
            app_open
        ])
        .on_window_event(|window, event| {
            if app_lifecycle::handles_window_lifecycle(window.label()) {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let app = window.app_handle().clone();
                    if let Err(error) =
                        hide_management_window(&app, ManagementWindowEvent::CloseRequested)
                    {
                        record_lifecycle_warning(&app, error);
                    }
                }
            }
        })
        .build(tauri::generate_context!())
        .map_err(|error| format!("MotionCue 运行失败: {error}"))?;
    #[cfg(target_os = "macos")]
    let mut app = app;
    #[cfg(target_os = "macos")]
    {
        app.set_activation_policy(tauri::ActivationPolicy::Accessory);
        app.set_dock_visibility(false);
    }
    app.run(|_app, _event| {
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = _event {
            app_lifecycle::record_macos_event(_app, "reopen-received");
            let app = _app.clone();
            let generation = app.state::<AppState>().macos_reopen.begin_reopen();
            std::thread::spawn(move || {
                std::thread::sleep(app_lifecycle::macos_reopen_delay());
                if app
                    .state::<AppState>()
                    .macos_reopen
                    .should_restore_window(generation)
                {
                    if let Err(error) =
                        open_management_window(&app, ManagementWindowEvent::OpenRequested)
                    {
                        record_lifecycle_warning(&app, error);
                    }
                }
            });
        }
    });
    Ok(())
}

fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let open = MenuItem::with_id(app, "open", "打开 MotionCue", true, None::<&str>)?;
    let check_update = MenuItem::with_id(app, "check_update", "检查更新", true, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop", "停止全部动画", true, None::<&str>)?;
    let mute = CheckMenuItem::with_id(
        app,
        "mute",
        "全局静音",
        true,
        app.state::<AppState>().config.get().settings.muted,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &check_update, &stop, &mute, &quit])?;
    app.manage(TrayState {
        mute_item: mute.clone(),
    });
    let tray = TrayIconBuilder::with_id("motioncue")
        .icon(
            app.default_window_icon()
                .cloned()
                .ok_or("默认窗口图标不可用")?,
        )
        .menu(&menu)
        .tooltip("MotionCue");
    tray.on_menu_event(|app, event| {
        match app_lifecycle::tray_management_action(event.id.as_ref()) {
            app_lifecycle::TrayManagementAction::Open => {
                let _ = app_open(app.clone());
            }
            app_lifecycle::TrayManagementAction::OpenAndCheckUpdate => {
                let _ = app_open(app.clone());
                let _ = app.emit("motioncue://check-update", serde_json::json!({}));
            }
            app_lifecycle::TrayManagementAction::None => match event.id.as_ref() {
                "stop" => {
                    let _ = animation_stop_all(app.clone());
                }
                "mute" => {
                    let current = app.state::<AppState>().config.get();
                    if let Ok(config) = settings_update(
                        app.clone(),
                        models::GlobalSettings {
                            muted: !current.settings.muted,
                            ..current.settings
                        },
                    ) {
                        sync_tray_mute_checked(app, config.settings.muted);
                    }
                }
                "quit" => {
                    let _ = app.exit(0);
                }
                _ => {}
            },
        }
    })
    .on_tray_icon_event(|tray, event| {
        if matches!(
            event,
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } | TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            }
        ) {
            let _ = app_open(tray.app_handle().clone());
        }
    })
    .build(app)?;
    Ok(())
}

fn sync_tray_mute_checked(app: &AppHandle, muted: bool) {
    if let Some(state) = app.try_state::<TrayState>() {
        let _ = state.mute_item.set_checked(muted);
    }
}

fn handle_ipc(app: &AppHandle, command: IpcCommand) -> IpcResponse {
    match command {
        IpcCommand::Play { request } => {
            app.state::<AppState>()
                .macos_reopen
                .suppress_for_protocol_playback();
            match play_request(app, request.command.clone(), request.params) {
                Ok(()) => IpcResponse {
                    ok: true,
                    message: "动画已提交".into(),
                    data: None,
                },
                Err(error) => {
                    app.state::<AppState>().diagnostics.record(
                        "error",
                        "invocation",
                        &error,
                        Some(&request.command),
                    );
                    IpcResponse {
                        ok: false,
                        message: error,
                        data: None,
                    }
                }
            }
        }
        IpcCommand::List => IpcResponse {
            ok: true,
            message: "ok".into(),
            data: Some(serde_json::json!(public_command_list(
                &app.state::<AppState>().config.get()
            ))),
        },
        IpcCommand::Stop => IpcResponse {
            ok: animation_stop_all(app.clone()).is_ok(),
            message: "已停止全部动画".into(),
            data: None,
        },
        IpcCommand::Open => IpcResponse {
            ok: app_open(app.clone()).is_ok(),
            message: "已打开管理界面".into(),
            data: None,
        },
        IpcCommand::ImportAnimation { path } => match import_animation_path(app.clone(), path) {
            Ok(config) => IpcResponse {
                ok: true,
                message: "动画包已导入".into(),
                data: Some(serde_json::json!(config)),
            },
            Err(error) => IpcResponse {
                ok: false,
                message: error,
                data: None,
            },
        },
        IpcCommand::InstallPlugin {
            path,
            allow_upgrade,
        } => match install_plugin_path(app.clone(), path, allow_upgrade) {
            Ok(_) => IpcResponse {
                ok: true,
                message: "插件已安装".into(),
                data: None,
            },
            Err(error) => IpcResponse {
                ok: false,
                message: error,
                data: None,
            },
        },
        IpcCommand::EnablePlugin { id, enabled } => {
            match set_plugin_enabled(app.clone(), id, enabled) {
                Ok(_) => IpcResponse {
                    ok: true,
                    message: "插件状态已更新".into(),
                    data: None,
                },
                Err(error) => IpcResponse {
                    ok: false,
                    message: error,
                    data: None,
                },
            }
        }
        IpcCommand::UninstallPlugin { id } => match uninstall_plugin_id(app.clone(), id) {
            Ok(_) => IpcResponse {
                ok: true,
                message: "插件已卸载".into(),
                data: None,
            },
            Err(error) => IpcResponse {
                ok: false,
                message: error,
                data: None,
            },
        },
        IpcCommand::Diagnostics => IpcResponse {
            ok: true,
            message: "ok".into(),
            data: Some(serde_json::json!(app
                .state::<AppState>()
                .diagnostics
                .list())),
        },
    }
}

fn command_for_secondary_launch(intent: LaunchIntent) -> Option<IpcCommand> {
    match intent {
        LaunchIntent::Forward(command) => Some(command),
        LaunchIntent::Gui => Some(IpcCommand::Open),
        LaunchIntent::Background => None,
    }
}

fn play_request(
    app: &AppHandle,
    command: String,
    params: BTreeMap<String, String>,
) -> Result<(), String> {
    app_lifecycle::record_macos_event(app, "play-request");
    let state = app.state::<AppState>();
    let result = state
        .playback
        .play(app, &state.config, &state.diagnostics, &command, params);
    if let Err(error) = &result {
        state
            .diagnostics
            .record("error", "playback", error, Some(&command));
    }
    result
}

fn preview_request_async(
    app: AppHandle,
    animation: models::AnimationDefinition,
    params: BTreeMap<String, String>,
) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let command = animation.command.clone();
        if let Err(error) =
            state
                .playback
                .preview(&app, &state.config, &state.diagnostics, animation, params)
        {
            state
                .diagnostics
                .record("error", "playback", error, Some(&command));
        }
    });
}

#[tauri::command]
fn config_get(state: tauri::State<'_, AppState>) -> AppConfig {
    state.config.get()
}

fn updater_runtime_config(app: &AppHandle) -> Result<UpdaterPluginRuntimeConfig, String> {
    let value = app
        .config()
        .plugins
        .0
        .get("updater")
        .cloned()
        .ok_or_else(|| "未找到 updater 配置".to_string())?;
    serde_json::from_value(value).map_err(|error| format!("解析 updater 配置失败: {error}"))
}

fn ensure_updater_is_configured(app: &AppHandle) -> Result<(), String> {
    let config = updater_runtime_config(app)?;
    let endpoints_ready = !config.endpoints.is_empty()
        && config.endpoints.iter().all(|endpoint| {
            let trimmed = endpoint.trim();
            !trimmed.is_empty() && trimmed.starts_with("https://")
        });
    let pubkey_ready = {
        let trimmed = config.pubkey.trim();
        !trimmed.is_empty() && trimmed != DEFAULT_UPDATER_PUBKEY
    };

    if endpoints_ready && pubkey_ready {
        Ok(())
    } else {
        Err("更新功能尚未完成发布配置，请先在 tauri.conf.json 中填写 updater 公钥。".to_string())
    }
}

fn emit_app_update_event(app: &AppHandle, payload: AppUpdateEventPayload) {
    if let Err(error) = app.emit(APP_UPDATE_EVENT, payload) {
        eprintln!("emit updater event failed: {error}");
    }
}

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
async fn check_app_update(app: AppHandle) -> Result<AppUpdateCheckResult, String> {
    let _guard = UpdateTaskGuard::acquire()?;
    ensure_updater_is_configured(&app)?;

    let current_version = app.package_info().version.to_string();
    let updater = app
        .updater_builder()
        .build()
        .map_err(|error| format!("初始化更新器失败: {error}"))?;
    let update = updater
        .check()
        .await
        .map_err(|error| format!("检查更新失败: {error}"))?;

    let update = update.map(|update| AppUpdateSummary {
        version: update.version,
        current_version: update.current_version,
        notes: update.body,
        pub_date: update.date.map(|date| date.to_string()),
        target: update.target,
        download_url: update.download_url.to_string(),
    });

    Ok(AppUpdateCheckResult {
        available: update.is_some(),
        current_version,
        update,
    })
}

#[tauri::command]
async fn download_and_install_update(app: AppHandle) -> Result<(), String> {
    let _guard = UpdateTaskGuard::acquire()?;
    ensure_updater_is_configured(&app)?;

    let updater = app
        .updater_builder()
        .build()
        .map_err(|error| format!("初始化更新器失败: {error}"))?;
    let update = updater
        .check()
        .await
        .map_err(|error| format!("检查更新失败: {error}"))?
        .ok_or_else(|| "当前已是最新版本，无需更新".to_string())?;

    let mut first_chunk = true;
    let mut downloaded_bytes = 0_u64;
    let app_handle = app.clone();

    update
        .download_and_install(
            move |chunk_length, content_length| {
                downloaded_bytes = downloaded_bytes.saturating_add(chunk_length as u64);
                if first_chunk {
                    first_chunk = false;
                    emit_app_update_event(
                        &app_handle,
                        AppUpdateEventPayload {
                            stage: "download_started".to_string(),
                            downloaded_bytes: Some(0),
                            chunk_length: None,
                            content_length,
                            message: Some("开始下载更新".to_string()),
                        },
                    );
                }
                emit_app_update_event(
                    &app_handle,
                    AppUpdateEventPayload {
                        stage: "download_progress".to_string(),
                        downloaded_bytes: Some(downloaded_bytes),
                        chunk_length: Some(chunk_length as u64),
                        content_length,
                        message: None,
                    },
                );
            },
            {
                let app_handle = app.clone();
                move || {
                    emit_app_update_event(
                        &app_handle,
                        AppUpdateEventPayload {
                            stage: "download_finished".to_string(),
                            downloaded_bytes: None,
                            chunk_length: None,
                            content_length: None,
                            message: Some("下载完成，正在安装更新".to_string()),
                        },
                    );
                }
            },
        )
        .await
        .map_err(|error| {
            emit_app_update_event(
                &app,
                AppUpdateEventPayload {
                    stage: "failed".to_string(),
                    downloaded_bytes: None,
                    chunk_length: None,
                    content_length: None,
                    message: Some(format!("安装更新失败: {error}")),
                },
            );
            format!("安装更新失败: {error}")
        })?;

    emit_app_update_event(
        &app,
        AppUpdateEventPayload {
            stage: "installed".to_string(),
            downloaded_bytes: None,
            chunk_length: None,
            content_length: None,
            message: Some("更新已安装完成".to_string()),
        },
    );

    Ok(())
}

#[tauri::command]
fn restart_app(app: AppHandle) -> Result<(), String> {
    app.request_restart();
    Ok(())
}

#[tauri::command]
fn settings_update(app: AppHandle, settings: models::GlobalSettings) -> Result<AppConfig, String> {
    let state = app.state::<AppState>();
    let config = state.config.update(|config| {
        config.settings = settings.clone();
        Ok(())
    })?;
    sync_tray_mute_checked(&app, config.settings.muted);
    state
        .diagnostics
        .set_retention(config.settings.diagnostics_retention);
    Ok(config)
}

#[tauri::command]
fn animation_save(
    state: tauri::State<'_, AppState>,
    animation: models::AnimationDefinition,
) -> Result<AppConfig, String> {
    state.config.update(|config| {
        if let Some(existing) = config
            .animations
            .iter_mut()
            .find(|item| item.id == animation.id)
        {
            *existing = animation.clone();
        } else {
            config.animations.push(animation.clone());
        }
        Ok(())
    })
}

#[tauri::command]
fn animation_delete(state: tauri::State<'_, AppState>, id: String) -> Result<AppConfig, String> {
    state.config.update(|config| {
        config.animations.retain(|animation| {
            animation.id != id || matches!(animation.kind, models::AnimationKind::Builtin)
        });
        Ok(())
    })
}

#[tauri::command]
fn animation_reset(state: tauri::State<'_, AppState>, id: String) -> Result<AppConfig, String> {
    let defaults = catalog::default_config();
    let default_animation = defaults
        .animations
        .into_iter()
        .find(|animation| animation.id == id)
        .ok_or_else(|| "没有对应内置动画".to_string())?;
    state.config.update(|config| {
        let item = config
            .animations
            .iter_mut()
            .find(|animation| animation.id == id)
            .ok_or_else(|| "动画不存在".to_string())?;
        *item = default_animation.clone();
        Ok(())
    })
}

#[tauri::command]
fn animation_play(
    app: AppHandle,
    command: String,
    params: BTreeMap<String, String>,
) -> Result<(), String> {
    play_request(&app, command, params)
}

#[tauri::command]
fn animation_preview(
    app: AppHandle,
    animation: models::AnimationDefinition,
    params: BTreeMap<String, String>,
) -> Result<(), String> {
    catalog::validate_animation(&animation)?;
    preview_request_async(app, animation, params);
    Ok(())
}

#[tauri::command]
fn animation_stop_all(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    state.playback.stop(&app, &state.diagnostics, "manual")
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnimationPackageMetadata {
    schema_version: u32,
}

#[tauri::command]
fn animation_export(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
    path: String,
) -> Result<(), String> {
    let config = state.config.get();
    let mut animation = config
        .animations
        .into_iter()
        .find(|animation| animation.id == id)
        .ok_or_else(|| "动画不存在".to_string())?;
    if matches!(animation.kind, models::AnimationKind::WebPlugin) {
        return Err("Web 插件请通过插件包分发".into());
    }
    let managed_audio = animation
        .audio
        .resource_id
        .clone()
        .filter(|value| !value.starts_with("builtin/"))
        .and_then(|resource_id| {
            let source = audio::resource_path(&app, &resource_id).ok()?;
            source.is_file().then_some(source)
        });
    if let Some(source) = &managed_audio {
        let extension = source
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("wav");
        animation.audio.resource_id = Some(format!("resources/audio.{extension}"));
    }
    let file = std::fs::File::create(path).map_err(|error| format!("创建动画包失败: {error}"))?;
    let mut archive = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    use std::io::Write;
    archive
        .start_file("package.json", options)
        .map_err(|error| format!("写入动画包元数据失败: {error}"))?;
    archive
        .write_all(
            &serde_json::to_vec_pretty(&AnimationPackageMetadata { schema_version: 1 })
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
    archive
        .start_file("animation.json", options)
        .map_err(|error| format!("写入动画包失败: {error}"))?;
    archive
        .write_all(&serde_json::to_vec_pretty(&animation).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())?;
    if let Some(source) = managed_audio {
        let package_path = animation.audio.resource_id.as_deref().unwrap();
        archive
            .start_file(package_path, options)
            .map_err(|error| format!("写入音效资源失败: {error}"))?;
        archive
            .write_all(
                &std::fs::read(source).map_err(|error| format!("读取音效资源失败: {error}"))?,
            )
            .map_err(|error| error.to_string())?;
    }
    archive
        .finish()
        .map_err(|error| format!("完成动画包失败: {error}"))?;
    Ok(())
}

#[tauri::command]
fn animation_import(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<AppConfig, String> {
    animation_import_inner(app, &state, path)
}

fn import_animation_path(app: AppHandle, path: String) -> Result<AppConfig, String> {
    let state = app.state::<AppState>();
    animation_import_inner(app.clone(), &state, path)
}

fn animation_import_inner(
    app: AppHandle,
    state: &AppState,
    path: String,
) -> Result<AppConfig, String> {
    let file = std::fs::File::open(path).map_err(|error| format!("打开动画包失败: {error}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| format!("动画包无效: {error}"))?;
    let metadata: AnimationPackageMetadata = {
        let entry = archive
            .by_name("package.json")
            .map_err(|_| "动画包缺少 package.json".to_string())?;
        if entry.size() > 64 * 1024 {
            return Err("动画包元数据超过大小限制".into());
        }
        serde_json::from_reader(entry).map_err(|error| format!("动画包元数据无效: {error}"))?
    };
    if metadata.schema_version != 1 {
        return Err(format!("不支持的动画包版本: {}", metadata.schema_version));
    }
    let mut animation: models::AnimationDefinition = {
        let entry = archive
            .by_name("animation.json")
            .map_err(|_| "动画包缺少 animation.json".to_string())?;
        if entry.size() > 512 * 1024 {
            return Err("动画配置超过大小限制".into());
        }
        serde_json::from_reader(entry).map_err(|error| format!("动画配置无效: {error}"))?
    };
    if matches!(animation.kind, models::AnimationKind::WebPlugin) {
        return Err("动画包不能导入 Web 插件".into());
    }
    let mut managed_resource: Option<PathBuf> = None;
    if let Some(package_resource) = animation
        .audio
        .resource_id
        .clone()
        .filter(|value| value.starts_with("resources/"))
    {
        let mut resource = archive
            .by_name(&package_resource)
            .map_err(|_| "动画包缺少声明的音效资源".to_string())?;
        if resource.size() > 8 * 1024 * 1024 {
            return Err("动画音效超过 8 MB 限制".into());
        }
        let mut bytes = Vec::with_capacity(resource.size() as usize);
        std::io::Read::read_to_end(&mut resource, &mut bytes)
            .map_err(|error| format!("读取动画音效失败: {error}"))?;
        let resource_id = audio::import_package_resource(&app, &package_resource, bytes)?;
        let target = audio::resource_path(&app, &resource_id)?;
        animation.audio.resource_id = Some(resource_id);
        managed_resource = Some(target);
    }
    let mut next = state.config.get();
    if next
        .animations
        .iter()
        .any(|existing| existing.id == animation.id)
    {
        return Err("动画 ID 与本地目录冲突，请重命名后再导入".into());
    }
    next.animations.push(animation);
    settings::validate_config(&next)?;
    match state.config.replace(next) {
        Ok(config) => Ok(config),
        Err(error) => {
            if let Some(target) = managed_resource {
                let _ = std::fs::remove_file(target);
            }
            Err(error)
        }
    }
}

#[tauri::command]
fn audio_import(app: AppHandle, path: String) -> Result<String, String> {
    audio::import_file(&app, &PathBuf::from(path))
}

#[tauri::command]
fn audio_library_list(app: AppHandle) -> Result<Vec<audio::AudioLibraryItem>, String> {
    audio::list_library(&app)
}

#[tauri::command]
fn audio_resource(app: AppHandle, resource_id: String) -> Result<String, String> {
    audio::resource_data_url(&app, &resource_id)
}

#[tauri::command]
fn diagnostics_get(state: tauri::State<'_, AppState>) -> Vec<models::DiagnosticEntry> {
    state.diagnostics.list()
}

#[tauri::command]
fn diagnostics_record(
    state: tauri::State<'_, AppState>,
    level: String,
    category: String,
    message: String,
) {
    state.diagnostics.record(&level, &category, message, None);
}

#[tauri::command]
fn plugins_get(state: tauri::State<'_, AppState>) -> Vec<models::PluginRecord> {
    state.config.get().plugins
}

#[tauri::command]
fn plugin_install(app: AppHandle, path: String, allow_upgrade: bool) -> Result<AppConfig, String> {
    install_plugin_path(app, path, allow_upgrade)
}

fn install_plugin_path(
    app: AppHandle,
    path: String,
    allow_upgrade: bool,
) -> Result<AppConfig, String> {
    let state = app.state::<AppState>();
    state.config.update(|config| {
        plugins::install(
            config,
            &state.plugins_root,
            &PathBuf::from(&path),
            allow_upgrade,
        )
        .map(|_| ())
    })
}

#[tauri::command]
fn plugin_set_enabled(app: AppHandle, id: String, enabled: bool) -> Result<AppConfig, String> {
    set_plugin_enabled(app, id, enabled)
}

fn set_plugin_enabled(app: AppHandle, id: String, enabled: bool) -> Result<AppConfig, String> {
    let state = app.state::<AppState>();
    state
        .config
        .update(|config| plugins::set_enabled(config, &id, enabled).map(|_| ()))
}

#[tauri::command]
fn plugin_uninstall(app: AppHandle, id: String) -> Result<AppConfig, String> {
    uninstall_plugin_id(app, id)
}

fn uninstall_plugin_id(app: AppHandle, id: String) -> Result<AppConfig, String> {
    let state = app.state::<AppState>();
    if state
        .playback
        .current()
        .as_ref()
        .and_then(|request| request.animation.plugin_id.as_deref())
        == Some(id.as_str())
    {
        state
            .playback
            .stop(&app, &state.diagnostics, "plugin-uninstall")?;
    }
    let path = {
        let mut config = state.config.get();
        let path = plugins::uninstall(&mut config, &id)?;
        let config = state.config.replace(config)?;
        (path, config)
    };
    if path.0.exists() {
        let _ = std::fs::remove_dir_all(path.0);
    }
    Ok(path.1)
}

#[tauri::command]
fn plugin_runtime_get(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<PluginRuntime, String> {
    plugins::runtime(&state.config.get(), &id)
}

#[tauri::command]
fn plugin_sound(app: AppHandle, plugin_id: String, sound_id: String) -> Result<String, String> {
    let state = app.state::<AppState>();
    if state.config.get().settings.muted {
        return Err("当前已全局静音".into());
    }
    let path = plugins::sound_path(&state.config.get(), &plugin_id, &sound_id)?;
    let bytes = std::fs::read(&path).map_err(|error| format!("读取插件音效失败: {error}"))?;
    use base64::Engine;
    let mime = mime_guess::from_path(&path).first_or_octet_stream();
    Ok(format!(
        "data:{};base64,{}",
        mime,
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

#[tauri::command]
fn playback_current(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Option<models::PlaybackRequest> {
    state.playback.current_for_window(window.label())
}

#[tauri::command]
fn playback_complete(app: AppHandle, session_id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    state
        .playback
        .complete(&app, &state.diagnostics, &session_id)
}

#[tauri::command]
fn playback_error(app: AppHandle, session_id: String, message: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let current = state.playback.current();
    let plugin_id = current
        .as_ref()
        .and_then(|request| request.animation.plugin_id.clone());
    state.diagnostics.record(
        "error",
        if plugin_id.is_some() {
            "plugin"
        } else {
            "playback"
        },
        &message,
        None,
    );
    if let Some(plugin_id) = plugin_id {
        let _ = state.config.update(|config| {
            if let Some(plugin) = config
                .plugins
                .iter_mut()
                .find(|plugin| plugin.manifest.id == plugin_id)
            {
                plugin.last_error = Some(message.clone());
            }
            Ok(())
        });
    }
    state
        .playback
        .complete(&app, &state.diagnostics, &session_id)
}

#[tauri::command]
fn app_open(app: AppHandle) -> Result<(), String> {
    open_management_window(&app, ManagementWindowEvent::OpenRequested)
}

fn open_gui_startup(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.hide();
        }
        let app = app.clone();
        let generation = app.state::<AppState>().macos_reopen.begin_reopen();
        std::thread::spawn(move || {
            std::thread::sleep(app_lifecycle::macos_reopen_delay());
            if app
                .state::<AppState>()
                .macos_reopen
                .should_restore_window(generation)
            {
                if let Err(error) = open_management_window(&app, ManagementWindowEvent::GuiStartup)
                {
                    record_lifecycle_warning(&app, error);
                }
            }
        });
    }
    #[cfg(not(target_os = "macos"))]
    if let Err(error) = open_management_window(app, ManagementWindowEvent::GuiStartup) {
        record_lifecycle_warning(app, error);
    }
}

fn open_management_window(app: &AppHandle, event: ManagementWindowEvent) -> Result<(), String> {
    app.state::<AppState>().macos_reopen.explicit_open();
    app_lifecycle::record_macos_event(
        app,
        match event {
            ManagementWindowEvent::GuiStartup => "gui-startup-open",
            _ => "management-open",
        },
    );
    let desired = app_lifecycle::desired_state(event);
    debug_assert!(desired.window_visible);
    if let Err(error) = app_lifecycle::apply_macos_app_state(app, event) {
        record_lifecycle_warning(app, error);
    }
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "管理窗口不存在".to_string())?;
    window.show().map_err(|error| error.to_string())?;
    window.unminimize().map_err(|error| error.to_string())?;
    if desired.focus_window {
        window.set_focus().map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn hide_management_window(app: &AppHandle, event: ManagementWindowEvent) -> Result<(), String> {
    app_lifecycle::record_macos_event(app, "management-hide");
    let desired = app_lifecycle::desired_state(event);
    debug_assert!(!desired.window_visible);
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "管理窗口不存在".to_string())?;
    window.hide().map_err(|error| error.to_string())?;
    app_lifecycle::apply_macos_app_state(app, event)
}

fn record_lifecycle_warning(app: &AppHandle, message: String) {
    if let Some(state) = app.try_state::<AppState>() {
        state
            .diagnostics
            .record("warning", "app-lifecycle", message, None);
    }
}

#[cfg(test)]
mod app_lifecycle_integration_tests {
    use super::*;

    #[test]
    fn secondary_gui_launch_requests_the_existing_management_window() {
        assert!(matches!(
            command_for_secondary_launch(LaunchIntent::Gui),
            Some(IpcCommand::Open)
        ));
    }

    #[test]
    fn secondary_background_launch_does_not_open_management_window() {
        assert!(command_for_secondary_launch(LaunchIntent::Background).is_none());
    }

    #[test]
    fn explicit_open_command_is_forwarded_to_the_shared_open_handler() {
        let args = vec!["open".to_string()];
        let intent = parse_launch_args(&args).unwrap();
        assert!(matches!(
            command_for_secondary_launch(intent),
            Some(IpcCommand::Open)
        ));
    }

    #[test]
    fn playback_protocol_is_forwarded_without_becoming_an_open_request() {
        let args = vec!["motioncue://play/success".to_string()];
        let intent = parse_launch_args(&args).unwrap();
        assert!(matches!(
            command_for_secondary_launch(intent),
            Some(IpcCommand::Play { .. })
        ));
    }
}

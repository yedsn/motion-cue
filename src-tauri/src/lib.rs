use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;

mod catalog;
mod audio;
mod diagnostics;
mod invocation;
mod models;
mod playback;
mod plugins;
mod settings;

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
    let app_data = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("com.motioncue.desktop");
    invocation::endpoint_path(&app_data)
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
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_single_instance::init(
            move |app, args, _cwd| {
                let command = match parse_launch_args(&args[1..]) {
                    Ok(LaunchIntent::Forward(command)) => command,
                    Ok(LaunchIntent::Gui) => IpcCommand::Open,
                    Ok(LaunchIntent::Background) => return,
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
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| format!("读取应用数据目录失败: {error}"))?;
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
            });
            let endpoint = invocation::endpoint_path(&data_dir);
            let app_handle = app.handle().clone();
            start_server(app.handle().clone(), endpoint, handle_ipc)?;
            setup_tray(app)?;
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
                for url in event.urls() {
                    match parse_deep_link(url.as_str(), "protocol") {
                        Ok(request) => {
                            let _ = play_request(&deep_link_app, request.command, request.params);
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
            if let Some(intent) = initial_intent_for_instance.lock().unwrap().take() {
                match intent {
                    LaunchIntent::Forward(command) => {
                        let _ = handle_ipc(&app_handle, command);
                    }
                    LaunchIntent::Background => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    }
                    LaunchIntent::Gui => {}
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
            app_open
        ])
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .map_err(|error| format!("MotionCue 运行失败: {error}"))
}

fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let open = MenuItem::with_id(app, "open", "打开 MotionCue", true, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop", "停止全部动画", true, None::<&str>)?;
    let mute = MenuItem::with_id(app, "mute", "切换全局静音", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &stop, &mute, &quit])?;
    let tray = TrayIconBuilder::with_id("motioncue")
        .icon(
            app.default_window_icon()
                .cloned()
                .ok_or("默认窗口图标不可用")?,
        )
        .menu(&menu)
        .tooltip("MotionCue");
    tray.on_menu_event(|app, event| match event.id.as_ref() {
        "open" => {
            let _ = app_open(app.clone());
        }
        "stop" => {
            let _ = animation_stop_all(app.clone());
        }
        "mute" => {
            let current = app.state::<AppState>().config.get();
            let _ = settings_update(
                app.clone(),
                models::GlobalSettings {
                    muted: !current.settings.muted,
                    ..current.settings
                },
            );
        }
        "quit" => {
            let _ = app.exit(0);
        }
        _ => {}
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

fn handle_ipc(app: &AppHandle, command: IpcCommand) -> IpcResponse {
    match command {
        IpcCommand::Play { request } => {
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

fn play_request(
    app: &AppHandle,
    command: String,
    params: BTreeMap<String, String>,
) -> Result<(), String> {
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

#[tauri::command]
fn settings_update(app: AppHandle, settings: models::GlobalSettings) -> Result<AppConfig, String> {
    let state = app.state::<AppState>();
    let config = state.config.update(|config| {
        config.settings = settings.clone();
        Ok(())
    })?;
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
            let source = app
                .path()
                .app_data_dir()
                .ok()?
                .join("resources")
                .join(&resource_id);
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
    let mut managed_resource: Option<(PathBuf, Vec<u8>)> = None;
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
        let extension = std::path::Path::new(&package_resource)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("wav")
            .to_ascii_lowercase();
        if !["wav", "mp3", "ogg"].contains(&extension.as_str()) {
            return Err("动画包音效格式不受支持".into());
        }
        let mut bytes = Vec::with_capacity(resource.size() as usize);
        std::io::Read::read_to_end(&mut resource, &mut bytes)
            .map_err(|error| format!("读取动画音效失败: {error}"))?;
        let resource_id = format!("audio/{}.{}", uuid::Uuid::new_v4(), extension);
        let target = app
            .path()
            .app_data_dir()
            .map_err(|error| error.to_string())?
            .join("resources")
            .join(&resource_id);
        animation.audio.resource_id = Some(resource_id);
        managed_resource = Some((target, bytes));
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
    if let Some((target, bytes)) = &managed_resource {
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        std::fs::write(target, bytes).map_err(|error| format!("保存动画音效失败: {error}"))?;
    }
    match state.config.replace(next) {
        Ok(config) => Ok(config),
        Err(error) => {
            if let Some((target, _)) = managed_resource {
                let _ = std::fs::remove_file(target);
            }
            Err(error)
        }
    }
}

#[tauri::command]
fn audio_import(app: AppHandle, path: String) -> Result<String, String> {
    let source = PathBuf::from(&path);
    let metadata = std::fs::metadata(&source).map_err(|error| format!("读取音效失败: {error}"))?;
    if metadata.len() > 8 * 1024 * 1024 {
        return Err("音效文件不能超过 8 MB".into());
    }
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !["wav", "mp3", "ogg"].contains(&extension.as_str()) {
        return Err("仅支持 WAV、MP3 和 OGG 音效".into());
    }
    let id = format!("audio/{}.{}", uuid::Uuid::new_v4(), extension);
    let target = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("resources")
        .join(&id);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::copy(source, &target).map_err(|error| format!("保存音效失败: {error}"))?;
    Ok(id)
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
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "管理窗口不存在".to_string())?;
    window.show().map_err(|error| error.to_string())?;
    window.unminimize().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())
}

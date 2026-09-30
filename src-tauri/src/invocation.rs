use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rand::RngCore;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use url::Url;

use crate::models::{now_ms, InvocationRequest, SCHEMA_VERSION};

const MAX_IPC_MESSAGE: usize = 64 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum IpcCommand {
    Play { request: InvocationRequest },
    List,
    Stop,
    Open,
    ImportAnimation { path: String },
    InstallPlugin { path: String, allow_upgrade: bool },
    EnablePlugin { id: String, enabled: bool },
    UninstallPlugin { id: String },
    Diagnostics,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcEnvelope {
    pub token: String,
    pub command: IpcCommand,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcResponse {
    pub ok: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EndpointInfo {
    port: u16,
    token: String,
}

#[derive(Clone)]
pub enum LaunchIntent {
    Background,
    Forward(IpcCommand),
    Gui,
}

pub fn parse_launch_args(args: &[String]) -> Result<LaunchIntent, String> {
    if args.iter().any(|arg| arg == "--background") {
        return Ok(LaunchIntent::Background);
    }
    let Some(first) = args.first() else {
        return Ok(LaunchIntent::Gui);
    };
    if first.starts_with("motioncue://") {
        return Ok(LaunchIntent::Forward(IpcCommand::Play {
            request: parse_deep_link(first, "protocol")?,
        }));
    }
    match first.as_str() {
        "play" => {
            let command = args.get(1).ok_or_else(|| "用法: motion-cue play <command> [--text value] [--color value] [--duration value]".to_string())?;
            let mut params = BTreeMap::new();
            let mut index = 2;
            while index < args.len() {
                let key = args[index]
                    .strip_prefix("--")
                    .ok_or_else(|| format!("无效参数: {}", args[index]))?;
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| format!("参数 --{key} 缺少值"))?;
                params.insert(key.to_string(), value.to_string());
                index += 2;
            }
            Ok(LaunchIntent::Forward(IpcCommand::Play {
                request: InvocationRequest {
                    schema_version: SCHEMA_VERSION,
                    source: "cli".into(),
                    command: command.clone(),
                    params,
                    requested_at: now_ms(),
                },
            }))
        }
        "list" => Ok(LaunchIntent::Forward(IpcCommand::List)),
        "stop" => Ok(LaunchIntent::Forward(IpcCommand::Stop)),
        "open" => Ok(LaunchIntent::Forward(IpcCommand::Open)),
        "import-animation" => Ok(LaunchIntent::Forward(IpcCommand::ImportAnimation {
            path: args
                .get(1)
                .ok_or_else(|| "用法: motion-cue import-animation <package>".to_string())?
                .clone(),
        })),
        "install-plugin" => Ok(LaunchIntent::Forward(IpcCommand::InstallPlugin {
            path: args
                .get(1)
                .ok_or_else(|| {
                    "用法: motion-cue install-plugin <package> [--allow-upgrade]".to_string()
                })?
                .clone(),
            allow_upgrade: args.iter().any(|arg| arg == "--allow-upgrade"),
        })),
        "enable-plugin" => Ok(LaunchIntent::Forward(IpcCommand::EnablePlugin {
            id: args
                .get(1)
                .ok_or_else(|| "用法: motion-cue enable-plugin <id>".to_string())?
                .clone(),
            enabled: true,
        })),
        "disable-plugin" => Ok(LaunchIntent::Forward(IpcCommand::EnablePlugin {
            id: args
                .get(1)
                .ok_or_else(|| "用法: motion-cue disable-plugin <id>".to_string())?
                .clone(),
            enabled: false,
        })),
        "uninstall-plugin" => Ok(LaunchIntent::Forward(IpcCommand::UninstallPlugin {
            id: args
                .get(1)
                .ok_or_else(|| "用法: motion-cue uninstall-plugin <id>".to_string())?
                .clone(),
        })),
        "diagnostics" => {
            if args.get(1).is_some_and(|arg| arg != "--json") {
                return Err("用法: motion-cue diagnostics [--json]".into());
            }
            Ok(LaunchIntent::Forward(IpcCommand::Diagnostics))
        }
        _ if first.starts_with('-') => Err(format!("未知参数: {first}")),
        _ => Ok(LaunchIntent::Gui),
    }
}

pub fn parse_deep_link(value: &str, source: &str) -> Result<InvocationRequest, String> {
    let url = Url::parse(value).map_err(|error| format!("自定义链接无效: {error}"))?;
    if url.scheme() != "motioncue" || url.host_str() != Some("play") {
        return Err("仅支持 motioncue://play/<command>".into());
    }
    let path_command = url
        .path_segments()
        .and_then(|mut segments| segments.find(|value| !value.is_empty()))
        .map(ToOwned::to_owned);
    let query = url.query_pairs().collect::<BTreeMap<_, _>>();
    let command = path_command
        .or_else(|| query.get("command").map(|value| value.to_string()))
        .ok_or_else(|| "自定义链接缺少 command".to_string())?;
    let params = query
        .into_iter()
        .filter(|(key, _)| key != "command")
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    Ok(InvocationRequest {
        schema_version: SCHEMA_VERSION,
        source: source.into(),
        command,
        params,
        requested_at: now_ms(),
    })
}

pub fn endpoint_path(app_data: &Path) -> PathBuf {
    app_data.join("runtime").join("ipc.json")
}

pub fn forward(
    endpoint_path: &Path,
    command: IpcCommand,
    timeout: Duration,
) -> Result<IpcResponse, String> {
    let content =
        std::fs::read_to_string(endpoint_path).map_err(|_| "MotionCue 未运行".to_string())?;
    let endpoint: EndpointInfo =
        serde_json::from_str(&content).map_err(|error| format!("IPC 端点无效: {error}"))?;
    let address = format!("127.0.0.1:{}", endpoint.port)
        .parse()
        .map_err(|error| format!("IPC 地址无效: {error}"))?;
    let mut stream = TcpStream::connect_timeout(&address, timeout)
        .map_err(|error| format!("连接 MotionCue 失败: {error}"))?;
    stream.set_read_timeout(Some(timeout)).ok();
    stream.set_write_timeout(Some(timeout)).ok();
    write_frame(
        &mut stream,
        &IpcEnvelope {
            token: endpoint.token,
            command,
        },
    )?;
    read_frame(&mut stream)
}

pub fn wait_and_forward(endpoint_path: &Path, command: IpcCommand) -> Result<IpcResponse, String> {
    let started = Instant::now();
    loop {
        match forward(endpoint_path, command.clone(), Duration::from_secs(2)) {
            Ok(response) => return Ok(response),
            Err(_) if started.elapsed() < Duration::from_secs(20) => {
                std::thread::sleep(Duration::from_millis(100))
            }
            Err(error) => return Err(error),
        }
    }
}

pub fn start_server(
    app: AppHandle,
    endpoint_path: PathBuf,
    handler: fn(&AppHandle, IpcCommand) -> IpcResponse,
) -> Result<(), String> {
    let listener =
        TcpListener::bind("127.0.0.1:0").map_err(|error| format!("启动 IPC 服务失败: {error}"))?;
    let port = listener
        .local_addr()
        .map_err(|error| format!("读取 IPC 端口失败: {error}"))?
        .port();
    let mut token_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut token_bytes);
    let token = token_bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if let Some(parent) = endpoint_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| format!("创建 IPC 目录失败: {error}"))?;
    }
    std::fs::write(
        &endpoint_path,
        serde_json::to_vec(&EndpointInfo {
            port,
            token: token.clone(),
        })
        .unwrap(),
    )
    .map_err(|error| format!("写入 IPC 端点失败: {error}"))?;
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let app = app.clone();
            let token = token.clone();
            std::thread::spawn(move || handle_stream(stream, &app, &token, handler));
        }
    });
    Ok(())
}

fn handle_stream(
    mut stream: TcpStream,
    app: &AppHandle,
    token: &str,
    handler: fn(&AppHandle, IpcCommand) -> IpcResponse,
) {
    let response = match read_frame::<IpcEnvelope>(&mut stream) {
        Ok(envelope) if envelope.token == token => handler(app, envelope.command),
        Ok(_) => IpcResponse {
            ok: false,
            message: "IPC 认证失败".into(),
            data: None,
        },
        Err(error) => IpcResponse {
            ok: false,
            message: error,
            data: None,
        },
    };
    let _ = write_frame(&mut stream, &response);
}

fn write_frame<T: Serialize>(stream: &mut TcpStream, value: &T) -> Result<(), String> {
    let data =
        serde_json::to_vec(value).map_err(|error| format!("序列化 IPC 消息失败: {error}"))?;
    if data.len() > MAX_IPC_MESSAGE {
        return Err("IPC 消息过大".into());
    }
    stream
        .write_all(&(data.len() as u32).to_be_bytes())
        .map_err(|error| format!("写入 IPC 长度失败: {error}"))?;
    stream
        .write_all(&data)
        .map_err(|error| format!("写入 IPC 消息失败: {error}"))
}

fn read_frame<T: for<'de> Deserialize<'de>>(stream: &mut TcpStream) -> Result<T, String> {
    let mut length = [0u8; 4];
    stream
        .read_exact(&mut length)
        .map_err(|error| format!("读取 IPC 长度失败: {error}"))?;
    let length = u32::from_be_bytes(length) as usize;
    if length > MAX_IPC_MESSAGE {
        return Err("IPC 消息超过限制".into());
    }
    let mut data = vec![0; length];
    stream
        .read_exact(&mut data)
        .map_err(|error| format!("读取 IPC 消息失败: {error}"))?;
    serde_json::from_slice(&data).map_err(|error| format!("解析 IPC 消息失败: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_path_and_query_links() {
        assert_eq!(
            parse_deep_link("motioncue://play/task-complete?text=Done", "protocol")
                .unwrap()
                .command,
            "task-complete"
        );
        assert_eq!(
            parse_deep_link("motioncue://play?command=success", "protocol")
                .unwrap()
                .command,
            "success"
        );
    }

    #[test]
    fn rejects_wrong_scheme() {
        assert!(parse_deep_link("https://example.com/play/confetti", "protocol").is_err());
    }

    #[test]
    fn parses_cli_params() {
        let args = vec![
            "play".into(),
            "success".into(),
            "--text".into(),
            "Done".into(),
        ];
        match parse_launch_args(&args).unwrap() {
            LaunchIntent::Forward(IpcCommand::Play { request }) => {
                assert_eq!(request.params["text"], "Done")
            }
            _ => panic!("unexpected intent"),
        }
    }

    #[test]
    fn rejects_cli_missing_value() {
        let args = vec!["play".into(), "success".into(), "--text".into()];
        assert!(parse_launch_args(&args).is_err());
    }

    #[test]
    fn parses_diagnostics_command() {
        let args = vec!["diagnostics".into(), "--json".into()];
        match parse_launch_args(&args).unwrap() {
            LaunchIntent::Forward(IpcCommand::Diagnostics) => {}
            _ => panic!("unexpected intent"),
        }
    }
}

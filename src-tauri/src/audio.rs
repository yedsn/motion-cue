use std::path::PathBuf;

use base64::Engine;
use tauri::{AppHandle, Manager};

const COMPLETION_SUCCESS_WAV: &[u8] = include_bytes!("../../src-ui/public/audio/completion-success.wav");
const SOFT_CHIME_WAV: &[u8] = include_bytes!("../../src-ui/public/audio/soft-chime.wav");
const BRIGHT_POP_WAV: &[u8] = include_bytes!("../../src-ui/public/audio/bright-pop.wav");

pub fn resource_data_url(app: &AppHandle, resource_id: &str) -> Result<String, String> {
    if let Some(data_url) = builtin_data_url(resource_id) {
        return Ok(data_url);
    }
    if !resource_id.starts_with("audio/")
        || resource_id.contains("..")
        || resource_id.contains('\\')
        || PathBuf::from(resource_id).is_absolute()
    {
        return Err("仅支持已声明的内置音效或已导入的外置音效".into());
    }
    let path = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("resources")
        .join(resource_id);
    let data = std::fs::read(&path).map_err(|error| format!("读取音效失败: {error}"))?;
    let mime = mime_guess::from_path(&path).first_or_octet_stream();
    Ok(data_url(mime.as_ref(), &data))
}

fn builtin_data_url(resource_id: &str) -> Option<String> {
    let bytes = match resource_id {
        "builtin/completion-success.wav" => COMPLETION_SUCCESS_WAV,
        "builtin/soft-chime.wav" => SOFT_CHIME_WAV,
        "builtin/bright-pop.wav" => BRIGHT_POP_WAV,
        _ => return None,
    };
    Some(data_url("audio/wav", bytes))
}

fn data_url(mime: &str, bytes: &[u8]) -> String {
    format!(
        "data:{};base64,{}",
        mime,
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

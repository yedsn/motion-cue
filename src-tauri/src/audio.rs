use std::path::{Path, PathBuf};

use base64::Engine;
use tauri::{AppHandle, Manager};

const MAX_AUDIO_BYTES: u64 = 8 * 1024 * 1024;

struct SeedAudio {
    file_name: &'static str,
    display_name: &'static str,
    bytes: &'static [u8],
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioLibraryItem {
    pub resource_id: String,
    pub name: String,
    pub file_name: String,
    pub size_bytes: u64,
    pub built_in: bool,
}

const SEEDED_AUDIO: &[SeedAudio] = &[
    SeedAudio {
        file_name: "completion-success.wav",
        display_name: "清脆完成",
        bytes: include_bytes!("../../src-ui/public/audio/completion-success.wav"),
    },
    SeedAudio {
        file_name: "soft-chime.wav",
        display_name: "柔和铃音",
        bytes: include_bytes!("../../src-ui/public/audio/soft-chime.wav"),
    },
    SeedAudio {
        file_name: "456965__funwithsound__short-success-sound-glockenspiel-treasure-video-game.mp3",
        display_name: "八音盒短铃",
        bytes: include_bytes!("../../src-ui/public/audio/456965__funwithsound__short-success-sound-glockenspiel-treasure-video-game.mp3"),
    },
    SeedAudio {
        file_name: "456966__funwithsound__success-fanfare-trumpets.mp3",
        display_name: "小号欢呼",
        bytes: include_bytes!("../../src-ui/public/audio/456966__funwithsound__success-fanfare-trumpets.mp3"),
    },
    SeedAudio {
        file_name: "538149__fupicat__notification.wav",
        display_name: "轻快通知",
        bytes: include_bytes!("../../src-ui/public/audio/538149__fupicat__notification.wav"),
    },
    SeedAudio {
        file_name: "541985__rob_marion__gasp_chimes_success_4.wav",
        display_name: "层叠风铃",
        bytes: include_bytes!("../../src-ui/public/audio/541985__rob_marion__gasp_chimes_success_4.wav"),
    },
    SeedAudio {
        file_name: "716450__scottyd0es__tone12_success.wav",
        display_name: "电子上扬",
        bytes: include_bytes!("../../src-ui/public/audio/716450__scottyd0es__tone12_success.wav"),
    },
    SeedAudio {
        file_name: "780010__lucy__success-notification.flac",
        display_name: "清亮提示",
        bytes: include_bytes!("../../src-ui/public/audio/780010__lucy__success-notification.flac"),
    },
];

pub fn seed_library(app: &AppHandle) -> Result<(), String> {
    let audio_dir = managed_audio_dir(app)?;
    std::fs::create_dir_all(&audio_dir).map_err(|error| format!("创建音效目录失败: {error}"))?;
    for audio in SEEDED_AUDIO {
        let target = audio_dir.join(audio.file_name);
        if !target.exists() {
            std::fs::write(&target, audio.bytes)
                .map_err(|error| format!("写入内置音效失败: {error}"))?;
        }
    }
    Ok(())
}

pub fn list_library(app: &AppHandle) -> Result<Vec<AudioLibraryItem>, String> {
    let audio_dir = managed_audio_dir(app)?;
    std::fs::create_dir_all(&audio_dir).map_err(|error| format!("创建音效目录失败: {error}"))?;
    let mut items = Vec::new();
    for entry in
        std::fs::read_dir(&audio_dir).map_err(|error| format!("读取音效目录失败: {error}"))?
    {
        let entry = entry.map_err(|error| format!("读取音效条目失败: {error}"))?;
        let path = entry.path();
        if !path.is_file() || !supported_extension(&path) {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let metadata = entry
            .metadata()
            .map_err(|error| format!("读取音效信息失败: {error}"))?;
        let built_in = SEEDED_AUDIO
            .iter()
            .any(|audio| audio.file_name == file_name);
        items.push(AudioLibraryItem {
            resource_id: format!("audio/{file_name}"),
            name: display_name(file_name).to_string(),
            file_name: file_name.to_string(),
            size_bytes: metadata.len(),
            built_in,
        });
    }
    items.sort_by(|left, right| {
        right
            .built_in
            .cmp(&left.built_in)
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(items)
}

pub fn import_file(app: &AppHandle, source: &Path) -> Result<String, String> {
    let metadata = std::fs::metadata(source).map_err(|error| format!("读取音效失败: {error}"))?;
    if metadata.len() > MAX_AUDIO_BYTES {
        return Err("音效文件不能超过 8 MB".into());
    }
    if !supported_extension(source) {
        return Err("仅支持 WAV、MP3、OGG 和 FLAC 音效".into());
    }
    let file_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "音效文件名无效".to_string())?;
    let safe_name = safe_file_name(file_name)?;
    let audio_dir = managed_audio_dir(app)?;
    std::fs::create_dir_all(&audio_dir).map_err(|error| format!("创建音效目录失败: {error}"))?;
    let target_name = available_file_name(&audio_dir, &safe_name);
    std::fs::copy(source, audio_dir.join(&target_name))
        .map_err(|error| format!("保存音效失败: {error}"))?;
    Ok(format!("audio/{target_name}"))
}

pub fn import_package_resource(
    app: &AppHandle,
    package_resource: &str,
    bytes: Vec<u8>,
) -> Result<String, String> {
    if bytes.len() as u64 > MAX_AUDIO_BYTES {
        return Err("动画音效超过 8 MB 限制".into());
    }
    let source_name = Path::new(package_resource)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("audio.wav");
    let safe_name = safe_file_name(source_name)?;
    if !supported_extension(Path::new(&safe_name)) {
        return Err("动画包音效格式不受支持".into());
    }
    let audio_dir = managed_audio_dir(app)?;
    std::fs::create_dir_all(&audio_dir).map_err(|error| format!("创建音效目录失败: {error}"))?;
    let target_name = available_file_name(&audio_dir, &safe_name);
    let target = audio_dir.join(&target_name);
    std::fs::write(&target, bytes).map_err(|error| format!("保存动画音效失败: {error}"))?;
    Ok(format!("audio/{target_name}"))
}

pub fn resource_path(app: &AppHandle, resource_id: &str) -> Result<PathBuf, String> {
    let relative = managed_relative_path(resource_id)?;
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("resources")
        .join(relative))
}

pub fn resource_data_url(app: &AppHandle, resource_id: &str) -> Result<String, String> {
    let path = if let Some(file_name) = legacy_builtin_file_name(resource_id) {
        managed_audio_dir(app)?.join(file_name)
    } else {
        resource_path(app, resource_id)?
    };
    let data = std::fs::read(&path).map_err(|error| format!("读取音效失败: {error}"))?;
    let mime = mime_guess::from_path(&path).first_or_octet_stream();
    Ok(data_url(mime.as_ref(), &data))
}

pub fn is_supported_extension(extension: &str) -> bool {
    matches!(extension, "wav" | "mp3" | "ogg" | "flac")
}

fn managed_audio_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("resources")
        .join("audio"))
}

fn managed_relative_path(resource_id: &str) -> Result<PathBuf, String> {
    if !resource_id.starts_with("audio/")
        || resource_id.contains("..")
        || resource_id.contains('\\')
        || PathBuf::from(resource_id).is_absolute()
    {
        return Err("仅支持已声明的内置音效或已导入的外置音效".into());
    }
    let file_name = resource_id
        .strip_prefix("audio/")
        .ok_or_else(|| "音效资源路径无效".to_string())?;
    Ok(PathBuf::from("audio").join(safe_file_name(file_name)?))
}

fn legacy_builtin_file_name(resource_id: &str) -> Option<&'static str> {
    match resource_id {
        "builtin/completion-success.wav" => Some("completion-success.wav"),
        "builtin/soft-chime.wav" => Some("soft-chime.wav"),
        "builtin/bright-pop.wav" => Some("bright-pop.wav"),
        _ => None,
    }
}

fn display_name(file_name: &str) -> &str {
    SEEDED_AUDIO
        .iter()
        .find(|audio| audio.file_name == file_name)
        .map(|audio| audio.display_name)
        .unwrap_or(file_name)
}

fn safe_file_name(value: &str) -> Result<String, String> {
    let path = Path::new(value);
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "音效文件名无效".to_string())?;
    if file_name != value
        || file_name.is_empty()
        || file_name.contains("..")
        || file_name.contains('/')
        || file_name.contains('\\')
    {
        return Err("音效文件名无效".into());
    }
    if !supported_extension(Path::new(file_name)) {
        return Err("仅支持 WAV、MP3、OGG 和 FLAC 音效".into());
    }
    Ok(file_name.to_string())
}

fn supported_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| is_supported_extension(&value.to_ascii_lowercase()))
        .unwrap_or(false)
}

fn available_file_name(directory: &Path, file_name: &str) -> String {
    let path = Path::new(file_name);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("audio");
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("wav");
    if !directory.join(file_name).exists() {
        return file_name.to_string();
    }
    for index in 2.. {
        let candidate = format!("{stem}-{index}.{extension}");
        if !directory.join(&candidate).exists() {
            return candidate;
        }
    }
    unreachable!()
}

fn data_url(mime: &str, bytes: &[u8]) -> String {
    format!(
        "data:{};base64,{}",
        mime,
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_supported_audio_formats() {
        for extension in ["wav", "mp3", "ogg", "flac"] {
            assert!(is_supported_extension(extension));
        }
    }

    #[test]
    fn rejects_unsafe_file_names() {
        assert!(safe_file_name("../bad.wav").is_err());
        assert!(safe_file_name("nested/bad.wav").is_err());
        assert!(safe_file_name("bad.exe").is_err());
    }

    #[test]
    fn keeps_original_file_name_when_available() {
        let directory = tempfile::tempdir().unwrap();
        assert_eq!(
            available_file_name(directory.path(), "sound.wav"),
            "sound.wav"
        );
        std::fs::write(directory.path().join("sound.wav"), b"x").unwrap();
        assert_eq!(
            available_file_name(directory.path(), "sound.wav"),
            "sound-2.wav"
        );
    }
}

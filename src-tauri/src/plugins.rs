use std::fs::{self, File};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use zip::read::ZipArchive;

use crate::catalog::normalize_command;
use base64::Engine;

use crate::models::{
    AnimationDefinition, AnimationKind, AppConfig, AudioConfig, MonitorTarget, PluginManifest,
    PluginRecord, PluginRuntime, RendererKind, SCHEMA_VERSION,
};

const MAX_PACKAGE_SIZE: u64 = 20 * 1024 * 1024;
const MAX_FILE_SIZE: u64 = 8 * 1024 * 1024;
const ALLOWED_EXTENSIONS: &[&str] = &[
    "json", "html", "js", "mjs", "css", "png", "jpg", "jpeg", "gif", "webp", "svg", "wav", "mp3",
    "ogg", "woff", "woff2",
];

pub fn install(
    config: &mut AppConfig,
    plugins_root: &Path,
    package: &Path,
    allow_upgrade: bool,
) -> Result<PluginRecord, String> {
    let metadata = fs::metadata(package).map_err(|error| format!("读取插件包失败: {error}"))?;
    if metadata.len() > MAX_PACKAGE_SIZE {
        return Err("插件包超过 20 MB 限制".into());
    }
    let file = File::open(package).map_err(|error| format!("打开插件包失败: {error}"))?;
    let mut archive =
        ZipArchive::new(file).map_err(|error| format!("插件包不是有效 ZIP: {error}"))?;
    let staging = plugins_root.join(format!(".staging-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&staging).map_err(|error| format!("创建插件临时目录失败: {error}"))?;
    let result = (|| {
        let mut total = 0u64;
        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|error| format!("读取插件文件失败: {error}"))?;
            if entry.is_dir() {
                continue;
            }
            if entry.size() > MAX_FILE_SIZE {
                return Err(format!("插件资源过大: {}", entry.name()));
            }
            total = total.saturating_add(entry.size());
            if total > MAX_PACKAGE_SIZE {
                return Err("插件解压资源超过限制".into());
            }
            if entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
            {
                return Err("插件包不能包含符号链接".into());
            }
            let relative = safe_relative(entry.name())?;
            let extension = relative
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !ALLOWED_EXTENSIONS.contains(&extension.as_str()) {
                return Err(format!("不支持的插件资源类型: {}", relative.display()));
            }
            let output = staging.join(&relative);
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            let mut target =
                File::create(&output).map_err(|error| format!("创建插件资源失败: {error}"))?;
            std::io::copy(&mut entry, &mut target)
                .map_err(|error| format!("解压插件资源失败: {error}"))?;
            target.flush().ok();
        }
        let manifest_path = staging.join("manifest.json");
        let manifest: PluginManifest = serde_json::from_slice(
            &fs::read(&manifest_path).map_err(|_| "插件包缺少 manifest.json".to_string())?,
        )
        .map_err(|error| format!("插件清单无效: {error}"))?;
        validate_manifest(&manifest, &staging)?;
        if config
            .plugins
            .iter()
            .any(|plugin| plugin.manifest.id == manifest.id)
            && !allow_upgrade
        {
            return Err(format!("插件 {} 已存在，需要明确确认升级", manifest.id));
        }
        let destination = plugins_root.join(&manifest.id).join(&manifest.version);
        let rollback = destination.with_extension(format!("rollback-{}", uuid::Uuid::new_v4()));
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        if destination.exists() {
            fs::rename(&destination, &rollback)
                .map_err(|error| format!("准备插件升级回滚失败: {error}"))?;
        }
        if let Err(error) = fs::rename(&staging, &destination) {
            if rollback.exists() {
                let _ = fs::rename(&rollback, &destination);
            }
            return Err(format!("安装插件失败: {error}"));
        }
        if rollback.exists() {
            let _ = fs::remove_dir_all(&rollback);
        }
        let record = PluginRecord {
            manifest: manifest.clone(),
            enabled: false,
            source: package.display().to_string(),
            installed_path: destination.display().to_string(),
            size_bytes: total,
            last_error: None,
        };
        if let Some(existing) = config
            .plugins
            .iter_mut()
            .find(|plugin| plugin.manifest.id == manifest.id)
        {
            *existing = record.clone();
        } else {
            config.plugins.push(record.clone());
        }
        config
            .animations
            .retain(|animation| animation.plugin_id.as_deref() != Some(&manifest.id));
        config.animations.push(plugin_animation(&manifest));
        Ok(record)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

pub fn set_enabled(config: &mut AppConfig, id: &str, enabled: bool) -> Result<(), String> {
    let plugin = config
        .plugins
        .iter_mut()
        .find(|plugin| plugin.manifest.id == id)
        .ok_or_else(|| "插件不存在".to_string())?;
    plugin.enabled = enabled;
    if let Some(animation) = config
        .animations
        .iter_mut()
        .find(|animation| animation.plugin_id.as_deref() == Some(id))
    {
        animation.enabled = enabled;
    }
    crate::catalog::command_index(config)?;
    Ok(())
}

pub fn uninstall(config: &mut AppConfig, id: &str) -> Result<PathBuf, String> {
    let index = config
        .plugins
        .iter()
        .position(|plugin| plugin.manifest.id == id)
        .ok_or_else(|| "插件不存在".to_string())?;
    let path = PathBuf::from(&config.plugins[index].installed_path);
    config.plugins.remove(index);
    config
        .animations
        .retain(|animation| animation.plugin_id.as_deref() != Some(id));
    Ok(path)
}

pub fn runtime(config: &AppConfig, id: &str) -> Result<PluginRuntime, String> {
    let plugin = config
        .plugins
        .iter()
        .find(|plugin| plugin.manifest.id == id && plugin.enabled)
        .ok_or_else(|| "插件不存在或未启用".to_string())?;
    let entry = PathBuf::from(&plugin.installed_path).join(&plugin.manifest.entry);
    let html = fs::read_to_string(entry).map_err(|error| format!("读取插件入口失败: {error}"))?;
    let html = bundle_local_resources(html, Path::new(&plugin.installed_path), &plugin.manifest)?;
    Ok(PluginRuntime {
        manifest: plugin.manifest.clone(),
        html,
    })
}

pub fn sound_path(config: &AppConfig, plugin_id: &str, sound_id: &str) -> Result<PathBuf, String> {
    let plugin = config
        .plugins
        .iter()
        .find(|plugin| plugin.manifest.id == plugin_id && plugin.enabled)
        .ok_or_else(|| "插件不存在或未启用".to_string())?;
    let relative = plugin
        .manifest
        .sounds
        .get(sound_id)
        .ok_or_else(|| "插件未声明该音效".to_string())?;
    Ok(PathBuf::from(&plugin.installed_path).join(safe_relative(relative)?))
}

fn validate_manifest(manifest: &PluginManifest, root: &Path) -> Result<(), String> {
    if manifest.schema_version != SCHEMA_VERSION {
        return Err("不支持的插件清单版本".into());
    }
    normalize_command(&manifest.id)?;
    normalize_command(&manifest.command)?;
    for alias in &manifest.aliases {
        normalize_command(alias)?;
    }
    let entry = safe_relative(&manifest.entry)?;
    if entry.extension().and_then(|value| value.to_str()) != Some("html")
        || !root.join(&entry).is_file()
    {
        return Err("插件入口必须是包内 HTML 文件".into());
    }
    for resource in &manifest.resources {
        if !root.join(safe_relative(resource)?).is_file() {
            return Err(format!("插件资源不存在: {resource}"));
        }
    }
    for resource in manifest.sounds.values() {
        if !root.join(safe_relative(resource)?).is_file() {
            return Err(format!("插件音效不存在: {resource}"));
        }
    }
    Ok(())
}

fn plugin_animation(manifest: &PluginManifest) -> AnimationDefinition {
    AnimationDefinition {
        id: format!("plugin:{}", manifest.id),
        kind: AnimationKind::WebPlugin,
        name: manifest.name.clone(),
        description: format!("Web 插件 {}", manifest.version),
        command: manifest.command.clone(),
        aliases: manifest.aliases.clone(),
        enabled: false,
        renderer: RendererKind::Plugin,
        duration_ms: 15_000,
        target: MonitorTarget::All,
        text: None,
        colors: vec!["#8bb8ff".into()],
        options: serde_json::Map::new(),
        audio: AudioConfig {
            enabled: false,
            resource_id: None,
            volume: 0.6,
            delay_ms: 0,
        },
        transition: None,
        plugin_id: Some(manifest.id.clone()),
    }
}

fn safe_relative(value: &str) -> Result<PathBuf, String> {
    let path = Path::new(value);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!("插件路径不安全: {value}"));
    }
    Ok(path.to_path_buf())
}

fn bundle_local_resources(
    mut html: String,
    root: &Path,
    manifest: &PluginManifest,
) -> Result<String, String> {
    for resource in &manifest.resources {
        let relative = safe_relative(resource)?;
        let path = root.join(&relative);
        let bytes = fs::read(&path)
            .map_err(|error| format!("读取插件资源失败 {}: {error}", relative.display()))?;
        let extension = relative
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if extension == "js" || extension == "mjs" {
            let script = String::from_utf8(bytes)
                .map_err(|_| format!("插件脚本不是 UTF-8: {}", relative.display()))?;
            html = html.replace(
                &format!("<script src=\"{}\"></script>", resource),
                &format!("<script>{script}</script>"),
            );
            html = html.replace(
                &format!("<script type=\"module\" src=\"{}\"></script>", resource),
                &format!("<script type=\"module\">{script}</script>"),
            );
        } else if extension == "css" {
            let style = String::from_utf8(bytes)
                .map_err(|_| format!("插件样式不是 UTF-8: {}", relative.display()))?;
            html = html.replace(
                &format!("<link rel=\"stylesheet\" href=\"{}\">", resource),
                &format!("<style>{style}</style>"),
            );
        } else {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            let data_url = format!(
                "data:{};base64,{}",
                mime,
                base64::engine::general_purpose::STANDARD.encode(bytes)
            );
            html = html.replace(resource, &data_url);
        }
    }
    Ok(html)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::default_config;
    use zip::write::SimpleFileOptions;

    #[test]
    fn rejects_path_traversal() {
        assert!(safe_relative("../secret.txt").is_err());
        assert!(safe_relative("C:/secret.txt").is_err());
        assert!(safe_relative("assets/main.js").is_ok());
    }

    #[test]
    fn installs_valid_plugin_disabled_by_default() {
        let directory = tempfile::tempdir().unwrap();
        let package = directory.path().join("plugin.zip");
        let file = File::create(&package).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        archive.start_file("manifest.json", options).unwrap();
        archive.write_all(br#"{"schemaVersion":1,"id":"safe-plugin","name":"Safe","version":"1.0.0","entry":"index.html","command":"safe","aliases":[],"resources":[],"sounds":{},"capabilities":{"webgl":false,"worker":false}}"#).unwrap();
        archive.start_file("index.html", options).unwrap();
        archive
            .write_all(
                b"<script>parent.postMessage({version:1,sessionId:'s',type:'ready'},'*')</script>",
            )
            .unwrap();
        archive.finish().unwrap();
        let mut config = crate::catalog::default_config();
        let record = install(
            &mut config,
            &directory.path().join("plugins"),
            &package,
            false,
        )
        .unwrap();
        assert!(!record.enabled);
        assert!(
            !config
                .animations
                .iter()
                .find(|animation| animation.plugin_id.as_deref() == Some("safe-plugin"))
                .unwrap()
                .enabled
        );
    }

    #[test]
    fn rejects_archive_traversal() {
        let directory = tempfile::tempdir().unwrap();
        let package = directory.path().join("bad.zip");
        let file = File::create(&package).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file("../escape.js", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"bad").unwrap();
        archive.finish().unwrap();
        let mut config = crate::catalog::default_config();
        assert!(install(
            &mut config,
            &directory.path().join("plugins"),
            &package,
            false
        )
        .is_err());
        assert!(!directory.path().join("escape.js").exists());
    }

    #[test]
    fn rejects_unknown_resource_types_without_leaving_staging_files() {
        let directory = tempfile::tempdir().unwrap();
        let package = directory.path().join("unknown.zip");
        let file = File::create(&package).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file("manifest.json", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(br#"{"schemaVersion":1,"id":"unknown-type","name":"Unknown","version":"1.0.0","entry":"index.html","command":"unknown-type","aliases":[],"resources":[],"sounds":{},"capabilities":{"webgl":false,"worker":false}}"#).unwrap();
        archive
            .start_file("index.html", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"<html></html>").unwrap();
        archive
            .start_file("payload.exe", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"not executable").unwrap();
        archive.finish().unwrap();

        let plugins_root = directory.path().join("plugins");
        let mut config = crate::catalog::default_config();
        assert!(install(&mut config, &plugins_root, &package, false).is_err());
        assert!(!plugins_root.join("unknown-type").exists());
        assert!(!plugins_root
            .read_dir()
            .map(|entries| entries
                .flatten()
                .any(|entry| entry.file_name().to_string_lossy().starts_with(".staging-")))
            .unwrap_or(false));
    }

    #[test]
    fn existing_plugin_requires_explicit_upgrade_confirmation() {
        let directory = tempfile::tempdir().unwrap();
        let package = directory.path().join("plugin.zip");
        let file = File::create(&package).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file("manifest.json", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(br#"{"schemaVersion":1,"id":"repeat-plugin","name":"Repeat","version":"1.0.0","entry":"index.html","command":"repeat-plugin","aliases":[],"resources":[],"sounds":{},"capabilities":{"webgl":false,"worker":false}}"#).unwrap();
        archive
            .start_file("index.html", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"<html></html>").unwrap();
        archive.finish().unwrap();

        let plugins_root = directory.path().join("plugins");
        let mut config = crate::catalog::default_config();
        install(&mut config, &plugins_root, &package, false).unwrap();
        assert!(install(&mut config, &plugins_root, &package, false).is_err());
    }

    #[test]
    fn rejects_unsupported_manifest_version() {
        let directory = tempfile::tempdir().unwrap();
        let package = directory.path().join("future.zip");
        let file = File::create(&package).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file("manifest.json", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(br#"{"schemaVersion":999,"id":"future-plugin","name":"Future","version":"1.0.0","entry":"index.html","command":"future-plugin","aliases":[],"resources":[],"sounds":{},"capabilities":{"webgl":false,"worker":false}}"#).unwrap();
        archive
            .start_file("index.html", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"<html></html>").unwrap();
        archive.finish().unwrap();

        let mut config = default_config();
        assert!(install(
            &mut config,
            &directory.path().join("plugins"),
            &package,
            false
        )
        .is_err());
    }
}

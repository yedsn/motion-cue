use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::catalog::{command_index, default_config, validate_animation};
use crate::models::{AppConfig, SCHEMA_VERSION};

pub struct ConfigStore {
    path: PathBuf,
    backup_path: PathBuf,
    state: RwLock<AppConfig>,
}

impl ConfigStore {
    pub fn load(path: PathBuf) -> Result<(Self, Option<String>), String> {
        let backup_path = path.with_extension("backup.json");
        let (config, recovery) = match read_valid(&path) {
            Ok(config) => (config, None),
            Err(primary_error) => match read_valid(&backup_path) {
                Ok(config) => (
                    config,
                    Some(format!("当前配置无效，已从备份恢复: {primary_error}")),
                ),
                Err(_) => (
                    default_config(),
                    Some(format!("当前配置无效，已恢复内置默认配置: {primary_error}")),
                ),
            },
        };
        let store = Self {
            path,
            backup_path,
            state: RwLock::new(config),
        };
        store.save_current()?;
        Ok((store, recovery))
    }

    pub fn get(&self) -> AppConfig {
        self.state.read().unwrap().clone()
    }

    pub fn replace(&self, config: AppConfig) -> Result<AppConfig, String> {
        validate_config(&config)?;
        self.persist(&config)?;
        *self.state.write().unwrap() = config.clone();
        Ok(config)
    }

    pub fn update<F>(&self, update: F) -> Result<AppConfig, String>
    where
        F: FnOnce(&mut AppConfig) -> Result<(), String>,
    {
        let mut next = self.get();
        update(&mut next)?;
        self.replace(next)
    }

    fn save_current(&self) -> Result<(), String> {
        let config = self.get();
        self.persist(&config)
    }

    fn persist(&self, config: &AppConfig) -> Result<(), String> {
        validate_config(&config)?;
        let parent = self
            .path
            .parent()
            .ok_or_else(|| "配置目录无效".to_string())?;
        fs::create_dir_all(parent).map_err(|error| format!("创建配置目录失败: {error}"))?;
        let temp = self.path.with_extension("tmp");
        let content = serde_json::to_vec_pretty(config)
            .map_err(|error| format!("序列化配置失败: {error}"))?;
        fs::write(&temp, content).map_err(|error| format!("写入临时配置失败: {error}"))?;
        if self.path.exists() {
            let _ = fs::copy(&self.path, &self.backup_path);
        }
        if self.path.exists() {
            fs::remove_file(&self.path).map_err(|error| format!("替换旧配置失败: {error}"))?;
        }
        fs::rename(&temp, &self.path).map_err(|error| format!("原子替换配置失败: {error}"))?;
        if !self.backup_path.exists() {
            let _ = fs::copy(&self.path, &self.backup_path);
        }
        Ok(())
    }
}

pub fn validate_config(config: &AppConfig) -> Result<(), String> {
    if config.schema_version != SCHEMA_VERSION {
        return Err(format!("不支持的配置版本: {}", config.schema_version));
    }
    if !(10..=2000).contains(&config.settings.diagnostics_retention) {
        return Err("诊断保留数量必须在 10 到 2000 之间".into());
    }
    if !(100..=60_000).contains(&config.settings.default_duration_ms)
        || !(0.0..=1.0).contains(&config.settings.default_volume)
    {
        return Err("全局动画默认值超出安全范围".into());
    }
    for animation in &config.animations {
        validate_animation(animation)?;
    }
    command_index(config)?;
    Ok(())
}

fn read_valid(path: &Path) -> Result<AppConfig, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("读取 {} 失败: {error}", path.display()))?;
    let config: AppConfig = serde_json::from_str(&content)
        .map_err(|error| format!("解析 {} 失败: {error}", path.display()))?;
    validate_config(&config)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_future_schema() {
        let mut config = default_config();
        config.schema_version = 999;
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn restores_backup_when_primary_is_corrupt() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.json");
        fs::write(&path, "bad json").unwrap();
        fs::write(
            path.with_extension("backup.json"),
            serde_json::to_vec(&default_config()).unwrap(),
        )
        .unwrap();
        let (store, recovery) = ConfigStore::load(path).unwrap();
        assert!(recovery.is_some());
        assert_eq!(store.get().schema_version, SCHEMA_VERSION);
    }
}

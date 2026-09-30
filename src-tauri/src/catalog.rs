use std::collections::{BTreeMap, HashMap};

use crate::models::{
    default_transition, AnimationDefinition, AnimationKind, AppConfig, AudioConfig, MonitorTarget,
    RendererKind, SCHEMA_VERSION,
};

pub fn default_config() -> AppConfig {
    AppConfig {
        schema_version: SCHEMA_VERSION,
        settings: crate::models::GlobalSettings {
            muted: false,
            overlay_topmost: true,
            default_target: MonitorTarget::All,
            default_duration_ms: 3000,
            default_volume: 0.6,
            default_transition: default_transition(),
            diagnostics_retention: 200,
        },
        animations: builtins(),
        plugins: Vec::new(),
    }
}

pub fn builtins() -> Vec<AnimationDefinition> {
    vec![
        animation(
            "confetti",
            "全屏彩纸",
            "经典全屏彩纸庆祝",
            "confetti",
            RendererKind::Confetti,
            3200,
            vec!["#d9ff9f", "#91f5d4", "#ffd27a", "#8bb8ff"],
            None,
        ),
        animation(
            "task-complete",
            "任务完成",
            "左右底部彩带与顶部完成提示",
            "task-complete",
            RendererKind::Completion,
            3600,
            vec!["#d9ff9f", "#91f5d4", "#ffd27a", "#f8fff1", "#8bb8ff"],
            Some("builtin/completion-success.wav"),
        ),
        animation(
            "success",
            "成功",
            "绿色光环与成功标记",
            "success",
            RendererKind::Badge,
            2200,
            vec!["#34c584", "#c9ef8f"],
            Some("builtin/soft-chime.wav"),
        ),
        animation(
            "milestone",
            "里程碑",
            "更强烈的里程碑庆祝",
            "milestone",
            RendererKind::Ring,
            4200,
            vec!["#ffd27a", "#ff8e72", "#d9ff9f"],
            Some("builtin/bright-pop.wav"),
        ),
        animation(
            "focus-start",
            "开始专注",
            "轻量聚焦提示",
            "focus-start",
            RendererKind::Pulse,
            1800,
            vec!["#8bb8ff", "#91f5d4"],
            None,
        ),
        animation(
            "error",
            "错误",
            "克制的错误抖动提示",
            "error",
            RendererKind::Shake,
            1800,
            vec!["#f06b61", "#ffb4ad"],
            None,
        ),
        animation(
            "silent-confetti",
            "静音彩纸",
            "不播放音效的彩纸",
            "silent-confetti",
            RendererKind::Confetti,
            3000,
            vec!["#d9ff9f", "#91f5d4", "#ffd27a"],
            None,
        ),
    ]
}

fn animation(
    id: &str,
    name: &str,
    description: &str,
    command: &str,
    renderer: RendererKind,
    duration_ms: u64,
    colors: Vec<&str>,
    sound: Option<&str>,
) -> AnimationDefinition {
    let mut options = serde_json::Map::new();
    options.insert("particleCount".into(), serde_json::json!(120));
    AnimationDefinition {
        id: id.into(),
        kind: AnimationKind::Builtin,
        name: name.into(),
        description: description.into(),
        command: command.into(),
        aliases: Vec::new(),
        enabled: true,
        renderer,
        duration_ms,
        target: MonitorTarget::All,
        text: Some(name.into()),
        colors: colors.into_iter().map(String::from).collect(),
        options,
        audio: AudioConfig {
            enabled: sound.is_some(),
            resource_id: sound.map(str::to_string),
            volume: 0.62,
            delay_ms: 0,
        },
        transition: None,
        plugin_id: None,
    }
}

pub fn normalize_command(command: &str) -> Result<String, String> {
    let command = command.trim().to_ascii_lowercase();
    if command.is_empty()
        || command.len() > 64
        || !command
            .chars()
            .enumerate()
            .all(|(index, ch)| ch.is_ascii_alphanumeric() || (index > 0 && matches!(ch, '-' | '_')))
    {
        return Err("命令只能包含字母、数字、短横线和下划线，最长 64 个字符".into());
    }
    Ok(command)
}

pub fn validate_animation(animation: &AnimationDefinition) -> Result<(), String> {
    normalize_command(&animation.command)?;
    for alias in &animation.aliases {
        normalize_command(alias)?;
    }
    if !(100..=60_000).contains(&animation.duration_ms) {
        return Err("动画时长必须在 100 到 60000 毫秒之间".into());
    }
    if !(0.0..=1.0).contains(&animation.audio.volume)
        || animation.audio.delay_ms > animation.duration_ms
    {
        return Err("音效配置超出安全范围".into());
    }
    if let Some(transition) = &animation.transition {
        validate_transition(
            transition.enter_ms,
            transition.exit_ms,
            animation.duration_ms,
        )?;
    }
    if animation.colors.is_empty() || animation.colors.iter().any(|color| !valid_color(color)) {
        return Err("动画颜色无效".into());
    }
    if animation.text.as_ref().is_some_and(|text| text.len() > 200) {
        return Err("动画文字不能超过 200 个字符".into());
    }
    for forbidden in ["script", "url", "html"] {
        if animation.options.contains_key(forbidden) {
            return Err(format!("配置型动画不能包含 {forbidden}"));
        }
    }
    if let Some(count) = animation
        .options
        .get("particleCount")
        .and_then(|value| value.as_u64())
    {
        if !(1..=500).contains(&count) {
            return Err("粒子数量必须在 1 到 500 之间".into());
        }
    }
    if let Some(angle) = animation
        .options
        .get("angle")
        .and_then(|value| value.as_f64())
    {
        if !(0.0..=180.0).contains(&angle) {
            return Err("喷发方向必须在 0 到 180 度之间".into());
        }
    }
    Ok(())
}

pub fn validate_transition(enter_ms: u64, exit_ms: u64, duration_ms: u64) -> Result<(), String> {
    if enter_ms > 5_000 || exit_ms > 5_000 || enter_ms.saturating_add(exit_ms) > duration_ms {
        return Err("过渡时间必须不超过 5000 毫秒，且进入与结尾过渡总时长不能超过动画时长".into());
    }
    Ok(())
}

pub fn command_index(config: &AppConfig) -> Result<HashMap<String, String>, String> {
    let mut index = HashMap::new();
    for animation in config
        .animations
        .iter()
        .filter(|animation| animation.enabled)
    {
        for raw in std::iter::once(&animation.command).chain(animation.aliases.iter()) {
            let command = normalize_command(raw)?;
            if let Some(existing) = index.insert(command.clone(), animation.id.clone()) {
                return Err(format!("命令 {command} 与动画 {existing} 冲突"));
            }
        }
    }
    Ok(index)
}

pub fn resolve(config: &AppConfig, command: &str) -> Result<AnimationDefinition, String> {
    let command = normalize_command(command)?;
    let index = command_index(config)?;
    let id = index
        .get(&command)
        .ok_or_else(|| format!("未知或已禁用命令: {command}"))?;
    config
        .animations
        .iter()
        .find(|animation| &animation.id == id)
        .cloned()
        .ok_or_else(|| "动画目录索引失效".into())
}

pub fn public_command_list(config: &AppConfig) -> Vec<BTreeMap<&'static str, serde_json::Value>> {
    config
        .animations
        .iter()
        .filter(|animation| animation.enabled)
        .map(|animation| {
            BTreeMap::from([
                ("id", serde_json::json!(animation.id)),
                ("name", serde_json::json!(animation.name)),
                ("command", serde_json::json!(animation.command)),
                ("aliases", serde_json::json!(animation.aliases)),
                (
                    "parameters",
                    serde_json::json!(["text", "color", "duration"]),
                ),
            ])
        })
        .collect()
}

fn valid_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].chars().all(|ch| ch.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_are_case_insensitive() {
        assert_eq!(
            resolve(&default_config(), "TASK-COMPLETE").unwrap().id,
            "task-complete"
        );
    }

    #[test]
    fn conflict_is_rejected() {
        let mut config = default_config();
        config.animations[1].aliases.push("confetti".into());
        assert!(command_index(&config).is_err());
    }

    #[test]
    fn unsafe_options_are_rejected() {
        let mut animation = builtins().remove(0);
        animation
            .options
            .insert("script".into(), serde_json::json!("alert(1)"));
        assert!(validate_animation(&animation).is_err());
    }

    #[test]
    fn invalid_command_names_are_rejected() {
        assert!(normalize_command("-starts-with-dash").is_err());
        assert!(normalize_command("contains space").is_err());
        assert!(normalize_command(&"x".repeat(65)).is_err());
    }

    #[test]
    fn duration_and_audio_ranges_are_rejected() {
        let mut animation = builtins().remove(0);
        animation.duration_ms = 60_001;
        assert!(validate_animation(&animation).is_err());

        let mut animation = builtins().remove(0);
        animation.audio.volume = 1.1;
        assert!(validate_animation(&animation).is_err());

        let mut animation = builtins().remove(0);
        animation.audio.delay_ms = animation.duration_ms + 1;
        assert!(validate_animation(&animation).is_err());
    }
}

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
            target_frame_rate: 60,
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
            Some("audio/completion-success.wav"),
        ),
        animation(
            "success",
            "成功",
            "绿色光环与成功标记",
            "success",
            RendererKind::Badge,
            2200,
            vec!["#34c584", "#c9ef8f"],
            Some("audio/soft-chime.wav"),
        ),
        animation(
            "focus-start",
            "开始专注",
            "轻量聚焦提示",
            "focus-start",
            RendererKind::FocusSpotlight,
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
        animation(
            "material-flow",
            "全屏材质",
            "流动粒子材质反馈",
            "material-flow",
            RendererKind::MaterialFlow,
            3600,
            vec!["#8bb8ff", "#91f5d4", "#f8fff1"],
            None,
        ),
        animation(
            "corner-fireworks",
            "角落烟花",
            "从屏幕角落发射的庆祝烟花",
            "corner-fireworks",
            RendererKind::CornerFireworks,
            3200,
            vec!["#ffd27a", "#ff8e72", "#d9ff9f", "#8bb8ff"],
            Some("audio/bright-pop.wav"),
        ),
        animation(
            "focus-spotlight",
            "专注光罩",
            "克制的全屏聚焦光罩",
            "focus-spotlight",
            RendererKind::FocusSpotlight,
            1800,
            vec!["#8bb8ff", "#91f5d4"],
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
    let options = default_options(&renderer);
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

fn default_options(renderer: &RendererKind) -> serde_json::Map<String, serde_json::Value> {
    let mut options = serde_json::Map::new();
    match renderer {
        RendererKind::Confetti | RendererKind::Completion => {
            options.insert("particleCount".into(), serde_json::json!(120));
            options.insert("particleSize".into(), serde_json::json!(1.0));
            options.insert("angle".into(), serde_json::json!(58));
        }
        RendererKind::CornerFireworks => {
            options.insert("particleCount".into(), serde_json::json!(72));
            options.insert("particleSize".into(), serde_json::json!(1.0));
            options.insert("burstCount".into(), serde_json::json!(4));
            options.insert("spread".into(), serde_json::json!(72));
            options.insert("speed".into(), serde_json::json!(1.0));
            options.insert(
                "corners".into(),
                serde_json::json!(["bottom-left", "bottom-right"]),
            );
        }
        RendererKind::MaterialFlow => {
            options.insert("intensity".into(), serde_json::json!(0.72));
            options.insert("speed".into(), serde_json::json!(0.8));
            options.insert("density".into(), serde_json::json!(120));
            options.insert("brightness".into(), serde_json::json!(0.72));
        }
        RendererKind::FocusSpotlight => {
            options.insert("spotlightSize".into(), serde_json::json!(0.42));
            options.insert("dimAmount".into(), serde_json::json!(0.38));
            options.insert("pulseStrength".into(), serde_json::json!(0.28));
            options.insert("showText".into(), serde_json::json!(true));
        }
        RendererKind::Badge
        | RendererKind::Pulse
        | RendererKind::Ring
        | RendererKind::Shake
        | RendererKind::Plugin => {}
    }
    options
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
    validate_options(animation)?;
    Ok(())
}

fn validate_options(animation: &AnimationDefinition) -> Result<(), String> {
    validate_number_option(
        animation,
        "particleCount",
        1.0,
        500.0,
        "粒子数量必须在 1 到 500 之间",
    )?;
    validate_number_option(
        animation,
        "particleSize",
        0.4,
        2.5,
        "粒子大小必须在 0.4 到 2.5 之间",
    )?;
    validate_number_option(
        animation,
        "angle",
        0.0,
        180.0,
        "喷发方向必须在 0 到 180 度之间",
    )?;
    match animation.renderer {
        RendererKind::MaterialFlow => {
            validate_number_option(animation, "intensity", 0.0, 1.0, "强度必须在 0 到 1 之间")?;
            validate_number_option(animation, "speed", 0.1, 3.0, "速度必须在 0.1 到 3 之间")?;
            validate_number_option(animation, "density", 1.0, 300.0, "密度必须在 1 到 300 之间")?;
            validate_number_option(animation, "brightness", 0.0, 1.0, "亮度必须在 0 到 1 之间")?;
        }
        RendererKind::CornerFireworks => {
            validate_number_option(
                animation,
                "burstCount",
                1.0,
                12.0,
                "烟花批次必须在 1 到 12 之间",
            )?;
            validate_number_option(
                animation,
                "spread",
                1.0,
                180.0,
                "扩散范围必须在 1 到 180 之间",
            )?;
            validate_number_option(animation, "speed", 0.1, 3.0, "速度必须在 0.1 到 3 之间")?;
            validate_corners(animation)?;
        }
        RendererKind::FocusSpotlight => {
            validate_number_option(
                animation,
                "spotlightSize",
                0.1,
                1.0,
                "光罩大小必须在 0.1 到 1 之间",
            )?;
            validate_number_option(
                animation,
                "dimAmount",
                0.0,
                0.75,
                "压暗程度必须在 0 到 0.75 之间",
            )?;
            validate_number_option(
                animation,
                "pulseStrength",
                0.0,
                1.0,
                "呼吸强度必须在 0 到 1 之间",
            )?;
            if let Some(value) = animation.options.get("showText") {
                if !value.is_boolean() {
                    return Err("文字显示开关必须为布尔值".into());
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_number_option(
    animation: &AnimationDefinition,
    key: &str,
    min: f64,
    max: f64,
    message: &str,
) -> Result<(), String> {
    let Some(value) = animation.options.get(key) else {
        return Ok(());
    };
    let Some(number) = value.as_f64() else {
        return Err(message.into());
    };
    if number < min || number > max {
        return Err(message.into());
    }
    Ok(())
}

fn validate_corners(animation: &AnimationDefinition) -> Result<(), String> {
    let Some(value) = animation.options.get("corners") else {
        return Ok(());
    };
    let Some(items) = value.as_array() else {
        return Err("发射角落配置无效".into());
    };
    if items.is_empty() || items.len() > 4 {
        return Err("发射角落配置无效".into());
    }
    for item in items {
        match item.as_str() {
            Some("top-left" | "top-right" | "bottom-left" | "bottom-right") => {}
            _ => return Err("发射角落配置无效".into()),
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
    fn default_catalog_has_revised_effects() {
        let ids = builtins()
            .into_iter()
            .map(|animation| animation.id)
            .collect::<Vec<_>>();
        assert_eq!(ids.len(), 9);
        assert!(ids.contains(&"material-flow".into()));
        assert!(ids.contains(&"corner-fireworks".into()));
        assert!(ids.contains(&"focus-spotlight".into()));
        assert!(!ids.contains(&"milestone".into()));
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
    fn renderer_options_are_validated() {
        let mut animation = builtins()
            .into_iter()
            .find(|animation| animation.id == "material-flow")
            .unwrap();
        animation
            .options
            .insert("density".into(), serde_json::json!(301));
        assert!(validate_animation(&animation).is_err());

        let mut animation = builtins()
            .into_iter()
            .find(|animation| animation.id == "corner-fireworks")
            .unwrap();
        animation
            .options
            .insert("corners".into(), serde_json::json!(["center"]));
        assert!(validate_animation(&animation).is_err());

        let mut animation = builtins()
            .into_iter()
            .find(|animation| animation.id == "confetti")
            .unwrap();
        animation
            .options
            .insert("particleSize".into(), serde_json::json!(2.6));
        assert!(validate_animation(&animation).is_err());
    }

    #[test]
    fn legacy_milestone_remains_valid() {
        let animation = animation(
            "milestone",
            "里程碑",
            "旧版里程碑",
            "milestone",
            RendererKind::Ring,
            4200,
            vec!["#ffd27a", "#ff8e72"],
            Some("audio/bright-pop.wav"),
        );
        assert!(validate_animation(&animation).is_ok());
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

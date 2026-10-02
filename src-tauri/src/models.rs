use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioConfig {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_id: Option<String>,
    pub volume: f64,
    pub delay_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum TransitionKind {
    None,
    Fade,
    Scale,
    SlideUp,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MotionTransitionConfig {
    pub enter: TransitionKind,
    pub exit: TransitionKind,
    pub enter_ms: u64,
    pub exit_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum AnimationKind {
    Builtin,
    Configured,
    WebPlugin,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum RendererKind {
    Confetti,
    Badge,
    Pulse,
    Ring,
    Shake,
    Completion,
    MaterialFlow,
    CornerFireworks,
    FocusSpotlight,
    Plugin,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MonitorTarget {
    All,
    Primary,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnimationDefinition {
    pub id: String,
    pub kind: AnimationKind,
    pub name: String,
    pub description: String,
    pub command: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub enabled: bool,
    pub renderer: RendererKind,
    pub duration_ms: u64,
    pub target: MonitorTarget,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub colors: Vec<String>,
    #[serde(default)]
    pub options: serde_json::Map<String, serde_json::Value>,
    pub audio: AudioConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<MotionTransitionConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSettings {
    pub muted: bool,
    #[serde(default = "default_overlay_topmost")]
    pub overlay_topmost: bool,
    #[serde(default = "default_target_frame_rate")]
    pub target_frame_rate: u32,
    pub default_target: MonitorTarget,
    #[serde(default = "default_duration_ms")]
    pub default_duration_ms: u64,
    #[serde(default = "default_volume")]
    pub default_volume: f64,
    #[serde(default = "default_transition")]
    pub default_transition: MotionTransitionConfig,
    pub diagnostics_retention: usize,
}

fn default_duration_ms() -> u64 {
    3000
}
fn default_volume() -> f64 {
    0.6
}

fn default_overlay_topmost() -> bool {
    true
}

fn default_target_frame_rate() -> u32 {
    60
}

pub fn default_transition() -> MotionTransitionConfig {
    MotionTransitionConfig {
        enter: TransitionKind::Fade,
        exit: TransitionKind::Fade,
        enter_ms: 180,
        exit_ms: 420,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PluginCapabilities {
    pub webgl: bool,
    pub worker: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub entry: String,
    pub command: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub resources: Vec<String>,
    #[serde(default)]
    pub sounds: BTreeMap<String, String>,
    pub capabilities: PluginCapabilities,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PluginRecord {
    pub manifest: PluginManifest,
    pub enabled: bool,
    pub source: String,
    pub installed_path: String,
    pub size_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub schema_version: u32,
    pub settings: GlobalSettings,
    pub animations: Vec<AnimationDefinition>,
    #[serde(default)]
    pub plugins: Vec<PluginRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvocationRequest {
    pub schema_version: u32,
    pub source: String,
    pub command: String,
    #[serde(default)]
    pub params: BTreeMap<String, String>,
    pub requested_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackRequest {
    pub session_id: String,
    pub animation: AnimationDefinition,
    pub params: BTreeMap<String, String>,
    pub started_at: u64,
    pub play_audio: bool,
    pub target_frame_rate: u32,
    pub transition: MotionTransitionConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_data_url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticEntry {
    pub id: String,
    pub timestamp: u64,
    pub level: String,
    pub category: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRuntime {
    pub manifest: PluginManifest,
    pub html: String,
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versioned_schemas_round_trip() {
        let config = crate::catalog::default_config();
        let decoded: AppConfig =
            serde_json::from_value(serde_json::to_value(&config).unwrap()).unwrap();
        assert_eq!(decoded.schema_version, SCHEMA_VERSION);
        assert_eq!(decoded.animations.len(), 9);

        let request = InvocationRequest {
            schema_version: SCHEMA_VERSION,
            source: "cli".into(),
            command: "success".into(),
            params: [("text".into(), "Done".into())].into(),
            requested_at: 1,
        };
        let decoded: InvocationRequest =
            serde_json::from_value(serde_json::to_value(&request).unwrap()).unwrap();
        assert_eq!(decoded.schema_version, SCHEMA_VERSION);
        assert_eq!(decoded.params["text"], "Done");

        let diagnostic = DiagnosticEntry {
            id: "d".into(),
            timestamp: 1,
            level: "info".into(),
            category: "playback".into(),
            message: "ok".into(),
            command: Some("success".into()),
        };
        let decoded: DiagnosticEntry =
            serde_json::from_value(serde_json::to_value(&diagnostic).unwrap()).unwrap();
        assert_eq!(decoded.command.as_deref(), Some("success"));

        let manifest = PluginManifest {
            schema_version: SCHEMA_VERSION,
            id: "safe-plugin".into(),
            name: "Safe".into(),
            version: "1.0.0".into(),
            entry: "index.html".into(),
            command: "safe-plugin".into(),
            aliases: vec!["safe".into()],
            resources: vec!["index.js".into()],
            sounds: [("impact".into(), "impact.wav".into())].into(),
            capabilities: PluginCapabilities {
                webgl: false,
                worker: false,
            },
        };
        let decoded: PluginManifest =
            serde_json::from_value(serde_json::to_value(&manifest).unwrap()).unwrap();
        assert_eq!(decoded.schema_version, SCHEMA_VERSION);
        assert_eq!(decoded.sounds["impact"], "impact.wav");
    }
}

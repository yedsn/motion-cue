export const SCHEMA_VERSION = 1;

export type AnimationKind = "builtin" | "configured" | "web-plugin";
export type RendererKind = "confetti" | "badge" | "pulse" | "ring" | "shake" | "completion" | "plugin";
export type MonitorTarget = "all" | "primary";
export type TransitionKind = "none" | "fade" | "scale" | "slide-up";

export type AudioConfig = {
  enabled: boolean;
  resourceId?: string;
  volume: number;
  delayMs: number;
};

export type MotionTransitionConfig = {
  enter: TransitionKind;
  exit: TransitionKind;
  enterMs: number;
  exitMs: number;
};

export type AnimationDefinition = {
  id: string;
  kind: AnimationKind;
  name: string;
  description: string;
  command: string;
  aliases: string[];
  enabled: boolean;
  renderer: RendererKind;
  durationMs: number;
  target: MonitorTarget;
  text?: string;
  colors: string[];
  options: Record<string, unknown>;
  audio: AudioConfig;
  transition?: MotionTransitionConfig;
  pluginId?: string;
};

export type GlobalSettings = {
  muted: boolean;
  overlayTopmost: boolean;
  targetFrameRate: number;
  defaultTarget: MonitorTarget;
  defaultDurationMs: number;
  defaultVolume: number;
  defaultTransition: MotionTransitionConfig;
  diagnosticsRetention: number;
};

export type PluginCapabilities = { webgl: boolean; worker: boolean };

export type PluginManifest = {
  schemaVersion: number;
  id: string;
  name: string;
  version: string;
  entry: string;
  command: string;
  aliases: string[];
  resources: string[];
  sounds: Record<string, string>;
  capabilities: PluginCapabilities;
};

export type PluginRecord = {
  manifest: PluginManifest;
  enabled: boolean;
  source: string;
  installedPath: string;
  sizeBytes: number;
  lastError?: string;
};

export type AppConfig = {
  schemaVersion: number;
  settings: GlobalSettings;
  animations: AnimationDefinition[];
  plugins: PluginRecord[];
};

export type InvocationSource = "protocol" | "cli" | "management" | "tray" | "single-instance";

export type InvocationRequest = {
  schemaVersion: number;
  source: InvocationSource;
  command: string;
  params: Record<string, string>;
  requestedAt: number;
};

export type PlaybackRequest = {
  sessionId: string;
  animation: AnimationDefinition;
  params: Record<string, string>;
  startedAt: number;
  playAudio: boolean;
  targetFrameRate: number;
  transition: MotionTransitionConfig;
  audioDataUrl?: string;
};

export type DiagnosticEntry = {
  id: string;
  timestamp: number;
  level: "info" | "warning" | "error";
  category: string;
  message: string;
  command?: string;
};

export type PluginHostMessage =
  | { version: 1; sessionId: string; type: "ready" }
  | { version: 1; sessionId: string; type: "complete" }
  | { version: 1; sessionId: string; type: "error"; message: string }
  | { version: 1; sessionId: string; type: "sound"; soundId: string };

export type PluginRuntime = {
  manifest: PluginManifest;
  html: string;
};

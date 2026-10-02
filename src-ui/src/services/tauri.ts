import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppConfig, AnimationDefinition, AppUpdateCheckResult, AppUpdateEventPayload, AudioLibraryItem, DiagnosticEntry, PlaybackRequest, PluginRecord, PluginRuntime } from "../types";

export const getConfig = () => invoke<AppConfig>("config_get");
export const saveAnimation = (animation: AnimationDefinition) => invoke<AppConfig>("animation_save", { animation });
export const deleteAnimation = (id: string) => invoke<AppConfig>("animation_delete", { id });
export const resetAnimation = (id: string) => invoke<AppConfig>("animation_reset", { id });
export const updateSettings = (settings: AppConfig["settings"]) => invoke<AppConfig>("settings_update", { settings });
export const playAnimation = (command: string, params: Record<string, string> = {}) => invoke<void>("animation_play", { command, params });
export const previewAnimation = (animation: AnimationDefinition, params: Record<string, string> = {}) => invoke<void>("animation_preview", { animation, params });
export const stopAnimations = () => invoke<void>("animation_stop_all");
export const importAudio = (path: string) => invoke<string>("audio_import", { path });
export const listAudioLibrary = () => invoke<AudioLibraryItem[]>("audio_library_list");
export const getAudioResource = (resourceId: string) => invoke<string>("audio_resource", { resourceId });
export const exportAnimation = (id: string, path: string) => invoke<void>("animation_export", { id, path });
export const importAnimation = (path: string) => invoke<AppConfig>("animation_import", { path });
export const installPlugin = (path: string, allowUpgrade = false) => invoke<AppConfig>("plugin_install", { path, allowUpgrade });
export const setPluginEnabled = (id: string, enabled: boolean) => invoke<AppConfig>("plugin_set_enabled", { id, enabled });
export const uninstallPlugin = (id: string) => invoke<AppConfig>("plugin_uninstall", { id });
export const getPluginRuntime = (id: string) => invoke<PluginRuntime>("plugin_runtime_get", { id });
export const requestPluginSound = (pluginId: string, soundId: string) => invoke<string>("plugin_sound", { pluginId, soundId });
export const getCurrentPlayback = () => invoke<PlaybackRequest | null>("playback_current");
export const getDiagnostics = () => invoke<DiagnosticEntry[]>("diagnostics_get");
export const recordDiagnostic = (level: string, category: string, message: string) => invoke<void>("diagnostics_record", { level, category, message });
export const getPlugins = () => invoke<PluginRecord[]>("plugins_get");
export const reportPlaybackComplete = (sessionId: string) => invoke<void>("playback_complete", { sessionId });
export const reportPlaybackError = (sessionId: string, message: string) => invoke<void>("playback_error", { sessionId, message });
export const getAppVersion = () => invoke<string>("get_app_version");
export const checkAppUpdate = () => invoke<AppUpdateCheckResult>("check_app_update");
export const downloadAndInstallUpdate = () => invoke<void>("download_and_install_update");
export const restartApp = () => invoke<void>("restart_app");

export function onPlayback(callback: (request: PlaybackRequest) => void): Promise<UnlistenFn> {
  return listen<PlaybackRequest>("motioncue://playback", (event) => callback(event.payload));
}

export function onPlaybackStop(callback: (sessionId?: string) => void): Promise<UnlistenFn> {
  return listen<{ sessionId?: string }>("motioncue://stop", (event) => callback(event.payload.sessionId));
}

export function onAppUpdateEvent(callback: (payload: AppUpdateEventPayload) => void): Promise<UnlistenFn> {
  return listen<AppUpdateEventPayload>("app-update-event", (event) => callback(event.payload));
}

export function onAppUpdateCheckRequest(callback: () => void): Promise<UnlistenFn> {
  return listen("motioncue://check-update", callback);
}

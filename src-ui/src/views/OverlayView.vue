<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { getCurrentPlayback, getPluginRuntime, onPlayback, onPlaybackStop, recordDiagnostic, reportPlaybackComplete, reportPlaybackError, requestPluginSound } from "../services/tauri";
import { renderAnimation } from "../services/renderers";
import { injectPluginPolicy } from "../services/pluginPolicy";
import { Howl } from "howler";
import { parsePluginHostMessage } from "../services/schemas";
import type { PlaybackRequest } from "../types";

const root = ref<HTMLElement>();
let runtime: { stop: () => void } | undefined;
let currentSession = "";
let currentRequest: PlaybackRequest | undefined;
let unlistenPlay: (() => void) | undefined;
let unlistenStop: (() => void) | undefined;
let completionTimer: number | undefined;
let pluginFrame: HTMLIFrameElement | undefined;
let messageCount = 0;
let messageWindowStarted = 0;
let readyTimer: number | undefined;
let currentPollTimer: number | undefined;

function buildSandboxDocument(html: string, capabilities: { webgl: boolean; worker: boolean }) {
  const document = new DOMParser().parseFromString(injectPluginPolicy(html, capabilities), "text/html");
  const source = Array.from(document.scripts).map((script) => script.textContent ?? "").join("\n;");
  for (const script of Array.from(document.scripts)) script.remove();
  const bytes = new TextEncoder().encode(source);
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  const script = document.createElement("script");
  script.src = `data:text/javascript;base64,${btoa(binary)}`;
  document.body.append(script);
  return `<!doctype html>${document.documentElement.outerHTML}`;
}

function stop() {
  runtime?.stop();
  runtime = undefined;
  pluginFrame?.remove();
  pluginFrame = undefined;
  if (readyTimer) window.clearTimeout(readyTimer);
  readyTimer = undefined;
  if (completionTimer) window.clearTimeout(completionTimer);
  completionTimer = undefined;
  if (currentPollTimer) window.clearTimeout(currentPollTimer);
  currentPollTimer = undefined;
  currentSession = "";
  currentRequest = undefined;
}

async function play(request: PlaybackRequest) {
  if (currentSession === request.sessionId) return;
  stop();
  currentSession = request.sessionId;
  currentRequest = request;
  if (!root.value) return;
  try {
    if (request.animation.renderer === "plugin" && request.animation.pluginId) {
      await playPlugin(request);
    } else {
      runtime = renderAnimation(root.value, request);
    }
  } catch (error) {
    await reportPlaybackError(request.sessionId, `插件播放初始化失败: ${String(error)}`);
    stop();
    return;
  }
  completionTimer = window.setTimeout(() => reportPlaybackComplete(request.sessionId), request.animation.durationMs);
}

async function playIfStillCurrent(request: PlaybackRequest) {
  const current = await getCurrentPlayback();
  if (current?.sessionId === request.sessionId) await play(current);
}

async function playCurrentWhenAvailable(attempt = 0) {
  const current = await getCurrentPlayback();
  if (current) {
    await play(current);
    return;
  }
  if (attempt < 20) {
    currentPollTimer = window.setTimeout(() => void playCurrentWhenAvailable(attempt + 1), 100);
  }
}

async function playPlugin(request: PlaybackRequest) {
  if (!root.value || !request.animation.pluginId) return;
  const plugin = await getPluginRuntime(request.animation.pluginId);
  const frame = document.createElement("iframe");
  let frameLoaded = false;
  frame.className = "plugin-frame";
  frame.setAttribute("sandbox", "allow-scripts");
  frame.referrerPolicy = "no-referrer";
  frame.dataset.pluginId = request.animation.pluginId;
  const postPlay = () => {
    frame.contentWindow?.postMessage({ version: 1, sessionId: request.sessionId, type: "play", payload: { params: request.params, viewport: { width: innerWidth, height: innerHeight } } }, "*");
  };
  frame.addEventListener("load", () => {
    frameLoaded = true;
    postPlay();
  });
  frame.addEventListener("error", () => void reportPlaybackError(request.sessionId, "插件 iframe 加载失败"), { once: true });
  pluginFrame = frame;
  frame.srcdoc = buildSandboxDocument(plugin.html, plugin.manifest.capabilities);
  root.value.append(frame);
  for (const delay of [50, 150, 450, 900, 1500]) window.setTimeout(postPlay, delay);
  readyTimer = window.setTimeout(async () => {
    await reportPlaybackError(request.sessionId, `插件未在就绪时限内响应: loaded=${frameLoaded}`);
    stop();
  }, 3000);
}

async function handlePluginMessage(event: MessageEvent) {
  if (!pluginFrame) return;
  if (event.source && event.source !== pluginFrame.contentWindow) return;
  let serialized = "";
  try { serialized = JSON.stringify(event.data); } catch { return; }
  if (serialized.length > 8192) {
    await reportPlaybackError(currentSession, "插件消息超过大小限制");
    stop();
    return;
  }
  const now = Date.now();
  if (now - messageWindowStarted > 1000) { messageWindowStarted = now; messageCount = 0; }
  messageCount += 1;
  if (messageCount > 40) {
    await reportPlaybackError(currentSession, "插件消息频率超过限制");
    stop();
    return;
  }
  const message = parsePluginHostMessage(event.data);
  if (!message || message.sessionId !== currentSession) {
    void recordDiagnostic("warning", "plugin", `插件消息无效或会话不匹配: ${serialized.slice(0, 180)}`);
    return;
  }
  if (message.type === "ready") {
    if (readyTimer) window.clearTimeout(readyTimer);
    readyTimer = undefined;
  }
  if (message.type === "complete") await reportPlaybackComplete(message.sessionId);
  if (message.type === "error") await reportPlaybackError(message.sessionId, message.message);
  if (message.type === "sound") {
    const pluginId = pluginFrame.dataset.pluginId;
    if (pluginId && currentRequest?.animation.audio.enabled) new Howl({ src: [await requestPluginSound(pluginId, message.soundId)], pool: 1, volume: currentRequest.animation.audio.volume }).play();
  }
}

onMounted(async () => {
  unlistenPlay = await onPlayback((request) => void playIfStillCurrent(request));
  unlistenStop = await onPlaybackStop(() => stop());
  window.addEventListener("message", handlePluginMessage);
  await playCurrentWhenAvailable();
});

onBeforeUnmount(() => {
  stop();
  unlistenPlay?.();
  unlistenStop?.();
  window.removeEventListener("message", handlePluginMessage);
});
</script>

<template><main ref="root" class="overlay-root" aria-hidden="true" /></template>

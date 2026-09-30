<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import { open, save } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Activity, Ban, Box, ChevronRight, CirclePlay, Download, Minus, Music2, PackagePlus, Plus, RotateCcw, Save, Settings2, Sparkles, Square, Trash2, Upload, X } from "lucide-vue-next";
import { deleteAnimation, exportAnimation, getAudioResource, getConfig, getDiagnostics, importAnimation, importAudio, installPlugin, previewAnimation, resetAnimation, saveAnimation, setPluginEnabled, stopAnimations, uninstallPlugin, updateSettings } from "../services/tauri";
import { validateAnimation } from "../services/schemas";
import type { AnimationDefinition, AppConfig, DiagnosticEntry } from "../types";

const builtinAudioOptions = [
  { id: "builtin/completion-success.wav", name: "完成提示" },
  { id: "builtin/soft-chime.wav", name: "柔和铃声" },
  { id: "builtin/bright-pop.wav", name: "明亮弹跳" },
];
const transitionOptions = [
  { value: "none", label: "无" },
  { value: "fade", label: "渐隐/渐显" },
  { value: "scale", label: "缩放" },
  { value: "slide-up", label: "上滑" },
] as const;

const config = ref<AppConfig>();
const diagnostics = ref<DiagnosticEntry[]>([]);
const selectedId = ref("task-complete");
const page = ref<"animations" | "plugins" | "settings" | "diagnostics">("animations");
const message = ref("");
const previewing = ref(false);
const externalAudioSelected = ref(false);
const draft = reactive<AnimationDefinition>(emptyAnimation());
const currentWindow = getCurrentWindow();

const selected = computed(() => config.value?.animations.find((animation) => animation.id === selectedId.value));
const plugins = computed(() => config.value?.plugins ?? []);
const validation = computed(() => validateAnimation(draft));
const audioResourceLabel = computed(() => builtinAudioOptions.find((item) => item.id === draft.audio.resourceId)?.name ?? (draft.audio.resourceId?.startsWith("audio/") ? "外置音效" : "未选择"));
const hasAudio = computed(() => Boolean(draft.audio.resourceId));
const isExternalAudio = computed(() => externalAudioSelected.value || (draft.audio.resourceId?.startsWith("audio/") ?? false));
const audioSelection = computed({
  get: () => draft.audio.resourceId?.startsWith("builtin/") ? draft.audio.resourceId : isExternalAudio.value ? "__external__" : "",
  set: (value: string) => { if (value === "__external__") useExternalAudio(); else useBuiltinAudio(value); },
});
const transitionOverrideEnabled = computed({
  get: () => Boolean(draft.transition),
  set: (enabled: boolean) => { if (enabled) enableTransitionOverride(); else clearTransitionOverride(); },
});

function defaultTransition() {
  return { enter: "fade" as const, exit: "fade" as const, enterMs: 180, exitMs: 420 };
}

function emptyAnimation(): AnimationDefinition {
  return { id: crypto.randomUUID(), kind: "configured", name: "新动画", description: "自定义动画", command: "new-animation", aliases: [], enabled: true, renderer: "confetti", durationMs: config.value?.settings.defaultDurationMs ?? 3000, target: config.value?.settings.defaultTarget ?? "all", text: "完成", colors: ["#c9ef8f", "#91f5d4"], options: { particleCount: 100, angle: 58 }, audio: { enabled: false, volume: config.value?.settings.defaultVolume ?? 0.6, delayMs: 0 } };
}

function cloneAnimation(animation: AnimationDefinition): AnimationDefinition {
  return JSON.parse(JSON.stringify(animation)) as AnimationDefinition;
}

function edit(animation?: AnimationDefinition) {
  const next = cloneAnimation(animation ?? emptyAnimation());
  Object.assign(draft, next);
  externalAudioSelected.value = next.audio.resourceId?.startsWith("audio/") ?? false;
  if (!next.transition) delete draft.transition;
}

async function reload() {
  config.value = await getConfig();
  config.value.settings.overlayTopmost ??= true;
  config.value.settings.defaultTransition ??= defaultTransition();
  const animation = selected.value ?? config.value.animations[0];
  if (animation) { selectedId.value = animation.id; edit(animation); }
}

async function saveDraft() {
  if (validation.value.length) return;
  config.value = await saveAnimation(cloneAnimation(draft));
  selectedId.value = draft.id;
  message.value = "动画配置已保存";
}

async function preview(animation = draft) {
  if (previewing.value) return;
  previewing.value = true;
  message.value = "正在播放动画预览";
  try {
    await withTimeout(
      previewAnimation(cloneAnimation(animation), animation.id === draft.id && draft.text ? { text: draft.text } : {}),
      3000,
      "动画预览请求超时，请稍后重试",
    );
    message.value = "动画预览已触发";
  } catch (error) {
    message.value = String(error);
  } finally {
    previewing.value = false;
  }
}

function withTimeout<T>(promise: Promise<T>, timeoutMs: number, timeoutMessage: string): Promise<T> {
  return new Promise((resolve, reject) => {
    const timer = window.setTimeout(() => reject(new Error(timeoutMessage)), timeoutMs);
    promise.then(resolve, reject).finally(() => window.clearTimeout(timer));
  });
}

async function previewPlugin(pluginId: string) {
  const animation = config.value?.animations.find((item) => item.pluginId === pluginId);
  if (animation) await preview(animation);
}

async function createAnimation() {
  selectedId.value = "";
  edit();
}

async function remove(animation: AnimationDefinition) {
  config.value = await deleteAnimation(animation.id);
  selectedId.value = config.value.animations[0]?.id ?? "";
  edit(config.value.animations[0]);
}

async function reset(animation: AnimationDefinition) {
  config.value = await resetAnimation(animation.id);
  edit(config.value.animations.find((item) => item.id === animation.id));
}

async function install() {
  const path = await open({ multiple: false, filters: [{ name: "MotionCue Plugin", extensions: ["zip", "mcp"] }] });
  if (typeof path === "string") {
    try { config.value = await installPlugin(path); }
    catch (error) {
      if (String(error).includes("确认升级") && window.confirm(`${String(error)}。是否继续？`)) config.value = await installPlugin(path, true);
      else throw error;
    }
  }
}

async function importPack() {
  const path = await open({ multiple: false, filters: [{ name: "MotionCue Animation", extensions: ["mca", "zip"] }] });
  if (typeof path === "string") config.value = await importAnimation(path);
}

async function exportPack() {
  if (!selected.value) return;
  const path = await save({ defaultPath: `${selected.value.command}.mca`, filters: [{ name: "MotionCue Animation", extensions: ["mca"] }] });
  if (path) await exportAnimation(selected.value.id, path);
}

async function chooseAudio() {
  const path = await open({ multiple: false, filters: [{ name: "Audio", extensions: ["wav", "mp3", "ogg"] }] });
  if (typeof path === "string") {
    draft.audio.resourceId = await importAudio(path);
    draft.audio.enabled = true;
    externalAudioSelected.value = true;
    await previewSelectedAudio(draft.audio.resourceId);
  }
}

function useBuiltinAudio(resourceId: string) {
  externalAudioSelected.value = false;
  draft.audio.resourceId = resourceId || undefined;
  draft.audio.enabled = Boolean(resourceId);
  if (resourceId) void previewSelectedAudio(resourceId);
}

function useExternalAudio() {
  externalAudioSelected.value = true;
  if (draft.audio.resourceId?.startsWith("builtin/")) {
    draft.audio.resourceId = undefined;
    draft.audio.enabled = false;
  }
}

async function previewSelectedAudio(resourceId = draft.audio.resourceId) {
  if (!resourceId) return;
  try {
    const source = resourceId.startsWith("builtin/") ? `/audio/${resourceId.slice("builtin/".length)}` : await getAudioResource(resourceId);
    const audio = new Audio(source);
    audio.volume = draft.audio.volume;
    await audio.play();
  } catch (error) {
    message.value = `音效试听失败：${String(error)}`;
  }
}

function enableTransitionOverride() {
  const transition = draft.transition ?? config.value?.settings.defaultTransition ?? defaultTransition();
  draft.transition = JSON.parse(JSON.stringify(transition));
}

function clearTransitionOverride() {
  draft.transition = undefined;
}

async function saveSettings() {
  if (!config.value) return;
  config.value = await updateSettings(config.value.settings);
  message.value = "全局设置已保存";
}

async function loadDiagnostics() { diagnostics.value = await getDiagnostics(); }

onMounted(async () => { await reload(); await loadDiagnostics(); });
</script>

<template>
  <main class="app-shell">
    <header class="titlebar" data-tauri-drag-region>
      <div class="brand"><span class="brand-mark"><Sparkles :size="16" /></span><strong>MotionCue</strong><small>全屏动画与音效触发器</small></div>
      <div class="window-actions"><button class="stop-button" @click="stopAnimations"><Square :size="14" />停止全部动画</button><button aria-label="最小化" @click="currentWindow.minimize()"><Minus :size="15" /></button><button aria-label="隐藏到托盘" @click="currentWindow.close()"><X :size="15" /></button></div>
    </header>
    <aside class="sidebar">
      <nav>
        <button :class="{ active: page === 'animations' }" @click="page = 'animations'"><Sparkles :size="17" />动画目录</button>
        <button :class="{ active: page === 'plugins' }" @click="page = 'plugins'"><Box :size="17" />插件</button>
        <button :class="{ active: page === 'settings' }" @click="page = 'settings'"><Settings2 :size="17" />全局设置</button>
        <button :class="{ active: page === 'diagnostics' }" @click="page = 'diagnostics'; loadDiagnostics()"><Activity :size="17" />诊断</button>
      </nav>
      <section class="callout"><small>外部调用</small><code>motioncue://play/task-complete</code><code>motion-cue play success</code></section>
    </aside>

    <section v-if="page === 'animations' && config" class="workspace">
      <header class="workspace-header"><div><small>CATALOG</small><h1>动画目录</h1><p>为每个动画配置调用命令、视觉参数和音效。</p></div><div class="header-actions"><button @click="importPack"><Upload :size="14" />导入</button><button @click="exportPack"><Download :size="14" />导出</button><button class="primary" @click="createAnimation"><Plus :size="14" />新建动画</button></div></header>
      <div class="catalog-layout">
        <div class="animation-list">
          <button v-for="animation in config.animations" :key="animation.id" :class="['animation-row', { selected: selectedId === animation.id }]" @click="selectedId = animation.id; edit(animation)">
            <span class="animation-icon" :style="{ '--row-color': animation.colors[0] }"><Sparkles :size="17" /></span>
            <span><strong>{{ animation.name }}</strong><small>{{ animation.command }}</small></span><em>{{ animation.enabled ? '已启用' : '已禁用' }}</em><ChevronRight :size="15" />
          </button>
        </div>
        <form class="editor-card" @submit.prevent="saveDraft">
          <div class="editor-preview"><span :style="{ '--preview-color': draft.colors[0] }"><Sparkles :size="28" /></span><div><small>实时预览</small><strong>{{ draft.text || draft.name }}</strong></div><button type="button" :disabled="previewing" @click="preview()"><CirclePlay :size="15" />{{ previewing ? '播放中' : '播放' }}</button></div>
          <div class="form-grid">
            <label>名称<input v-model="draft.name" /></label>
            <label>调用命令<input v-model="draft.command" /></label>
            <label class="span-2">命令别名（逗号分隔）<input :value="draft.aliases.join(', ')" @input="draft.aliases = ($event.target as HTMLInputElement).value.split(',').map(value => value.trim()).filter(Boolean)" /></label>
            <label>渲染器<select v-model="draft.renderer"><option v-for="renderer in ['confetti','badge','pulse','ring','shake','completion']" :key="renderer">{{ renderer }}</option></select></label>
            <label>目标显示器<select v-model="draft.target"><option value="all">全部显示器</option><option value="primary">主显示器</option></select></label>
            <label class="span-2">说明<input v-model="draft.description" /></label>
            <label class="span-2">展示文字<input v-model="draft.text" maxlength="200" /></label>
            <label>持续时间（毫秒）<input v-model.number="draft.durationMs" type="number" min="100" max="60000" /></label>
            <label>粒子数量<input v-model.number="draft.options.particleCount" type="number" min="1" max="500" /></label>
            <label>喷发方向（角度）<input v-model.number="draft.options.angle" type="number" min="0" max="180" /></label>
            <label>主题颜色<input v-model="draft.colors[0]" type="color" /></label>
            <label>音效来源<select v-model="audioSelection"><option value="">不选择</option><option v-for="audio in builtinAudioOptions" :key="audio.id" :value="audio.id">内置 · {{ audio.name }}</option><option value="__external__">外置音效</option></select></label>
            <label class="audio-picker">当前音效<input :value="audioResourceLabel" readonly /><button type="button" :disabled="!hasAudio" @click="previewSelectedAudio()"><CirclePlay :size="14" />试听</button></label>
            <label>音量<input v-model.number="draft.audio.volume" type="range" min="0" max="1" step="0.05" /></label>
            <label>音效延迟<input v-model.number="draft.audio.delayMs" type="number" min="0" :max="draft.durationMs" /></label>
            <label v-if="isExternalAudio" class="audio-picker span-2">外置音效<input :value="draft.audio.resourceId" readonly /><button type="button" @click="chooseAudio"><Music2 :size="14" />导入/替换</button></label>
            <label class="switch span-2"><input v-model="transitionOverrideEnabled" type="checkbox" />单独覆盖该动画的开始/结尾效果</label>
            <template v-if="draft.transition">
              <label>开始效果<select v-model="draft.transition.enter"><option v-for="option in transitionOptions" :key="option.value" :value="option.value">{{ option.label }}</option></select></label>
              <label>结尾效果<select v-model="draft.transition.exit"><option v-for="option in transitionOptions" :key="option.value" :value="option.value">{{ option.label }}</option></select></label>
              <label>开始过渡（毫秒）<input v-model.number="draft.transition.enterMs" type="number" min="0" max="5000" /></label>
              <label>结尾过渡（毫秒）<input v-model.number="draft.transition.exitMs" type="number" min="0" max="5000" /></label>
            </template>
          </div>
          <label class="switch"><input v-model="draft.enabled" type="checkbox" />启用该动画</label><label class="switch"><input v-model="draft.audio.enabled" type="checkbox" />启用该动画音效</label>
          <ul v-if="validation.length" class="errors"><li v-for="error in validation" :key="error">{{ error }}</li></ul>
          <footer><div><button v-if="draft.kind === 'builtin'" type="button" @click="reset(draft)"><RotateCcw :size="14" />恢复默认</button><button v-else class="danger" type="button" @click="remove(draft)"><Trash2 :size="14" />删除</button></div><button class="primary" :disabled="validation.length > 0"><Save :size="14" />保存配置</button></footer>
        </form>
      </div>
    </section>

    <section v-else-if="page === 'plugins' && config" class="workspace"><header class="workspace-header"><div><small>PLUGINS</small><h1>沙箱 Web 插件</h1><p>插件只能渲染本地动画，不能访问网络、文件或系统能力。</p></div><button class="primary" @click="install"><PackagePlus :size="14" />安装插件包</button></header><div class="plugin-grid"><article v-for="plugin in plugins" :key="plugin.manifest.id" class="plugin-card"><header><span><Box :size="20" /></span><div><strong>{{ plugin.manifest.name }}</strong><small>{{ plugin.manifest.id }} · {{ plugin.manifest.version }}</small></div></header><p>{{ plugin.source }}</p><small>{{ (plugin.sizeBytes / 1024).toFixed(1) }} KB</small><p v-if="plugin.lastError" class="plugin-error">最近错误：{{ plugin.lastError }}</p><div class="chips"><em v-if="plugin.manifest.capabilities.webgl">WebGL</em><em v-if="plugin.manifest.capabilities.worker">Worker</em><em>离线资源</em></div><footer><label class="switch"><input :checked="plugin.enabled" type="checkbox" @change="setPluginEnabled(plugin.manifest.id, ($event.target as HTMLInputElement).checked).then(value => config = value)" />启用</label><button type="button" :disabled="!plugin.enabled || previewing" @click="previewPlugin(plugin.manifest.id)"><CirclePlay :size="14" />{{ previewing ? '预览中' : '预览' }}</button><button class="danger" @click="uninstallPlugin(plugin.manifest.id).then(value => config = value)"><Trash2 :size="14" />卸载</button></footer></article><div v-if="!plugins.length" class="empty"><Ban :size="34" /><strong>尚未安装插件</strong><p>安装后的插件默认禁用，需要明确确认后才能调用。</p></div></div></section>

    <section v-else-if="page === 'settings' && config" class="workspace narrow"><header class="workspace-header"><div><small>SETTINGS</small><h1>全局设置</h1><p>设备级偏好不会修改单个动画定义。</p></div></header><div class="settings-card"><label class="switch large"><input v-model="config.settings.muted" type="checkbox" /><span><strong>全局静音</strong><small>保留所有视觉动画，但不创建可听音频输出。</small></span></label><label class="switch large"><input v-model="config.settings.overlayTopmost" type="checkbox" /><span><strong>动画显示在最顶层</strong><small>关闭后，动画仍鼠标穿透且不抢焦点，但可被其他应用窗口盖住。</small></span></label><label>默认目标<select v-model="config.settings.defaultTarget"><option value="all">全部显示器</option><option value="primary">主显示器</option></select></label><label>新动画默认时长<input v-model.number="config.settings.defaultDurationMs" type="number" min="100" max="60000" /></label><label>新动画默认音量<input v-model.number="config.settings.defaultVolume" type="range" min="0" max="1" step="0.05" /></label><div class="settings-section"><strong>默认动画过渡</strong><small>没有单独覆盖的动画会使用以下开始和结尾效果。</small><div class="form-grid"><label>开始效果<select v-model="config.settings.defaultTransition.enter"><option v-for="option in transitionOptions" :key="option.value" :value="option.value">{{ option.label }}</option></select></label><label>结尾效果<select v-model="config.settings.defaultTransition.exit"><option v-for="option in transitionOptions" :key="option.value" :value="option.value">{{ option.label }}</option></select></label><label>开始过渡（毫秒）<input v-model.number="config.settings.defaultTransition.enterMs" type="number" min="0" max="5000" /></label><label>结尾过渡（毫秒）<input v-model.number="config.settings.defaultTransition.exitMs" type="number" min="0" max="5000" /></label></div></div><label>诊断记录保留数量<input v-model.number="config.settings.diagnosticsRetention" type="number" min="10" max="2000" /></label><button class="primary" @click="saveSettings"><Save :size="14" />保存设置</button></div></section>

    <section v-else class="workspace"><header class="workspace-header"><div><small>DIAGNOSTICS</small><h1>诊断记录</h1><p>默认不保存完整外部文字参数。</p></div><button @click="loadDiagnostics"><RotateCcw :size="14" />刷新</button></header><div class="diagnostic-list"><article v-for="entry in diagnostics" :key="entry.id" :class="entry.level"><time>{{ new Date(entry.timestamp).toLocaleString() }}</time><strong>{{ entry.category }}</strong><span>{{ entry.message }}</span><code v-if="entry.command">{{ entry.command }}</code></article><div v-if="!diagnostics.length" class="empty"><Activity :size="34" /><strong>暂无诊断记录</strong></div></div></section>
    <div v-if="message" class="toast" @animationend="message = ''">{{ message }}</div>
  </main>
</template>

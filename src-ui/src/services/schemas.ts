import type { AnimationDefinition, PluginHostMessage } from "../types";

const commandPattern = /^[a-z0-9][a-z0-9_-]{0,63}$/;
const colorPattern = /^#[0-9a-f]{6}$/i;

export function normalizeCommand(value: string) {
  return value.trim().toLowerCase();
}

export function validateAnimation(animation: AnimationDefinition): string[] {
  const errors: string[] = [];
  const command = normalizeCommand(animation.command);
  if (!commandPattern.test(command)) errors.push("命令只能包含字母、数字、短横线和下划线，最长 64 个字符");
  if (animation.aliases.some((alias) => !commandPattern.test(normalizeCommand(alias)))) errors.push("存在无效命令别名");
  if (animation.durationMs < 100 || animation.durationMs > 60_000) errors.push("持续时间必须在 100 到 60000 毫秒之间");
  if (animation.audio.volume < 0 || animation.audio.volume > 1) errors.push("音量必须在 0 到 1 之间");
  if (animation.audio.delayMs < 0 || animation.audio.delayMs > animation.durationMs) errors.push("音效延迟不能超过动画时长");
  if (animation.transition) {
    if (animation.transition.enterMs < 0 || animation.transition.exitMs < 0 || animation.transition.enterMs > 5000 || animation.transition.exitMs > 5000) errors.push("过渡时长必须在 0 到 5000 毫秒之间");
    if (animation.transition.enterMs + animation.transition.exitMs > animation.durationMs) errors.push("进入和结尾过渡总时长不能超过动画持续时间");
  }
  if (!animation.colors.length || animation.colors.some((color) => !colorPattern.test(color))) errors.push("颜色必须是六位十六进制值");
  if (animation.text && animation.text.length > 200) errors.push("文字不能超过 200 个字符");
  const particleCount = animation.options.particleCount;
  if (typeof particleCount === "number" && (particleCount < 1 || particleCount > 500)) errors.push("粒子数量必须在 1 到 500 之间");
  const particleSize = animation.options.particleSize;
  if (typeof particleSize === "number" && (particleSize < 0.4 || particleSize > 2.5)) errors.push("粒子大小必须在 0.4 到 2.5 之间");
  const angle = animation.options.angle;
  if (typeof angle === "number" && (angle < 0 || angle > 180)) errors.push("喷发方向必须在 0 到 180 度之间");
  validateRendererOptions(animation, errors);
  if ("script" in animation.options || "url" in animation.options || "html" in animation.options) errors.push("配置型动画不能包含脚本、网址或 HTML");
  return errors;
}

function validateRendererOptions(animation: AnimationDefinition, errors: string[]) {
  if (animation.renderer === "material-flow") {
    validateNumber(animation.options.intensity, 0, 1, "强度必须在 0 到 1 之间", errors);
    validateNumber(animation.options.speed, 0.1, 3, "速度必须在 0.1 到 3 之间", errors);
    validateNumber(animation.options.density, 1, 300, "密度必须在 1 到 300 之间", errors);
    validateNumber(animation.options.brightness, 0, 1, "亮度必须在 0 到 1 之间", errors);
  }
  if (animation.renderer === "corner-fireworks") {
    validateNumber(animation.options.burstCount, 1, 12, "烟花批次必须在 1 到 12 之间", errors);
    validateNumber(animation.options.spread, 1, 180, "扩散范围必须在 1 到 180 之间", errors);
    validateNumber(animation.options.speed, 0.1, 3, "速度必须在 0.1 到 3 之间", errors);
    validateCorners(animation.options.corners, errors);
  }
  if (animation.renderer === "focus-spotlight") {
    validateNumber(animation.options.spotlightSize, 0.1, 1, "光罩大小必须在 0.1 到 1 之间", errors);
    validateNumber(animation.options.dimAmount, 0, 0.75, "压暗程度必须在 0 到 0.75 之间", errors);
    validateNumber(animation.options.pulseStrength, 0, 1, "呼吸强度必须在 0 到 1 之间", errors);
    if ("showText" in animation.options && typeof animation.options.showText !== "boolean") errors.push("文字显示开关必须为布尔值");
  }
}

function validateNumber(value: unknown, min: number, max: number, message: string, errors: string[]) {
  if (value === undefined) return;
  if (typeof value !== "number" || value < min || value > max) errors.push(message);
}

function validateCorners(value: unknown, errors: string[]) {
  if (value === undefined) return;
  const allowed = new Set(["top-left", "top-right", "bottom-left", "bottom-right"]);
  if (!Array.isArray(value) || value.length < 1 || value.length > 4 || value.some((item) => typeof item !== "string" || !allowed.has(item))) errors.push("发射角落配置无效");
}

export function parsePluginHostMessage(value: unknown): PluginHostMessage | null {
  if (!value || typeof value !== "object") return null;
  const message = value as Record<string, unknown>;
  if (message.version !== 1 || typeof message.sessionId !== "string" || message.sessionId.length > 80 || typeof message.type !== "string") return null;
  if (message.type === "ready" || message.type === "complete") return message as PluginHostMessage;
  if (message.type === "error" && typeof message.message === "string" && message.message.length <= 500) return message as PluginHostMessage;
  if (message.type === "sound" && typeof message.soundId === "string" && message.soundId.length <= 64) return message as PluginHostMessage;
  return null;
}

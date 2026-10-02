import confetti from "canvas-confetti";
import { Howl } from "howler";
import type { AnimationDefinition, MotionTransitionConfig, PlaybackRequest } from "../types";

type Runtime = { stop: () => void };
let customShapes: confetti.Shape[] | undefined;

export function renderAnimation(root: HTMLElement, request: PlaybackRequest): Runtime {
  root.replaceChildren();
  root.dataset.renderer = request.animation.renderer;
  root.className = "overlay-root";
  root.style.setProperty("--fx-duration", `${request.animation.durationMs}ms`);
  root.style.setProperty("--fx-enter", `${request.transition.enterMs}ms`);
  root.style.setProperty("--fx-exit", `${request.transition.exitMs}ms`);
  root.style.setProperty("--fx-hold", `${Math.max(0, request.animation.durationMs - request.transition.enterMs - request.transition.exitMs)}ms`);
  root.style.animation = transitionAnimation(request.transition, request.animation.durationMs);
  const cleanups: Array<() => void> = [];
  const canvas = document.createElement("canvas");
  canvas.className = "fx-canvas";
  root.append(canvas);
  const fire = confetti.create(canvas, { resize: true, useWorker: true });
  cleanups.push(() => fire.reset());

  const animation = request.animation;
  if (animation.renderer === "confetti" || animation.renderer === "completion") {
    launchConfetti(fire, animation, request.transition, request.targetFrameRate, cleanups);
  }
  if (animation.renderer === "corner-fireworks") {
    launchCornerFireworks(fire, animation, request.targetFrameRate, cleanups);
  }
  if (animation.renderer === "material-flow") {
    launchMaterialFlow(canvas, animation, request.targetFrameRate, cleanups);
  }
  if (animation.renderer === "focus-spotlight") {
    root.append(createSpotlight(animation));
  }
  if (shouldCreateBadge(animation)) {
    root.append(createBadge(animation));
  }
  if (animation.renderer === "ring") {
    const ring = document.createElement("div");
    ring.className = "fx-ring";
    ring.style.setProperty("--accent", animation.colors[0]);
    root.append(ring);
  }

  let sound: Howl | undefined;
  let soundTimer: number | undefined;
  if (request.playAudio) {
    const source = audioSource(request);
    if (source) {
      soundTimer = window.setTimeout(() => {
        sound = new Howl({ src: [source], volume: animation.audio.volume, html5: false, pool: 1 });
        sound.play();
      }, animation.audio.delayMs);
    }
  }

  return {
    stop: () => {
      cleanups.forEach((cleanup) => cleanup());
      if (soundTimer) window.clearTimeout(soundTimer);
      sound?.stop();
      sound?.unload();
      root.replaceChildren();
      root.style.animation = "";
    },
  };
}

function shouldCreateBadge(animation: AnimationDefinition) {
  if (animation.renderer === "confetti" || animation.renderer === "material-flow") return false;
  if (animation.renderer === "focus-spotlight") return booleanOption(animation, "showText", true);
  return true;
}

export function transitionAnimation(transition: MotionTransitionConfig, durationMs: number) {
  const animations: string[] = [];
  if (transition.enter !== "none" && transition.enterMs > 0) {
    animations.push(`overlay-enter-${transition.enter} ${transition.enterMs}ms ease-out both`);
  }
  const holdMs = Math.max(0, durationMs - transition.enterMs - transition.exitMs);
  if (transition.exit !== "none" && transition.exitMs > 0) {
    animations.push(`overlay-exit-${transition.exit} ${transition.exitMs}ms ease-in ${transition.enterMs + holdMs}ms both`);
  }
  return animations.join(", ");
}

export function audioSchedule(request: PlaybackRequest) {
  const source = audioSource(request);
  return { shouldPlay: request.playAudio && Boolean(source), source, delayMs: request.animation.audio.delayMs, volume: request.animation.audio.volume };
}

export function audioSource(request: PlaybackRequest) {
  const builtin = request.animation.audio.resourceId?.startsWith("builtin/") ? request.animation.audio.resourceId.replace("builtin/", "/audio/") : undefined;
  const packaged = request.animation.audio.resourceId?.startsWith("audio/") ? request.animation.audio.resourceId.replace("audio/", "/audio/") : undefined;
  return request.audioDataUrl ?? builtin ?? packaged;
}

export function effectiveTransition(request: PlaybackRequest): MotionTransitionConfig {
  return request.transition;
}

function launchConfetti(fire: ReturnType<typeof confetti.create>, animation: AnimationDefinition, transition: MotionTransitionConfig, targetFrameRate: number, cleanups: Array<() => void>) {
  const timers: number[] = [];
  const colors = animation.colors;
  const quality = renderQuality(targetFrameRate);
  const particleCount = Math.max(1, Math.round(numberOption(animation, "particleCount", animation.renderer === "completion" ? 64 : 100) * quality.particleScale));
  const particleSize = numberOption(animation, "particleSize", 1);
  const direction = numberOption(animation, "angle", 58);
  const shapes = particleShapes();
  const side = (x: number, angle: number, delay: number, count = particleCount) => {
    timers.push(window.setTimeout(() => fire({ particleCount: Math.round(count * random(0.86, 1.14)), colors, angle: angle + random(-5, 5), spread: random(24, 38), startVelocity: random(58, 74), decay: random(.92, .95), gravity: random(.38, .62), drift: x < .5 ? random(.04,.18) : random(-.18,-.04), scalar: random(.72,1.12) * particleSize, ticks: Math.round(random(185,245) * quality.tickScale), origin: { x: x + random(-.02,.02), y: random(.88,.98) }, shapes }), delay));
  };
  side(0.03, direction, 0);
  side(0.97, 180 - direction, 40);
  const usableDuration = Math.max(600, animation.durationMs - transition.exitMs);
  if (animation.renderer === "completion" || usableDuration > 3_600) {
    for (const ratio of [0.24, 0.52, 0.78]) {
      const delay = Math.min(usableDuration - 300, Math.round(usableDuration * ratio));
      side(0.08, direction + 4, delay, particleCount * .48);
      side(0.92, 176 - direction, delay + 90, particleCount * .48);
    }
  }
  cleanups.push(() => timers.forEach((timer) => window.clearTimeout(timer)));
}

function launchCornerFireworks(fire: ReturnType<typeof confetti.create>, animation: AnimationDefinition, targetFrameRate: number, cleanups: Array<() => void>) {
  const timers: number[] = [];
  const quality = renderQuality(targetFrameRate);
  const particleCount = Math.max(1, Math.round(numberOption(animation, "particleCount", 72) * quality.particleScale));
  const particleSize = numberOption(animation, "particleSize", 1);
  const burstCount = Math.max(1, Math.round(numberOption(animation, "burstCount", 4)));
  const spread = numberOption(animation, "spread", 72);
  const speed = numberOption(animation, "speed", 1);
  const corners = cornerOptions(animation);
  const usableDuration = Math.max(600, animation.durationMs - 280);
  for (let index = 0; index < burstCount; index += 1) {
    const corner = corners[index % corners.length];
    const delay = Math.round((usableDuration / Math.max(1, burstCount)) * index);
    timers.push(window.setTimeout(() => fire({
      particleCount: Math.round(particleCount * random(.68, 1.08)),
      colors: animation.colors,
      angle: corner.angle + random(-10, 10),
      spread: spread + random(-8, 8),
      startVelocity: random(46, 72) * speed,
      decay: random(.9, .94),
      gravity: random(.55, .82),
      drift: corner.drift * random(.2, .58),
      scalar: random(.72, 1.12) * particleSize,
      ticks: Math.round(random(145, 220) * quality.tickScale),
      origin: { x: corner.x, y: corner.y },
      shapes: particleShapes(),
    }), delay));
  }
  cleanups.push(() => timers.forEach((timer) => window.clearTimeout(timer)));
}

function launchMaterialFlow(canvas: HTMLCanvasElement, animation: AnimationDefinition, targetFrameRate: number, cleanups: Array<() => void>) {
  const context = canvas.getContext("2d");
  if (!context) return;
  const quality = renderQuality(targetFrameRate);
  const density = Math.max(1, Math.round(numberOption(animation, "density", 120) * quality.particleScale));
  const speed = numberOption(animation, "speed", .8);
  const intensity = numberOption(animation, "intensity", .72);
  const brightness = numberOption(animation, "brightness", .72);
  const particles = Array.from({ length: density }, () => ({ x: Math.random(), y: Math.random(), r: random(.7, 2.8), phase: random(0, Math.PI * 2), drift: random(.12, .7) }));
  let frame = 0;
  let raf = 0;
  const draw = () => {
    const width = canvas.width = canvas.clientWidth || innerWidth;
    const height = canvas.height = canvas.clientHeight || innerHeight;
    context.clearRect(0, 0, width, height);
    context.globalCompositeOperation = "lighter";
    const time = frame * .012 * speed;
    for (let index = 0; index < particles.length; index += 1) {
      const particle = particles[index];
      particle.x = (particle.x + .0008 * speed * particle.drift) % 1;
      particle.y = (particle.y + Math.sin(time + particle.phase) * .0007 * speed + 1) % 1;
      const color = animation.colors[index % animation.colors.length] ?? "#ffffff";
      const alpha = Math.max(.04, Math.min(.42, intensity * brightness * random(.18, .34)));
      context.fillStyle = hexToRgba(color, alpha);
      context.beginPath();
      context.arc(particle.x * width, particle.y * height, particle.r * (1 + intensity * 2.2), 0, Math.PI * 2);
      context.fill();
    }
    context.globalCompositeOperation = "source-over";
    frame += 1;
    raf = window.requestAnimationFrame(draw);
  };
  raf = window.requestAnimationFrame(draw);
  cleanups.push(() => {
    window.cancelAnimationFrame(raf);
    context.clearRect(0, 0, canvas.width, canvas.height);
  });
}

function particleShapes() {
  if (customShapes) return customShapes;
  try {
    customShapes = ["square", "square", "circle", "star",
      confetti.shapeFromPath({ path: "M50 0 L96 38 L72 100 L10 76 Z", matrix: new DOMMatrix([0.1,0,0,0.1,-5,-5]) }),
      confetti.shapeFromPath({ path: "M0 18 C26 0 72 2 100 22 L76 50 L100 82 C70 100 30 98 0 78 L24 48 Z", matrix: new DOMMatrix([0.1,0,0,0.1,-5,-5]) }),
      confetti.shapeFromPath({ path: "M58 0 L16 56 H46 L34 100 L86 40 H56 Z", matrix: new DOMMatrix([0.1,0,0,0.1,-5,-5]) }),
      confetti.shapeFromPath({ path: "M167 72 C186 34 204 16 242 16 C284 16 318 49 318 91 C318 167 242 242 167 318 C91 242 16 167 16 91 C16 49 49 16 91 16 C129 16 148 34 167 72 Z", matrix: new DOMMatrix([0.03333333333333333,0,0,0.03333333333333333,-5.566666666666666,-5.533333333333333]) }),
    ];
  } catch { customShapes = ["square", "square", "circle", "star"]; }
  return customShapes;
}

function random(min: number, max: number) { return min + Math.random() * (max - min); }

function normalizeFrameRate(value: number) {
  return Math.min(60, Math.max(30, Math.round(Number.isFinite(value) ? value : 60)));
}

function renderQuality(targetFrameRate: number) {
  const frameRate = normalizeFrameRate(targetFrameRate);
  if (frameRate < 45) return { particleScale: 0.68, tickScale: 0.86 };
  if (frameRate < 60) return { particleScale: 0.82, tickScale: 0.93 };
  return { particleScale: 1, tickScale: 1 };
}

function createBadge(animation: AnimationDefinition) {
  const badge = document.createElement("section");
  badge.className = `fx-badge fx-${animation.renderer}`;
  badge.style.setProperty("--accent", animation.colors[0]);
  badge.innerHTML = `<span>${icon(animation.renderer)}</span><div><small>${eyebrow(animation.renderer)}</small><strong></strong></div>`;
  const title = badge.querySelector("strong");
  if (title) title.textContent = animation.text || animation.name;
  return badge;
}

function createSpotlight(animation: AnimationDefinition) {
  const spotlight = document.createElement("div");
  spotlight.className = "fx-spotlight";
  spotlight.style.setProperty("--accent", animation.colors[0]);
  spotlight.style.setProperty("--spotlight-size", `${numberOption(animation, "spotlightSize", .42) * 100}%`);
  spotlight.style.setProperty("--dim", `${numberOption(animation, "dimAmount", .38)}`);
  spotlight.style.setProperty("--pulse", `${numberOption(animation, "pulseStrength", .28)}`);
  return spotlight;
}

function icon(renderer: AnimationDefinition["renderer"]) {
  if (renderer === "shake") return "!";
  if (renderer === "pulse") return "◎";
  if (renderer === "ring") return "★";
  if (renderer === "corner-fireworks") return "✦";
  if (renderer === "focus-spotlight") return "◎";
  return "✓";
}

function eyebrow(renderer: AnimationDefinition["renderer"]) {
  if (renderer === "shake") return "需要注意";
  if (renderer === "pulse" || renderer === "focus-spotlight") return "进入状态";
  if (renderer === "ring") return "里程碑达成";
  if (renderer === "corner-fireworks") return "庆祝完成";
  return "MotionCue";
}

function numberOption(animation: AnimationDefinition, key: string, fallback: number) {
  const value = animation.options[key];
  return typeof value === "number" ? value : fallback;
}

function booleanOption(animation: AnimationDefinition, key: string, fallback: boolean) {
  const value = animation.options[key];
  return typeof value === "boolean" ? value : fallback;
}

function cornerOptions(animation: AnimationDefinition) {
  const defaults = ["bottom-left", "bottom-right"];
  const raw = Array.isArray(animation.options.corners) ? animation.options.corners : defaults;
  const corners = raw.filter((value): value is string => typeof value === "string");
  const selected = corners.length ? corners : defaults;
  return selected.map((corner) => {
    if (corner === "top-left") return { x: .03, y: .03, angle: 315, drift: .4 };
    if (corner === "top-right") return { x: .97, y: .03, angle: 225, drift: -.4 };
    if (corner === "bottom-right") return { x: .97, y: .97, angle: 135, drift: -.4 };
    return { x: .03, y: .97, angle: 45, drift: .4 };
  });
}

function hexToRgba(hex: string, alpha: number) {
  const value = hex.replace("#", "");
  const number = Number.parseInt(value, 16);
  const red = (number >> 16) & 255;
  const green = (number >> 8) & 255;
  const blue = number & 255;
  return `rgba(${red}, ${green}, ${blue}, ${alpha})`;
}

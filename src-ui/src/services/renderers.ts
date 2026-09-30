import confetti from "canvas-confetti";
import { Howl } from "howler";
import type { AnimationDefinition, PlaybackRequest } from "../types";

type Runtime = { stop: () => void };
let customShapes: confetti.Shape[] | undefined;

export function renderAnimation(root: HTMLElement, request: PlaybackRequest): Runtime {
  root.replaceChildren();
  root.dataset.renderer = request.animation.renderer;
  const cleanups: Array<() => void> = [];
  const canvas = document.createElement("canvas");
  canvas.className = "fx-canvas";
  root.append(canvas);
  const fire = confetti.create(canvas, { resize: true, useWorker: true });
  cleanups.push(() => fire.reset());

  const animation = request.animation;
  if (animation.renderer === "confetti" || animation.renderer === "completion") {
    launchConfetti(fire, animation, cleanups);
  }
  if (animation.renderer !== "confetti") {
    root.append(createBadge(animation));
  }
  if (animation.renderer === "ring" || animation.renderer === "completion") {
    const ring = document.createElement("div");
    ring.className = "fx-ring";
    ring.style.setProperty("--accent", animation.colors[0]);
    root.append(ring);
  }

  let sound: Howl | undefined;
  let soundTimer: number | undefined;
  if (request.playAudio) {
      const source = request.audioDataUrl ?? (animation.audio.resourceId === "builtin/completion-success.wav" ? "/audio/completion-success.wav" : undefined);
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
    },
  };
}

export function audioSchedule(request: PlaybackRequest) {
  const source = request.audioDataUrl ?? (request.animation.audio.resourceId === "builtin/completion-success.wav" ? "/audio/completion-success.wav" : undefined);
  return { shouldPlay: request.playAudio && Boolean(source), source, delayMs: request.animation.audio.delayMs, volume: request.animation.audio.volume };
}

function launchConfetti(fire: ReturnType<typeof confetti.create>, animation: AnimationDefinition, cleanups: Array<() => void>) {
  const timers: number[] = [];
  const colors = animation.colors;
  const particleCount = numberOption(animation, "particleCount", animation.renderer === "completion" ? 64 : 100);
  const direction = numberOption(animation, "angle", 58);
  const shapes = particleShapes();
  const side = (x: number, angle: number, delay: number, count = particleCount) => {
    timers.push(window.setTimeout(() => fire({ particleCount: Math.round(count * random(0.86, 1.14)), colors, angle: angle + random(-5, 5), spread: random(24, 38), startVelocity: random(58, 74), decay: random(.92, .95), gravity: random(.38, .62), drift: x < .5 ? random(.04,.18) : random(-.18,-.04), scalar: random(.72,1.12), ticks: Math.round(random(185,245)), origin: { x: x + random(-.02,.02), y: random(.88,.98) }, shapes }), delay));
  };
  side(0.03, direction, 0);
  side(0.97, 180 - direction, 40);
  if (animation.renderer === "completion") {
    side(0.08, direction + 4, 420, particleCount * .72);
    side(0.92, 176 - direction, 520, particleCount * .72);
    side(0.03, direction - 3, 900, particleCount * .46);
    side(0.97, 183 - direction, 980, particleCount * .46);
  }
  cleanups.push(() => timers.forEach((timer) => window.clearTimeout(timer)));
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

function createBadge(animation: AnimationDefinition) {
  const badge = document.createElement("section");
  badge.className = `fx-badge fx-${animation.renderer}`;
  badge.style.setProperty("--accent", animation.colors[0]);
  badge.innerHTML = `<span>${icon(animation.renderer)}</span><div><small>${eyebrow(animation.renderer)}</small><strong></strong></div>`;
  const title = badge.querySelector("strong");
  if (title) title.textContent = animation.text || animation.name;
  return badge;
}

function icon(renderer: AnimationDefinition["renderer"]) {
  if (renderer === "shake") return "!";
  if (renderer === "pulse") return "◎";
  if (renderer === "ring") return "★";
  return "✓";
}

function eyebrow(renderer: AnimationDefinition["renderer"]) {
  if (renderer === "shake") return "需要注意";
  if (renderer === "pulse") return "进入状态";
  if (renderer === "ring") return "里程碑达成";
  return "MotionCue";
}

function numberOption(animation: AnimationDefinition, key: string, fallback: number) {
  const value = animation.options[key];
  return typeof value === "number" ? value : fallback;
}

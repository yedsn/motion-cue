import { beforeEach, describe, expect, it, vi } from "vitest";
import { renderAnimation } from "./renderers";
import type { AnimationDefinition, PlaybackRequest } from "../types";

const fireMock = vi.fn();

vi.mock("canvas-confetti", () => ({
  default: {
    create: () => Object.assign(fireMock, { reset: vi.fn() }),
    shapeFromPath: () => "square",
  },
}));

const baseAnimation: AnimationDefinition = {
  id: "task-complete",
  kind: "builtin",
  name: "Task Complete",
  description: "",
  command: "task-complete",
  aliases: [],
  enabled: true,
  renderer: "completion",
  durationMs: 1200,
  target: "all",
  text: "Done",
  colors: ["#34c584", "#ffd27a"],
  options: { particleCount: 16, angle: 58 },
  audio: { enabled: false, delayMs: 0, volume: 0.6 },
};

function request(animation: AnimationDefinition): PlaybackRequest {
  return {
    sessionId: "session",
    params: {},
    startedAt: 1,
    playAudio: false,
    targetFrameRate: 60,
    transition: { enter: "fade", exit: "fade", enterMs: 180, exitMs: 240 },
    animation,
  };
}

describe("animation rendering", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    fireMock.mockClear();
  });

  it("does not append a center ring for completion playback", () => {
    const root = document.createElement("main");
    const runtime = renderAnimation(root, request(baseAnimation));
    expect(root.querySelector(".fx-badge")).toBeTruthy();
    expect(root.querySelector(".fx-ring")).toBeNull();
    runtime.stop();
  });

  it("does not create an empty badge when focus spotlight text is hidden", () => {
    const root = document.createElement("main");
    const runtime = renderAnimation(root, request({ ...baseAnimation, renderer: "focus-spotlight", options: { showText: false, spotlightSize: .42, dimAmount: .38, pulseStrength: .2 } }));
    expect(root.querySelector(".fx-spotlight")).toBeTruthy();
    expect(root.querySelector(".fx-badge")).toBeNull();
    runtime.stop();
  });

  it("uses configured corners for corner fireworks origins", () => {
    const root = document.createElement("main");
    const runtime = renderAnimation(root, request({ ...baseAnimation, renderer: "corner-fireworks", options: { particleCount: 8, burstCount: 1, corners: ["top-left"] } }));
    vi.runOnlyPendingTimers();
    expect(fireMock).toHaveBeenCalled();
    expect(fireMock.mock.calls[0]?.[0].origin).toEqual({ x: .03, y: .03 });
    runtime.stop();
  });

  it("applies configured particle size to confetti scalar", () => {
    const root = document.createElement("main");
    const runtime = renderAnimation(root, request({ ...baseAnimation, renderer: "confetti", options: { particleCount: 8, particleSize: 2 } }));
    vi.runOnlyPendingTimers();
    expect(fireMock).toHaveBeenCalled();
    expect(fireMock.mock.calls[0]?.[0].scalar).toBeGreaterThanOrEqual(1.44);
    runtime.stop();
  });
});

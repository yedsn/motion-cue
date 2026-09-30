import { describe, expect, it } from "vitest";
import { audioSchedule, transitionAnimation } from "./renderers";
import type { PlaybackRequest } from "../types";

const request: PlaybackRequest = {
  sessionId: "session",
  params: {},
  startedAt: 1,
  playAudio: true,
  targetFrameRate: 60,
  transition: { enter: "fade", exit: "fade", enterMs: 180, exitMs: 420 },
  animation: {
    id: "success", kind: "builtin", name: "Success", description: "", command: "success", aliases: [], enabled: true,
    renderer: "badge", durationMs: 2000, target: "all", colors: ["#34c584"], options: {},
    audio: { enabled: true, resourceId: "builtin/completion-success.wav", delayMs: 120, volume: 0.7 },
  },
};

describe("audio scheduling", () => {
  it("uses one built-in audio source with configured delay", () => expect(audioSchedule(request)).toEqual({ shouldPlay: true, source: "/audio/completion-success.wav", delayMs: 120, volume: 0.7 }));
  it("maps any built-in audio resource to packaged audio", () => expect(audioSchedule({ ...request, animation: { ...request.animation, audio: { ...request.animation.audio, resourceId: "builtin/soft-chime.wav" } } }).source).toBe("/audio/soft-chime.wav"));
  it("honors host mute decision", () => expect(audioSchedule({ ...request, playAudio: false }).shouldPlay).toBe(false));
});

describe("transition timing", () => {
  it("keeps exit transition aligned to configured duration", () => {
    expect(transitionAnimation({ enter: "fade", exit: "fade", enterMs: 200, exitMs: 500 }, 10_000)).toContain("9500ms both");
  });

  it("omits disabled transitions", () => {
    expect(transitionAnimation({ enter: "none", exit: "none", enterMs: 0, exitMs: 0 }, 10_000)).toBe("");
  });
});

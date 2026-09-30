import { describe, expect, it } from "vitest";
import { audioSchedule } from "./renderers";
import type { PlaybackRequest } from "../types";

const request: PlaybackRequest = {
  sessionId: "session",
  params: {},
  startedAt: 1,
  playAudio: true,
  animation: {
    id: "success", kind: "builtin", name: "Success", description: "", command: "success", aliases: [], enabled: true,
    renderer: "badge", durationMs: 2000, target: "all", colors: ["#34c584"], options: {},
    audio: { enabled: true, resourceId: "builtin/completion-success.wav", delayMs: 120, volume: 0.7 },
  },
};

describe("audio scheduling", () => {
  it("uses one built-in audio source with configured delay", () => expect(audioSchedule(request)).toEqual({ shouldPlay: true, source: "/audio/completion-success.wav", delayMs: 120, volume: 0.7 }));
  it("honors host mute decision", () => expect(audioSchedule({ ...request, playAudio: false }).shouldPlay).toBe(false));
});


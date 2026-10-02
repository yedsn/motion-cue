import { describe, expect, it } from "vitest";
import { parsePluginHostMessage, validateAnimation } from "./schemas";
import type { AnimationDefinition } from "../types";

const animation: AnimationDefinition = {
  id: "custom",
  kind: "configured",
  name: "Custom",
  description: "",
  command: "release-party",
  aliases: [],
  enabled: true,
  renderer: "confetti",
  durationMs: 3000,
  target: "all",
  colors: ["#34c584"],
  options: { particleCount: 100 },
  audio: { enabled: false, volume: 0.6, delayMs: 0 },
};

describe("animation schema", () => {
  it("accepts a safe configured animation", () => expect(validateAnimation(animation)).toEqual([]));
  it("rejects scripts and unsafe duration", () => expect(validateAnimation({ ...animation, durationMs: 70_000, options: { script: "alert(1)" } }).length).toBeGreaterThan(0));
  it("rejects particle counts outside the supported range", () => expect(validateAnimation({ ...animation, options: { particleCount: 501 } })).toContain("粒子数量必须在 1 到 500 之间"));
  it("rejects particle sizes outside the supported range", () => expect(validateAnimation({ ...animation, options: { particleSize: 2.6 } })).toContain("粒子大小必须在 0.4 到 2.5 之间"));
  it("rejects emission angles outside the supported range", () => expect(validateAnimation({ ...animation, options: { angle: 181 } })).toContain("喷发方向必须在 0 到 180 度之间"));
  it("rejects material values outside supported ranges", () => expect(validateAnimation({ ...animation, renderer: "material-flow", options: { density: 301 } })).toContain("密度必须在 1 到 300 之间"));
  it("rejects invalid fireworks corners", () => expect(validateAnimation({ ...animation, renderer: "corner-fireworks", options: { corners: ["center"] } })).toContain("发射角落配置无效"));
  it("rejects invalid focus spotlight switches", () => expect(validateAnimation({ ...animation, renderer: "focus-spotlight", options: { showText: "yes" } })).toContain("文字显示开关必须为布尔值"));
});

describe("plugin host messages", () => {
  it("accepts versioned completion", () => expect(parsePluginHostMessage({ version: 1, sessionId: "s", type: "complete" })?.type).toBe("complete"));
  it("rejects unknown and oversized messages", () => expect(parsePluginHostMessage({ version: 1, sessionId: "s", type: "exec", command: "calc" })).toBeNull());
});

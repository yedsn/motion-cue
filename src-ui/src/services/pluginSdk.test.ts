import { describe, expect, it } from "vitest";
import { parsePluginHostMessage } from "./schemas";
import { injectPluginPolicy } from "./pluginPolicy";

describe("plugin SDK lifecycle messages", () => {
  it.each(["ready", "complete"])("accepts %s", (type) => expect(parsePluginHostMessage({ version: 1, sessionId: "session", type })?.type).toBe(type));
  it("accepts declared sound-shaped messages", () => expect(parsePluginHostMessage({ version: 1, sessionId: "session", type: "sound", soundId: "impact" })?.type).toBe("sound"));
  it("rejects oversized error messages", () => expect(parsePluginHostMessage({ version: 1, sessionId: "session", type: "error", message: "x".repeat(501) })).toBeNull());
  it("injects a host policy without external capabilities", () => {
    const policy = injectPluginPolicy("<html><head></head><body></body></html>", { webgl: false, worker: false });
    expect(policy).toContain("connect-src 'none'");
    expect(policy).toContain("navigate-to 'none'");
    expect(policy).toContain("form-action 'none'");
    expect(policy).toContain("object-src 'none'");
    expect(policy).toContain("frame-src 'none'");
    expect(policy).toContain("window.Worker=undefined");
    expect(policy).toContain("window.SharedWorker=undefined");
    expect(policy).toContain("window.open=()=>null");
    expect(policy).toContain("window.showOpenFilePicker=undefined");
    expect(policy).toContain("addEventListener('drop'");
    expect(policy).toContain("getContext=function(type,...args){if(type==='webgl'||type==='webgl2')return null");
    expect(policy).not.toContain("window.__TAURI__");
  });

  it("keeps declared rendering capabilities available", () => {
    const policy = injectPluginPolicy("<html><head></head><body></body></html>", { webgl: true, worker: true });
    expect(policy).not.toContain("window.Worker=undefined");
    expect(policy).not.toContain("getContext=function(type,...args)");
    expect(policy).toContain("connect-src 'none'");
  });
});

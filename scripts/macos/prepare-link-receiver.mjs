import { mkdirSync, writeFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

if (process.platform === "darwin") {
  const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
  const app = resolve(root, "src-tauri/target/link-receiver/MotionCueLink.app");
  mkdirSync(`${app}/Contents/MacOS`, { recursive: true });
  const target = process.env.TAURI_ENV_TARGET_TRIPLE;
  const arch = target?.startsWith("x86_64") ? "x86_64" : target?.startsWith("aarch64") ? "arm64" : process.arch === "arm64" ? "arm64" : "x86_64";
  execFileSync("clang", ["-arch", arch, "-mmacosx-version-min=11.0", "-framework", "AppKit", resolve(root, "scripts/macos/link-receiver.m"), "-o", `${app}/Contents/MacOS/motioncue-link`], { stdio: "inherit" });
  writeFileSync(`${app}/Contents/Info.plist`, `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>com.motioncue.desktop.link</string>
<key>CFBundleName</key><string>MotionCueLink</string>
<key>CFBundleExecutable</key><string>motioncue-link</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>1</string>
<key>LSBackgroundOnly</key><true/>
<key>CFBundleURLTypes</key><array><dict><key>CFBundleURLSchemes</key><array><string>motioncue</string></array><key>CFBundleTypeRole</key><string>Editor</string></dict></array>
</dict></plist>
`);
  const identity = process.env.APPLE_SIGNING_IDENTITY || "-";
  execFileSync("codesign", ["--force", "--options", "runtime", "--sign", identity, app], { stdio: "inherit" });
}

# Proposal

## Why

macOS 打包版通过播放链接触发动画时可能激活 MotionCue，使调用应用失去键盘焦点，用户必须重新点击才能继续输入。开发版表现正常不能替代打包版验收，需要将后台播放与显式打开管理界面的激活行为分开。

## What Changes

- 从 macOS 应用包启动阶段建立后台应用身份，避免仅依赖运行后的模式切换。
- 播放型 URL 和 CLI 调用保持当前前台应用及输入焦点，包括冷启动、后台常驻和管理窗口可见的情况。
- 明确区分播放请求与管理窗口打开请求，避免伴随 URL 的 Reopen 或延迟启动动作显示并聚焦管理窗口。
- 保留 Finder 正常启动、菜单栏打开、检查更新和 CLI open 的管理窗口与 Dock 行为。
- 增加打包版焦点验收，覆盖连续输入和快速重复调用。

## Capabilities

### New Capabilities

- `macos-app-lifecycle`: 在主规格尚未建立的情况下，沿用现有变更中的能力路径，补充 macOS 打包版后台启动、播放焦点保持与显式管理窗口激活的契约。

### Modified Capabilities

无。当前 `openspec/specs` 没有主规格。本变更承接 `manage-macos-dock-visibility` 和 `create-motion-cue-desktop-app` 的相关要求；未来同步时合并同一路径，不创建平行能力。

## Impact

- macOS Info.plist、Tauri 打包配置，以及 `src-tauri/src/app_lifecycle.rs`、`lib.rs` 的启动和链接事件仲裁。
- 保留 `playback.rs` 的非激活覆盖窗口显示，必要时仅修正经实机证实的焦点路径。
- 更新 Rust 生命周期测试和 `docs/acceptance.md` 的打包版验收记录要求。
- 不改变 URL 参数、动画内容、插件权限或 Windows 行为；不引入抢回焦点作为正常播放机制。

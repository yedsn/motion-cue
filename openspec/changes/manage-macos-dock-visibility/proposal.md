# Proposal

## Why

MotionCue 在 macOS 上作为常驻菜单栏应用运行时，管理窗口关闭后仍会保留 Dock 图标，使应用看起来仍有一个可切换的前台窗口。需要让 Dock 图标与管理窗口的可见状态保持一致：窗口打开时可正常切换，窗口关闭后仅保留菜单栏入口。

## What Changes

- 在 macOS 上显示管理窗口时显示 MotionCue 的 Dock 图标。
- 在 macOS 上关闭管理窗口时阻止应用退出、隐藏窗口并隐藏 Dock 图标，同时保留菜单栏图标和后台运行能力。
- 从菜单栏、检查更新操作或外部 `open` 调用重新打开管理窗口时，先恢复 Dock 图标，再显示并聚焦窗口。
- 保持菜单栏退出操作为彻底退出应用的唯一显式退出路径之一。
- 保持 Windows 现有托盘和窗口生命周期行为不变。

## Capabilities

### New Capabilities

- `macos-app-lifecycle`: 定义 macOS 管理窗口、Dock 图标、菜单栏图标和后台进程之间的生命周期行为。

### Modified Capabilities

无。

## Impact

- 主要影响 `src-tauri/src/lib.rs` 中的应用初始化、管理窗口关闭处理和统一窗口打开入口。
- 使用 Tauri 现有的 macOS Dock 可见性接口，不新增第三方运行时依赖。
- 需要补充 Rust 层状态转换测试或可测试的生命周期决策逻辑，并在真实 macOS 环境验证 Dock、菜单栏和窗口切换行为。
- Windows 构建、托盘菜单、动画播放和外部调用协议不改变。

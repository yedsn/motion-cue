# Tasks

## 1. 生命周期动作与平台封装

- [x] 1.1 在 Rust 宿主中定义管理窗口正常启动、后台启动、打开和关闭对应的生命周期动作，并用单元测试验证每个动作要求的窗口与 Dock 可见性结果
- [x] 1.2 封装仅在 macOS 调用 Tauri Dock 可见性接口的辅助函数，并通过 `cargo test --manifest-path src-tauri/Cargo.toml` 验证非 macOS 构建与测试不受平台 API 影响

## 2. 管理窗口生命周期接入

- [x] 2.1 更新统一 `app_open` 路径，在 macOS 上先恢复 Dock 图标再显示、还原和聚焦管理窗口，并通过 Rust 测试或可注入动作测试验证菜单栏打开、检查更新、CLI `open` 和第二实例唤醒复用该路径
- [x] 2.2 更新主窗口关闭事件，在阻止退出并隐藏窗口后隐藏 macOS Dock 图标，同时确保覆盖窗口事件不会触发该动作，并运行相关 Rust 测试验证标签与动作分支
- [x] 2.3 更新 `LaunchIntent::Background` 冷启动路径，在隐藏管理窗口时同步隐藏 macOS Dock 图标，并用测试验证后台播放与协议调用不会执行管理窗口打开动作
- [x] 2.4 保持菜单栏“退出”和 Windows 托盘生命周期不变，并通过现有测试及 `npm run build` 验证跨平台条件编译和前端构建成功

## 3. 平台验收与文档

- [ ] 3.1 在 macOS 实机上验证正常 GUI 启动显示窗口、菜单栏与 Dock，关闭窗口后仅保留菜单栏，并记录 Apple Silicon 或 Intel、系统版本和验收结果
- [ ] 3.2 在 macOS 实机上验证菜单栏打开、菜单栏检查更新、CLI `motion-cue open` 和第二实例唤醒均恢复 Dock 并聚焦同一管理窗口
- [ ] 3.3 在 macOS 实机上验证无实例时的后台动画调用仅启动菜单栏实例、播放动画且不显示管理窗口或 Dock，并验证菜单栏“退出”彻底终止应用
- [ ] 3.4 在 macOS 实机上连续执行快速关闭与重开，确认没有重复 Dock 图标、窗口失焦或无入口状态；macOS 生命周期验收步骤已补充到项目验收文档，实机观察结果待填写
- [x] 3.5 运行 `npm test`、`npm run build`、`cargo test --manifest-path src-tauri/Cargo.toml` 和 `openspec validate manage-macos-dock-visibility --strict`，确认全部自动化检查通过

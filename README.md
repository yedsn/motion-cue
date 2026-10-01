# MotionCue

<p align="center">
  <img src="docs/public/logo.svg" alt="MotionCue" width="160">
</p>

<p align="center"><strong>一个 Windows 优先的全屏动画与音效触发器。</strong></p>
<p align="center">为发布脚本、自动化任务、编辑器 Hook 和本地工具提供清晰、即时、可感知的工作流反馈。</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-2021-000000?logo=rust" alt="Rust 2021">
  <img src="https://img.shields.io/badge/Tauri-2.x-24C8DB?logo=tauri" alt="Tauri 2">
  <img src="https://img.shields.io/badge/UI-Vue_3-4FC08D?logo=vue.js" alt="Vue 3">
  <img src="https://img.shields.io/badge/Docs-VitePress-646CFF?logo=vitepress" alt="VitePress">
  <a href="https://github.com/yedsn/motion-cue/actions/workflows/pages.yml">
    <img src="https://github.com/yedsn/motion-cue/actions/workflows/pages.yml/badge.svg?branch=main" alt="Docs Site">
  </a>
  <a href="https://github.com/yedsn/motion-cue/actions/workflows/release.yml">
    <img src="https://github.com/yedsn/motion-cue/actions/workflows/release.yml/badge.svg" alt="Release">
  </a>
  <img src="https://img.shields.io/badge/License-MIT-yellow" alt="MIT License">
</p>

<p align="center">
  <a href="https://yedsn.github.io/motion-cue/">官网</a>
  ·
  <a href="https://github.com/yedsn/motion-cue">GitHub</a>
  ·
  <a href="https://gitee.com/hongxiaojian/motion-cue">Gitee</a>
  ·
  <a href="docs/guide/quick-start.md">快速开始</a>
  ·
  <a href="docs/guide/invocation.md">调用方式</a>
</p>

---

## 一句话介绍

MotionCue 使用 **Tauri 2 + Rust + Vue 3** 构建，常驻系统托盘，允许其他应用、Hook 和脚本通过自定义链接或命令行触发透明、置顶、鼠标穿透的多显示器动画与音效。

它适合放在这些地方：

- 发布脚本完成后弹出全屏成功提示
- 构建、测试、同步任务失败时播放错误反馈
- 编辑器 Hook、自动化流程或本地工具需要提醒你“事情已经完成”
- 长时间任务结束时，用动画和音效把注意力拉回来

## 为什么值得用

| 方向 | 你得到什么 |
|------|-------------|
| 即时反馈 | 通过 URL Scheme 或 CLI 在任何脚本里触发动画和音效 |
| 桌面覆盖层 | 透明、置顶、鼠标穿透，多显示器全屏提示不打断当前操作 |
| 内置动画 | 彩纸、成功、错误、专注、里程碑等常用提示开箱可用 |
| 自动化友好 | 支持命令行调用、JSON 清单、停止播放和打开管理界面 |
| 安全插件 | Web 插件运行在受限 iframe 中，无 Tauri/Node 桥接，能力由清单约束 |

## 核心概念

### Cue

Cue 是 MotionCue 的动画命令，例如 `task-complete`、`success`、`error`。外部脚本只需要指定命令名，就能触发对应的视觉与音效反馈。

### 调用入口

MotionCue 提供两种触发方式：

- **URL Scheme**：适合从浏览器、应用 Hook 或支持打开链接的工具调用。
- **命令行**：适合接入 PowerShell、CI 辅助脚本、本地构建脚本和编辑器任务。

### 桌面覆盖层

播放时会创建透明、置顶、鼠标穿透的覆盖层，把动画显示在桌面上，同时尽量不阻塞用户继续操作。

### Web 插件

插件使用本地 ZIP 包安装，入口页面通过 `window.motionCue` SDK 接收播放事件、报告完成或请求清单声明的本地音效。插件默认禁用，并运行在受限 sandbox iframe 中。

## 核心工作流

### 在脚本中提示任务完成

```powershell
try {
  npm run build
  motion-cue play task-complete
} catch {
  motion-cue play error --text "构建失败"
  throw
}
```

### 通过 URL Scheme 触发

```text
motioncue://play/task-complete
motioncue://play/success?text=发布完成&color=%2334c584
motioncue://stop
```

### 通过命令行触发

```powershell
motion-cue play task-complete
motion-cue play success --text "发布完成"
motion-cue list --json
motion-cue stop
motion-cue open
```

## 内置动画

| 命令 | 用途 |
|------|------|
| `confetti` | 全屏彩纸庆祝 |
| `task-complete` | 底部彩带、完成提示和成功音效 |
| `success` | 成功标记 |
| `milestone` | 里程碑光环 |
| `focus-start` | 开始专注 |
| `error` | 错误抖动 |
| `silent-confetti` | 静音彩纸 |

使用命令行查看当前清单：

```powershell
motion-cue list --json
```

## 快速开始

### 1. 安装应用

从 [GitHub Releases](https://github.com/yedsn/motion-cue/releases) 下载最新安装包。安装后启动 MotionCue，应用会常驻系统托盘。

### 2. 触发第一个动画

在 PowerShell 中执行：

```powershell
motion-cue play task-complete
```

也可以直接调用 URL Scheme：

```text
motioncue://play/task-complete
```

完整参数见：[调用方式](docs/guide/invocation.md)。

## 开发

### 准备环境

- Node.js 18+
- Rust stable 工具链
- Windows Visual Studio C++ 构建工具
- WebView2 Runtime

### 启动项目

```powershell
npm install
npm run tauri:dev
```

如果只想启动 Web 前端：

```powershell
npm run dev
```

### 检查与测试

```powershell
npm run typecheck
npm test
cargo test --manifest-path src-tauri/Cargo.toml
```

### 构建发布包

```powershell
npm run tauri:build
```

构建产物默认位于：

- Windows 可执行文件：`src-tauri/target/release/motion-cue.exe`
- Windows NSIS 安装包：`src-tauri/target/release/bundle/nsis/`

发布流程说明见：[GitHub Release](docs/develop/release.md)。

## 插件开发速览

MotionCue 支持使用 Web 插件扩展动画能力。

- 插件是本地 ZIP 包，根目录必须包含 `manifest.json` 和一个 HTML 入口。
- 插件页面运行在无 Tauri/Node/Rust 桥接的 sandbox iframe 中。
- CSP 禁止网络请求、远程媒体、外部导航、弹窗、下载和表单提交。
- 插件通过 `window.motionCue` SDK 接收播放事件、播放清单声明的本地音效并上报完成状态。

完整说明见：[Web 插件格式](docs/guide/plugin-format.md)。

## 平台边界

- 首版正式支持 Windows。
- 发布工作流已配置 Windows x64 NSIS 安装包，并包含 macOS Apple Silicon / Intel 应用包构建。
- 安全桌面、UAC 提权界面、锁屏和全屏独占游戏可能覆盖 MotionCue；应用不会通过注入或提升权限绕过系统限制。
- 混合 DPI、多显示器热插拔和鼠标穿透依赖 Windows WebView2 与窗口管理能力。
- 插件采用运行时间、消息大小、消息频率和资源包大小限制，但不承诺精确的跨设备 CPU/GPU 配额。

## 项目状态

这是一个面向本地自动化和开发工作流反馈的桌面工具项目：

- ✅ Windows 优先
- ✅ 支持 URL Scheme 与命令行接入
- ✅ 内置动画和受限 Web 插件模型
- ✅ GitHub Pages 文档站与 GitHub Releases 发布流程
- ✅ Tauri updater 发布配置与应用内手动更新入口

问题反馈和改进建议可以提交到 [GitHub Issues](https://github.com/yedsn/motion-cue/issues)。

## License

本项目采用 **MIT License** 许可证。

- 允许个人和商业使用、修改、分发、私有 fork
- 需要在副本中保留版权声明和许可声明

完整条款见：[LICENSE](LICENSE)。

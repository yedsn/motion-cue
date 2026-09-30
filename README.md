# MotionCue

MotionCue 是一个 Windows 优先的全屏动画与音效触发器。它常驻系统托盘，允许其他应用、Hook 和脚本通过自定义链接或命令行触发透明、置顶、鼠标穿透的多显示器动画。

## 调用方式

```text
motioncue://play/task-complete
motioncue://play/success?text=发布完成&color=%2334c584
```

```powershell
motion-cue play task-complete
motion-cue play success --text "发布完成"
motion-cue list --json
motion-cue stop
motion-cue open
```

## 内置动画

- `confetti`：全屏彩纸
- `task-complete`：左右底部多段彩带、顶部完成提示和成功音效
- `success`：成功标记
- `milestone`：里程碑光环
- `focus-start`：开始专注
- `error`：错误抖动
- `silent-confetti`：静音彩纸

## 自定义与插件

- 配置型动画只能使用受信任的内置渲染器，不执行用户脚本。
- Web 插件使用本地 ZIP 包安装，安装后默认禁用。
- 插件运行在无 Tauri/Node 桥接的 sandbox iframe 中，CSP 禁止网络、外部导航、表单和对象嵌入。
- 插件只能通过 `window.motionCue` SDK 接收播放事件、报告完成或请求清单声明的本地音效。

插件清单示例见 `docs/plugin-format.md`，恶意能力探测夹具见 `tests/adversarial-plugin/`。

## 开发

```powershell
npm install
npm run typecheck
npm test
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri:dev
npm run tauri:build
```

发布构建生成：

- `src-tauri/target/release/motion-cue.exe`
- `src-tauri/target/release/bundle/nsis/MotionCue_0.1.0_x64-setup.exe`

## 平台边界

- 首版正式支持 Windows。
- 安全桌面、UAC 提权界面、锁屏和全屏独占游戏可能覆盖 MotionCue；应用不会通过注入或提升权限绕过系统限制。
- 混合 DPI、多显示器热插拔和鼠标穿透依赖 Windows WebView2 与窗口管理能力。
- 插件采用运行时间、消息大小、消息频率和资源包大小限制，但不承诺精确的跨设备 CPU/GPU 配额。


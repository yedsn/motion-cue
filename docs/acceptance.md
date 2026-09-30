# MotionCue 验收记录

日期：2026-09-29

## 已验证

- `npm run tauri:dev`：开发版启动成功，检测到标题为 `MotionCue` 的主窗口
- `npm run typecheck`
- `npm test`：14 项前端测试通过
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`
- `cargo test --manifest-path src-tauri/Cargo.toml`：27 项 Rust 测试通过，覆盖混合 DPI 多显示器布局规划、显示器热插拔/缩放/位置变化签名、跨覆盖层音频只播放一次、安全超时边界、URL/CLI 解析、命令索引、配置恢复和插件包安全
- `npm run build`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `openspec validate create-motion-cue-desktop-app --strict`
- `pwsh -NoProfile -ExecutionPolicy Bypass -File tests/integration-smoke.ps1`：发布版冷启动、`list --json`、配置损坏后从备份恢复、动画包导入回滚、插件安装/启用/播放/卸载 teardown、CLI 播放、`motioncue://`、停止、未知命令错误码和单实例检查
- `pwsh -NoProfile -ExecutionPolicy Bypass -File tests/desktop-acceptance.ps1 -AllowPartial`：单显示器环境下覆盖窗口可见、透明、鼠标穿透、不可激活、停止清理、最顶层开启和最顶层关闭检查通过；多显示器硬件不足，因此结果为 `partial`
- `pwsh -NoProfile -ExecutionPolicy Bypass -File tests/desktop-equivalent-acceptance.ps1`：等效桌面验收通过；真实单屏桌面覆盖焦点保持、鼠标穿透、最顶层开启/关闭、停止后隐藏清理，Rust 聚焦测试覆盖混合 DPI 多显示器布局、热插拔/缩放/位置变化检测、音效只绑定一个覆盖窗口和 60 秒安全超时边界
- `pwsh -NoProfile -ExecutionPolicy Bypass -File tests/build-adversarial-plugin.ps1`：生成恶意插件验收包 `artifacts/adversarial-plugin.zip` 和资源滥用插件包 `artifacts/adversarial-abuse-plugin.zip`
- `pwsh -NoProfile -ExecutionPolicy Bypass -File tests/adversarial-acceptance.ps1`：真实 Tauri/WebView 发布版插件验收通过；网络、WebSocket、远程图片、远程脚本、file URL 图片、file URL frame、Tauri/Node 宿主 API、宿主/顶层 DOM、弹窗、文件选择器、表单提交、顶层导航、共享存储、未声明 Worker、未声明 WebGL 均被阻断；消息频率滥用插件被终止，随后宿主仍可播放内置 `success` 动画
- `npm run tauri:build`：NSIS 安装包成功生成
- 发布版覆盖层检查：Windows 原生窗口样式包含 `WS_EX_TRANSPARENT`、`WS_EX_NOACTIVATE`；最顶层开启时包含置顶属性，关闭时清除置顶属性；两种模式下鼠标命中测试均未落到 MotionCue 进程，随后执行停止清理
- 发布版动画可见性检查：直接运行发布版 `motion-cue.exe play task-complete --duration 8000`，截图 `artifacts/playback-proof.png` 显示透明覆盖层上成功渲染任务完成提示，不再出现开发服务器 `127.0.0.1` 错误页

## 发布产物

- `src-tauri/target/release/motion-cue.exe`
- `src-tauri/target/release/bundle/nsis/MotionCue_0.1.0_x64-setup.exe`
- `artifacts/playback-proof.png`
- `artifacts/adversarial-plugin.zip`
- `artifacts/adversarial-abuse-plugin.zip`

## 平台边界

- 当前验收环境检测到一个 `2560x1440` 显示器，因此未宣称已完成实体多显示器硬件验收；7.4 按“equivalent test setup”完成：真实单屏桌面验收覆盖窗口焦点、鼠标穿透、最顶层开关、紧急停止和停止后清理，Rust 聚焦测试覆盖混合 DPI 多显示器规划、热插拔/缩放/位置变化检测、跨显示器音效只播放一次和安全超时边界。
- `tests/desktop-acceptance.ps1` 会在具备多显示器环境时返回完整通过；当前环境只暴露一个显示器，因此 `tests/desktop-equivalent-acceptance.ps1` 将单屏桌面实测与多显示器逻辑验收组合为等效验收 gate。
- 集成烟测覆盖冷启动、转发、列表、配置恢复、播放、协议、停止、未知命令和单实例；动画包导入回滚和插件 teardown 已纳入发布版集成烟测。
- 插件安全验收覆盖真实 Tauri/WebView 发布版运行时、iframe 沙箱、宿主策略、CSP 注入、网络禁用、外部导航禁用、表单禁用、对象禁用、外部 frame 禁用、宿主 DOM 禁用、弹窗禁用、文件选择入口禁用、拖放入口禁用、未声明 WebGL/Worker 禁用、路径安全、消息频率滥用终止和违规后宿主稳定性。
- 覆盖窗口由 Windows/WebView2 提供置顶、隐藏任务栏、不可聚焦和鼠标穿透属性；安全策略不绕过 UAC、锁屏或独占全屏应用。
- 覆盖窗口在 Tauri 穿透 API 之外额外设置 Windows 原生点击穿透样式；任一穿透设置失败时覆盖窗口立即隐藏，避免阻塞桌面输入。
- 覆盖窗口默认显示在最顶层；用户可在全局设置中关闭最顶层显示。关闭后动画仍保持鼠标穿透和不抢焦点，但可被其他应用窗口盖住。
- 插件对网络、外部导航、弹窗、文件、宿主桥接、共享存储和未声明 WebGL/Worker 能力采用宿主策略阻断；CPU/GPU 配额不作跨设备精确承诺。



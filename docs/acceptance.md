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

## macOS 管理窗口与 Dock 生命周期验收

本节用于 `manage-macos-dock-visibility` 变更的实机验收。必须在真实 macOS 桌面环境执行；无界面测试和 Windows 环境不能替代 Dock 与菜单栏观察。

记录环境：

- 日期：待填写
- macOS 版本：待填写
- 处理器：待填写（Apple Silicon / Intel）
- MotionCue 版本或提交：待填写

在 Mac 源码目录中构建并启动：

```bash
npm ci
npm test
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri:build -- --bundles app,dmg
xattr -cr src-tauri/target/release/bundle/macos/MotionCue.app
open src-tauri/target/release/bundle/macos/MotionCue.app
```

下文中的 CLI 命令可直接使用应用包内可执行文件：

```bash
MOTION_CUE_BIN="$PWD/src-tauri/target/release/bundle/macos/MotionCue.app/Contents/MacOS/motion-cue"
"$MOTION_CUE_BIN" open
"$MOTION_CUE_BIN" play success
```

验收步骤：

- [ ] 从 Finder 或 Launchpad 正常启动 MotionCue，确认管理窗口、菜单栏图标和 Dock 图标同时显示，且可以通过 Dock 返回窗口。
- [ ] 点击管理窗口关闭按钮，确认应用没有退出、管理窗口消失、Dock 图标消失，并且菜单栏图标仍然存在。
- [ ] 点击菜单栏图标或“打开 MotionCue”，确认 Dock 图标恢复，原管理窗口显示并获得焦点，没有创建第二个实例。
- [ ] 关闭管理窗口后从菜单栏选择“检查更新”，确认 Dock 图标和管理窗口先恢复，再显示检查更新结果。
- [ ] 关闭管理窗口后执行 `motion-cue open`，确认 Dock 图标恢复并聚焦同一管理窗口。
- [ ] 在应用已运行且窗口隐藏时再次从 Finder 或 Launchpad 启动，确认现有实例的 Dock 图标与管理窗口恢复，不出现第二套菜单栏图标。
- [ ] 完全退出 MotionCue 后执行一次有效的后台播放调用，确认仅出现菜单栏图标和动画覆盖层，不出现管理窗口或 Dock 图标。
- [ ] 后台运行期间再次触发普通动画，确认动画播放不会自动显示管理窗口或 Dock 图标。
- [ ] 管理窗口和 Dock 已隐藏时，连续通过 `motioncue://play/...` 触发动画，确认只显示覆盖动画，Dock 与管理窗口保持隐藏，发起调用的程序不失去焦点。
- [ ] 完全退出 MotionCue 后首次通过 `motioncue://play/...` 冷启动，确认启动阶段也不短暂显示 Dock 或管理窗口，发起调用的程序始终保持焦点。
- [ ] 启动 MotionCue 后始终不打开管理窗口，连续调用 `motioncue://play/success`，确认动画可重复播放、MotionCue 不会无响应且不需要先激活主窗口。
- [ ] 清除已有 overlay 后首次调用 `motioncue://play/success`，确认首次创建覆盖窗口不会令 MotionCue 出现“应用程序无响应”。
- [ ] 连续至少 10 次执行“关闭窗口 -> 立即从菜单栏重新打开”，确认没有重复 Dock 图标、窗口失焦或只剩后台进程但无法打开的状态。
- [ ] 从菜单栏选择“退出”，确认进程、菜单栏图标和 Dock 图标均消失。

验收结果：待填写（通过 / 失败，并附失败步骤、截图或录屏路径）。



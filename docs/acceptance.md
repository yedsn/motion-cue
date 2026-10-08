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

## macOS 打包版播放焦点验收

关联变更：`preserve-macos-playback-focus`。普通调用必须保持焦点，不能用 `open -g` 或播放后恢复焦点代替。每项记录触发前、播放中、结束后的前台应用，并在实际调用应用连续输入，确认字符没有丢失。

环境字段：日期、macOS 版本、架构、提交/版本、协议处理应用绝对路径、是否存在旧实例。每项结果字段：前台应用变化、管理窗口与 Dock 状态、连续输入结果、动画结果、诊断/录屏位置。

| 场景 | 要求 | 结果 |
| --- | --- | --- |
| 完全退出后普通链接冷启动 | 后台启动，持续输入，隐藏管理窗口和 Dock | 待验收 |
| 管理窗口关闭后普通链接播放 | 原应用保持输入，窗口和 Dock 保持隐藏 | 待验收 |
| 管理窗口可见，调用应用在前台 | 原应用保持输入，管理窗口和 Dock 状态不变 | 候选方案失败，见下文 |
| 首次创建 overlay | 动画显示且原应用保持输入 | 待验收 |
| 连续不同参数与重复调用 | 每次播放及结束均不抢焦点 | 待验收 |
| 至少十次快速关闭与重开后播放 | 单实例、Dock 正常、输入不受影响 | 待验收 |
| Finder、菜单栏打开/检查更新、CLI open | 显式打开时聚焦管理窗口并恢复 Dock | 待验收 |

2026-10-08 初步定位：macOS 27.0.1（26A434）、arm64、MotionCue 0.1.14。待测应用为本仓库 `src-tauri/target/release/bundle/macos/MotionCue.app`，运行进程绝对路径已确认。通过 `open -a <待测应用> motioncue://play/task-complete?...` 指定包以避免系统旧协议缓存影响，未使用 `-g`。这是指定包的定位测试，尚不能替代默认协议及真实调用应用的验收。

- 未增加 LSUIElement 的诊断包：观察到前台从 Finder（PID 751）切换到 MotionCue（PID 85168）；播放日志正常。
- 增加 LSUIElement=true 并在产物中确认该键后的诊断包：前台仍从 Finder（PID 751）切换到 MotionCue（PID 87503）；链接接收时应用已经激活，策略为 Regular，管理窗口可见。
- 观察器每 10ms 读取 NSWorkspace 的前台应用并运行事件循环，不读取键盘内容。记录位于 `/tmp/motioncue-focus/before-fix.log` 和 `/tmp/motioncue-focus/lsuielement-visible-settled.log`，该临时目录可能被清理。
- 判定：LSUIElement 加动态 Regular/Accessory 方案尚未满足管理窗口可见时的焦点要求，不能宣称修复完成；需要修订链接接收架构。真实调用应用连续输入、菜单栏与 Dock 回归尚未完成。


2026-10-08 独立接收器验证：主包 Resources 包含 MotionCueLink.app，主包 CFBundleURLTypes 为空。接收器使用 LSBackgroundOnly 和 Prohibited 策略，普通协议默认处理器设置为 com.motioncue.desktop.link。管理窗口可见、主应用 Regular 模式时，普通 open motioncue://play/task-complete 调用的观察记录仅有 Finder，主应用诊断 active=false/main_visible=true，动画开始及 complete 均正常。记录 /tmp/motioncue-focus/default-helper.log。UIElement 接收器曾在重复调用时成为前台，因此最终采用 BackgroundOnly。

后续冷启动测试时桌面前台变为 com.apple.loginwindow，主应用报告“未找到可用显示器”；这是当前桌面会话不可用的证据，不能将该次测试计为通过。DMG 打包脚本同期失败，需在解锁且有显示器的桌面会话重试。真实连续输入、菜单栏、Dock 和快速关闭重开仍待验收。生命周期 Rust 专项 14 项测试通过。

最新自动化检查：前端 27/27 通过，npm run build 通过；Rust 48/49 通过，原有 rejects_path_traversal 在 macOS 对 C:/secret.txt 的拒绝断言失败；OpenSpec 严格校验和 git diff --check 通过。构建默认主包严格签名检查失败，临时对本地产物执行 codesign --force --deep --sign - 后严格检查通过，说明嵌套包可签名，但发布构建的自动签名接入仍须完成，不能以手动签名替代任务验收。当前桌面观察仍为 loginwindow。

自动签名接入已验证：macOS 配置新增 signingIdentity=-，使用正常 Tauri app 构建流程自动签名主包；无需事后手动签名，codesign --verify --deep --strict 主包及接收器独立严格检查均通过。该签名为本地 ad-hoc 签名，开发者证书/公证仍沿用发布环境设置。桌面会话依然为 loginwindow，交互验收待桌面恢复后进行。

Rust 全量复查已通过：49/49。插件路径校验补充跨平台 Windows 盘符与反斜线拒绝，新增 drive-relative、反斜线 traversal 和 UNC 断言，原有 macOS 失败已消除。当前桌面仍为 loginwindow，交互验收未获得新证据。

接收器交叉架构检查：指定 TAURI_ENV_TARGET_TRIPLE=x86_64-apple-darwin 编译得到 x86_64 Mach-O，并通过独立严格签名检查；随后恢复本机 arm64 资源。接收器签名读取 APPLE_SIGNING_IDENTITY，无证书时使用 ad-hoc，启用 hardened runtime；转发子进程非零退出会写入系统日志且不包含链接文字。完整 Intel 主应用及 DMG 未在本机验收。

桌面恢复后冷启动复验：终止主应用进程，保留禁止激活的接收器，普通 open motioncue://play/task-complete 调用成功启动 --background 主应用。观察记录 /tmp/motioncue-focus/resumed-cold.log 全程保持 cc.ggbond.mactools（PID 17348），主应用 active=false、Accessory、管理窗口隐藏，动画开始并 complete 正常。该结果证明主应用冷启动路径和首次 overlay 可播放且未观察到前台切换；接收器也完全退出的冷启动及实际连续输入仍需验证。

完整冷启动和 DMG 验证：app/dmg 构建命令退出 0，DMG hdiutil verify 校验有效。终止主应用及 MotionCueLink 两个进程后执行普通 open motioncue://play/task-complete，系统重新启动接收器和后台主应用，动画正常开始及 complete；10ms 前台观察始终为 cc.ggbond.mactools（PID 17348），主应用 active=false、Accessory、管理窗口隐藏。记录 /tmp/motioncue-focus/all-cold.log。真实连续输入和管理窗口/Dock 人工回归仍未完成。

管理窗口可见时重复播放复验：CLI open 后确认诊断 main_visible=true、Regular；切到 Finder 后普通链接连续传入不同文字，日志记录第一次 replaced、第二次 complete，两次播放 active=false。观察 /tmp/motioncue-focus/final-repeated-visible.log 没有 MotionCue 或接收器成为前台，末尾用户切到 mactools。生命周期专项新增重复调用和迟到请求取消测试，16/16 通过。真实输入及菜单栏/Dock 回归仍待反馈。

最终源码自动化复查：前端 27/27、Rust 51/51，npm run build、OpenSpec 严格校验、git diff --check 均通过。最近新增的两项生命周期测试属于测试代码，运行时打包产物与已验证实现保持一致。实际调用程序连续输入和菜单栏/Dock/退出/十次快速关闭重开仍未收到人工验收结果，不将这些项目视为完成。

### 用户实机验收反馈（2026-10-08）

用户在收到实际调用程序连续输入和管理窗口/Dock 回归验收问题后反馈：“试了一下，确实是不失焦了，窗口和Dock也都正常”。该反馈确认实际使用中的输入焦点、窗口及 Dock 行为正常，与普通协议调用、完整冷启动、首次覆盖窗口及可见管理窗口连续播放的前台观察结果一致。没有记录用户逐项操作次数，因此十次快速关闭重开以及检查更新、菜单栏退出的逐项结果仍不得推定为已验证。

### 最终管理入口验收（2026-10-08）

在明确询问菜单栏检查更新、退出及至少十次关闭/重新打开后，用户回复“都正常了”。结合上一轮用户确认不失焦且窗口/Dock 正常，管理入口回归通过。原 manage-macos-dock-visibility 的窗口、Dock 和菜单栏验收以本次反馈交叉记录；无需推定未报告的架构或系统环境。

最终结论：独立后台接收器修复普通协议播放引发的激活问题，冷启动、后台和管理窗口可见时的播放观察通过，真实输入和管理入口由用户验收通过；app/dmg、签名及自动化检查通过。历史原包关闭窗口场景只有用户报告，实际捕获的失焦定位来自管理窗口可见的诊断包；任务 1.1 中旧包关闭窗口场景的精确复现仍缺少证据，未声称重现了旧包关闭窗口的全部事件轨迹。

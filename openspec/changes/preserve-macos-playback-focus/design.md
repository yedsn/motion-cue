# Design

## Context

动机见 proposal.md。现有覆盖窗口已设置不可聚焦、鼠标穿透，macOS 用 AppKit orderFrontRegardless 显示。lib.rs 默认隐藏主窗口，进入事件循环前切换 Accessory，后台关闭时隐藏 Dock；播放 URL 抑制 Reopen，GUI/Reopen 在 250ms 后决定是否打开窗口，抑制期为 750ms。

安装包 Info.plist 没有 LSUIElement。Tao 的启动路径即使 activate_ignoring_other_apps 为 false 仍调用 activateIgnoringOtherApps(false)，该配置不等于禁止激活。用户报告打包版失焦，开发版正常；尚未记录完整系统事件轨迹，不能把任一候选原因当作已验证根因。

主规格库存为空，相关契约位于已有变更。沿用 macos-app-lifecycle 路径，本变更补充打包版焦点边界，不重写旧变更；归档或同步时与原生命周期要求合并。旧设计排除 LSUIElement 的决定将在本变更中重新验证。

## Goals / Non-Goals

**Goals:**
- 从应用包身份、宿主启动、链接事件仲裁和窗口显示四层保持非激活播放。
- 用可测试决策区分播放和显式管理操作，以真实打包版的持续输入作为验收依据。

**Non-Goals:**
- 不调用辅助功能接口注入按键，不记录输入内容，不通过播放后抢回焦点补偿失焦。
- 不改动 URL 格式、动画渲染、Windows 生命周期或添加用户设置。

## Decisions

### 1. 应用包声明默认后台身份

通过 src-tauri/Info.plist 合并 LSUIElement=true（Tauri 已支持同目录自动合并），构建后检查产物实际键值与 URL 注册。后台身份在 LaunchServices 启动进程前生效，运行时仍保留 Accessory；显式管理操作才切换 Regular 并恢复 Dock。

此选项优于只在 URL 回调内隐藏窗口，因为后者无法阻止回调前的应用激活。LSUIElement 本身不保证所有焦点场景通过，必须结合后续仲裁和验收；不使用 LSBackgroundOnly，因为需要菜单栏、WebView 和可交互管理窗口。

### 2. 播放不进入应用激活路径

在 app_lifecycle.rs 中表达启动/播放/显式打开的决策；lib.rs 的 URL、CLI、单实例、GUI 启动和 Reopen 使用一致决策。收到播放请求取消尚未执行的 GUI/Reopen 打开动作，不改变已有管理窗口可见性，不在播放回调重复切换 Dock 或激活策略。

实现前以轻量诊断记录事件来源、顺序、应用激活状态及窗口打开原因，复现冷启动和管理窗口可见的链接调用。日志不包含 URL 文本内容。不能仅增大 250/750ms 常量作为修复；计时只用于必要的系统事件合并，显式请求拥有明确语义，覆盖迟到 URL 和快速多次请求测试。

管理窗口可见时 Regular 身份仍可能被 LaunchServices 激活。该场景必须单独验证；若主应用身份切换无法保证不激活，则需修订设计并评估独立后台 URL 接收入口，不以恢复焦点宣称通过，也不把未验证方案直接发布。

### 3. 保留覆盖窗口的非激活显示

沿用 orderFrontRegardless、set_focusable(false) 和鼠标穿透，检查首次创建与复用窗口都不调用 set_focus 或普通 show。所有 AppKit 操作在主线程执行，播放工作继续离开深链事件循环，避免同步创建 WebView 自锁。只有实测定位到窗口问题时才评估非激活 NSPanel；本次不预先替换窗口系统。

### 4. 验收以打包产物与持续输入为准

记录实际协议处理应用路径、版本、macOS 版本及架构，隔离开发版和旧安装版。覆盖冷启动、后台运行、管理窗口可见但不在前台、首次 overlay、连续不同参数和快速关闭重开。检查触发前、播放期间、结束后的前台应用并人工确认连续输入不丢失。允许对照 open -g 或 CLI 用于定位，但普通调用方无需修改打开链接的方式才算修复完成。

## Risks / Trade-offs

- [LSUIElement 与动态 Regular/Dock 切换可能影响正常打开] → 实机覆盖 Finder、菜单栏、检查更新、CLI open 和关闭后播放；不以固定隐藏 Dock 牺牲管理入口。
- [系统链接启动与 Reopen 顺序不固定] → 记录事件轨迹，测试两种顺序、迟到事件及重复调用，避免计时常量承担唯一正确性保障。
- [Regular 管理界面存在时系统仍可能激活应用] → 设置明确验收门槛，未通过时阻止发布并修订架构方案。
- [运行中实例或系统协议缓存指向旧包] → 验收前确认处理应用路径并仅保留待测实例。
- [单元测试不能观察键盘焦点] → Rust 测试验证决策，打包版桌面验收验证用户输入，分别记录结果。

## Migration Plan

无数据迁移。按当前发布方式构建新 app/dmg，验证 Info.plist 和协议声明后进行实机验收。回滚恢复旧包与对应源码，不更改用户配置或插件数据。与 manage-macos-dock-visibility 的未完成实机验收交叉记录，不能因自动化通过将旧验收项直接标为完成。

## 已批准的设计调整（2026-10-08）

实机证明管理窗口处于 Regular 模式时 LSUIElement 不能阻止系统链接激活。用户批准独立后台 URL 接收入口：在主应用 Resources 内打包 MotionCueLink.app，由该应用独占 macOS 的 motioncue 协议注册。接收器使用禁止激活策略，无窗口和 Dock，通过主包可执行文件现有 CLI/鉴权 IPC 转发链接；冷启动沿用 --background。主包不再声明播放协议，显式管理打开保持原有行为。构建 hook 按目标架构编译接收器，主应用运行时注册随包接收器；更新包与 DMG 包含接收器。接收器不接受任意执行路径，不执行 shell，转发失败写入系统日志。Windows 保留原协议配置。

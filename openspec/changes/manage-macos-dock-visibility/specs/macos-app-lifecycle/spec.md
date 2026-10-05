# Spec Delta

## Purpose

规范 MotionCue 在 macOS 上管理窗口、Dock 图标和菜单栏图标之间的可见性联动，使应用既能像普通窗口应用一样操作，也能在窗口关闭后作为轻量菜单栏应用继续运行。

## ADDED Requirements

### Requirement: 管理窗口可见时显示 Dock 图标
系统 SHALL 在 macOS 管理窗口可见时显示 MotionCue 的 Dock 图标，使用户能够通过 Dock 和系统应用切换器返回管理窗口。

#### Scenario: 正常启动桌面应用
- **WHEN** 用户从 Finder、Launchpad 或 Dock 正常启动 MotionCue，且该启动不是后台调用
- **THEN** 系统显示管理窗口、菜单栏图标和 Dock 图标，并使管理窗口可交互

#### Scenario: 从菜单栏重新打开管理窗口
- **WHEN** 管理窗口已隐藏且用户通过菜单栏图标或“打开 MotionCue”操作请求打开管理窗口
- **THEN** 系统恢复 Dock 图标，并显示、还原和聚焦管理窗口

#### Scenario: 从外部打开命令恢复管理窗口
- **WHEN** 管理窗口已隐藏且现有实例收到语义为打开管理界面的命令或第二实例唤醒请求
- **THEN** 系统恢复 Dock 图标，并显示、还原和聚焦同一个管理窗口，不创建重复实例

#### Scenario: 检查更新时打开管理窗口
- **WHEN** 管理窗口已隐藏且用户从菜单栏选择“检查更新”
- **THEN** 系统先恢复 Dock 图标并显示管理窗口，再进入检查更新流程

### Requirement: 关闭管理窗口后仅保留菜单栏入口
系统 MUST 在 macOS 用户关闭管理窗口时阻止应用进程退出，隐藏管理窗口和 Dock 图标，并保持菜单栏图标、后台调用处理和动画播放能力可用。

#### Scenario: 用户关闭管理窗口
- **WHEN** 用户点击管理窗口关闭按钮或触发等价的窗口关闭操作
- **THEN** 系统隐藏管理窗口和 Dock 图标，保留菜单栏图标，并继续运行当前实例

#### Scenario: 关闭窗口后触发动画
- **WHEN** 管理窗口和 Dock 图标已隐藏，外部调用触发一个有效动画
- **THEN** 系统正常播放动画且不因该后台播放自动显示管理窗口或 Dock 图标

### Requirement: 后台启动不显示管理窗口或 Dock 图标
系统 SHALL 在 macOS 因播放、协议转发或其他后台调用而冷启动常驻实例时，仅建立菜单栏和后台运行能力，不显示管理窗口或 Dock 图标。

#### Scenario: 无运行实例时触发后台播放
- **WHEN** MotionCue 尚未运行且外部命令或自定义链接触发动画播放
- **THEN** 系统启动单个后台实例、显示菜单栏图标并处理调用，同时保持管理窗口和 Dock 图标隐藏

### Requirement: 显式退出终止常驻实例
系统 SHALL 保留菜单栏“退出”操作，并在用户选择退出时彻底终止应用，而不是仅隐藏管理窗口或 Dock 图标。

#### Scenario: 从菜单栏退出
- **WHEN** 用户从菜单栏菜单选择“退出”
- **THEN** 系统停止常驻实例并移除菜单栏图标和 Dock 图标

### Requirement: 非 macOS 平台行为保持兼容
系统 MUST 将 Dock 可见性联动限制在 macOS，不改变其他受支持平台现有的托盘、窗口关闭和窗口打开行为。

#### Scenario: Windows 用户关闭并重新打开管理窗口
- **WHEN** 用户在 Windows 上关闭管理窗口后再从系统托盘打开 MotionCue
- **THEN** 系统继续按照现有 Windows 托盘生命周期隐藏和恢复窗口，不执行 macOS Dock 操作

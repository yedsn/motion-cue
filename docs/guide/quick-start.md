# 快速开始

MotionCue 是一个 Windows 优先的动画与音效触发器，适合接入发布脚本、自动化任务、编辑器 Hook 和本地工具。

## 安装与启动

从 [GitHub Releases](https://github.com/yedsn/motion-cue/releases) 下载最新安装包。安装后启动 MotionCue，应用会常驻系统托盘。

开发环境启动方式见[开发环境](/develop/setup)。

## 第一次触发

在 PowerShell 中执行：

```powershell
motion-cue play task-complete
```

也可以调用 URL Scheme：

```text
motioncue://play/task-complete
```

完整参数和返回值见[调用方式](/guide/invocation)。

## 内置动画

MotionCue 自带成功、错误、专注和里程碑等动画，不安装插件也可以直接使用。可用命令见[内置动画](/guide/built-in-cues)。

# 调用方式

MotionCue 提供 URL Scheme 和命令行两种调用入口。两者共享同一套动画命令和参数。

## URL Scheme

```text
motioncue://play/task-complete
motioncue://play/success?text=发布完成&color=%2334c584
motioncue://stop
```

参数值应进行 URL 编码。`text` 用于覆盖提示文案，`color` 用于指定主题色。

## 命令行

```powershell
motion-cue play task-complete
motion-cue play success --text "发布完成"
motion-cue list --json
motion-cue stop
motion-cue open
```

## 接入脚本

发布脚本可以在任务成功或失败的分支中调用对应命令：

```powershell
try {
  npm run build
  motion-cue play task-complete
} catch {
  motion-cue play error --text "构建失败"
  throw
}
```

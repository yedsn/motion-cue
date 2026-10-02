# 内置动画

当前版本提供以下内置动画：

| 命令 | 用途 |
| --- | --- |
| `confetti` | 全屏彩纸庆祝 |
| `task-complete` | 底部彩带、完成提示和成功音效 |
| `success` | 成功标记 |
| `focus-start` | 开始专注 |
| `error` | 错误抖动 |
| `silent-confetti` | 静音彩纸 |
| `material-flow` | 全屏流光材质 |
| `corner-fireworks` | 角落烟花庆祝 |
| `focus-spotlight` | 专注光罩 |

新安装的默认目录不再包含 `milestone`。如果旧版本配置里已经存在该动画，升级后会保留原配置，用户可以继续使用、禁用或删除。

使用命令行查看当前清单：

```powershell
motion-cue list --json
```

使用 `motion-cue stop` 可以停止当前播放。

## 内置音效

应用启动后会把内置音效文件复制到应用数据目录的 `resources/audio` 下，已有同名文件不会被覆盖。需要替换内置音效时，可以直接替换这个目录里的同名文件。

从管理界面导入的外置音效也会复制到同一目录，并保留原文件名；如果同名文件已存在，会自动追加序号。当前支持 `wav`、`mp3`、`ogg` 和 `flac`。

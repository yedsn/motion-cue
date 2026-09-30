# Web 插件格式

插件是一个 ZIP 文件，根目录必须包含 `manifest.json` 和一个 HTML 入口。包内不得包含绝对路径、`..`、符号链接或未知文件类型。

## 清单示例

```json
{
  "schemaVersion": 1,
  "id": "release-fireworks",
  "name": "Release Fireworks",
  "version": "1.0.0",
  "entry": "index.html",
  "command": "release-fireworks",
  "aliases": ["release-party"],
  "resources": ["main.js", "style.css", "spark.png"],
  "sounds": { "impact": "impact.wav" },
  "capabilities": { "webgl": false, "worker": false }
}
```

## SDK

入口页面通过宿主注入的 `window.motionCue` 通信：

```js
motionCue.onPlay(({ params, viewport }) => {
  startAnimation(params, viewport);
  motionCue.sound("impact");
});

motionCue.ready();
motionCue.complete();
```

可用方法包括 `onPlay`、`ready`、`complete`、`error` 和 `sound`。插件也可以监听 `motioncue-stop` 事件清理动画。

## 安全边界

- 无 Tauri、Node、Rust 命令和任意文件访问。
- 禁止网络请求、远程媒体、外部导航、弹窗、下载和表单提交。
- WebGL 和 Worker 必须在清单中声明。
- 单次消息、消息频率、插件包和单资源均有宿主限制。

原始格式说明见仓库中的 [`docs/plugin-format.md`](https://github.com/yedsn/motion-cue/blob/main/docs/plugin-format.md)。

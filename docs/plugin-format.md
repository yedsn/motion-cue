# MotionCue Web 插件格式

插件是一个 ZIP 文件，根目录必须包含 `manifest.json` 和一个 HTML 入口。包内不得包含绝对路径、`..`、符号链接或未知文件类型。

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
// 完成时调用 motionCue.complete()
```

可用方法：

- `onPlay(callback)`：接收校验后的参数和窗口尺寸。
- `ready()`：报告插件已准备完成，必须在三秒内调用。
- `complete()`：报告动画完成。
- `error(message)`：报告不超过 500 字符的错误。
- `sound(id)`：请求播放 `manifest.json` 中声明的本地音效。

插件可以监听 `motioncue-stop` 事件清理动画。

## 安全边界

- 无 Tauri、Node、Rust 命令和任意文件访问。
- `connect-src 'none'`，禁止 fetch、WebSocket 和远程媒体。
- 禁止弹窗、下载、表单、顶层导航和外部页面。
- 不提供共享 Cookie 或持久化插件身份存储。
- WebGL 和 Worker 必须在清单声明；未声明时宿主会禁用对应构造能力。
- 单次消息、消息频率、插件包和单资源均有宿主限制；持续违规会终止会话。


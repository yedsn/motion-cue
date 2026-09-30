# 开发环境

## 前置依赖

- Node.js 18 或更高版本
- Rust stable 工具链
- Windows 上的 Visual Studio C++ 构建工具
- WebView2 Runtime

安装依赖：

```powershell
npm install
```

## 启动项目

只启动前端：

```powershell
npm run dev
```

启动完整 Tauri 桌面应用：

```powershell
npm run tauri:dev
```

VS Code 的 `launch.json` 已提供桌面开发、前端检查、测试、文档开发和文档构建入口。

# GitHub Release

MotionCue 通过 `.github/workflows/release.yml` 发布 Windows 安装包。

## 推荐发布方式

项目提供了与 OpenDock 类似的 Windows 发布脚本，会同步更新三个版本文件、更新锁文件、创建 release commit 和 annotated tag。

直接执行并推送：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/release/release.ps1 -Push
```

脚本会提示版本号，默认使用当前版本的下一个 patch 版本，例如 `0.1.0` -> `0.1.1`。也可以显式传入版本：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/release/release.ps1 0.1.1 -Push
```

VS Code 中可以直接运行 `.vscode/launch.json` 里的 `release` 配置。

脚本会把当前工作区所有改动加入 release commit，因此运行前应确认工作区中的改动都属于本次发布。

## 触发方式

创建并推送 `v*` 标签：

```powershell
git tag v0.1.0
git push origin v0.1.0
```

工作流会校验以下版本必须与标签一致：

- `package.json`
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`

校验通过后会在 Windows runner 上构建 NSIS 安装包，并上传到对应的 GitHub Release。

## 发布前检查

```powershell
npm run typecheck
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri:build
```

当前项目尚未启用 Tauri updater，因此发布工作流不会要求签名密钥，也不会生成 `latest.json`。

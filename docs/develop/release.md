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

## Gitee Release 同步

GitHub Release 发布成功后，`sync-gitee` job 会在 self-hosted runner 上把 GitHub Release 附件同步到 Gitee 固定 `latest` Release：

```text
https://gitee.com/hongxiaojian/motion-cue/releases/download/latest/
```

同步脚本：

```text
scripts/release/sync_gitee_release.py
```

同步流程：

1. 使用 `GITHUB_TOKEN` 读取当前 tag 对应的 GitHub Release。
2. 通过代理 `http://192.168.3.36:7890` 下载 GitHub Release 附件。
3. 使用 `GITEE_ACCESS_TOKEN` 创建或更新 Gitee 的 `latest` Release。
4. 删除 Gitee `latest` Release 上的旧附件，并上传本次发布附件。
5. 如果未来启用 Tauri updater 并生成 `latest.json`，同步脚本会把其中的下载地址改写为 Gitee 地址。

当前项目尚未启用 Tauri updater，因此同步目标主要是安装包等 Release 附件。

## GitHub 仓库配置

需要在 GitHub 仓库中完成以下配置：

1. `Settings -> Actions -> General`：确认 GitHub Actions 已启用；工作流中已声明 `contents: write` 用于发布 Release 资产。
2. `Settings -> Secrets and variables -> Actions`：新增仓库 Secret `GITEE_ACCESS_TOKEN`，值为具备 Gitee Release 创建、更新和附件上传权限的访问令牌。
3. `Settings -> Actions -> Runners`：新增仓库级 self-hosted runner，并确保标签包含 `self-hosted`、`linux`、`x64`、`gitee-sync`。
4. 确认 runner 机器已安装 `python3`、`curl`、`git`，并能访问 GitHub Release 下载地址与 `https://gitee.com/api/v5`。

`sync-gitee` job 使用的 runner 标签为：

```yaml
runs-on: [self-hosted, linux, x64, gitee-sync]
```

runner 代理环境为：

```yaml
https_proxy: http://192.168.3.36:7890
http_proxy: http://192.168.3.36:7890
no_proxy: gitee.com,localhost,127.0.0.1
```

## Debian self-hosted runner 一键配置

服务器上可以直接运行项目提供的安装脚本：

```bash
sudo bash scripts/release/setup_github_runner_debian.sh
```

脚本会交互式要求输入：

- GitHub 仓库地址，默认 `https://github.com/yedsn/motion-cue`
- GitHub runner registration token
- runner 名称，默认 `motion-cue`
- runner 标签，默认 `self-hosted,linux,x64,gitee-sync`
- Linux 运行用户，默认 `github-runner`
- 安装目录名，默认 `/home/github-runner/motion-cue`
- 下载代理，默认 `http://192.168.3.36:7890`，输入 `none` 可禁用代理

获取 registration token 的入口：

1. 打开 GitHub 仓库 `https://github.com/yedsn/motion-cue`
2. 进入 `Settings -> Actions -> Runners`
3. 点击 `New self-hosted runner`
4. 选择 `Linux`
5. 复制页面显示的 token，并在脚本提示时粘贴

脚本会创建专用用户、下载 GitHub Actions runner、校验 sha256、配置 runner、安装 systemd 服务并启动。安装完成后回到 GitHub runner 页面确认状态为 `Online`。

脚本支持失败后继续执行：

- 如果 runner 压缩包已下载且校验通过，会直接复用。
- 如果 runner 文件已解压，会跳过重复解压。
- 如果 `.runner` 已存在，会跳过 `config.sh`，继续安装或启动服务。
- 如果安装目录非空但不像 GitHub runner 目录，脚本会停止，避免误操作其他目录。

如果配置阶段出现类似下面的错误：

```text
Http response code: NotFound from 'POST https://api.github.com/actions/runner-registration'
```

优先检查：

- token 必须来自 `https://github.com/yedsn/motion-cue` 仓库的 `Settings -> Actions -> Runners -> New self-hosted runner` 页面，不能复用其他仓库的 token。
- token 有有效期，过期后需要重新生成。
- 脚本提示的 GitHub 仓库地址必须与生成 token 的仓库一致。
- 如果服务器访问 GitHub 需要代理，脚本的下载代理不要填 `none`。脚本会把该代理同时用于 runner 下载和 `config.sh` 注册请求。

## 发布前检查

```powershell
npm run typecheck
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri:build
```

当前项目尚未启用 Tauri updater，因此发布工作流不会要求签名密钥，也不会生成 `latest.json`。

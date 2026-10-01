# GitHub Release

MotionCue 通过 `.github/workflows/release.yml` 发布 Windows 安装包和 macOS 应用包。

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

校验通过后会构建并上传以下 GitHub Release 资产：

- Windows x64：NSIS 安装包
- macOS Apple Silicon：`.app` 和 `.dmg`
- macOS Intel：`.app` 和 `.dmg`

## Gitee Release 同步

GitHub Release 发布成功后，`sync-gitee` job 可以在 self-hosted runner 上把 GitHub Release 附件同步到 Gitee 固定 `latest` Release。该 job 受 GitHub Actions Variable `ENABLE_GITEE_SYNC` 控制，只有设置为 `true` 时才会运行：

```text
https://gitee.com/hongxiaojian/motion-cue/releases/download/latest/
```

同步脚本：

```text
scripts/release/sync_gitee_release.py
```

同步流程：

1. 使用 `GITHUB_TOKEN` 读取当前 tag 对应的 GitHub Release。
2. 如果配置了 `GITEE_SYNC_PROXY`，通过该代理下载 GitHub Release 附件。
3. 使用 `GITEE_ACCESS_TOKEN` 创建或更新 Gitee 的 `latest` Release。
4. 删除 Gitee `latest` Release 上的旧附件，并上传本次发布附件。
5. 对 `latest.json` 中的下载地址改写为 Gitee 地址，保证应用内更新优先走 Gitee 镜像。

当前项目已启用 Tauri updater，发布资产会包含 `latest.json`、安装包和对应签名文件。同步脚本会把 `latest.json` 中的下载地址改写为 Gitee `latest` Release 附件地址，应用会优先检查 Gitee，再回退到 GitHub。

## Tauri updater 签名配置

首次正式发布前需要生成一次 updater 签名密钥：

```powershell
npx tauri signer generate
```

生成后只提交 public key：

1. 把 public key 填入 `src-tauri/tauri.conf.json` 的 `plugins.updater.pubkey`，替换 `REPLACE_WITH_TAURI_UPDATER_PUBLIC_KEY`。
2. 不要提交 private key。
3. 在 GitHub 仓库 `Settings -> Secrets and variables -> Actions` 中新增：
   - `TAURI_SIGNING_PRIVATE_KEY`
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

Release workflow 会在构建前校验 updater 公钥和签名 Secrets。如果仍是占位公钥，或者没有配置 Secrets，workflow 会提前失败。

## GitHub 仓库配置

需要在 GitHub 仓库中完成以下配置：

1. `Settings -> Actions -> General`：确认 GitHub Actions 已启用；工作流中已声明 `contents: write` 用于发布 Release 资产。
2. `Settings -> Secrets and variables -> Actions -> Secrets`：新增仓库 Secret `GITEE_ACCESS_TOKEN`，值为具备 Gitee Release 创建、更新和附件上传权限的访问令牌；同时新增 updater 签名 Secrets `TAURI_SIGNING_PRIVATE_KEY` 和 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。
3. `Settings -> Secrets and variables -> Actions -> Variables`：如果需要同步到 Gitee，新增 `ENABLE_GITEE_SYNC=true`；如果 runner 需要代理下载 GitHub Release 资产，新增 `GITEE_SYNC_PROXY`，例如 `http://192.168.3.36:7890`。
4. `Settings -> Actions -> Runners`：新增仓库级 self-hosted runner，并确保标签包含 `self-hosted`、`linux`、`x64`、`gitee-sync`。
5. 确认 runner 机器已安装 `python3`、`curl`、`git`，并能访问 GitHub Release 下载地址与 `https://gitee.com/api/v5`。

`sync-gitee` job 使用的 runner 标签为：

```yaml
runs-on: [self-hosted, linux, x64, gitee-sync]
```

runner 代理通过 GitHub Actions Variable 配置：

```yaml
ENABLE_GITEE_SYNC: true
GITEE_SYNC_PROXY: http://192.168.3.36:7890
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

## 发布后验证

发布完成后检查：

1. GitHub Release 中存在 Windows NSIS、macOS DMG、`.sig` 和 `latest.json`。
2. Gitee `latest` Release 中存在同步后的安装包、签名文件和 `latest.json`。
3. 直接打开 `https://gitee.com/hongxiaojian/motion-cue/releases/download/latest/latest.json`，确认其中的下载地址指向 Gitee。
4. 安装旧版本 MotionCue，在“全局设置 -> 应用更新”中检查更新、下载并安装，然后重启确认版本号已更新。

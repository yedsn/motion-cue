# 构建与验证

## 前端检查

```powershell
npm run typecheck
npm test
npm run build
```

## Rust 检查

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

## 桌面安装包

```powershell
npm run tauri:build
```

Windows 安装包会生成在 `src-tauri/target/release/bundle/nsis/`。

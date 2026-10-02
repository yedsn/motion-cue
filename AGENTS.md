# Repository Guidelines

## Project Structure & Module Organization

MotionCue is a Tauri 2 app with a Vue 3 frontend. UI source lives in `src-ui/src`, with views in `views`, shared services in `services`, and audio assets in `src-ui/public/audio`. Rust desktop code lives in `src-tauri/src`; Tauri config, capabilities, generated schemas, and icons are under `src-tauri`. Documentation is in `docs`, OpenSpec artifacts in `openspec`, release helpers in `scripts/release`, and acceptance scripts plus test plugins in `tests`. Build outputs are kept in `dist` and `artifacts`.

## Build, Test, and Development Commands

- `npm run dev`: starts the Vite UI server on `127.0.0.1`.
- `npm run tauri:dev`: runs the full desktop app in development mode.
- `npm run build`: type-checks Vue/TypeScript and builds the UI.
- `npm run tauri:build`: creates a packaged Tauri release build.
- `npm test`: runs Vitest unit tests for frontend services.
- `cargo test --manifest-path src-tauri/Cargo.toml`: runs Rust tests.
- `npm run docs:dev` / `npm run docs:build`: previews or builds VitePress docs.
- `powershell -File tests/desktop-acceptance.ps1`: runs Windows desktop acceptance checks after a release build.

## Coding Style & Naming Conventions

Use TypeScript, Vue single-file components, and Rust 2021 idioms already present in the codebase. Keep TypeScript indentation at two spaces and Rust formatted with `cargo fmt`. Use `camelCase` for TypeScript variables and serialized JSON fields, `PascalCase` for Vue components/types, and `snake_case` for Rust modules/functions. Prefer small service modules and focused Rust modules instead of expanding `lib.rs` unnecessarily.

## Testing Guidelines

Place frontend unit tests next to covered code with `*.test.ts` names, using Vitest `describe`/`it` blocks. Add Rust tests in the relevant module when changing parsing, catalog, playback, settings, or plugin behavior. Use `tests/*.ps1` for desktop, integration, and adversarial plugin acceptance coverage. Run `npm test`, `npm run build`, and relevant acceptance scripts before release-facing changes.

## Commit & Pull Request Guidelines

Recent history uses short subjects such as `release: v0.1.9` and Chinese summaries like `检测更新`. Keep commits concise and action-oriented; include the version prefix for release commits. Pull requests should describe the user-visible change, list tests run, link related issues or OpenSpec changes, and include screenshots for UI, overlay, or plugin behavior changes.

## Security & Configuration Tips

Treat plugin loading and IPC paths as security-sensitive. Preserve sandbox restrictions for web plugins, validate manifest and message payloads, and avoid exposing Tauri or Node capabilities to plugin iframes. Do not commit local app data, generated secrets, updater keys, or machine-specific configuration.

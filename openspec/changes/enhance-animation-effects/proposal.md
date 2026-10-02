# Proposal

## Why

当前内置动画里部分效果辨识度不足，`task-complete` 的中间圆圈也会分散对顶部提示和彩带的注意力。MotionCue 需要一组更有用、可调、适合全屏覆盖层的动画模板，让用户能按任务场景配置反馈强度，而不是只能选择少量固定效果。

## What Changes

- 调整 `task-complete`：保留顶部提示、左右底部彩带和成功音效，移除中间圆圈。
- 下线 `milestone` 内置动画：首次默认目录不再提供该动画；既有用户配置不得因升级丢失或命令冲突。
- 新增三类内置动画效果：
  - `material-flow`：全屏流光/粒子材质，适合较高级的完成反馈。
  - `corner-fireworks`：从屏幕角落发射的小型烟花/彩带，适合成功或发布完成。
  - `focus-spotlight`：开始专注时的光罩/聚焦提示，替代单纯徽章脉冲。
- 扩展配置型动画参数，让用户可调整强度、速度、密度、位置、亮度、扩散范围等安全范围内的视觉参数。
- 保留 `success`、`focus-start`、`error`、`confetti`、`task-complete` 和 `silent-confetti` 的命令用途，并让 `focus-start` 可迁移到新的聚焦效果。

## Capabilities

### New Capabilities

- `animation-effects`: Defines the built-in effect catalog, effect-specific visual behavior, and safe parameter customization for first-party animation renderers.

### Modified Capabilities

None. The project currently has no durable main specs under `openspec/specs`; this change captures the behavior as a new capability rather than modifying an archived change spec.

## Impact

- Affects the built-in animation catalog in `src-tauri/src/catalog.rs` and renderer type definitions in Rust and TypeScript.
- Affects built-in rendering code in `src-ui/src/services/renderers.ts` and overlay styles in `src-ui/src/styles.css`.
- Affects the management UI controls for renderer selection and effect parameters in `src-ui/src/views/ManagementView.vue`.
- May add a small animation dependency only if it materially improves the selected effects; prefer extending the existing `canvas-confetti` and local Canvas/CSS first.
- Requires updates to frontend schema validation, Rust catalog validation, unit tests, and integration smoke tests that currently assume seven built-in animations.

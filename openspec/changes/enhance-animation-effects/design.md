# Design

## Context

The current catalog is produced in Rust by `src-tauri/src/catalog.rs`, with renderer kinds mirrored in Rust models and TypeScript types. Built-in visual rendering is centralized in `src-ui/src/services/renderers.ts`, while the management UI exposes renderer selection and a small set of options: particle count, angle, color, duration, audio, and transitions. The existing `completion` renderer launches confetti and also adds the same center ring used by `ring`/`milestone`.

The project already depends on `canvas-confetti` and Howler. Plugin rendering exists, but these new effects are first-party renderers so they must remain safe, local, deterministic enough to validate, and compatible with transparent pointer-through overlay windows.

## Goals / Non-Goals

**Goals:**

- Add useful first-party effects without turning the renderer layer into a plugin marketplace.
- Keep `task-complete` visually focused on the top prompt and side celebration.
- Replace the default catalog's weak `milestone` slot with more distinct effect choices.
- Expose effect-specific parameters through the existing animation definition model and validation pipeline.
- Preserve existing user configurations across upgrade.

**Non-Goals:**

- No remote animation marketplace, remote asset loading, or user-supplied scripts.
- No mandatory migration that deletes a user's persisted `milestone` configuration.
- No adoption of heavyweight animation engines unless local Canvas/CSS and `canvas-confetti` cannot meet an accepted effect.
- No change to overlay focus, mouse passthrough, audio-once, or plugin sandbox boundaries.

## Decisions

### Prefer local renderer extensions over heavy libraries

Implement `material-flow`, `corner-fireworks`, and `focus-spotlight` as first-party renderers using Canvas, CSS, and the existing `canvas-confetti` dependency where appropriate.

Alternatives considered:

- `tsparticles`: strong parameterized particle engine, but adds a broader engine surface than needed for the first pass.
- `fireworks-js`: small and focused, but `canvas-confetti` can already produce corner burst behavior with less dependency churn.
- `lottie-web` or Rive: good for designer-authored effects, but they introduce asset management and are better suited to future plugin/import workflows.
- GSAP, PixiJS, Three.js: powerful but too large for this catalog refresh.

### Add renderer kinds, not one-off hardcoded animations

Extend the first-party renderer enum and TypeScript union with reusable renderer kinds such as `material-flow`, `corner-fireworks`, and `focus-spotlight`. Built-in animations are instances of those renderer kinds with defaults; configured animations can reuse them with different safe options.

This matches the existing catalog model and avoids special-casing every built-in by ID.

### Keep effect options in `AnimationDefinition.options`

Continue using the existing `options` map, but add renderer-aware validation. Common options stay shared where sensible, while renderer-specific options are accepted only for the renderers that use them.

Suggested option ranges:

- `particleCount`: 1 to 500
- `angle`: 0 to 180
- `intensity`: 0 to 1
- `speed`: 0.1 to 3
- `density`: 1 to 300
- `brightness`: 0 to 1
- `spread`: 1 to 180
- `burstCount`: 1 to 12
- `pulseStrength`: 0 to 1
- `dimAmount`: 0 to 0.75
- `spotlightSize`: 0.1 to 1

Structured choices such as launch corners should be represented with known string values or arrays of known string values, never arbitrary code or markup.

### Remove completion's ring through renderer composition

Change the completion renderer path so it creates the top badge and confetti bursts but does not append `.fx-ring`. The ring remains available only for legacy or configured animations that explicitly use the `ring` renderer.

### Remove milestone from fresh defaults without destructive migration

Update the default built-in catalog to omit `milestone`. Existing persisted configurations remain loaded as stored because MotionCue already stores the user's full animation list. Reset-to-default behavior should restore the new default catalog shape, while ordinary app startup should not delete persisted entries.

If validation requires a known renderer for an existing `milestone` entry, keep the `ring` renderer type supported unless a separate migration is designed. The behavior change is removing the default built-in, not removing the renderer immediately.

### Keep UI controls effect-aware

The current editor always shows particle count and angle. Replace or extend that with renderer-aware controls so users see relevant parameters for the selected renderer. Keep unsupported fields hidden rather than saving ignored values.

## Risks / Trade-offs

- [Canvas material effects could cost more GPU/CPU than simple CSS] -> Tie density and frame workload to target frame rate and cap option ranges.
- [Adding renderer-aware options increases validation complexity] -> Keep a single allowlist per renderer and mirror it in Rust and TypeScript tests.
- [Removing `milestone` changes smoke tests and user expectations] -> Update tests to assert revised fresh defaults and add an upgrade preservation case.
- [Local Canvas effects can look weak without visual tuning] -> Include preview-driven acceptance and keep options broad enough to tune without code changes.
- [Preserving legacy `milestone` means old and new catalogs can differ] -> Treat persisted user config as user-owned and document that fresh defaults differ from upgraded configs.

## Migration Plan

1. Add new renderer kinds and renderer-aware option validation while keeping existing renderer kinds supported.
2. Update fresh default catalog to remove `milestone` and add `material-flow`, `corner-fireworks`, and `focus-spotlight`.
3. Implement visual renderers and update management UI controls.
4. Update tests that count built-ins, add tests for new defaults and legacy persisted milestone preservation.
5. Verify build, unit tests, and smoke tests against the revised catalog.

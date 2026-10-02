# Tasks

## 1. Catalog and Schema

- [x] 1.1 Add first-party renderer kinds for `material-flow`, `corner-fireworks`, and `focus-spotlight` in Rust and TypeScript, and verify type/schema tests compile without unknown renderer errors.
- [x] 1.2 Update the fresh default catalog to remove `milestone` and add the three new built-ins with commands, names, descriptions, colors, durations, and safe default options, and verify catalog tests assert the revised default IDs.
- [x] 1.3 Preserve support for existing persisted `milestone` entries and the `ring` renderer, and verify a config containing `milestone` still loads without command reassignment.
- [x] 1.4 Add renderer-aware option validation for common and effect-specific parameters, and verify Rust and frontend tests reject out-of-range values and executable option keys.

## 2. Visual Renderers

- [x] 2.1 Remove the center ring from `task-complete` while preserving the top prompt and side bursts, and verify a focused renderer test or DOM-level test confirms no ring element is appended for completion playback.
- [x] 2.2 Implement `material-flow` as a local first-party full-screen effect using safe Canvas/CSS rendering, and verify preview/playback cleans up timers, canvases, and animation frames on stop.
- [x] 2.3 Implement `corner-fireworks` using existing confetti infrastructure where practical, and verify configured launch corners affect burst origins as expected.
- [x] 2.4 Implement `focus-spotlight` with configurable dimming, spotlight size, pulse strength, color, and text visibility, and verify text can be disabled without leaving an empty badge.
- [x] 2.5 Ensure all new renderers respect configured duration, transition enter/exit timing, target frame rate scaling, transparent background, and pointer-through overlay constraints, and verify via existing playback tests plus a manual preview check.

## 3. Management UI

- [x] 3.1 Update renderer selection to include the new renderer kinds and omit `milestone` as a default built-in choice while keeping legacy renderer values editable when loaded, and verify the animation editor opens for both new and legacy entries.
- [x] 3.2 Replace always-visible particle controls with renderer-aware parameter controls for confetti, completion, material flow, corner fireworks, and focus spotlight, and verify changing each control updates `draft.options` with the expected key/value shape.
- [x] 3.3 Add validation messaging for renderer-specific option ranges in the editor, and verify invalid values block saving with field-specific feedback.
- [x] 3.4 Keep preview behavior consistent for new renderers, custom text, colors, audio settings, and transitions, and verify each new built-in can be previewed from the catalog UI.

## 4. Tests and Acceptance

- [x] 4.1 Update integration and smoke tests that assume seven built-ins, and verify they assert the revised fresh catalog without `milestone`.
- [x] 4.2 Add regression coverage for upgrade preservation of a persisted `milestone` animation, and verify loading that config does not delete or silently rename it.
- [x] 4.3 Add frontend renderer tests for completion ring removal, unsafe option rejection, and representative new renderer option handling, and verify `npm test` passes.
- [x] 4.4 Run `npm run build` and verify Vue/TypeScript type checking and the Vite build pass with the new renderer kinds.
- [x] 4.5 Run `cargo test --manifest-path src-tauri/Cargo.toml` and verify Rust catalog, validation, and settings tests pass.

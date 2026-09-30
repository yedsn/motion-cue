## 1. Project Foundation

- [x] 1.1 Scaffold a Tauri 2 + Vue 3 + TypeScript desktop project with MotionCue product metadata and verify the development window starts successfully.
- [x] 1.2 Add the Rust and frontend module boundaries for invocation, playback, catalog, audio, plugins, diagnostics, and settings, and verify the project type-checks and compiles.
- [x] 1.3 Configure Windows packaging, tray icon, single-instance support, `motioncue://` protocol registration, and the `motion-cue` CLI entry, then verify a packaged build contains the expected executable and protocol metadata.
- [x] 1.4 Define shared versioned request, animation, command, audio, plugin, and diagnostic schemas, and verify JSON round-trips and unknown-version rejection with unit tests.

## 2. Invocation and Single Instance

- [x] 2.1 Implement canonical command normalization, alias indexing, argument schema validation, and deterministic error types, and verify case-insensitive matching, invalid names, length limits, and range limits.
- [x] 2.2 Implement `motioncue://play/<command>` and query-form parsing, and verify valid links produce one normalized request while malformed or unknown links produce actionable errors without fallback playback.
- [x] 2.3 Implement CLI parsing for `play`, `list --json`, `stop`, and `open`, and verify successful, unknown-command, and invalid-argument exit codes plus valid machine-readable JSON output.
- [x] 2.4 Implement single-instance startup and local request forwarding with bounded message size and timeout, and verify a second process forwards to the first without creating a second tray icon or playback runtime.
- [x] 2.5 Add invocation deduplication and latest-request replacement rules, and verify identical requests within 300 ms merge while different parameters replace the active session.

## 3. Settings and Animation Catalog

- [x] 3.1 Implement atomic versioned JSON settings persistence with a last-known-good backup, and verify restart recovery, malformed-config fallback, and rejection of unsupported future schema versions.
- [x] 3.2 Register the seven built-in animations (`confetti`, `task-complete`, `success`, `milestone`, `focus-start`, `error`, and `silent-confetti`) with metadata and default commands, and verify first-run discovery, preview metadata, and enablement.
- [x] 3.3 Implement command/alias create, update, disable, delete, and conflict validation for built-in and configured animations, and verify the global case-insensitive namespace never silently overwrites an existing mapping.
- [x] 3.4 Implement schema-driven configured renderers for confetti, badge, pulse, ring, shake, and completion compositions, and verify valid settings persist while unsafe values and arbitrary script/URL fields are rejected.
- [x] 3.5 Build the animation catalog UI with preview, enable/disable, command editing, and reset-to-default actions, and verify changes are reflected in the invocation list without restarting the app.
- [x] 3.6 Implement managed audio import, relative resource IDs, per-animation enablement, volume, delay, and preview, and verify supported local audio is copied safely while oversized or unsupported files are rejected.
- [x] 3.7 Implement animation package export/import with configuration and managed resources, and verify package validation, command-conflict prompts, version checks, and no inclusion of device-level settings.

## 4. Overlay and Audio Playback

- [x] 4.1 Implement the playback coordinator with one active session, normal duration, 60-second safety timeout, stop/replace cleanup, and diagnostic outcomes, and verify completion, timeout, explicit stop, and replacement release all timers and resources.
- [x] 4.2 Implement per-monitor transparent overlay window creation and reuse with topmost, taskbar-hidden, non-focusable, pointer-through settings, and verify the originating app retains focus and pointer/keyboard input.
- [x] 4.3 Implement monitor enumeration, physical bounds, DPI-aware sizing, target selection, and topology-change handling, and verify all-monitor and primary-monitor playback across mixed-DPI displays without orphan windows.
- [x] 4.4 Implement the browser-side playback protocol for built-in renderers and verified completion/error reporting, and verify each overlay renders the same session with its own viewport dimensions.
- [x] 4.5 Migrate the work-report completion celebration into the generic `task-complete` renderer, including the multi-burst confetti, top feedback card, custom particle shapes, and local success WAV, and verify it no longer depends on work-report task state.
- [x] 4.6 Implement the host-controlled audio service with global mute, per-animation mute, volume, delay, single playback across monitors, and stop-on-replacement, and verify missing/invalid audio does not fail visual playback.
- [x] 4.7 Add tray and management-window “stop all animations” controls, and verify built-in, configured, and plugin sessions plus delayed sounds stop immediately and all overlays hide.

## 5. Sandboxed Web Plugins

- [x] 5.1 Define and validate the plugin manifest format with plugin ID, version, entry, resource inventory, command defaults, sound IDs, and declared WebGL/Worker capabilities, and verify malformed or unsupported manifests are rejected.
- [x] 5.2 Implement secure plugin package extraction and atomic installation, and verify absolute paths, traversal, symlinks, oversized resources, unknown file types, and partial-install cleanup are blocked.
- [x] 5.3 Implement plugin registry state with disabled-by-default install, enable/disable, upgrade, rollback, and uninstall, and verify command mappings and plugin files remain consistent across failed upgrades and active-session removal.
- [x] 5.4 Implement the isolated plugin playback context with strict sandboxing, no Tauri/Node bridge, restrictive CSP, no network, no external navigation, and package-resource-only loading, and verify adversarial plugins cannot access host or system resources.
- [x] 5.5 Implement the versioned `postMessage` SDK for `ready`, `play`, `stop`, `complete`, `error`, and declared `sound` events with session IDs, payload limits, and rate limits, and verify unknown or oversized messages are ignored or terminate abusive sessions.
- [x] 5.6 Add plugin readiness, runtime, message-frequency, and playback-duration watchdogs, and verify hung, crashing, or abusive plugins are destroyed without affecting the tray, management UI, or later animations.
- [x] 5.7 Build plugin management UI showing source, version, declared capabilities, size, and recent errors, and verify explicit enablement, preview, disablement, upgrade, and uninstall flows.

## 6. Management Experience and Diagnostics

- [x] 6.1 Build the tray menu for open, stop all, global mute, and exit, and verify hiding the management window keeps the app resident while exit stops sessions and releases resources.
- [x] 6.2 Add diagnostics for parse failures, forwarding failures, monitor/window failures, audio failures, plugin violations, and recovery events, and verify logs avoid storing full sensitive invocation payloads by default.
- [x] 6.3 Add global settings for mute, default target monitor, animation defaults, and diagnostics retention, and verify settings apply to new sessions without changing persisted animation definitions.

## 7. Verification and Release

- [x] 7.1 Add Rust tests for URL/CLI parsing, command indexing, parameter validation, deduplication, atomic settings, and plugin path security, and verify the focused test suite passes.
- [x] 7.2 Add frontend tests for renderer schemas, audio scheduling, overlay message validation, and plugin SDK lifecycle, and verify the focused test suite passes.
- [x] 7.3 Add integration tests for cold-start protocol invocation, second-process forwarding, list JSON, stop, configuration recovery, package import rollback, and plugin teardown, and verify the integration suite passes.
- [x] 7.4 Perform Windows desktop acceptance on mixed-DPI multi-monitor hardware or an equivalent test setup, covering focus, pointer-through behavior, hot-plug, audio-once behavior, timeout cleanup, and emergency stop.
- [x] 7.5 Perform adversarial plugin acceptance covering network, file, host API, popup, navigation, storage, and resource-abuse attempts, and verify every prohibited capability is blocked without destabilizing the host.
- [x] 7.6 Run type-check, unit tests, integration tests, packaging, and OpenSpec strict validation, then record the final artifact paths and known platform limitations.






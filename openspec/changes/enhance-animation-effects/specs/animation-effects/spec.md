# Spec Delta

## Purpose

Defines the first-party animation effects MotionCue provides, how users can configure their visual parameters safely, and how catalog changes preserve existing user configurations during upgrades.

## ADDED Requirements

### Requirement: Built-in effect catalog is useful and non-overlapping
The system SHALL provide a default built-in effect catalog focused on distinct workflow feedback use cases: general celebration, task completion, success, focus start, error, silent celebration, full-screen material flow, corner fireworks, and focus spotlight. The default built-in catalog MUST NOT include the legacy `milestone` effect for new installations.

#### Scenario: First launch shows revised built-ins
- **WHEN** a user launches MotionCue with no existing configuration
- **THEN** the animation catalog includes `confetti`, `task-complete`, `success`, `focus-start`, `error`, `silent-confetti`, `material-flow`, `corner-fireworks`, and `focus-spotlight`

#### Scenario: Legacy milestone is not added to new defaults
- **WHEN** MotionCue creates a fresh default catalog
- **THEN** the catalog does not include a callable `milestone` built-in animation

### Requirement: Task completion keeps feedback without center ring
The `task-complete` animation SHALL show the top completion prompt and the bottom-left and bottom-right celebratory bursts, and MUST NOT show a center ring or large center badge element during playback.

#### Scenario: Preview task completion
- **WHEN** the user previews `task-complete`
- **THEN** the overlay shows the top completion prompt and side bursts without rendering a center ring

#### Scenario: Play task completion from command
- **WHEN** an external caller triggers `task-complete` with custom text
- **THEN** the custom text appears in the top prompt and the center of the screen remains free of the removed ring effect

### Requirement: New full-screen material effect is configurable
The system SHALL provide a `material-flow` first-party effect for full-screen ambient material feedback. Users SHALL be able to configure its colors, intensity, speed, density, brightness, and duration within validated safety ranges.

#### Scenario: Configure material intensity
- **WHEN** the user sets `material-flow` to low intensity and low density
- **THEN** playback uses a subtler full-screen material effect while keeping the overlay transparent and pointer-through

#### Scenario: Reject unsafe material parameters
- **WHEN** the user saves material parameters outside supported ranges
- **THEN** the system rejects the save and identifies the invalid parameter without changing the last valid animation configuration

### Requirement: New corner fireworks effect is configurable
The system SHALL provide a `corner-fireworks` first-party effect that launches celebratory bursts from one or more screen corners. Users SHALL be able to configure burst count, particle count, launch corners, spread, speed, colors, and duration within validated safety ranges.

#### Scenario: Play corner fireworks
- **WHEN** the user previews `corner-fireworks`
- **THEN** the overlay plays celebratory bursts from configured screen corners without blocking the center prompt area

#### Scenario: Configure launch corners
- **WHEN** the user configures `corner-fireworks` to use only bottom corners
- **THEN** playback launches bursts only from the bottom-left and bottom-right corner areas

### Requirement: New focus spotlight effect is configurable
The system SHALL provide a `focus-spotlight` first-party effect for entering a focused work state. Users SHALL be able to configure spotlight size, dim amount, pulse strength, color, duration, and whether text is shown within validated safety ranges.

#### Scenario: Play focus spotlight
- **WHEN** the user triggers a focus-start style command
- **THEN** the overlay shows a restrained focus spotlight effect that fades away without celebration particles

#### Scenario: Configure text visibility
- **WHEN** the user disables text for `focus-spotlight`
- **THEN** playback shows the visual focus effect without a text badge

### Requirement: Effect parameters are validated consistently
The system SHALL validate all first-party effect options before saving or playing configured animations. Unsupported option keys that would execute code, load remote content, inject markup, or exceed renderer safety limits MUST be rejected.

#### Scenario: Save supported custom effect parameters
- **WHEN** the user creates a configured animation using a supported first-party renderer with parameters inside allowed ranges
- **THEN** the system saves the animation and uses those parameters for both preview and external playback

#### Scenario: Reject unsafe option keys
- **WHEN** an imported animation package contains script, URL, HTML, or otherwise executable visual options
- **THEN** the system rejects the package or animation configuration before it can be previewed or invoked

### Requirement: Existing user configurations survive catalog changes
The system SHALL preserve existing user-defined and previously persisted built-in animation configurations during upgrade. Removing `milestone` from fresh defaults MUST NOT delete a user's existing persisted `milestone` entry or silently reassign its command.

#### Scenario: Upgrade user with existing milestone
- **WHEN** a user upgrades from a version whose persisted configuration includes `milestone`
- **THEN** the system preserves that existing entry and its command unless the user deletes or disables it

#### Scenario: Fresh install gets revised catalog
- **WHEN** a new user installs MotionCue after this change
- **THEN** the user receives the revised default catalog without `milestone`

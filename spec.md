# Cloud STT Phase A Spec (MAI-Transcribe-1 + Local Fallback)

## Status

- Draft: Ready for implementation
- Target: Phase A (MVP)
- Platforms: macOS + Windows (Linux should continue to work with local-only path)

## Summary

Add cloud transcription with `MAI-Transcribe-1` while preserving Handy's current local `transcribe-rs` flow as a first-class fallback.

This Phase A intentionally avoids large abstractions and new platform dependencies:

- No OS keychain dependency in Phase A
- No multi-provider module tree in Phase A
- No benchmark tooling in Phase A

## Goals

1. Support cloud transcription via `MAI-Transcribe-1`.
2. Keep local transcription behavior stable and available.
3. Add clear routing modes:
   - `cloud_first_local_fallback`
   - `local_only`
   - `cloud_only`
4. Make failures deterministic via explicit timeout policy.
5. Keep secrets and transcript text out of logs.
6. Keep implementation aligned with existing Handy architecture and command patterns.

## Non-Goals (Phase A)

1. Multiple cloud STT providers.
2. Dynamic provider registry in settings.
3. Model maps per provider.
4. OS keychain integration.
5. Benchmark scripts/checklists.
6. Confidence-threshold routing or profile presets.

## Key Design Decisions

### 1) Integration Point

Cloud/local routing will be implemented inside:

- [transcription.rs](/Users/anderson/Documents/Projects/Handy/src-tauri/src/managers/transcription.rs)

Specifically in `TranscriptionManager::transcribe()`, before the local engine call boundary.  
`actions.rs` remains orchestration/UI flow and should not become STT routing logic.

### 2) Helper Module Scope

Add one focused helper:

- [cloud_transcription.rs](/Users/anderson/Documents/Projects/Handy/src-tauri/src/managers/cloud_transcription.rs)

No new `stt/` module hierarchy in Phase A.

### 3) API Key Storage

Use existing `SecretMap` pattern in settings for MAI key storage (same redaction behavior used by post-processing).

### 4) Retry Policy

To preserve responsiveness:

1. `cloud_first_local_fallback`: **no retries**. One cloud attempt, then immediate local fallback on retryable cloud failure.
2. `cloud_only`: **one retry** for `429/5xx` with short backoff, then fail.

### 5) Provider/Model Settings Simplification

Phase A uses one fixed cloud provider and one default model:

- No `stt_providers` settings vector.
- No `stt_models` map.
- Use a single `stt_cloud_model` string (default `MAI-Transcribe-1`).
- Keep MAI base URL as a Rust constant in backend code.

### 6) Audio Upload Strategy

Primary MVP approach: encode upload payload from in-memory `Vec<f32>` to WAV bytes inside cloud helper.

Rationale:

1. Current manager API already receives `Vec<f32>`.
2. Avoid coupling transcription routing to history file persistence timing.
3. Keep changes localized in manager + helper.

Future optimization (post-MVP): optionally upload already-written WAV file if pipeline is refactored to pass stable file path without adding latency.

## Settings Schema (Phase A)

All changes in:

- [settings.rs](/Users/anderson/Documents/Projects/Handy/src-tauri/src/settings.rs)

### New Enums

1. `SttProviderKind`
   - `local`
   - `mai`
2. `SttFallbackStrategy`
   - `cloud_first_local_fallback`
   - `local_only`
   - `cloud_only`

### New `AppSettings` Fields

1. `stt_provider: SttProviderKind` (default `local`)
2. `stt_fallback_strategy: SttFallbackStrategy` (default `cloud_first_local_fallback`)
3. `stt_cloud_model: String` (default `"MAI-Transcribe-1"`)
4. `stt_api_keys: SecretMap` (must include `"mai"` key)
5. `stt_connect_timeout_ms: u64` (default: MVP value)
6. `stt_request_timeout_ms: u64` (default: MVP value)

### Migration Rules

During settings load:

1. Add missing new fields with defaults.
2. Ensure `stt_api_keys["mai"]` exists.
3. Validate enum-backed values; unknown/invalid values are coerced to safe defaults.
4. Persist normalized settings only if changed.
5. Log normalization with non-sensitive warnings only.

## Backend Commands

Implement in:

- [shortcut/mod.rs](/Users/anderson/Documents/Projects/Handy/src-tauri/src/shortcut/mod.rs)

Register in:

- [lib.rs](/Users/anderson/Documents/Projects/Handy/src-tauri/src/lib.rs)

Commands:

1. `change_stt_provider_setting`
2. `change_stt_fallback_strategy_setting`
3. `change_stt_cloud_model_setting`
4. `change_stt_api_key_setting`
5. `change_stt_connect_timeout_setting`
6. `change_stt_request_timeout_setting`

Notes:

1. Follow existing `change_*_setting` style already used in this file.
2. Keep validation strict and return explicit errors for invalid values.

## Cloud Routing Behavior

Inside `TranscriptionManager::transcribe()`:

1. Read STT settings.
2. If `local_only`, run existing local path unchanged.
3. If provider is `mai`:
   - Attempt MAI request with configured timeouts.
   - Apply retry policy by fallback mode.
4. On cloud error:
   - `cloud_first_local_fallback`: run local path immediately.
   - `cloud_only`: return clear blocking error.
5. Preserve existing post-processing flow behavior after transcription text is produced.

## Failure Classification

Treat as retryable cloud failures:

1. Connection timeout
2. Request timeout
3. Network transport errors
4. HTTP `429`
5. HTTP `5xx`

Non-retryable failures (example):

1. Invalid API key/auth failure (surface explicit user-facing error)
2. Malformed request configuration

## Logging + Redaction Requirements

Phase A requirements:

1. Never log API keys.
2. Never log auth headers.
3. Never log transcript text (cloud or local).
4. Log only metadata:
   - provider used
   - latency ms
   - fallback used
   - error class (sanitized)

This may require tightening existing transcription logs in:

- [transcription.rs](/Users/anderson/Documents/Projects/Handy/src-tauri/src/managers/transcription.rs)

## Frontend Scope (PR3)

Files:

- [settingsStore.ts](/Users/anderson/Documents/Projects/Handy/src/stores/settingsStore.ts)
- [useSettings.ts](/Users/anderson/Documents/Projects/Handy/src/hooks/useSettings.ts)
- [AdvancedSettings.tsx](/Users/anderson/Documents/Projects/Handy/src/components/settings/advanced/AdvancedSettings.tsx)
- [translation.json](/Users/anderson/Documents/Projects/Handy/src/i18n/locales/en/translation.json)
- New component: `src/components/settings/stt/SttSettings.tsx`

UI must provide:

1. Provider select (`local` vs `mai`)
2. Cloud model field (`MAI-Transcribe-1`, editable string)
3. MAI API key field
4. Fallback strategy selector
5. Timeout fields (if exposed in UI for Phase A)

Behavior requirements:

1. Feature shown only when `experimental_enabled` is true.
2. `cloud_only` + missing/invalid key must show a clear blocking error.
3. No silent fallback in `cloud_only`.

## PR Plan

### PR1: Settings + Migration + Commands

Scope:

1. Add schema fields and enums.
2. Add migration/default normalization.
3. Add and register STT settings commands.
4. Add i18n keys and update TS bindings.

Acceptance:

1. Existing settings load without crash.
2. Invalid legacy values normalized safely.
3. Commands persist and reload correctly.
4. `bun run check:translations` passes.

### PR2: Cloud Helper + Routing in Manager

Scope:

1. Add `cloud_transcription.rs`.
2. Implement WAV bytes encoding for MAI upload.
3. Route in `TranscriptionManager::transcribe()`.
4. Implement fallback-mode-specific retry policy.
5. Enforce redaction-safe logging.

Acceptance:

1. `local_only` unchanged.
2. `cloud_first_local_fallback` falls back immediately on retryable cloud errors.
3. `cloud_only` blocks with clear error on cloud failure.
4. Logs contain no key/header/transcript text.

### PR3: Frontend UI + Wiring

Scope:

1. Add STT settings UI in experimental section.
2. Add store/hook actions for backend commands.
3. Add validation and user-facing error handling.

Acceptance:

1. End-to-end config from UI works.
2. Settings persist and hydrate correctly.
3. `cloud_only` invalid key path is clearly surfaced.

## Windows QA Matrix

### Functional

1. Verify hotkeys start/stop transcription.
2. Verify fallback behaviors per mode (`local_only`, `cloud_first_local_fallback`, `cloud_only`).
3. Verify offline behavior and retryable HTTP failure paths.

### Paste Validation

Test transcription paste into:

1. Notepad
2. Word
3. Browser textarea/contenteditable
4. Teams/Slack chat input
5. Terminal editor contexts

### Safety

1. Confirm logs on Windows do not include keys, auth headers, or transcript text.
2. Confirm microphone/no-device errors still surface via existing user-friendly toasts.

## Deferred to Phase B

1. Add second cloud STT provider.
2. Promote dynamic provider metadata list in settings.
3. Expand model handling beyond single cloud model field.
4. Evaluate keychain integration.
5. Optional optimization: upload existing recorded WAV file path where beneficial.

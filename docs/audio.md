# type-globe Audio Pipeline

## Decision

Listening RPG uses **runtime speech synthesis** as the primary audio path.

- **Chosen baseline**: OS-native TTS through the `tts` crate
- **Not the baseline**: pre-generated voice clips checked into the repo
- **Why**: prompt banks grow, boss hints are layered and dynamic, and pre-rendering every variant would explode maintenance cost

This keeps ordinary prompts and replay on one live pipeline, and prepares future boss hints on that same surface.

## Audio Kinds

The runtime path distinguishes a small set of utterance intents:

- `PromptAnswer`: the normal reading of the target answer
- `PromptReplay`: replay of the same answer, slightly slower when rate control exists
- `BossHint { layer }`: structured reverse-Akinator hint reading, slower on early layers and closer to normal speed on later layers
- `BossReveal`: final explicit reveal when a boss encounter chooses to speak the answer directly

These intents are represented in code by `src/audio/tts.rs` as `TtsRequestKind`. The current build actively uses `PromptAnswer` and `PromptReplay`; `BossHint` / `BossReveal` are prepared for the later boss UI work.

## Speech Policy

`TtsEngine` now accepts a `TtsRequest` instead of treating every utterance identically.

- Voice selection remains **best-effort by language primary tag** (`ja`, `en`)
- Interrupt behavior remains `true` for all current intents so replay replaces in-flight audio
- Rate is set **only if the backend supports it**
- If rate control is unavailable, the request still speaks normally instead of failing

Current rate policy:

- `PromptAnswer`: normal speed
- `PromptReplay`: slightly slower
- `BossHint` early layers: slowest
- `BossHint` later layers: closer to normal
- `BossReveal`: near-normal / slightly brisk

## Environment Viability

Listening is classified at runtime in three practical buckets:

1. **Unavailable**
   `TtsEngine::new()` fails. On Linux this is commonly missing or stopped `speech-dispatcher`.
2. **Basic**
   TTS initialises, but some controls such as rate/stop may be unavailable.
   The mode is still usable for ordinary listening prompts.
3. **Preferred**
   TTS initialises and supports both `stop` and `rate`.
   This is the target baseline for the full RPG and boss-hint pacing.

In code this is exposed as `TtsRuntimeSupport`, with `Unavailable` represented by constructor failure rather than an enum variant.

## Platform Policy

- **Linux**: supported through `speech-dispatcher`; this is the main operational dependency
- **macOS**: supported through AVFoundation/AppKit via `tts`
- **Windows**: supported through WinRT/SAPI paths provided by `tts`

We accept that voice identity differs by machine. What must remain stable is:

- the language selection
- whether replay interrupts correctly
- whether a run can speak at all
- whether boss/normal pacing can request slower or normal speech when supported

## Scope Boundaries

What this issue settles:

- runtime synthesis is the default architecture
- boss hints use the same pipeline as normal prompts
- rate/voice controls are best-effort capability-driven, not mandatory
- implementation may proceed on top of `tts` without waiting for a separate studio pipeline

What remains for later issues:

- exact boss-hint script format
- per-language content authoring rules for hint text
- user-facing audio settings persistence
- optional alternate backends such as a dedicated studio voice service

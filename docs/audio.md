# type-globe Audio Pipeline

## Decision

Listening RPG uses **runtime speech synthesis** as the primary audio path.

- **Chosen baseline**: the pluggable `SpeechBackendHandle` boundary
- **Default backend**: OS-native TTS through the `tts` crate
- **Local backend hook**: `local-command` JSONL bridge for `offline-voice-runtime` style daemons
- **Not the baseline**: pre-generated voice clips checked into the repo
- **Why**: prompt banks grow, boss hints are layered and dynamic, and pre-rendering every variant would explode maintenance cost

This keeps ordinary prompts and replay on one live pipeline, prepares future boss hints on that same surface, and makes `type-globe` the first proving ground for the shared local speech foundation later reused by `esuna` and `osaka-kenpo`.

## Audio Kinds

The runtime path distinguishes a small set of utterance intents:

- `PromptAnswer`: the normal reading of the target answer
- `PromptReplay`: replay of the same answer, slightly slower when rate control exists
- `BossHint { layer }`: structured reverse-Akinator hint reading, slower on early layers and closer to normal speed on later layers
- `BossReveal`: final explicit reveal when a boss encounter chooses to speak the answer directly

These intents are represented above the concrete engine by `src/audio/speech.rs` as `SpeechRequestKind`. The current build actively uses all four: `PromptAnswer` / `PromptReplay` in regular encounters, and `BossHint` / `BossReveal` in the stacked-hint miniboss / boss flow.

## Speech Policy

UI and RPG code now submit a `SpeechRequest` instead of depending on a concrete TTS engine.

- Voice selection remains **best-effort by language primary tag** (`ja`, `en`)
- Interrupt behavior remains `true` for all current intents so replay replaces in-flight audio
- Rate is set **only if the backend supports it**
- If rate control is unavailable, the request still speaks normally instead of failing
- `voice_role` exists on the request shape for future fixed-role routing, but current `type-globe` calls pass `None`

Current rate policy:

- `PromptAnswer`: normal speed
- `PromptReplay`: slightly slower
- `BossHint` early layers: slowest
- `BossHint` later layers: closer to normal
- `BossReveal`: near-normal / slightly brisk

## Environment Viability

Listening is classified at runtime in three practical buckets:

1. **Unavailable**
   Backend construction fails. For the default `system` backend on Linux this is commonly missing or stopped `speech-dispatcher`.
2. **Basic**
   Speech initialises, but some controls such as rate/stop may be unavailable.
   The mode is still usable for ordinary listening prompts.
3. **Preferred**
   Speech initialises and supports both `stop` and `rate`.
   This is the target baseline for the full RPG and boss-hint pacing.

In code this is exposed as `SpeechCapabilities`, with `Unavailable` represented by constructor failure rather than an enum variant.

## Local Command Protocol

`type-globe rpg --speech-backend local-command --speech-command '<command>'` starts a long-lived child process and communicates through newline-delimited JSON. Each request is written to the child's stdin and must be acknowledged by one JSON line on stdout:

```json
{"ok":true}
```

Speak requests look like this:

```json
{"type":"speak","text":"apple","lang":"en","kind":"prompt_replay","layer":null,"voice_role":null,"interrupt":true,"rate_multiplier":0.96}
```

The same process also receives `{"type":"stop"}` and `{"type":"shutdown"}`. `--speech-command` may be replaced by the `OFFLINE_VOICE_RUNTIME_COMMAND` environment variable.

## Platform Policy

- **Linux**: the default `system` backend is supported through `speech-dispatcher`; this is the main operational dependency
- **macOS**: supported through AVFoundation/AppKit via `tts`
- **Windows**: supported through WinRT/SAPI paths provided by `tts`
- **Local daemon**: supported anywhere a command can speak the JSONL protocol

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
- UI/RPG logic no longer depends on the concrete `tts` implementation
- `local-command` can be swapped in without changing RPG/UI code

What remains for later issues:

- exact boss-hint script format
- per-language content authoring rules for hint text
- user-facing audio settings persistence
- the actual `offline-voice-runtime` model daemon implementation and quality tuning

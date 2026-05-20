# type-globe — Overview

## What is type-globe?

type-globe is a TUI typing game with one rule:

> **The string you must type is never shown on screen.**

You derive the answer from a quiz prompt or from listening, then type it blind. Reflex-typing the visible text — the staple of every other typing game — is impossible here by design. type-globe rewards **knowledge, memory, and listening comprehension**.

The metaphor is competitive Hyakunin Isshu karuta: the lower verse is never recited, and players strike from memory.

## Two Axes (never crossed)

- **Quiz × Score-attack modes** — knowledge-driven, competitive, public.
- **Listening × Hack-and-Slash RPG** — comprehension-driven, personal, progression-oriented.

Quiz prompts lead to single-run scoring, Time Attack 25, and (in `type-globe-online`) world ranking via Nostralgic Ranking. Listening prompts feed into a roguelike "one run = ten enemies" RPG.

> **Records vs Ranking.** Local self-best history is **Records**. **Ranking** is reserved for world-vs-world ordering through Nostralgic Ranking, which lands in the v0.3.0+ `type-globe-online` build. Don't blur them.

## Game Modes

- **Quiz Mode** — Read the question, see the four choices, and **type the correct one's text** (no arrow-key selection). Exact matches auto-confirm, and non-prefix typos are rejected immediately.
- **Time Attack 25** — A **four-seat** quiz battle. The current build ships a **local prototype**: one human plus three CPU seats, a 25-question run, and a sequential 5x5 board fill. `nostr_arena` online play remains future work, but the seat model and board UI are already live.
- **Listening RPG (TypeQuest)** — Audio-first blind typing. The current build ships a **10-battle prototype run**: regular encounters on 1-4 / 6-9, a timed miniboss on 5, and a manual stacked-hint boss on 10. HP / EXP persistence is still in progress.

## Current Shipping State

- **Available now**: Quiz, Records
- **Practice available now**: Time Attack 25 local prototype, 10-battle Listening RPG prototype
- **In progress**: Listening RPG persistence (HP / EXP / titles / records)
- **Planned**: `type-globe-online`, world Ranking

## Display Animation (`jiwa` crate)

Question text is revealed **one character at a time**, with each character **fading in** through TrueColor interpolation. Players who already know the answer may type during the reveal — the input layer accepts keystrokes concurrently. The animation has **no skip key** (fairness).

For listening mode, no text is shown; a `♪` note pulses with the same animation while audio plays.

## Audio

Audio is generated at runtime via the `tts` crate (a cross-platform wrapper over speech-dispatcher / AVSpeechSynthesizer / SAPI — the same model as the browser's `SpeechSynthesisUtterance`). No audio files ship with the binary. Replay is unlimited and unpenalized; the only cost is the time it consumes.

The baseline direction is **real-time speech synthesis**, not pre-baked clips. Future voice backends may be swapped in, but the game design assumes prompts and layered boss hints can be spoken on demand.

Ordinary prompts and replay already use the same runtime pipeline, and future boss hints are prepared on that same API surface. Where the backend exposes rate control, type-globe can slow replay and future early hint layers; otherwise it degrades to normal-speed speech and keeps the run playable.

## Design Philosophy

- **Reflex-free** — knowledge and memory beat fast eyes.
- **Offline-first** — the v0.2.0 release runs without an internet connection.
- **Short sessions** — one run is ten prompts. Pick up, finish, put down.
- **Multilingual** — Japanese / English at startup; more languages welcome (TTS does the rest).
- **Keyboard-only** — no mouse, ever.

## Tech Stack

| Component | Technology |
|---|---|
| Language | Rust |
| TUI | `ratatui` + `crossterm` |
| Audio | `tts` crate (cross-platform OS TTS) |
| Animation | [`jiwa`](https://crates.io/crates/jiwa) crate (typewriter + RGB fade), extracted from this repo's former `jiwa_core` module |
| Storage | local JSON |

## Quiz Content

Quiz questions span three buckets:

- **IT / Programming** (programming, web, IT terminology)
- **Anime / Games / Manga** (subculture, VTuber/internet)
- **General Knowledge** (geography, history, science, math, language, culture)

100 questions ship today; v0.2.0 will scale to 1,000.

## What's Next

Deferred to v0.3.0 and beyond:

- **Image Quiz** (terminal graphics protocol; kitty / iTerm2 / wezterm)
- **Stealth Mode** (CLI-disguise UI)
- **type-globe-online** — mypace WebSocket integration plus **Nostralgic Ranking** submission (world ranking via Nostr) and Nostr feed posting. Same repository, different brand label.

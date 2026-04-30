# type-globe — Overview

## What is type-globe?

type-globe is a TUI typing game with one rule:

> **The string you must type is never shown on screen.**

You derive the answer from a quiz prompt or from listening, then type it blind. Reflex-typing the visible text — the staple of every other typing game — is impossible here by design. type-globe rewards **knowledge, memory, and listening comprehension**.

The metaphor is competitive Hyakunin Isshu karuta: the lower verse is never recited, and players strike from memory.

## Two Axes (never crossed)

- **Quiz × Ranking-style modes** — knowledge-driven, competitive, public.
- **Listening × Hack-and-Slash RPG** — comprehension-driven, personal, progression-oriented.

Quiz prompts lead to single-run scoring, Time Attack 25, and global rankings. Listening prompts feed into a roguelike "one run = ten enemies" RPG.

## Game Modes

- **Quiz Mode** — Read the question, see the four choices, and **type the correct one's text** (no arrow-key selection). Press Enter to confirm.
- **Time Attack 25** — 5×5 panel battle versus CPU, inspired by the *Attack 25* TV show. Total time (thinking + typing) decides the ranking.
- **Listening RPG (TypeQuest)** — Audio-only prompts. Each prompt is an enemy; ten enemies = one run. Mistypes cost EXP, never end the run.

## Display Animation (`jiwa_core`)

Question text is revealed **one character at a time**, with each character **fading in** through TrueColor interpolation. Players who already know the answer may type during the reveal — the input layer accepts keystrokes concurrently. The animation has **no skip key** (fairness).

For listening mode, no text is shown; a `♪` note pulses with the same animation while audio plays.

## Audio

Audio is generated at runtime via the `tts` crate (a cross-platform wrapper over speech-dispatcher / AVSpeechSynthesizer / SAPI — the same model as the browser's `SpeechSynthesisUtterance`). No audio files ship with the binary. Replay is unlimited and unpenalized; the only cost is the time it consumes.

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
| Animation | in-tree `jiwa_core` (typewriter + RGB fade) |
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
- **type-globe-online** — mypace WebSocket integration for live rankings and Nostr feed posting. Same repository, different brand label.

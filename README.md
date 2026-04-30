# type-globe

[日本語](README.ja.md)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

> A typing game where the string you must type is **never shown on screen**.

Inspired by competitive Hyakunin Isshu karuta — where the lower verse is never recited and players strike from memory — type-globe rewards **knowledge, memory, and listening comprehension** instead of visual reflex.

## How it works

Two presentation styles, paired with two game structures. **They are never crossed.**

| Presentation | Paired with | What it rewards |
|---|---|---|
| **Quiz** — read the question and four choices, then type the correct one's text | Single-run, Time Attack 25, **ranking** | knowledge |
| **Listening** — hear the prompt, type what you heard | **Hack-and-slash RPG** (10 prompts = 1 run) | comprehension |

In Quiz mode, there are **no arrow keys for selection** — you select by typing the correct choice's text directly. Press Enter to confirm.

In Listening mode, **no text appears on screen at all**. A `♪` note pulses while audio plays. You type what you heard, blind.

## Why type-globe?

- **Reflex-free typing** — every other typing game shows you the text. This one doesn't.
- **Short sessions** — one RPG run is ten prompts. Pick up, finish, put down.
- **Offline-first** — single binary, no network required.
- **Multilingual** — Japanese and English at startup; TTS handles the rest.
- **Animated reveal (jiwa)** — quiz text fades in one character at a time. If you already know the answer, you can start typing before the reveal finishes.

## Quick Start

```sh
cargo install type-globe
type-globe
```

## Game Modes

| Mode | Status | Description |
|---|---|---|
| Quiz (single-run) | v0.2.0 | Type-to-select 4-choice quiz, ten questions per run |
| Time Attack 25 | v0.2.0 | 5×5 panel battle vs. CPU, total time decides ranking |
| Listening RPG (TypeQuest) | v0.2.0 | Audio-only prompts, ten enemies per dungeon run, EXP / level / titles |
| Ranking | v0.2.0 | Top 10 per mode per language, persisted locally |
| Image Quiz | v0.3.0+ | Requires terminal graphics protocol (kitty / iTerm2 / wezterm) |
| Stealth | v0.3.0+ | Disguises the UI as a generic CLI tool |
| `type-globe-online` | v0.3.0+ | mypace WebSocket integration for live rankings, posted to Nostr |

## Display Rules

**Forbidden** — would compromise the core principle:
- Showing the answer string before you type it
- Skipping the reveal animation

**Allowed**:
- Question text and choice labels (Quiz only — Listening shows none)
- Characters **you have actually typed** (echoed back so you can fix typos)
- Status data (HP, level, EXP, time, CPM, WPM)

## Scoring

Both **CPM** (characters per minute) and **WPM** (words per minute) are displayed side by side.

## Audio

Listening prompts are synthesized at runtime via the [`tts`](https://crates.io/crates/tts) crate, which wraps the OS-native TTS engine (speech-dispatcher on Linux, AVSpeechSynthesizer on macOS, SAPI on Windows). No audio files ship with the binary. Replay is **unlimited and unpenalized** — the only cost is the time it consumes.

## Key Bindings

**Quiz**

| Key | Action |
|---|---|
| Letters | Type the correct choice's text |
| `Enter` | Confirm |
| `Tab` | Skip question |
| `F5` | Restart run |
| `Esc` | Quit |

**Listening RPG**

| Key | Action |
|---|---|
| Letters | Type what you heard |
| `Enter` | Confirm |
| `Space` | Replay sound (unlimited) |
| `F5` | New run |
| `Esc` | Return to town |

## Install

### From crates.io

```sh
cargo install type-globe
```

### From source

```sh
git clone https://github.com/kako-jun/type-globe.git
cd type-globe
cargo build --release
./target/release/type-globe
```

## Requirements

- A terminal with Unicode and ANSI escape support
- An OS TTS backend (most modern macOS / Windows / Linux distros work out of the box)
- Rust 1.70+ to build from source

## License

MIT

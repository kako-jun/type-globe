# type-globe

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

> A typing game where the string you must type is **never shown on screen**.

This repository is currently in the middle of the v0.2.0 redesign. The sections below describe the **target design**, while the current `main` branch still contains some legacy quiz interaction that will be replaced in follow-up issues.

Inspired by competitive Hyakunin Isshu karuta — where the lower verse is never recited and players strike from memory — type-globe rewards **knowledge, memory, and listening comprehension** instead of visual reflex.

## How it works

Two presentation styles, paired with two game structures. **They are never crossed.**

| Presentation | Paired with | What it rewards |
|---|---|---|
| **Quiz** — read the question and four choices, then type the correct one's text | Single-run, Time Attack 25, **ranking** | knowledge |
| **Listening** — hear the prompt, type what you heard | **Hack-and-slash RPG** (10 prompts = 1 run) | comprehension |

Target v0.2.0 behavior: in Quiz mode, there are **no arrow keys for selection** — you select by typing the correct choice's text directly. Press Enter to confirm.

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
| Quiz (single-run) | target: v0.2.0 | The standard play mode: type-to-select 4-choice quiz, ten questions per run |
| Time Attack 25 | target: v0.2.0 | A Quiz variant with a 5×5 panel battle vs. CPU |
| Listening RPG (TypeQuest) | target: v0.2.0 | A separate ruleset: audio-only prompts, ten enemies per dungeon run |
| Records / Ranking | target: v0.2.0 | A records view that spans Quiz, Time Attack 25, and Listening RPG |
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

Quiz answers are typed directly — there is no arrow / number-key fallback. The exact choice text must be entered, then `Enter` confirms. Prefix matches do not auto-confirm (e.g. `mov` does not pick `move`).

| Key | Action |
|---|---|
| Letters | Append to the typed answer |
| `Backspace` | Erase the last character |
| `Enter` | Confirm — exact match against one of the four choices |
| `Tab` | Skip the current question |
| `Esc` / `Ctrl+C` | Quit |

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

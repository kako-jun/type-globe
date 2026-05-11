# type-globe

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

> A typing game where the string you must type is **never shown on screen**.

This repository now ships the v0.2.0 blind-typing redesign. Some roadmap items (notably Time Attack 25 and the full ten-prompt RPG loop) remain follow-up work, but the core "the answer is never shown" interaction is already the live behavior.

Inspired by competitive Hyakunin Isshu karuta — where the lower verse is never recited and players strike from memory — type-globe rewards **knowledge, memory, and listening comprehension** instead of visual reflex.

## How it works

Two presentation styles, paired with two game structures. **They are never crossed.**

| Presentation | Paired with | What it rewards |
|---|---|---|
| **Quiz** — read the question and four choices, then type the correct one's text | Single-run, Time Attack 25, **Records** | knowledge |
| **Listening** — hear the prompt, type what you heard | **Listening RPG** (10 prompts = 1 run) | comprehension |

In Quiz mode, there are **no arrow keys for selection** — you select by typing the correct choice's text directly. An exact match auto-confirms immediately.

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
type-globe          # main menu
```

You can also jump directly into a mode:

```sh
type-globe quiz             # Quiz mode, language selected at startup
type-globe rpg              # Listening RPG, language selected at startup
type-globe ta25             # Time Attack 25 (coming in v0.2.0)
type-globe ranking          # View local Records

type-globe quiz --lang ja   # Jump straight to Japanese Quiz
type-globe rpg  --lang en --no-tts  # Listening RPG without TTS (silent mode)
```

Run `type-globe --help` or `type-globe <subcommand> --help` for all options.

## Game Modes

| Mode | Status | Description |
|---|---|---|
| Quiz (single-run) | target: v0.2.0 | The standard play mode: type-to-select 4-choice quiz, ten questions per run |
| Time Attack 25 | target: v0.2.0 | A Quiz variant with a 5×5 panel battle vs. CPU |
| Listening RPG (TypeQuest) | target: v0.2.0 | A separate ruleset: audio-only prompts, ten enemies per dungeon run |
| Records | target: v0.2.0 | Local self-best history across Quiz, Time Attack 25, and Listening RPG |
| Image Quiz | v0.3.0+ | Requires terminal graphics protocol (kitty / iTerm2 / wezterm) |
| Stealth | v0.3.0+ | Disguises the UI as a generic CLI tool |
| `type-globe-online` | v0.3.0+ | mypace WebSocket + **Nostralgic Ranking** (world ranking via Nostr) + Nostr feed |

> **Records vs Ranking.** Local self-best history is **Records**. **Ranking** means world ordering through Nostralgic Ranking, only available in `type-globe-online` (v0.3.0+).

## Display Rules

**Forbidden** — would compromise the core principle:
- Showing the answer string before you type it
- Skipping the reveal animation

**Allowed**:
- Question text and choice labels (Quiz only — Listening shows none)
- Characters **accepted as a valid answer prefix** (echoed back with typo feedback)
- Status data (HP, level, EXP, time, CPM, WPM)

## Scoring

Both **CPM** (characters per minute) and **WPM** (words per minute) are displayed side by side.

## Audio

Listening prompts are synthesized at runtime via the [`tts`](https://crates.io/crates/tts) crate, which wraps the OS-native TTS engine (speech-dispatcher on Linux, AVSpeechSynthesizer on macOS, SAPI on Windows). No audio files ship with the binary. Replay is **unlimited and unpenalized** — the only cost is the time it consumes.

On Linux, the `speech-dispatcher` daemon must be installed and running. If it is not, type-globe shows a clear "Listening mode is unavailable on this system" message and returns to the menu rather than crashing the binary; Quiz / Records / Time Attack 25 still work without TTS.

## Key Bindings

**Quiz** (`type-globe quiz`)

Quiz answers are typed directly — there is no arrow / number-key fallback. An exact match auto-confirms immediately, and non-prefix typos are rejected on the spot instead of being inserted into the buffer. Matching is case-insensitive (`H2O` / `h2o`, `TOKYO` / `tokyo`). In JA mode, a single answer may intentionally accept multiple romanized spellings (`tokyo` / `toukyou`, `osaka` / `oosaka`, etc.), and bundled data can declare them explicitly via `ja_typings`. When a choice has a well-established official Latin spelling (for example a proper name), that spelling may also be accepted.

| Key | Action |
|---|---|
| Letters | Append only if they keep the input on a valid answer prefix |
| `Backspace` | Erase the last character |
| `Tab` | Skip the current question |
| `Esc` / `Ctrl+C` | Quit |

**Listening RPG** (`type-globe rpg`)

| Key | Action |
|---|---|
| Letters | Append only if they keep the input on a valid answer prefix |
| `Space` | Replay sound (unlimited, no penalty) |
| `Esc` | Return to menu |

The v0.2.0 build ships the **listening foundation**: TTS, the prompt data structure, and a single-prompt practice flow (word-kind prompts only, since `Space` is reserved for replay). The full ten-prompt RPG run with HP / EXP / boss placement is the next epic (#32–#37).

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
- An OS TTS backend for Listening mode (macOS / Windows work out of the box; on Linux, install and start `speech-dispatcher`)
- Rust 1.78+ to build from source (Linux build also needs `libspeechd-dev`)

## Maintenance scripts

The bundled question banks (`data/questions_<lang>.json`) ship with both a
display form (`question_text.<lang>`) and a phonetic reading
(`question_text_reading.<lang>`). When the JA display form drifts toward
hiragana-heavy phrasing, refresh it with the migration scripts under
`scripts/`. They expect a local Ollama at `http://127.0.0.1:11434` running
`gemma4:e4b` (or a model name you pass via `--model`).

```sh
# 1. preserve current display as reading (idempotent, safe to re-run)
uv run python3 scripts/backfill_question_text_reading.py data/questions_ja.json data/questions_en.json

# 2. survey what needs work
uv run python3 scripts/question_text_stats.py data/questions_ja.json

# 3. rewrite hiragana-heavy display forms via local LLM (reading is preserved)
uv run python3 scripts/restore_ja_question_texts_with_ollama.py data/questions_ja.json --dry-run
uv run python3 scripts/restore_ja_question_texts_with_ollama.py data/questions_ja.json

# 4. list any stragglers for manual review
uv run python3 scripts/list_suspect_question_texts.py data/questions_ja.json

# 5. final lint
cargo run --bin lint-questions -- data/questions_ja.json data/questions_en.json
```

## License

MIT

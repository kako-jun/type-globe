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

Japanese quiz data may keep two parallel question strings: `question_text` for the visible prompt (kanji/katakana mixed) and `question_text_reading` for reading-preservation workflows. Choice labels stay input-oriented, so Japanese answers remain hiragana / katakana / ASCII rather than kanji.

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

### Demo mode (auto-play, for screencasts and unattended displays)

`--demo` runs Quiz in a fully automated way: each question waits, then the correct answer is typed character by character through the normal input path, so sound / reveal / transitions all fire as if a human were playing. Designed for promo recordings, onboarding loops, and unattended kiosks (e.g. llll-ll-media).

```sh
type-globe --demo                                       # 10 questions, 1 s wait, 20 cps, JA default
type-globe --demo --demo-count 5 --lang en              # 5 questions in English
type-globe --demo --demo-wait-ms 500 --demo-type-cps 40 # faster pacing
type-globe --demo --demo-loop                           # endless loop for kiosk display
type-globe --demo --genre science                       # restrict to one genre
```

Flags:
- `--demo-count <N>` — number of questions per run (default `10`)
- `--demo-wait-ms <MS>` — pause before auto-typing each answer (default `1000`)
- `--demo-type-cps <N>` — characters-per-second typing speed (default `20`)
- `--demo-loop` — restart the run forever (until `Esc` / `Ctrl+C`)
- `--lang`, `--genre` — same as quiz mode; `--lang` defaults to `ja` when omitted

Press `Esc` (or `Ctrl+C`) at any time to exit. `q` is reserved as a typing character so it cannot be used to quit. Demo runs do **not** write to Records.

Run `type-globe --help` or `type-globe <subcommand> --help` for all options.

For quiz-data migration work, the repository also ships:
- `uv run python3 scripts/backfill_question_text_reading.py data/questions_ja.json --dry-run`
- `uv run python3 scripts/question_text_stats.py data/questions_ja.json`
- `uv run python3 scripts/list_suspect_question_texts.py data/questions_ja.json`
- `uv run python3 scripts/restore_ja_question_texts_with_ollama.py data/questions_ja.json --dry-run`

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

Quiz answers are typed directly — there is no arrow / number-key fallback. An exact match auto-confirms immediately, and non-prefix typos are rejected on the spot instead of being inserted into the buffer. Matching is case-insensitive (`H2O` / `h2o`, `TOKYO` / `tokyo`). In JA mode, bundled data declares IME-wapuro spellings via `ja_typings`; runtime normalization absorbs input-method variants that produce the same kana, while long vowels stay strict (`toukyou` is not `tokyo`). Multiple `ja_typings` entries are reserved for true reading variants such as `nihon` / `nippon`.

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

# 3. rewrite hiragana-heavy question display forms via local LLM (reading is preserved)
uv run python3 scripts/restore_ja_question_texts_with_ollama.py data/questions_ja.json --dry-run
uv run python3 scripts/restore_ja_question_texts_with_ollama.py data/questions_ja.json

# Or refresh every JA question and answer display label through the same model path.
# This preserves question_text_reading.ja and choices[].ja_typings.
uv run python3 scripts/restore_ja_question_texts_with_ollama.py data/questions_ja.json --all --include-choices --dry-run
uv run python3 scripts/restore_ja_question_texts_with_ollama.py data/questions_ja.json --all --include-choices

# 4. list any stragglers for manual review
uv run python3 scripts/list_suspect_question_texts.py data/questions_ja.json

# 5. final lint
cargo run --bin lint-questions -- data/questions_ja.json data/questions_en.json
```

## License

MIT

# type-globe

[日本語](README.ja.md)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A quiz and typing game for your terminal. Learn things, type faster, and have fun -- all without leaving the command line.

## Why type-globe?

- **Learn while you play** -- 100 quiz questions spanning programming, anime, science, history, and more
- **Measure your typing speed** -- Real-time WPM and accuracy tracking
- **Stealth mode** (planned) -- Looks like a terminal. Play at work. We won't tell
- **No internet required** -- Everything runs locally as a single binary
- **Bilingual** -- Full Japanese and English support

## Features

- 4-choice quiz mode with score tracking and accuracy stats
- Typing mode with live WPM, accuracy, and progress display
- TUI powered by [ratatui](https://github.com/ratatui/ratatui) -- runs in any terminal
- 100 curated questions across 15 genres
- Language selection at startup (Japanese / English)
- Single binary, zero configuration

## Quick Start

```sh
cargo install type-globe
type-globe
```

## Game Modes

| Mode | Status | Description |
|------|--------|-------------|
| Quiz | Available | Answer 4-choice questions. Track your score and accuracy |
| Typing | Available | Type the displayed text. Measure WPM and accuracy in real time |
| Quiz + Typing | Planned | Answer the quiz, then type the correct answer |
| Time Attack 25 | Planned | Panel-based quiz inspired by Attack 25 |
| RPG (TypeQuest) | Planned | Earn XP and level up by answering questions |
| Stealth | Planned | A mode that blends into your terminal workflow |

## Gameplay

```
┌──────────────────────────────────────────┐
│      TypeGlobe - Quiz & Typing Game      │
├──────────────────────────────────────────┤
│  Select Language:                        │
│  > 日本語 (Japanese)                     │
│    English                               │
├──────────────────────────────────────────┤
│  ↑↓: Select | Enter: Confirm | q: Quit  │
└──────────────────────────────────────────┘
```

```
┌──────────────────────────────────────────┐
│         TypeGlobe - Quiz Mode            │
├──────────────────────────────────────────┤
│ Progress: ████████░░░░░░░░░░░░  5/20     │
├──────────────────────────────────────────┤
│ What does "HTTP" stand for?              │
│                                          │
│  1. HyperText Transfer Protocol          │
│  2. High Tech Transfer Process           │
│ >3. Home Tool Transfer Protocol          │
│  4. HyperText Transmission Path          │
├──────────────────────────────────────────┤
│ ↑↓: Select | 1-4: Jump | Enter: Answer  │
└──────────────────────────────────────────┘
```

### Key Bindings

**Menu**

| Key | Action |
|-----|--------|
| `↑` `↓` | Navigate |
| `Enter` | Confirm |
| `Esc` | Back |
| `q` | Quit |

**Quiz Mode**

| Key | Action |
|-----|--------|
| `↑` `↓` | Select answer |
| `1`-`4` | Jump to answer |
| `Enter` / `Space` | Submit answer |
| `s` | Skip question |
| `q` | Quit |

**Typing Mode**

| Key | Action |
|-----|--------|
| Any character | Type |
| `Backspace` | Delete |
| `Esc` / `Ctrl+q` | Quit |

## Quiz Genres

IT & Programming, Web Development, Science, Math, History, Geography, Language, Culture, Anime, Manga, Games, VTuber & Internet Culture, General Knowledge, and more.

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

- A terminal that supports Unicode and basic ANSI escape codes
- Rust 1.70+ (for building from source)

## License

MIT

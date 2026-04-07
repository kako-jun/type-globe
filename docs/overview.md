# TypeGlobe - Overview

## What is TypeGlobe?

TypeGlobe is a TUI (Text-based User Interface) game that combines quizzes with typing practice. Players improve both their knowledge and typing skills while having fun. The game runs entirely offline and supports multiple languages.

## Game Modes

- **Quiz Mode** - Answer 4-choice questions using arrow keys or number keys (1-4).
- **Typing Mode** - Type displayed words/sentences; measures WPM (Words Per Minute) and accuracy in real-time.
- **Quiz + Typing Mode** - Answer quiz questions by typing the correct choice in full (romaji).
- **Time Attack 25** - A 5x5 panel battle mode inspired by the TV show "Attack 25." Compete against CPU by answering quizzes to claim panels. Total time (thinking + answering) determines the ranking.
- **RPG Mode (TypeQuest)** - Earn EXP for correct answers, level up, and gain titles.
- **Stealth Mode** - Disguises the game as a CLI tool (e.g., "Analyzing logs..." or "Monitoring data stream...") so you can play at work.

## Design Philosophy

- **Educational + Fun** - Learning should never feel like a chore.
- **Offline-first** - All core features work without an internet connection.
- **Multi-language** - Players select their language (Japanese / English) at startup. Questions and UI adapt accordingly.
- **Stealth** - Sometimes you just need a break at work. Stealth mode has you covered.
- **Keyboard-only** - All interactions are done via keyboard. No mouse required.

## Tech Stack

| Component | Technology |
|---|---|
| Language | Rust |
| TUI framework | ratatui + crossterm |
| Serialization | serde + serde_json |
| Data storage | Local JSON files |

## Quiz Content

100 questions across multiple categories:

- **IT / Programming**: 40 questions (programming, web dev, technology, IT terminology)
- **Anime / Games / Manga**: 25 questions (anime, games, manga, VTuber/internet culture)
- **General Knowledge**: 25 questions (geography, history, science, math, language, culture)

Questions are generated using AI and stored in JSON format. A generation script (`scripts/generate_questions.py`) and a policy document (`.claude/quiz-generation-policy.md`) govern content quality.

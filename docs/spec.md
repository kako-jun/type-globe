# type-globe — Specification

> Version: targeting v0.2.0 (offline-complete edition).
> This document supersedes all v0.1.x specs. The previous "display-and-type" mode has been removed.

## Core Principle

**The string you must type is never shown on screen before you type it.**

Inspired by competitive Hyakunin Isshu karuta (where the lower verse is never read aloud — the reader recites only the upper verse, and players strike the matching card from memory). type-globe rewards **knowledge, memory, and listening comprehension**, not visual reflex.

## Two Axes (do not cross)

| Presentation | Game Structure | Character |
|---|---|---|
| **Quiz** (4-choice; image-quiz later) | Single-run score / Time Attack 25 / **Ranking** | Competitive, public |
| **Listening** (audio in EN / JA / ...) | **Hack-and-slash RPG** | Personal, progression |

Quiz is paired with ranking-style modes; listening is paired with the RPG. The two presentation styles are never crossed.

## Display Rules

**Forbidden** (would compromise the core principle)
- Showing the answer string before the player has typed it
- Skipping the reveal animation to see the question instantly

**Allowed**
- The question text and choice labels (Quiz only — listening shows no text)
- Characters the player has actually typed (echoed for typo recovery)
- Status data: HP, level, EXP, remaining time, CPM, WPM

## Game Modes

### Quiz Mode (Single-run / Ranking-eligible)

3-pane layout:

```
┌─────────────────────────────────────┬──────────┐
│ Q3/10                                │ Score    │
│ Which keyword moves ownership in     │ 12,340   │
│ Rust?                                ├──────────┤
│                                      │ Time     │
│ A) borrow                            │ 0:42     │
│ B) move                              ├──────────┤
│ C) ref                               │ CPM 230  │
│ D) clone                             │ WPM  46  │
├──────────────────────────────────────┴──────────┤
│ > mo▌                                            │
└──────────────────────────────────────────────────┘
[Esc] Quit  [Tab] Skip  [F5] Restart
```

- **No arrow-key selection.** Players type the correct choice's text directly; this both selects and answers.
- **Enter to confirm.** Required because some choices may share prefixes (e.g. `move` vs `movement`).
- Input echo shows only the characters the player has typed.
- Score = function(CPM, accuracy, correctness).

### Time Attack 25

- 5×5 panel grid (homage to the Japanese TV show *Attack 25*).
- CPU opponent. Whoever answers correctly first claims the panel.
- Total elapsed time (thinking + typing) determines ranking.

### Listening × Hack-and-Slash RPG

4-pane layout:

```
┌──────────────────────┬──────────────────┐
│   ♪ Listening...     │ Lv. 5            │
│      [▼ ▼ ▼]         │ EXP ███░░ 60%    │
│                      │ HP  ████░ 80%    │
│   Enemy: ♪/emoji     ├──────────────────┤
│   HP ███░░            │ Floor 3 / Goal 10│
│                      │ Run time 1:20    │
├──────────────────────┴──────────────────┤
│ > the quick brown f▌                     │
├──────────────────────────────────────────┤
│ ▸ Hit! 45 dmg                            │
│ ▸ Slime defeated! +20 EXP                │
└──────────────────────────────────────────┘
[Esc] Return to town  [Space] Replay sound  [F5] New run
```

- **The prompt is audio only.** No text is shown.
- Enemies are represented by emoji or symbols (no ASCII art).
- A ♪ note pulses with `jiwa_core` animation while audio plays.
- **One prompt = one enemy. One run = 10 enemies (fixed)** — a roguelike "go down, come back" cycle.
- **No failure state in v0.2.0.** Mistyping reduces EXP gain only; a run always completes after 10 prompts.
- **Audio replay is unlimited** (`Space`); no penalty other than the time it consumes.

## `jiwa_core` In-Repo Animation Module

Implemented in-tree as `src/jiwa_core/`. To be extracted into the standalone `jiwa` crate once stable.

- **Typewriter reveal** — characters appear one at a time.
- **Per-character fade-in** — TrueColor (24-bit RGB) interpolation from dim to full intensity.
- **Concurrent input acceptance** — players who already know the answer can begin typing before the reveal completes.
- **No skip key** — the reveal must always play to its end (fairness).

## Scoring

Both **CPM** (characters per minute) and **WPM** (words per minute) are displayed side by side. Quiz scores combine CPM, accuracy, and correctness; the exact formula is implementation-defined per mode.

## Data Structures

### Quiz question (`data/questions_<lang>.json`)

```json
[
  {
    "id": "q001",
    "genre": "programming",
    "question_text": "Which keyword moves ownership in Rust?",
    "choices": ["borrow", "move", "ref", "clone"],
    "correct_answer_index": 1,
    "image_path": null
  }
]
```

Validation: no two choices may share a prefix that would make a typed answer ambiguous before `Enter`. (Enforced by a build-time linter.)

### Listening prompt (`data/listening_<lang>.json`)

```json
[
  {
    "id": "l001",
    "text": "the quick brown fox jumps over the lazy dog",
    "lang": "en"
  }
]
```

The TTS layer turns `text` into audio at runtime via the `tts` crate; no audio files are shipped.

### Player progress (`player.json`)

```json
{
  "player_name": "User",
  "language": "ja",
  "rpg_stats": {
    "level": 1,
    "exp": 0,
    "hp_max": 100,
    "titles_unlocked": []
  }
}
```

### Ranking (`ranking_<lang>.json`)

```json
{
  "quiz_single": [
    {"name": "Player1", "score": 1500, "cpm": 230, "wpm": 46, "ts": "2026-04-30T10:00:00Z"}
  ],
  "time_attack_25": [
    {"name": "PlayerX", "time_seconds": 180, "ts": "2026-04-30T10:05:00Z"}
  ]
}
```

Top 10 per mode per language.

## Source Architecture (target)

```
src/
├── main.rs
├── types.rs
├── config.rs
├── jiwa_core/           # in-tree animation module (typewriter + fade + concurrent input)
│   ├── mod.rs
│   ├── typewriter.rs
│   ├── fade.rs
│   └── input.rs
├── game/
│   ├── mod.rs
│   ├── quiz.rs          # quiz presentation + "type-to-select" logic
│   ├── time_attack.rs   # 5x5 panel battle
│   └── hack.rs          # listening × RPG run loop
├── audio/
│   ├── mod.rs
│   └── tts.rs           # `tts` crate wrapper, language routing
├── io/
│   ├── mod.rs
│   ├── data_loader.rs
│   └── storage.rs       # player.json / ranking_*.json
└── ui/
    ├── mod.rs
    ├── menu.rs
    ├── quiz.rs          # 3-pane layout
    ├── hack.rs          # 4-pane layout
    ├── time_attack.rs
    ├── ranking.rs
    └── help_line.rs     # always-on bottom helpline
```

## Out of Scope for v0.2.0

These are deferred to v0.3.0+:

- **Image quiz** (requires terminal graphics protocol — kitty / iTerm2 / wezterm)
- **Stealth mode** (CLI-disguise UI)
- **mypace WebSocket integration** — to be released under the `type-globe-online` brand (same repo, different label)

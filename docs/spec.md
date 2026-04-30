# type-globe — Specification

> Version: targeting v0.2.0 (offline-complete edition).
> This document supersedes all v0.1.x specs. The previous "display-and-type" mode has been removed.
>
> Note: this is a **target specification** for the v0.2.0 redesign. The current `main` branch still uses JSON/`serde_json`, and some quiz interaction remains on the legacy selection model until follow-up issues land.

## Core Principle

**The string you must type is never shown on screen before you type it.**

Inspired by competitive Hyakunin Isshu karuta (where the lower verse is never read aloud — the reader recites only the upper verse, and players strike the matching card from memory). type-globe rewards **knowledge, memory, and listening comprehension**, not visual reflex.

## Two Axes (do not cross)

| Presentation | Game Structure | Character |
|---|---|---|
| **Quiz** (4-choice; image-quiz later) | Single-run score / Time Attack 25 / **Records** | Competitive, public |
| **Listening** (audio in EN / JA / ...) | **Hack-and-slash RPG** | Personal, progression |

Quiz is paired with score-attack modes; listening is paired with the RPG. The two presentation styles are never crossed.

> **Terminology.** "Records" = local self-best history. "Ranking" is reserved for world-vs-world comparison via Nostralgic Ranking, which lands in v0.3.0+ inside the `type-globe-online` brand label. Do not call self-best lists "ranking".

## Display Rules

**Forbidden** (would compromise the core principle)
- Showing the answer string before the player has typed it
- Skipping the reveal animation to see the question instantly

**Allowed**
- The question text and choice labels (Quiz only — listening shows no text)
- Characters the player has actually typed (echoed for typo recovery)
- Status data: HP, level, EXP, remaining time, CPM, WPM

## Game Modes

### Quiz Mode (Single-run / Records-eligible)

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
- Total elapsed time (thinking + typing) is the recorded result; the local self-best lands in Records.

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

Target v0.2.0 direction: all data files use **YAML** for readability and inline comments, with Rust side moved to `serde_yaml`.

Current `main` branch status: question banks / player progress / records are still JSON-backed and use `serde_json`.

### Answer-form classification (`kind`)

Every answer string is classified into one of three forms. This drives the hack-and-slash boss placement and lets the renderer choose appropriate enemy visuals.

| kind | form | examples | role |
|---|---|---|---|
| `word` | a single word | `Tokyo` / `move` / `borrow` | regular enemy |
| `phrase` | space-separated proper noun / compound | `George Washington` / `HyperText Transfer Protocol` | mid-tier enemy |
| `sentence` | a short sentence | `the quick brown fox jumps over the lazy dog` | **boss** |

### Hack-and-slash boss placement (Plan A — fixed)

Within one 10-prompt run:

- prompts 1–7 → `word`
- prompts 8–9 → `phrase`
- **prompt 10 → `sentence` (boss)** — guaranteed dramatic finish; the TTS readout is also longer, reinforcing the boss feel acoustically.

Quiz-side runs are not bound by this layout — quiz questions may freely mix kinds.

### Quiz question (`data/questions_<lang>.yaml`)

```yaml
- id: q001
  genre: programming
  question: Which keyword moves ownership in Rust?
  choices: [borrow, move, ref, clone]
  correct: 1                # 0-indexed
  kind: word

- id: q050
  genre: history
  question: Who was the first president of the United States?
  choices:
    - George Washington
    - Abraham Lincoln
    - Thomas Jefferson
    - John Adams
  correct: 0
  kind: phrase

- id: q099
  genre: literature
  question: How does Descartes' famous proposition begin in English?
  choices:
    - I think therefore I am
    - I drink therefore I sleep
    - I see therefore I know
    - I am therefore I think
  correct: 0
  kind: sentence
```

Validation: no two choices in a question may share a prefix that would make a typed answer ambiguous before `Enter`. Enforced by `cargo run --bin lint-questions -- <files>` (CI job `lint-data`) and by the unit tests `shipped_question_data_is_clean_{ja,en}` in `src/io/validator.rs`.

### Listening prompt (`data/listening_<lang>.yaml`)

```yaml
- id: l001
  text: Tokyo
  kind: word

- id: l050
  text: George Washington
  kind: phrase

- id: l100
  text: the quick brown fox jumps over the lazy dog
  kind: sentence
```

The TTS layer turns `text` into audio at runtime via the `tts` crate; no audio files are shipped.

### Player progress (`player.yaml`)

```yaml
player_name: User
language: ja
rpg_stats:
  level: 1
  exp: 0
  hp_max: 100
  titles_unlocked: []
```

### Records (`records_<lang>.yaml`)

```yaml
quiz_single:
  - name: Player1
    score: 1500
    cpm: 230
    wpm: 46
    ts: 2026-04-30T10:00:00Z
time_attack_25:
  - name: PlayerX
    time_seconds: 180
    ts: 2026-04-30T10:05:00Z
```

Top 10 per mode per language. This is a local self-best file — never call it a "ranking". World ranking (Nostralgic Ranking) is wired in the v0.3.0+ `type-globe-online` build and submits the same entries to a Nostr-relay-backed feed.

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
│   └── storage.rs       # current main: JSON, target v0.2.0: YAML
└── ui/
    ├── mod.rs
    ├── menu.rs
    ├── quiz.rs          # 3-pane layout
    ├── hack.rs          # 4-pane layout
    ├── time_attack.rs
    ├── records.rs
    └── help_line.rs     # always-on bottom helpline
```

## Out of Scope for v0.2.0

These are deferred to v0.3.0+:

- **Image quiz** (requires terminal graphics protocol — kitty / iTerm2 / wezterm)
- **Stealth mode** (CLI-disguise UI)
- **mypace WebSocket integration** — to be released under the `type-globe-online` brand (same repo, different label)

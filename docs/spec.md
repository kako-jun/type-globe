# type-globe — Specification

> Version: v0.2.0 (offline-first blind-typing edition).
> This document supersedes all v0.1.x specs. The previous "display-and-type" mode has been removed.
>
> Note: `main` now ships this blind-typing interaction model. Storage still uses JSON/`serde_json` in this release line; the YAML migration remains future work.

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
│ 3/10                                 │ Score    │
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

- **Sound cues** (Issue #73). Quiz mode synthesizes five cues at runtime via `rodio` — no asset files. The cues are: `QuestionReveal` ("ダダン") fired as each new question appears, `Correct` ("ピンポーン") fired the instant the correct answer is typed, `Wrong` ("ブブー") fired on Tab skip, `Keystroke` (quiet tick) on each accepted character, and `Mistype` (slightly louder, lower) when an off-prefix character is rejected. If no audio device is available the engine falls back to silent operation; the game stays fully playable.
- **The four choices are shuffled per question** so the answer's display position varies. Labels A/B/C/D are positional, not identity-based; the typing match is identity-based and stays correct under any shuffle.
- **The choices fade in after the question text.** The question reveal starts immediately; the choices block stays invisible for ~0.5 s, then all four fade in together over ~0.3 s. This frames the question first and the options second.
- **Question text settles to a soft green** (`Rgb(160, 220, 160)`) so it stays distinct from the choices and the input echo.
- **No arrow-key selection.** Players type the correct choice's text directly; this both selects and answers.
- **Exact match auto-confirms and immediately advances** to the next question — there is no "Correct!" interstitial and no Enter-to-continue. The flow is a continuous typing rhythm.
- **Only the correct choice's typings are accepted as a valid prefix.** Any divergence (including the full text of a wrong choice) is treated as a mistype: the input flashes red and the buffer resets to zero, and the run does not advance. The player can only proceed by typing the correct answer.
- Matching is **case-insensitive**.
- A single logical answer may accept **multiple typed spellings**.
- In JA mode, romanized aliases may be accepted for the same answer (for example `tokyo` and `toukyou`, `osaka` and `oosaka`). Question data may declare these explicitly with `ja_typings`. When a choice has a well-established official Latin spelling (for example a proper name), that spelling may also be accepted. The implementation may also derive additional common ASCII aliases from kana as long as it never reveals the answer string before typing.
- Score = function(CPM, accuracy, correctness).
- One run is fixed at **10 questions** (constant `QUIZ_RUN_LENGTH`), sampled from the language's question pool. The total Time is **frozen at the last correct keystroke** of the final question (or at the moment the final question is skipped via Tab) — it does not keep ticking on the Summary / Records-entry screens. After the 10th question, the UI shows a Summary (Score / Correct / Accuracy / CPM / WPM / Time), then a Records-entry screen prompts for a name and writes a `ScoreEntry` to `records_<lang>.json` (Top 10 by score; ts as tiebreaker). Esc on either screen returns to the menu without saving.

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
- A ♪ note pulses with `jiwa_core::pulse::PulseHandle` (sinusoidal dim↔bright cycle) while audio plays. The listening UI calls `start("♪", PulseOpts::default_listening())` for the duration of the audio playback and snapshots it once per render frame.
- **One prompt = one enemy. One run = 10 enemies (fixed)** — a roguelike "go down, come back" cycle.
- **No failure state in v0.2.0.** Mistyping reduces EXP gain only; a run always completes after 10 prompts.
- **Audio replay is unlimited** (`Space`); no penalty other than the time it consumes.

## `jiwa_core` In-Repo Animation Module

Implemented in-tree as `src/jiwa_core/`. To be extracted into the standalone `jiwa` crate once stable.

- **Typewriter reveal** — characters appear one at a time. Iteration is by Unicode grapheme cluster (UAX #29), so combining marks and ZWJ sequences advance as one unit. Implemented in `jiwa_core::reveal::RevealHandle` (#19/#20).
- **Per-character fade-in** — TrueColor (24-bit RGB) interpolation from `fade_from` to `fade_to` over `fade_duration`. Linear interpolation per channel. The renderer maps the resulting `Rgb(u8,u8,u8)` to `ratatui::Color::Rgb`. (#21)
- **Pure / time-injectable** — every entry point takes an explicit `Instant`, so callers tick the reveal at their own redraw cadence and tests advance time without sleeping.
- **Concurrent input acceptance** — players who already know the answer can begin typing before the reveal completes. Implemented in `src/ui/input_loop.rs`: a worker thread runs `event::poll` / `event::read` and pushes `KeyEvent`s through an `mpsc::channel`; the render thread blocks on `recv_timeout(REDRAW)` so input never has to wait for the next frame, and the redraw cadence is independent of input cadence (#22).
- **Listening pulse** — `jiwa_core::pulse::PulseHandle` drives the `♪` symbol on the listening pane: a sinusoidal dim↔bright cycle (`PulseOpts::default_listening` = 1.5 s period). Same pure / time-injectable shape as `RevealHandle`. (#23)
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

Validation: no two choices in a question may share a prefix that would make an auto-confirm ambiguous. Enforced by `cargo run --bin lint-questions -- <files>` (CI job `lint-data`) and by the unit tests `shipped_question_data_is_clean_{ja,en}` in `src/io/validator.rs`.

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

The TTS layer turns `text` into audio at runtime via the `tts` crate (#28); no audio files are shipped. The current `main` ships `data/listening_<lang>.json` (JSON during the JSON-era of v0.2.0); the YAML migration renames `.json` → `.yaml` 1:1 with no schema change.

#### Linux runtime requirement

The `tts` crate's Linux backend is `speech-dispatcher`, which must be installed and running before listening mode can speak. Build-time, `libspeechd-dev` must be installed (CI installs it in the test / clippy / lint-data jobs). When the daemon isn't available at runtime, the listening UI shows a "Listening mode is unavailable on this system" message and returns to the menu rather than crashing — Quiz / Records / Time Attack 25 stay reachable.

#### v0.2.0 foundation scope (#28-#31)

The foundation epic ships:
- `tts` crate integration (`src/audio/tts.rs`),
- the listening prompt schema and bilingual data (`data/listening_<lang>.json`),
- a single-prompt practice flow under the **Listening RPG** menu entry that exercises the blind-input judge end-to-end.

The ten-prompt run loop with HP / EXP and the fixed boss placement (1-7 word, 8-9 phrase, 10 sentence) is the next epic (#32-#37). Until then, the practice mode filters the pool to `word`-kind prompts because `Space` is reserved for replay (per the key bindings above) and a phrase / sentence answer cannot be typed without rebinding the input model — that rebinding is part of #32-#37.

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

The Records menu entry opens a read-only browser (`src/ui/records.rs`) that shows three sections — Quiz, Time Attack 25, Listening RPG — with the most recent ts in each section highlighted so the player can spot a just-saved entry without scrolling. Esc / Enter / `q` returns to the menu.

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
│   └── rpg.rs           # listening × RPG run loop
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
    ├── rpg.rs           # 4-pane layout
    ├── time_attack.rs
    ├── records.rs
    └── help_line.rs     # always-on bottom helpline
```

## Out of Scope for v0.2.0

These are deferred to v0.3.0+:

- **Image quiz** (requires terminal graphics protocol — kitty / iTerm2 / wezterm)
- **Stealth mode** (CLI-disguise UI)
- **mypace WebSocket integration** — to be released under the `type-globe-online` brand (same repo, different label)

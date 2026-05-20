# type-globe — Specification

> Version: v0.2.0 (offline-first blind-typing edition).
> This document supersedes all v0.1.x specs. The previous "display-and-type" mode has been removed.
>
> Note: `main` now ships this blind-typing interaction model. Storage uses YAML/`serde_yaml` for Player and Records files; question banks remain JSON.

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
- **Inline code rendering** (Issue #97). Markdown-style single-backtick spans (`` `code` ``) inside `question_text` and choice strings are rendered with a distinct color (`Rgb(255, 200, 60)`, orange-leaning amber — intentionally a different RGB from the input-echo `Color::Yellow` so the two never get confused on a typical terminal palette) and Bold weight; the backticks themselves are stripped before display. During the question's typewriter reveal the per-grapheme fade color is preserved and only the Bold modifier is added; once the reveal settles, code graphemes switch to the dedicated inline-code color. This is a display-layer-only feature — `ja_typings`, romaji canonicalization, and the typing-match path are all untouched, so backticks never appear in the data the player needs to type. Out of scope: escaped backticks (`` \` ``) and double/triple-backtick fences are not interpreted; an unmatched opening backtick is preserved verbatim as plain text.
- **No arrow-key selection.** Players type the correct choice's text directly; this both selects and answers.
- **Exact match auto-confirms and immediately advances** to the next question — there is no "Correct!" interstitial and no Enter-to-continue. The flow is a continuous typing rhythm.
- **Only the correct choice's typings are accepted as a valid prefix.** Any divergence (including the full text of a wrong choice) is treated as a mistype: the input flashes red and the buffer resets to zero, and the run does not advance. The player can only proceed by typing the correct answer.
- Matching is **case-insensitive**.
- A single logical answer may accept **multiple typed spellings** only when they are genuine answer/readings variants.
- In JA mode, `ja_typings` stores one IME-wapuro-correct spelling per kana reading. Runtime canonicalization absorbs input-method variants that produce the same kana, so data must not enumerate redundant pairs such as `ninnshou` / `ninshou`, `dairanntou` / `dairantou`, or `thi` / `texi`. Long vowels are strict: `toukyou` and `tokyo`, or `oosaka` and `osaka`, are not equivalent. Multiple `ja_typings` entries are reserved for true reading variants such as `nihon` / `nippon`; official Latin spellings are not added merely as alternate romanization for the same Japanese reading.
- **Input matching: canonical romaji normalization (#96, expanded in v0.7.0).** In **JA mode only**, both the player's typed input and every `ja_typings` candidate are mapped to a shared canonical form before prefix / equality comparison. EN mode (and any future non-JA mode) uses plain lowercase comparison so that English words containing Japanese-romaji substrings are not silently rewritten. Type-globe defines IME-wapuro as the "正" (canonical) typing — every accepted input must produce the target kana when typed through a real IME. Details:
    - Rules are substring rewrites on the lowercased input. They are applied in a fixed order so chained patterns collapse correctly (`tsushi` → `tusi`, `shixya` → `sixya` → `sya`).
    - **Punctuation shortcut keys** (IME 全角ショートカット): `/` (・), `,` (、), `.` (。) are preserved and must match positionally. They also act as IME commit boundaries, so a preceding single `n` canonicalizes to `nn`.
    - **Non-standard digraph IME alt-paths**:
        - ファ系: `huxa/hula/fuxa/fula → fa` etc. for `fi/fe/fo` too
        - ヴァ系: `vuxa/vula → va`, etc.
        - ティ系: `texi/teli → thi`
        - ディ系: `dexi/deli → dhi`
        - ぢ系: `dji/dzi → di`; `di` and `dhi` are distinct because IME maps them to ぢ and ディ
        - ヅ: `dzu → du`
        - ウェ系: `uxe/ule → we`, `uxi/uli → wi`, `uxo/ulo → wo` (`uxa/ula → wa` excluded: ウァ ≠ ワ)
    - **Yoon (拗音) Hepburn ↔ Kunrei**: `sha/sya`, `shu/syu`, `sho/syo`, `cha/tya`, `chu/tyu`, `cho/tyo`, `ja/zya`, `ju/zyu`, `jo/zyo`.
    - **Single Hepburn ↔ Kunrei**: `shi → si`, `chi → ti`, `tsu → tu`, `ji → zi`, `fu → hu`. (`wo/o` is **not** unified — different IME keys yield different kana.)
    - **Yoon explicit-small-ya path**: `Cixya/Cilya/Cixyu/.../Cilyo` → `Cya/Cyu/Cyo`. Combined with the single rules, `kixya ≡ kya`, `shixya ≡ sha ≡ sya`, etc.
    - **Sokuon (促音) explicit-small-tsu path**: `ltu/xtu/ltsu/xtsu` followed by a consonant `C` collapses to `CC`. So `rokeltsuto ≡ roketto`, `maltsucha ≡ matcha ≡ mattya`.
    - **n-run handling**: a `nn` pair is preserved before a vowel / `y` / `n` (so ん+母音 and ん+ナ行 stay distinct), and collapsed to `n` before a consonant or end-of-word. Runs of 3+ n's are **not** collapsed — `sennorikyuu` (2-n, would type as せんおりきゅう in IME) and `sennnorikyuu` (3-n, the correct せんのりきゅう) canonicalize to different strings. Data must register the IME-correct form.
    - **Pass-through**: katakana `ー` → `-` (data registers `-`); long-vowel hiragana (`ou`/`oo`/`uu`) is preserved.
    - Consequence: a player may type Hepburn or Kunrei single/yoon, the explicit-small-kana IME paths (`ltsu`, `xtu`, `kixya`, `huxa`, `uxe`, etc.), or the punctuation-shortcut keys (`/`, `,`, `.`), and any path that produces the target kana through a real IME is accepted. The data file keeps a single canonical entry per kana reading.
    - The build-time prefix-conflict linter (`io::validator`) also compares in canonical space, so real conflicts (e.g. `to` vs `tokyo`) are still detected while spurious conflicts caused only by spelling variants are no longer flagged.
    - **Data coverage test** (`src/game/quiz.rs::data_typings_are_prefix_typeable`): every registered `ja_typings` entry is type-tested prefix-by-prefix (1 char → ... → full) through `is_valid_correct_typed_prefix`. ~40 ms in release builds; runs in the default test suite, so any future data edit that breaks IME typability fails CI.
- Score = function(CPM, accuracy, correctness).
- One run is fixed at **10 questions** (constant `QUIZ_RUN_LENGTH`), sampled from the language's question pool. The total Time is **frozen at the last correct keystroke** of the final question (or at the moment the final question is skipped via Tab) — it does not keep ticking on the Summary / Records-entry screens. After the 10th question, the UI shows a Summary (Score / Correct / Accuracy / CPM / WPM / Time), then a Records-entry screen prompts for a name and writes a `ScoreEntry` to `records_<lang>.yaml` (Top 10 by score; ts as tiebreaker). Esc on either screen returns to the menu without saving.

### Time Attack 25

- Intended structure: **four seats**, not a simplified head-to-head duel. The default local roster is **one human + three CPU seats**; whenever extra humans are available, they replace CPU seats instead of changing the underlying game shape.
- Canonical seat colors are **red / blue / green / yellow**.
- The current prototype model is a four-slot roster with, per seat: `color`, `kind` (`human` / `cpu` / `empty`), `player_id`, and `display_name`.
- The standard local roster is: `red=You(human), blue=CPU 1(cpu), green=CPU 2(cpu), yellow=CPU 3(cpu)`.
- 5×5 panel grid (homage to the Japanese TV show *Attack 25*).
- Current shipping state: **local prototype playable now**. The build ships a 25-question offline run with a visible 5×5 board, three pseudo-CPU opponents, and sequential panel acquisition for panel 01..25.
- Prototype scope note: panel choice is intentionally simplified for now. The current build fills the board in order so pacing, color ownership, and CPU tempo can be tuned before full `nostr_arena` play and richer capture rules land.
- Prototype persistence note: TA25 result saving is **not wired yet**. The Records screen already reserves a TA25 section, but the current prototype is for board / tempo / roster validation first.
- Final direction: whoever answers correctly first claims the panel; total elapsed time (thinking + typing) becomes the recorded result once the TA25 Records path is implemented.

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
- A ♪ note pulses with `jiwa::PulseHandle` (sinusoidal dim↔bright cycle) while audio plays. The listening UI calls `start("♪", PulseOpts::cyan_breath())` for the duration of the audio playback and snapshots it once per render frame.
- **One prompt = one enemy. One run = 10 enemies (fixed)** — a roguelike "go down, come back" cycle.
- **No failure state in v0.2.0.** Mistyping reduces EXP gain only; a run always completes after 10 prompts.
- **Audio replay is unlimited** (`Space`); no penalty other than the time it consumes.
- Current shipping state: the build now exposes a **10-battle prototype run**. Encounters 1-4 and 6-9 use the ordinary listening pane; prompt 5 is a timed reverse-Akinator miniboss; prompt 10 is a manual stacked-hint boss.
- HP / EXP / title persistence is still follow-up work, but the run no longer stays a flat dictation stream.

#### Boss Encounter UI (planned layout)

Boss encounters do **not** reuse the ordinary listening pane unchanged. They use a stacked-hint variant of the same 4-pane family:

```
┌──────────────────────────────────────┬──────────────────┐
│ Miniboss 5/10                        │ Hint 2 / 4       │
│ [1] Part of speech: noun             │ Reveal timed     │
│ [2] Used when: train travel          │ Plays  2         │
│ [3] Not this: airport                │ Run time 1:08    │
│ [4] First letter: t                  │ Boss HP ███░     │
├──────────────────────────────────────┴──────────────────┤
│ > to▌                                                │
├────────────────────────────────────────────────────────┤
│ ▸ Hint 2 voiced                                       │
│ ▸ Hint 3 opens in 4.0 s                               │
└────────────────────────────────────────────────────────┘
[Esc] Quit  [Space] Replay  [S-Tab] Prev hint
```

- The **main pane** becomes a **hint stack**. Earlier hints stay visible; the newest hint is the active one.
- The **side pane** shows boss-only state such as hint count, reveal mode, replay count, run time, and enemy/boss HP.
- The **input echo** stays unchanged: the player still types blind and exact-match auto-confirm remains the intended answer flow.
- The **log pane** holds event history: which hint was voiced, when the next hint opens, and hit/miss results after answer.
- **Current input-model constraint**: as long as `Space` remains replay and exact-match auto-confirm stays intact, boss answers must remain **`word`-kind prompts**. Phrase / sentence boss answers are deferred until the listening input model itself is rebased in the later RPG issue set.

Two reveal modes are planned:

- **Prompt 5 miniboss**: default to **timed reveal**. Hints open automatically after short delays, keeping the fight brisk.
- **Prompt 10 boss**: default to **manual reveal**. The player controls escalation and can ask for the next hint deliberately.

Planned active-phase keys:

- `Esc`: quit the run
- `Space`: replay the currently focused voiced hint
- `Tab`: next hint (manual boss only)
- `S-Tab`: previous hint / re-focus an older stacked hint
- `Backspace`: erase typed input

`Enter` is intentionally **not** the primary answer-confirm key here either; the intended rule stays exact-match auto-confirm on the final character.

## `jiwa` Animation Crate

Extracted from this repo's former `src/jiwa_core/` module into the standalone [`jiwa`](https://crates.io/crates/jiwa) crate (since v0.1.0). type-globe consumes it via `Cargo.toml`'s `jiwa = "0.1"` dependency.

- **Typewriter reveal** — characters appear one at a time. Iteration is by Unicode grapheme cluster (UAX #29), so combining marks and ZWJ sequences advance as one unit. Implemented in `jiwa::RevealHandle` (#19/#20).
- **Per-character fade-in** — TrueColor (24-bit RGB) interpolation from `fade_from` to `fade_to` over `fade_duration`. Linear interpolation per channel. The renderer maps the resulting `jiwa::Rgb(u8,u8,u8)` to `ratatui::Color::Rgb`. (#21)
- **Pure / time-injectable** — every entry point takes an explicit `Instant`, so callers tick the reveal at their own redraw cadence and tests advance time without sleeping.
- **Concurrent input acceptance** — players who already know the answer can begin typing before the reveal completes. Implemented in `src/ui/input_loop.rs`: a worker thread runs `event::poll` / `event::read` and pushes `KeyEvent`s through an `mpsc::channel`; the render thread blocks on `recv_timeout(REDRAW)` so input never has to wait for the next frame, and the redraw cadence is independent of input cadence (#22).
- **Listening pulse** — `jiwa::PulseHandle` drives the `♪` symbol on the listening pane: a sinusoidal dim↔bright cycle (`PulseOpts::cyan_breath` = 1.5 s period). Same pure / time-injectable shape as `RevealHandle`. (#23)
- **No skip key** — the reveal must always play to its end (fairness).

## Scoring

Both **CPM** (characters per minute) and **WPM** (words per minute) are displayed side by side. Quiz scores combine CPM, accuracy, and correctness; the exact formula is implementation-defined per mode.

## Data Structures

All persistent data files use **YAML** (`serde_yaml`). Question banks (`data/questions_<lang>.json`) remain JSON because they are authored/generated externally. Listening prompt banks (`data/listening_<lang>.yaml`) use YAML.

### Answer-form classification (`kind`)

Every answer string is classified into one of three forms. This guides ordinary enemy selection and lets the renderer choose appropriate enemy visuals. Boss encounters may override the plain `kind` pacing with their own presentation format.

| kind | form | examples | role |
|---|---|---|---|
| `word` | a single word | `Tokyo` / `move` / `borrow` | regular enemy |
| `phrase` | space-separated proper noun / compound | `George Washington` / `HyperText Transfer Protocol` | mid-tier enemy |
| `sentence` | a short sentence | `the quick brown fox jumps over the lazy dog` | heavy / high-tier prompt |

### Hack-and-slash run pacing (current direction)

Within one 10-prompt run:

- prompts 1–4 → regular listening encounters, usually `word` / short-form material
- **prompt 5 → miniboss** — a layered-hint encounter may override the plain dictation format
- prompts 6–9 → regular encounters again, with `word` / `phrase` / harder prompt mixes as needed
- **prompt 10 → boss** — expected to use a reverse-Akinator-style layered-hint format instead of only reading the final answer verbatim

The `kind` field still matters for data organization and ordinary enemy flavor. The boss slots are a higher-level run-structure rule layered on top of it, not a full replacement for `kind`.

Quiz-side runs are not bound by this layout — quiz questions may freely mix kinds.

### Quiz question (`data/questions_<lang>.json`)

```json
[
  {
    "id": "q001",
    "genre": "programming",
    "question_text": {
      "ja": "Rustで所有権を移動するキーワードはどれ？",
      "en": "Which keyword moves ownership in Rust?"
    },
    "question_text_reading": {
      "ja": "らすとでしょゆうけんをいどうするきーわーどはどれ？",
      "en": "Which keyword moves ownership in Rust?"
    },
    "choices": [
      { "ja": "borrow", "en": "borrow" },
      { "ja": "move", "en": "move" },
      { "ja": "ref", "en": "ref" },
      { "ja": "clone", "en": "clone" }
    ],
    "correct_answer_index": 1,
    "image_path": null
  }
]
```

`question_text` is the on-screen display text. In JA, this should use normal kanji/katakana mixed writing. `question_text_reading` is an optional reading-preservation field for TTS / conversion workflows; in JA it should stay hiragana-first. When `question_text_reading` is absent, the runtime falls back to `question_text`. Quiz choice labels remain input-oriented: JA choices should stay hiragana / katakana / ASCII so players can type without kana-kanji conversion.

Recommended migration order for existing JA quiz banks:
1. Backfill `question_text_reading.ja` from the current `question_text.ja`.
2. Rewrite only `question_text.ja` into kanji/katakana mixed display text.
3. Run stats/lint checks and manually inspect hiragana-heavy leftovers with `scripts/list_suspect_question_texts.py`.

Validation: no two choices in a question may share a prefix that would make an auto-confirm ambiguous. Enforced by `cargo run --bin lint-questions -- <files>` (CI job `lint-data`) and by the unit tests `shipped_question_data_is_clean_{ja,en}` in `src/io/validator.rs`. The same lint binary also flags `ja_typings redundant-variant` — multiple typings in the same choice that collapse to the same canonical form (e.g. `ninnshou` / `ninshou`, where redundant `nn` before a consonant collapses to `n`). Such duplicates are noise after v0.7.0's canonical_romaji expansion; only register one form per canonical group. Genuine reading variants (`日本` = `nihon` / `nippon`) are preserved because their canonical forms differ (the geminate `pp` distinguishes them).

### Listening prompt (`data/listening_<lang>.yaml`)

```yaml
- id: l-en-001
  text_reading: apple
  text_display: apple
  kind: word

- id: l-ja-071
  text_reading: とうきょう えき
  text_display: 東京駅
  kind: phrase

- id: l-en-091
  text_reading: the quick brown fox jumps over the lazy dog
  text_display: the quick brown fox jumps over the lazy dog
  kind: sentence
```

`text_reading` is passed to TTS and used for romaji conversion (hiragana-only for JA). `text_display` is shown on screen (kanji/katakana for JA; same as `text_reading` for EN). Prompt banks live in `data/listening_<lang>.yaml` (100 items each: word×70 / phrase×20 / sentence×10).

Boss-ready prompt entries may also carry optional `boss` metadata:

```yaml
- id: l-en-boss-010
  text_reading: ticket
  text_display: ticket
  kind: word
  boss:
    tier: boss
    reveal_mode: manual
    hints:
      - label: Part of speech
        text_display: noun
      - label: Used when
        text_display: when boarding a train
      - label: First letter
        text_display: t
        text_reading: tee
```

Boss fields:

- `boss.tier`: `miniboss` or `boss`
- `boss.reveal_mode`: `manual` or `timed`
- `boss.hints[]`: stacked hint steps
- `boss.hints[].label`: short on-screen heading
- `boss.hints[].text_display`: visible hint text
- `boss.hints[].text_reading`: optional TTS-specific reading override
- `boss.hints[].auto_reveal_after_ms`: optional timed-mode delay
- For the current blind-input model, boss prompt answers should stay `kind: word`. The hints themselves may be longer phrases.

#### Linux runtime requirement

The `tts` crate's Linux backend is `speech-dispatcher`, which must be installed and running before listening mode can speak. Build-time, `libspeechd-dev` must be installed (CI installs it in the test / clippy / lint-data jobs). When the daemon isn't available at runtime, the listening UI shows a "Listening mode is unavailable on this system" message and returns to the menu rather than crashing — Quiz / Records / Time Attack 25 stay reachable.

The baseline audio strategy is **runtime synthesis**, not pre-generated voice files. This keeps the system compatible with growing prompt banks and future boss-hint variants; alternate voices may be added later as swappable runtime backends.

The speech pipeline distinguishes at least four utterance intents: ordinary prompt read, replay, boss hint, and boss reveal. The current build actively uses ordinary prompt read and replay, and prepares boss hint / boss reveal on the same runtime TTS path. If the backend supports speech-rate control, replay and future early boss hints may be slowed down, and if it does not, the system falls back to normal-speed speech rather than failing the run. Runtime support is treated in three operational buckets: unavailable (engine init fails), basic (speech works but rate/stop controls may be missing), and preferred (speech + stop + rate available). See `docs/audio.md` for the operational policy.

#### v0.2.0 foundation scope (#28-#31)

The foundation epic ships:
- `tts` crate integration (`src/audio/tts.rs`),
- the listening prompt schema and bilingual data (`data/listening_<lang>.yaml`),
- a 10-battle prototype flow under the **Listening RPG** menu entry that exercises the blind-input judge, regular encounters, and boss-hint pacing end-to-end.

The ten-prompt run loop with HP / EXP and boss-beat pacing (regular fights, miniboss at prompt 5, boss at prompt 10) is the next epic (#32-#37). Until then, the practice mode filters the pool to `word`-kind prompts because `Space` is reserved for replay (per the key bindings above) and a phrase / sentence answer cannot be typed without rebinding the input model — that rebinding is part of #32-#37.

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
quiz_mode:
  - name: Player1
    score: 1500
    cpm: 230
    wpm: 46
    ts: 2026-04-30T10:00:00Z
time_attack_25:
  - name: PlayerX
    time_seconds: 180
    ts: 2026-04-30T10:05:00Z
rpg:
  - name: Player1
    score: 800
    cpm: 180
    wpm: 36
    ts: 2026-04-30T10:10:00Z
```

Top 10 per mode per language. This is a local self-best file — never call it a "ranking". World ranking (Nostralgic Ranking) is wired in the v0.3.0+ `type-globe-online` build and submits the same entries to a Nostr-relay-backed feed.

The Records menu entry opens a read-only browser (`src/ui/records.rs`) that shows three sections — Quiz, Time Attack 25, Listening RPG — with the most recent ts in each section highlighted so the player can spot a just-saved entry without scrolling. Esc / Enter / `q` returns to the menu.

## Source Architecture (target)

```
src/
├── main.rs
├── types.rs
├── config.rs
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

# Changelog

## v0.8.0 — 2026-05-22

### Added

- **Time Attack 25 results are now saved to Records** (#41 / #42 / #43 / #44).
  After the 25th panel resolves, the UI shows a Summary (per-seat panel
  counts + total elapsed time), then a name-entry screen writes a
  `TimeEntry` to the `time_attack_25` section of `records_<lang>.yaml`
  (Top 10 by time; ts as tiebreaker). Enter saves, Esc on either screen
  skips the save and returns to the menu. Mirrors `QuizUI` patterns
  (`Phase` state machine, `NAME_MAX_CHARS = 16`, twice-Enter dismiss
  prevention, `pending_warnings` flush after `LeaveAlternateScreen`).
- **TA25 HelpLine is now phase-aware.** Playing / Summary / NamingForRecord
  (saved=false) / NamingForRecord (saved=true) each get a dedicated help row
  so the player can drive the run from the bottom HelpLine alone.

### Changed

- **`now_rfc3339` extracted to `src/ui/timestamp.rs`** as a shared helper
  for `QuizUI` and `TimeAttack25UI`. Output format unchanged
  (`YYYY-MM-DDTHH:MM:SSZ`).
- `listen_boss.rs` collapsed two pairs of nested `match`/`if` into guard
  patterns to satisfy the newer Rust 1.95 clippy `collapsible_match` lint.
  Behavior unchanged.

## v0.7.7 — 2026-05-17

### Changed

- **Animation primitives extracted to standalone `jiwa` crate.** The in-tree
  `src/jiwa_core/` module — `RevealHandle` (typewriter + per-grapheme fade)
  and `PulseHandle` (♪ pulse) — is now the [`jiwa = "0.1"`](https://crates.io/crates/jiwa)
  dependency, shared with `curion` and `gitpp`. The renderer-agnostic API
  (`Rgb(u8,u8,u8)` returned per grapheme/frame; explicit `Instant`-injectable
  timing) is unchanged. The two preset constructors were renamed from
  use-case names to color-descriptive names so the library reads cleanly:
  `RevealOpts::default_quiz()` → `soft_green()`,
  `PulseOpts::default_listening()` → `cyan_breath()`.
  No user-visible behavior change — colors, timings, grapheme handling,
  and concurrent input acceptance are identical.

### Removed

- `src/jiwa_core/` (lifted into the `jiwa` crate).

## v0.7.6 — 2026-05-14

JA 入力を日本語 IME ルール準拠へ再整理（`ザ=za`、`ティ=thi`、`ディ=dhi`、
`ぢ=di`、`ガンディー=gandhi-`、`おお/おう` は `oo/ou` 非同一視）。`ja_typings`
は原則 1 候補、`日本` の `nihon/nippon` のような読みそのものが複数の場合だけ
複数登録を許す方針に統一。

## v0.6.0 ~ v0.7.5

Quiz UX 大改造、効果音 5 種、`--demo` / `--demo-loop`、735 問データ再構築、
IME-wapuro strict 仕様確立、prefix conflict 修正、records 用語整理ほか。

## v0.5.0 and earlier

See git history.

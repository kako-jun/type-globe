# Changelog

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

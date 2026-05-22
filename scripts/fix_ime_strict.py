#!/usr/bin/env python3
"""Mechanical auto-fixer for IME-strict ja_typings violations.

Handles the categories identified by lint_ja_typings.py:
  A1 + S1  remove ASCII/English-literal alt typings when a proper wapuro
           sibling typing already exists in the same choice
  T1       `ti` → `thi` where the ja side contains ティ (positional match)
  D1       `di` → `dhi` where the ja side contains ディ
  K1       kunrei `si/ti/tu/hu/zi` → Hepburn `shi/chi/tsu/fu/ji`
           (applied positionally where the corresponding Hepburn kana is
           confirmed in the ja surface form)

L1/L2 are handled by fix_long_vowel.py (run that first).
"""
from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATA_FILES = [ROOT / "data" / "questions_ja.json", ROOT / "data" / "questions_en.json"]

ASCII_VARIANT_RE = re.compile(r"[+()\[\]^]|\d")
WHITESPACE_RE = re.compile(r"[\s　]")


def _ja_has_ascii(ja: str) -> bool:
    return bool(re.search(r"[A-Za-z0-9+()\[\]^]", ja))


def is_english_variant(ja: str, typing: str) -> bool:
    """ASCII/English literal alt-typing forbidden by the IME-strict skill.

    True if ja is pure kana/kanji (or only contains ・ as separator) but
    typing contains digits/symbols or whitespace not present in ja.
    `・` in ja maps to `/` in typing per the IME-strict skill, so a typing
    with whitespace where ja has `・` is still a violation.
    """
    if _ja_has_ascii(ja):
        return False
    if ASCII_VARIANT_RE.search(typing):
        return True
    # Whitespace in typing is only OK if ja itself has whitespace. `・` in
    # ja must become `/` in typing — whitespace separators are not allowed.
    if WHITESPACE_RE.search(typing) and not WHITESPACE_RE.search(ja):
        return True
    return False


def fix_twu(ja: str, typing: str) -> tuple[str, bool]:
    """トゥ uses IME wapuro `twu` (or explicit-small ゥ as `toxu`/`tolu`).

    Bare `tu` types つ, so `tu-mureida-` is a wapuro mismatch for
    トゥームレイダー. We rewrite `tu` → `twu` only when ja contains トゥ.
    """
    if "トゥ" not in ja:
        return typing, False
    count = ja.count("トゥ")
    new = re.sub(r"(?<![a-z])tu", "twu", typing, count=count)
    return new, new != typing


def fix_t_d_kana(ja: str, typing: str) -> tuple[str, bool]:
    """Fix `ti`→`thi` (ティ), `di`→`dhi` (ディ).

    Conservative: only rewrite when ja contains the relevant katakana AND
    typing currently has the bare `ti`/`di` form. We don't touch `ti`
    sequences that are actually for ち (Hepburn `chi`) — those are
    K1-category and handled separately.
    """
    changed = False
    new = typing
    if "ティ" in ja:
        # Replace `ti` with `thi`, but only where it isn't already `thi`
        # and isn't part of a longer alphabetic identifier — we keep this
        # narrow because of overlaps with chi-as-`ti` kunrei.
        # Heuristic: ja must contain at least as many ティ occurrences as
        # the replacements we'd make.
        ti_count = ja.count("ティ")
        candidate = re.sub(r"ti(?!h)", "thi", new, count=ti_count)
        if candidate != new:
            new = candidate
            changed = True
    if "ディ" in ja:
        di_count = ja.count("ディ")
        candidate = re.sub(r"di(?!h)", "dhi", new, count=di_count)
        if candidate != new:
            new = candidate
            changed = True
    return new, changed


# Kunrei → Hepburn mapping. Apply only when the corresponding Hepburn
# kana is *present in ja*. Mapping uses negative lookbehind/lookahead so
# we don't accidentally touch substrings inside already-correct forms
# (e.g. `shi` won't be re-rewritten because it doesn't start with `si`).
KUNREI_RULES = [
    # (regex, replacement, ja_kana_required)
    (re.compile(r"(?<![a-z])si"), "shi", ("し", "シ")),
    (re.compile(r"(?<![a-z])ti"), "chi", ("ち", "チ")),
    (re.compile(r"(?<![a-z])tu"), "tsu", ("つ", "ツ")),
    (re.compile(r"(?<![a-z])hu"), "fu", ("ふ", "フ")),
    (re.compile(r"(?<![a-z])zi"), "ji", ("じ", "ジ")),
]


def fix_kunrei(ja: str, typing: str) -> tuple[str, bool]:
    new = typing
    changed = False
    for pat, replacement, kana_set in KUNREI_RULES:
        if not any(k in ja for k in kana_set):
            continue
        # Limit substitutions to the count of the kana in ja, to avoid
        # over-rewriting tokens that happened to start with `si` for an
        # unrelated reason.
        max_subs = sum(ja.count(k) for k in kana_set)
        candidate = pat.sub(replacement, new, count=max_subs)
        if candidate != new:
            new = candidate
            changed = True
    return new, changed


def process(data: list[dict]) -> tuple[int, int, int]:
    removed_variants = 0
    fixed_td = 0
    fixed_k = 0
    for q in data:
        for choice in q.get("choices", []):
            ja = choice.get("ja", "")
            typings = list(choice.get("ja_typings") or [])
            if not typings:
                continue
            # Drop A1/S1 variants when at least one proper wapuro form
            # remains.
            proper = [t for t in typings if not is_english_variant(ja, t)]
            if proper and len(proper) != len(typings):
                removed_variants += len(typings) - len(proper)
                typings = proper
            # Strip stray whitespace from a sole remaining typing: when ja
            # has no whitespace, the typing shouldn't either. Bulk imports
            # produced entries like `ruroun ikenshin` (only typing,
            # misplaced space) — collapse to `rurounikenshin`. We don't
            # touch typings tied to a space-containing ja or to a typing
            # that legitimately mirrors `・` as `/` (already separator-clean).
            if not WHITESPACE_RE.search(ja):
                for i, t in enumerate(typings):
                    if WHITESPACE_RE.search(t):
                        stripped = re.sub(r"[\s　]+", "", t)
                        if stripped:
                            typings[i] = stripped
            # T1/D1
            for i, t in enumerate(typings):
                new, c = fix_t_d_kana(ja, t)
                if c:
                    typings[i] = new
                    fixed_td += 1
            # K1
            for i, t in enumerate(typings):
                new, c = fix_kunrei(ja, t)
                if c:
                    typings[i] = new
                    fixed_k += 1
            # トゥ → twu
            for i, t in enumerate(typings):
                new, c = fix_twu(ja, t)
                if c:
                    typings[i] = new
                    fixed_k += 1
            choice["ja_typings"] = typings
    return removed_variants, fixed_td, fixed_k


def main() -> int:
    for path in DATA_FILES:
        with path.open(encoding="utf-8") as f:
            data = json.load(f)
        removed, td, k = process(data)
        path.write_text(
            json.dumps(data, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        print(f"{path.name}: removed {removed} English-variant typings, "
              f"fixed {td} ティ/ディ, fixed {k} kunrei")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

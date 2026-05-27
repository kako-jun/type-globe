#!/usr/bin/env python3
"""IME-strict ja_typings linter — bulk-import regression detector.

The Round 3/4/5 bulk imports (commits c84ac08, 6d5293d, b532885) added
~2400 questions and slipped multiple classes of IME-strict violations
past review. This script categorises all current violations so they can
be auto-fixed (where mechanical) or filed as Issues (where not).

Categories detected:

  L1  ー written as a doubled vowel  (rukusooru   → rukuso-ru)
  L2  ー dropped entirely             (akademia    → ...akademia missing -)
  K1  kunrei `si/ti/tu/hu/zi` used    (rukusooru-si... should be -shi-)
  T1  ティ written as `ti`             (gandii      → gandhi-)
  D1  ディ written as `di`             (aidi-       → adhi-)
  N1  ン+ナ行 not doubled (`n` not `nn`)
  N2  ン+ヤ行 not doubled
  N3  ン+母音 not doubled
  A1  ASCII / English-letter variant  (c++20, base64url, o(1), jinsei game)
  S1  全角 space inside typing
  S2  ・ retained as raw rather than `/`
  P1  `/` used as a bare separator (more `/` than the label's ・ + literal `/`)

Output: scripts/lint_report.json with per-violation entries.
"""
from __future__ import annotations

import json
import re
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATA_FILES = [ROOT / "data" / "questions_ja.json", ROOT / "data" / "questions_en.json"]
REPORT = ROOT / "scripts" / "lint_report.json"

# Hepburn-required forms (IME-strict): typing these via the listed kunrei
# spellings is forbidden by the skill's "Hepburn 許可 / kunrei 不許可" rule.
KUNREI_RE = re.compile(r"(?<![a-z])(si|ti|tu|hu|zi)(?![a-z])", re.I)

# `ti`/`di` adjacency to vowel that the ja side indicates are ティ/ディ.
# We can only flag from typing if the typing has `ti`/`di` and the ja has
# ティ/ディ visible in the surface form.
ASCII_RE = re.compile(r"[A-Za-z]")
# A typing is an "ASCII/English variant" if it contains characters that
# wapuro IME never emits — `+`, `(`, `)`, `^`, `[`, `]`, digits inside the
# romaji body (note: 1, 2, … are wapuro-legal only when typing 数字 keys
# during a non-IME pass, which doesn't happen mid-word).
ASCII_VARIANT_RE = re.compile(r"[+()\[\]^]|\d")

# Mojibake-prone characters that should not appear in a typing string.
FORBIDDEN_RE = re.compile(r"[・　、。「」『』]")


def _ja_has_ascii(ja: str) -> bool:
    return bool(re.search(r"[A-Za-z0-9+()\[\]^]", ja))


def _ja_is_pure_ascii(ja: str) -> bool:
    return bool(ja) and all(ord(c) < 128 for c in ja)


def categorise(ja: str, typing: str) -> list[str]:
    cats: list[str] = []
    ja_has_ascii = _ja_has_ascii(ja)
    # A1 — typing contains `+()[]^` or digits, while ja is pure kana/kanji
    # (no ASCII surface). This is the "English literal as alt-typing" rule
    # violation. When ja itself is `O(log n)` or `K2`, the typing
    # mirroring those chars is fine.
    if not ja_has_ascii and ASCII_VARIANT_RE.search(typing):
        cats.append("A1")
    # L1/L2 — long vowel ー not handled.
    if "ー" in ja and "-" not in typing:
        # Distinguish doubled-vowel vs dropped: presence of an obvious
        # doubled vowel near a kana position implies L1; otherwise L2.
        if re.search(r"(aa|ii|uu|ee|oo|ou|ei)", typing):
            cats.append("L1")
        else:
            cats.append("L2")
    # K1 — kunrei forms. Skip when ja is pure ASCII (the typing is a
    # direct echo of the label, not a kana→romaji rendering).
    if not _ja_is_pure_ascii(ja) and KUNREI_RE.search(typing):
        cats.append("K1")
    # T1 — ティ in ja but `ti` in typing (not the kunrei `ti`-for-chi which
    # would also trip K1; this catches the ティ-specific bug).
    if "ティ" in ja and re.search(r"ti(?!h)", typing):
        cats.append("T1")
    if "ディ" in ja and re.search(r"di(?!h)", typing):
        cats.append("D1")
    # N1/N2/N3 — left as future work. Reliable detection needs full
    # kana-typing alignment (the regex above produced too many false
    # positives because plain ナ行 syllables also match `n[aiueoy]`).
    # S1 — whitespace inside typing. Acceptable only when ja itself contains
    # whitespace (e.g. ja "O(log n)" with typing "o(log n)").
    if re.search(r"[\s　]", typing) and not re.search(r"[\s　]", ja):
        cats.append("S1")
    # S2 — raw ・ / fullwidth punct inside typing.
    if FORBIDDEN_RE.search(typing):
        cats.append("S2")
    # P1 — a `/` in the typing not justified by the label. The matcher
    # (src/io/normalize.rs::canonical_romaji) keeps `/` and matches it
    # positionally: `/` is the keystroke for ・, and a literal `/` in the
    # label types itself. A `/` used as a bare word separator (e.g.
    # `tokugawa/iemochi` for 徳川家茂, which has no ・) is unreachable — the
    # player typing the natural reading never presses `/`, so the answer
    # can never be completed. Allow at most as many `/` as the label has
    # ・ plus literal `/`.
    if typing.count("/") > ja.count("・") + ja.count("/"):
        cats.append("P1")
    return cats


def main() -> int:
    report: list[dict] = []
    for path in DATA_FILES:
        with path.open(encoding="utf-8") as f:
            data = json.load(f)
        for q in data:
            qid = q.get("id", "?")
            for cidx, choice in enumerate(q.get("choices", [])):
                ja = choice.get("ja", "")
                for tidx, t in enumerate(choice.get("ja_typings", []) or []):
                    cats = categorise(ja, t)
                    if not cats:
                        continue
                    report.append({
                        "file": path.name,
                        "question_id": qid,
                        "choice_index": cidx,
                        "typing_index": tidx,
                        "ja": ja,
                        "typing": t,
                        "categories": cats,
                    })
    REPORT.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    # Summary.
    counter: Counter[str] = Counter()
    for entry in report:
        for c in entry["categories"]:
            counter[c] += 1
    print(f"total flagged entries: {len(report)}")
    for cat, n in sorted(counter.items()):
        print(f"  {cat}: {n}")
    print(f"report: {REPORT.relative_to(ROOT)}")
    return 0 if not report else 1


if __name__ == "__main__":
    raise SystemExit(main())

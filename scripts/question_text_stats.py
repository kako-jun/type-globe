"""
questions_*.json の question_text の状態を集計する。

主な指標:
- total: 問題数
- has_reading: question_text_reading.ja が入っている問題数
- no_kanji: question_text.ja に漢字が一文字も無い
- hira_heavy: ひらがな比率 (>= --min-hiragana-ratio) かつ 漢字数 <= --max-kanji
- equal_to_reading: question_text.ja と question_text_reading.ja が一致

usage:
    uv run python3 scripts/question_text_stats.py data/questions_ja.json
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

KANJI_RE = re.compile(r"[一-鿿]")


def is_hiragana(c: str) -> bool:
    return "ぁ" <= c <= "ゟ"


def is_katakana(c: str) -> bool:
    return "ァ" <= c <= "ヿ"


def hira_ratio(s: str) -> float:
    h = sum(1 for c in s if is_hiragana(c))
    t = sum(1 for c in s if c.isalnum() or is_hiragana(c) or is_katakana(c) or KANJI_RE.match(c))
    return h / t if t else 0.0


def kanji_count(s: str) -> int:
    return len(KANJI_RE.findall(s))


def stats(path: Path, min_hira: float, max_kanji: int) -> None:
    data = json.loads(path.read_text(encoding="utf-8"))
    total = len(data)
    has_reading = 0
    no_kanji = 0
    hira_heavy = 0
    equal_to_reading = 0
    for q in data:
        ja = (q.get("question_text") or {}).get("ja", "")
        ja_reading = (q.get("question_text_reading") or {}).get("ja", "")
        if ja_reading:
            has_reading += 1
        if kanji_count(ja) == 0:
            no_kanji += 1
        if hira_ratio(ja) >= min_hira and kanji_count(ja) <= max_kanji:
            hira_heavy += 1
        if ja and ja == ja_reading:
            equal_to_reading += 1

    print(f"file: {path}")
    print(f"  total: {total}")
    print(f"  question_text_reading.ja present: {has_reading}")
    print(f"  no kanji in question_text.ja: {no_kanji}")
    print(f"  hira_ratio>={min_hira} & kanji<={max_kanji}: {hira_heavy}")
    print(f"  display == reading (ja): {equal_to_reading}")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("path", type=Path, nargs="+")
    ap.add_argument("--min-hiragana-ratio", type=float, default=0.6)
    ap.add_argument("--max-kanji", type=int, default=1)
    args = ap.parse_args()
    for p in args.path:
        stats(p, args.min_hiragana_ratio, args.max_kanji)
    return 0


if __name__ == "__main__":
    sys.exit(main())

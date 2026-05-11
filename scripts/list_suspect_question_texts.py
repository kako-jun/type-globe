"""
変換後にまだ「ひらがな寄りで漢字がほぼ無い」 question_text.ja を列挙する。

usage:
    uv run python3 scripts/list_suspect_question_texts.py data/questions_ja.json
    uv run python3 scripts/list_suspect_question_texts.py data/questions_ja.json --limit 100 --min-hiragana-ratio 0.7 --max-kanji 0
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


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("path", type=Path)
    ap.add_argument("--limit", type=int, default=50)
    ap.add_argument("--min-hiragana-ratio", type=float, default=0.6)
    ap.add_argument("--max-kanji", type=int, default=1)
    args = ap.parse_args()

    data = json.loads(args.path.read_text(encoding="utf-8"))
    suspects = []
    for q in data:
        ja = (q.get("question_text") or {}).get("ja", "")
        en = (q.get("question_text") or {}).get("en", "")
        kanji = len(KANJI_RE.findall(ja))
        ratio = hira_ratio(ja)
        if ratio >= args.min_hiragana_ratio and kanji <= args.max_kanji:
            suspects.append((q.get("id", "?"), ratio, kanji, ja, en))

    print(f"# suspect: {len(suspects)} (showing up to {args.limit})")
    for qid, ratio, kanji, ja, en in suspects[: args.limit]:
        print(f"{qid}\thira={ratio:.2f}\tkanji={kanji}\tja={ja!r}\ten={en!r}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

"""
question_text_reading をバックフィルする。

各 Question の question_text に登録されているすべての言語について、
question_text_reading にエントリが無ければ question_text の値をそのまま
コピーする。これにより:

- ja の現状ひらがな表記を「読み」として安全に退避できる
- restore_ja_question_texts_with_ollama.py で question_text.ja を漢字
  かな交じりへ書き換えても、読みは失われない

何度実行しても安全 (idempotent)。

usage:
    uv run python3 scripts/backfill_question_text_reading.py data/questions_ja.json
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def backfill(path: Path) -> tuple[int, int]:
    data = json.loads(path.read_text(encoding="utf-8"))
    added = 0
    touched = 0
    for q in data:
        qt = q.get("question_text") or {}
        qtr = q.get("question_text_reading") or {}
        any_added = False
        for lang, text in qt.items():
            if lang not in qtr:
                qtr[lang] = text
                added += 1
                any_added = True
        if any_added:
            q["question_text_reading"] = qtr
            touched += 1
    path.write_text(
        json.dumps(data, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    return added, touched


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("path", type=Path, nargs="+", help="questions_*.json files")
    args = ap.parse_args()
    for p in args.path:
        added, touched = backfill(p)
        print(f"{p}: added {added} reading entries across {touched} questions")
    return 0


if __name__ == "__main__":
    sys.exit(main())

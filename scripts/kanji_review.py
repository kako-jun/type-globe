#!/usr/bin/env python3
"""LLM-judge pipeline for kanji-bearing `ja_typings` (#134).

kana / ASCII choices are verified deterministically by the
`review-ja-typings` binary (#135). Kanji-bearing choices have ambiguous
readings (compounds, proper nouns, number readings) that a reading engine
like kakasi gets wrong ~95% of the time, so they need an LLM that can read
the kanji *in context* (the `en` answer disambiguates e.g. 陽子 = proton
`youshi` vs the name `youko`).

This script does the deterministic halves; an LLM does the judgement in
between:

    extract <scope>      -> manifest.json   (choices an LLM must judge)
      ... LLM produces verdicts.json ...
    apply <verdicts.json>                    (rewrite typings / set ja_reviewed)

`scope` is a genre name, a `q`-id prefix (e.g. `q013`), or `all`.

A question is set `ja_reviewed=true` only when every one of its
kanji-bearing choices has an `ok` (or successfully-applied `fix`) verdict.
After `apply`, always re-run `lint-questions`, `review-ja-typings verify`
and `scripts/lint_ja_typings.py` — a `fix` typing still has to pass the
IME-strict form gate.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
QUESTIONS_PATH = ROOT / "data" / "questions_ja.json"

KANJI_RE = re.compile(r"[一-鿿㐀-䶿]")


def has_kanji(s: str) -> bool:
    return bool(KANJI_RE.search(s))


def in_scope(q: dict, scope: str) -> bool:
    if scope == "all":
        return True
    if q.get("genre") == scope:
        return True
    qid = q.get("id", "")
    return scope.startswith("q") and qid.startswith(scope)


def load() -> list[dict]:
    with QUESTIONS_PATH.open(encoding="utf-8") as f:
        return json.load(f)


def extract(scope: str) -> int:
    data = load()
    manifest = []
    for q in data:
        if q.get("ja_reviewed", False) or not in_scope(q, scope):
            continue
        kanji_choices = [
            {
                "idx": i,
                "ja": c.get("ja", ""),
                "en": c.get("en", ""),
                "stored": c.get("ja_typings") or [],
            }
            for i, c in enumerate(q.get("choices", []))
            if has_kanji(c.get("ja", ""))
        ]
        if not kanji_choices:
            continue
        manifest.append(
            {
                "qid": q.get("id"),
                "genre": q.get("genre"),
                "question": q.get("question_text", {}).get("ja", ""),
                "kanji_choices": kanji_choices,
            }
        )
    json.dump(manifest, sys.stdout, ensure_ascii=False, indent=2)
    print()
    sys.stderr.write(
        f"manifest: {len(manifest)} questions, "
        f"{sum(len(m['kanji_choices']) for m in manifest)} kanji choices "
        f"(scope={scope})\n"
    )
    return 0


def apply(verdicts_path: str) -> int:
    with open(verdicts_path, encoding="utf-8") as f:
        verdicts = json.load(f)
    # (qid, idx) -> verdict dict
    table: dict[tuple[str, int], dict] = {}
    for v in verdicts:
        table[(v["qid"], int(v["idx"]))] = v

    data = load()
    confirmed = 0
    fixed = 0
    skipped_incomplete = 0
    skipped_wrong = 0

    for q in data:
        if q.get("ja_reviewed", False):
            continue
        choices = q.get("choices", [])
        kanji_idxs = [i for i, c in enumerate(choices) if has_kanji(c.get("ja", ""))]
        if not kanji_idxs:
            continue  # handled by review-ja-typings (#135)

        # Every kanji choice must have a verdict, and none may be "wrong"
        # (a "wrong" with no usable typing means the data is bad and a human
        # must look — never auto-confirm it).
        verdicts_here = [table.get((q["id"], i)) for i in kanji_idxs]
        if any(v is None for v in verdicts_here):
            skipped_incomplete += 1
            continue
        if any(v.get("verdict") == "wrong" for v in verdicts_here):
            skipped_wrong += 1
            continue

        # Apply fixes, then confirm.
        for i, v in zip(kanji_idxs, verdicts_here):
            if v.get("verdict") == "fix":
                typing = v.get("typing") or []
                if not typing:
                    skipped_wrong += 1
                    break
                choices[i]["ja_typings"] = list(typing)
                fixed += 1
        else:
            q["ja_reviewed"] = True
            confirmed += 1

    QUESTIONS_PATH.write_text(
        json.dumps(data, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    print(f"questions confirmed (ja_reviewed=true): {confirmed}")
    print(f"typings fixed                         : {fixed}")
    print(f"skipped (verdict missing for a choice): {skipped_incomplete}")
    print(f"skipped (a choice judged wrong)       : {skipped_wrong}")
    return 0


def main() -> int:
    if len(sys.argv) >= 3 and sys.argv[1] == "extract":
        return extract(sys.argv[2])
    if len(sys.argv) >= 3 and sys.argv[1] == "apply":
        return apply(sys.argv[2])
    sys.stderr.write(
        "usage:\n"
        "  kanji_review.py extract <genre|qid-prefix|all>   > manifest.json\n"
        "  kanji_review.py apply <verdicts.json>\n"
    )
    return 2


if __name__ == "__main__":
    raise SystemExit(main())

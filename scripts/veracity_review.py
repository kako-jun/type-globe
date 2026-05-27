#!/usr/bin/env python3
"""Veracity-review pipeline for question facts (follow-up to the 英じゃ find).

The bulk-generated questions contain hallucinated terms and wrong answers
(~6% in an 80-question sample, concentrated in net-culture / grammar). This
does the deterministic halves; a web-equipped LLM does the judgement:

    extract <genre|qid-prefix|all>  -> manifest.json
      ... LLM produces verdicts.json ...
    apply <verdicts.json>           (repoint wrong answers / remove fabrications)

Verdict per question (one object each):
- {"qid": "...", "verdict": "ok"}
- {"qid": "...", "verdict": "fix_answer", "correct": "<exact choice ja label>", "note": "..."}
- {"qid": "...", "verdict": "remove", "note": "..."}

`apply` edits BOTH questions_ja.json and questions_en.json identically (they
share choices + correct_answer_index), so the files stay in sync.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
JA = ROOT / "data" / "questions_ja.json"
EN = ROOT / "data" / "questions_en.json"


def in_scope(q: dict, scope: str) -> bool:
    if scope == "all":
        return True
    if q.get("genre") == scope:
        return True
    qid = q.get("id", "")
    return scope.startswith("q") and qid.startswith(scope)


def extract(scope: str) -> int:
    data = json.loads(JA.read_text(encoding="utf-8"))
    manifest = []
    for q in data:
        if not in_scope(q, scope):
            continue
        ci = q.get("correct_answer_index", 0)
        choices = [c.get("ja", "") for c in q.get("choices", [])]
        manifest.append(
            {
                "qid": q.get("id"),
                "genre": q.get("genre"),
                "q": q.get("question_text", {}).get("ja", ""),
                "q_en": q.get("question_text", {}).get("en", ""),
                "choices": choices,
                "correct": choices[ci] if ci < len(choices) else "?",
            }
        )
    json.dump(manifest, sys.stdout, ensure_ascii=False, indent=2)
    print()
    sys.stderr.write(f"manifest: {len(manifest)} questions (scope={scope})\n")
    return 0


def apply(verdicts_path: str) -> int:
    verdicts = {v["qid"]: v for v in json.loads(Path(verdicts_path).read_text(encoding="utf-8"))}

    removed = repointed = ok = skipped = bad = 0
    remove_ids = set()

    # First pass on the ja file to decide actions (choices are identical in en).
    ja = json.loads(JA.read_text(encoding="utf-8"))
    for q in ja:
        v = verdicts.get(q["id"])
        if v is None:
            skipped += 1
            continue
        verdict = str(v.get("verdict", "")).strip()
        if verdict == "ok":
            ok += 1
        elif verdict == "remove":
            remove_ids.add(q["id"])
            removed += 1
        elif verdict == "fix_answer":
            want = v.get("correct")
            idx = [i for i, c in enumerate(q.get("choices", [])) if c.get("ja") == want]
            if not idx:
                sys.stderr.write(
                    f"WARN {q['id']}: fix_answer target {want!r} not among choices "
                    f"{[c.get('ja') for c in q.get('choices', [])]} — left unchanged\n"
                )
                bad += 1
                continue
            q["correct_answer_index"] = idx[0]
            repointed += 1
        else:
            sys.stderr.write(f"WARN {q['id']}: unknown verdict {verdict!r} — skipped\n")
            bad += 1

    # Apply the same correct_answer_index + removals to en, keyed by id.
    repoint_idx = {q["id"]: q["correct_answer_index"] for q in ja}
    for path in (JA, EN):
        data = json.loads(path.read_text(encoding="utf-8"))
        data = [q for q in data if q["id"] not in remove_ids]
        for q in data:
            if q["id"] in repoint_idx:
                q["correct_answer_index"] = repoint_idx[q["id"]]
        path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    print(f"ok            : {ok}")
    print(f"answers fixed : {repointed}")
    print(f"removed       : {removed}")
    print(f"skipped (no verdict): {skipped}")
    if bad:
        print(f"bad verdicts ignored: {bad}")
    return 0


def main() -> int:
    if len(sys.argv) >= 3 and sys.argv[1] == "extract":
        return extract(sys.argv[2])
    if len(sys.argv) >= 3 and sys.argv[1] == "apply":
        return apply(sys.argv[2])
    sys.stderr.write(
        "usage:\n"
        "  veracity_review.py extract <genre|qid-prefix|all> > manifest.json\n"
        "  veracity_review.py apply <verdicts.json>\n"
    )
    return 2


if __name__ == "__main__":
    raise SystemExit(main())

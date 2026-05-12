"""Apply per-entry A/B/C decisions to data/questions_ja.json.

- A: replace ja_typings with expected_typings
- B: merge expected_typings into existing ja_typings (sorted, dedup)
- C: skip (no change)
"""
import json
from pathlib import Path
from decisions_part1 import DECISIONS_PART1
from decisions_part2 import DECISIONS_PART2
from decisions_part3 import DECISIONS_PART3

DECISIONS = {**DECISIONS_PART1, **DECISIONS_PART2, **DECISIONS_PART3}

def main():
    root = Path(__file__).resolve().parent.parent
    suspicious = json.loads((root / "scripts/suspicious_ja_typings.json").read_text())
    data_path = root / "data/questions_ja.json"
    questions = json.loads(data_path.read_text())
    by_qid = {q["id"]: q for q in questions}

    stats = {"A": 0, "B": 0, "C": 0, "missing": 0}
    missing = []
    for s in suspicious:
        key = (s["question_id"], s["choice_index"])
        if key not in DECISIONS:
            stats["missing"] += 1
            missing.append(key)
            continue
        action, _reason = DECISIONS[key]
        stats[action] += 1
        if action == "C":
            continue
        choice = by_qid[s["question_id"]]["choices"][s["choice_index"]]
        if action == "A":
            choice["ja_typings"] = sorted(set(s["expected_typings"]))
        elif action == "B":
            merged = set(choice["ja_typings"]) | set(s["expected_typings"])
            choice["ja_typings"] = sorted(merged)

    data_path.write_text(json.dumps(questions, ensure_ascii=False, indent=2) + "\n")
    print("stats:", stats)
    if missing:
        print("missing keys (first 10):", missing[:10])

if __name__ == "__main__":
    main()

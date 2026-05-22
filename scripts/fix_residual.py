#!/usr/bin/env python3
"""One-shot manual fixes for the residual IME-strict violations that the
automated passes (fix_long_vowel.py, fix_ime_strict.py) could not resolve
because they involve ASCII chars in `ja`, wrong-base romaji, or partial
coverage — see scripts/lint_report.json.

Operates on (question_id, ja) and applies the literal replacement table.
"""
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATA_FILES = [ROOT / "data" / "questions_ja.json", ROOT / "data" / "questions_en.json"]

# (qid, ja) → list of fixes, each (old_typing, new_typing | None).
# new_typing=None means drop the typing.
FIXES: dict[tuple[str, str], list[tuple[str, str | None]]] = {
    ("q0529", "オットー1世"): [("otto1sei", None)],  # ASCII variant duplicate
    ("q01481", "サルデーニャ島"): [("sardeenyatou", "sarude-nyatou")],
    ("q02069", "バスティーユ襲撃"): [("basthiyu/shuugeki", "basuthi-yu/shuugeki")],
    ("q01529", "Tu"): [("tu", None)],  # ja is single Latin letter; bare `tu` ambiguous
    ("q02453", "TFTP"): [("ti-efuti-pi-", "thi-efuthi-pi-")],
    ("q02472", "Bluetooth"): [("buru-tu-su", "buru-twu-su")],  # ブルートゥース uses トゥ = `twu`
    ("q02477", "HTTP"): [("etchi-ti-ti-pi-", "etchi-thi-thi-pi-")],
    ("q02477", "MQTT"): [("emukyu-ti-ti-", "emukyu-thi-thi-")],
    ("q02478", "TPU"): [("ti-pi-yu-", "thi-pi-yu-")],
    ("q02984", "ハリーポッター 魔法同盟"): [
        ("mahou doumei", None),
        ("mahoudoumei", "hari-potta-mahoudoumei"),
    ],
    ("q02992", "ハリーポッター 魔法同盟"): [
        ("mahou doumei", None),
        ("mahoudoumei", "hari-potta-mahoudoumei"),
    ],
    ("q03013", "ハリーポッター 魔法同盟"): [
        ("mahou doumei", None),
        ("mahoudoumei", "hari-potta-mahoudoumei"),
    ],
    ("q03203", "フェルマー"): [
        ("herumaa", "feruma-"),
        ("felumaa", None),
    ],
    ("q03207", "フェルマー"): [
        ("herumaa", "feruma-"),
        ("felumaa", None),
    ],
    ("q03216", "アンドリュー・ワイルズ"): [
        ("andolyuu/wailuzu", "andoryu-/wairuzu"),
    ],
}


def process(data: list[dict]) -> int:
    n = 0
    for q in data:
        qid = q.get("id", "")
        for choice in q.get("choices", []):
            ja = choice.get("ja", "")
            key = (qid, ja)
            if key not in FIXES:
                continue
            typings = list(choice.get("ja_typings") or [])
            for old, new in FIXES[key]:
                if old not in typings:
                    continue
                if new is None:
                    typings.remove(old)
                else:
                    typings[typings.index(old)] = new
                n += 1
            # Deduplicate while preserving order.
            seen: list[str] = []
            for t in typings:
                if t not in seen:
                    seen.append(t)
            choice["ja_typings"] = seen
    return n


def main() -> int:
    for path in DATA_FILES:
        with path.open(encoding="utf-8") as f:
            data = json.load(f)
        n = process(data)
        path.write_text(
            json.dumps(data, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        print(f"{path.name}: applied {n} residual fixes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

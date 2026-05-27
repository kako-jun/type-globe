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
EN_PATH = ROOT / "data" / "questions_en.json"

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


VALID_VERDICTS = {"ok", "fix", "wrong"}


def apply(verdicts_path: str) -> int:
    with open(verdicts_path, encoding="utf-8") as f:
        verdicts = json.load(f)
    # (qid, idx) -> verdict dict, with verdict normalised to lowercase.
    table: dict[tuple[str, int], dict] = {}
    malformed = 0
    for v in verdicts:
        try:
            key = (v["qid"], int(v["idx"]))
        except (KeyError, ValueError, TypeError):
            malformed += 1
            continue
        v = dict(v)
        v["verdict"] = str(v.get("verdict", "")).strip().lower()
        table[key] = v

    data = load()
    confirmed = 0
    fixed = 0
    skipped_incomplete = 0
    skipped_problem = 0

    for q in data:
        if q.get("ja_reviewed", False):
            continue
        choices = q.get("choices", [])
        kanji_idxs = [i for i, c in enumerate(choices) if has_kanji(c.get("ja", ""))]
        if not kanji_idxs:
            continue  # handled by review-ja-typings (#135)

        verdicts_here = [table.get((q["id"], i)) for i in kanji_idxs]

        # Every kanji choice must have an *understood* verdict. A missing
        # verdict or an unrecognised value (typo, wrong case, hand-edit) is
        # treated as not-yet-judged — never silently confirmed.
        if any(v is None or v["verdict"] not in VALID_VERDICTS for v in verdicts_here):
            skipped_incomplete += 1
            continue
        # A "wrong" verdict means a human must look — never auto-confirm.
        if any(v["verdict"] == "wrong" for v in verdicts_here):
            skipped_problem += 1
            continue

        # Stage fixes first; only commit them once we know the whole question
        # is confirmable. This keeps a skipped question completely untouched
        # rather than leaving a half-applied typing behind.
        pending_fixes: list[tuple[int, list[str]]] = []
        bad = False
        for i, v in zip(kanji_idxs, verdicts_here):
            if v["verdict"] == "fix":
                typing = v.get("typing") or []
                if not typing or not all(isinstance(t, str) for t in typing):
                    bad = True
                    break
                pending_fixes.append((i, list(typing)))
        if bad:
            skipped_problem += 1
            continue

        for i, typing in pending_fixes:
            choices[i]["ja_typings"] = typing
            fixed += 1
        q["ja_reviewed"] = True
        confirmed += 1

    QUESTIONS_PATH.write_text(
        json.dumps(data, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    print(f"questions confirmed (ja_reviewed=true): {confirmed}")
    print(f"typings fixed                         : {fixed}")
    print(f"skipped (verdict missing/unknown)     : {skipped_incomplete}")
    print(f"skipped (wrong / unusable fix)        : {skipped_problem}")
    if malformed:
        print(f"malformed verdict entries ignored     : {malformed}")
    return 0


def _strip_spurious_slashes(t: str) -> str:
    """Drop bare `/` separators, mirroring `canonical_romaji`'s handling so
    the result types the same kana. A lone `n` before `/`+(vowel/n/y) doubles
    to `nn` (`kan/no` -> `kanno`-with-the-extra-n); an already-doubled `nn/`
    is protected first so it never becomes `nnn` (matches the matcher's
    sentinel trick in normalize.rs)."""
    sentinel = "\x01"
    t = t.replace("nn/", sentinel)
    t = re.sub(r"n/([aiueoyn])", r"nn\1", t)
    t = t.replace(sentinel, "nn")
    return t.replace("/", "")


def fix_spurious_slashes() -> int:
    """Remove `/` used as a bare word separator (lint rule P1).

    The matcher (src/io/normalize.rs::canonical_romaji) keeps `/` and matches
    it positionally — it is the keystroke for ・. A `/` with no ・ (or literal
    `/`) in the label is unreachable, so blind-typing the reading can never
    complete the answer. We strip such `/`, re-doubling a preceding lone `n`
    to `nn` when it now lands before a vowel / `n` / `y` (e.g.
    `kan/no/butei` -> `kannnobutei`), which is exactly what canonical_romaji
    would have done at the `/`. Legitimate `/` (HTTP/1.1, らんま1/2) is left
    untouched because the label carries a matching ・ or literal `/`.
    """
    data = load()
    fixed = 0
    for q in data:
        for c in q.get("choices", []):
            ja = c.get("ja", "")
            allowed = ja.count("・") + ja.count("/")
            typings = c.get("ja_typings") or []
            new_typings = []
            for t in typings:
                # Only auto-fix when the label justifies *no* slash at all
                # (allowed == 0): then every `/` is spurious and safe to drop.
                # If the label has a ・/literal `/` but the typing has *more*
                # slashes than that, which to keep is ambiguous — leave it for
                # a human (P1 lint still flags it) rather than risk stripping a
                # legitimate ・ keystroke.
                if allowed == 0 and "/" in t:
                    t2 = _strip_spurious_slashes(t)
                    if t2 != t:
                        fixed += 1
                    new_typings.append(t2)
                else:
                    new_typings.append(t)
            if new_typings != typings:
                c["ja_typings"] = new_typings
    QUESTIONS_PATH.write_text(
        json.dumps(data, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    print(f"spurious-slash typings fixed: {fixed}")
    return 0


def _choices_aligned(a: list[dict], b: list[dict]) -> bool:
    """True if both files hold the same questions and choice labels in the
    same order — the precondition for copying choice data positionally."""
    if [q.get("id") for q in a] != [q.get("id") for q in b]:
        return False
    for qa, qb in zip(a, b):
        ca, cb = qa.get("choices", []), qb.get("choices", [])
        if [c.get("ja") for c in ca] != [c.get("ja") for c in cb]:
            return False
        if [c.get("en") for c in ca] != [c.get("en") for c in cb]:
            return False
    return True


def sync_en() -> int:
    """Copy the language-independent choice data (`ja_typings`) and
    `ja_reviewed` from questions_ja.json into questions_en.json.

    The two files hold the same questions and choices; only `question_text`
    differs by language. The review pipeline (#134/#135) only edits the ja
    file, so this propagates the result to the en file. Positional copy is
    safe only when the choices align, which we assert first.
    """
    ja = load()
    with EN_PATH.open(encoding="utf-8") as f:
        en = json.load(f)
    if not _choices_aligned(ja, en):
        sys.stderr.write("ja/en questions or choice labels are not aligned; aborting\n")
        return 1
    changed = 0
    for qa, qe in zip(ja, en):
        if qe.get("ja_reviewed") != qa.get("ja_reviewed", False):
            qe["ja_reviewed"] = qa.get("ja_reviewed", False)
            changed += 1
        for ca, ce in zip(qa["choices"], qe["choices"]):
            if ce.get("ja_typings") != (ca.get("ja_typings") or []):
                ce["ja_typings"] = ca.get("ja_typings") or []
                changed += 1
    EN_PATH.write_text(
        json.dumps(en, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    print(f"en fields synced from ja: {changed}")
    return 0


def verify_sync() -> int:
    """CI gate: the two files' choice data (`ja_typings`) and `ja_reviewed`
    must be identical, so a review applied to one is never lost on the other.
    """
    with QUESTIONS_PATH.open(encoding="utf-8") as f:
        ja = json.load(f)
    with EN_PATH.open(encoding="utf-8") as f:
        en = json.load(f)
    if not _choices_aligned(ja, en):
        print("ja/en questions or choice labels are NOT aligned")
        return 1
    problems = []
    for qa, qe in zip(ja, en):
        if qa.get("ja_reviewed", False) != qe.get("ja_reviewed", False):
            problems.append(f"{qa['id']}: ja_reviewed differs")
        for i, (ca, ce) in enumerate(zip(qa["choices"], qe["choices"])):
            if ca.get("ja_typings") != ce.get("ja_typings"):
                problems.append(f"{qa['id']}#{i}: ja_typings differs")
    print(f"ja/en choice-data sync problems: {len(problems)}")
    for p in problems[:20]:
        print(f"  {p}")
    return 0 if not problems else 1


def main() -> int:
    if len(sys.argv) >= 2 and sys.argv[1] == "sync-en":
        return sync_en()
    if len(sys.argv) >= 2 and sys.argv[1] == "verify-sync":
        return verify_sync()
    if len(sys.argv) >= 3 and sys.argv[1] == "extract":
        return extract(sys.argv[2])
    if len(sys.argv) >= 3 and sys.argv[1] == "apply":
        return apply(sys.argv[2])
    if len(sys.argv) >= 2 and sys.argv[1] == "fix-slashes":
        return fix_spurious_slashes()
    sys.stderr.write(
        "usage:\n"
        "  kanji_review.py extract <genre|qid-prefix|all>   > manifest.json\n"
        "  kanji_review.py apply <verdicts.json>\n"
        "  kanji_review.py fix-slashes\n"
        "  kanji_review.py sync-en       # copy ja_typings + ja_reviewed ja -> en\n"
        "  kanji_review.py verify-sync   # CI gate: ja/en choice data identical\n"
    )
    return 2


if __name__ == "__main__":
    raise SystemExit(main())

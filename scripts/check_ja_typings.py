#!/usr/bin/env python3
"""Detect mismatched ja_typings entries in data/questions_ja.json.

For each choice whose `ja` contains kanji, we read it with pykakasi and
convert the resulting hiragana into Hepburn-style romaji using the same
rules as src/io/romaji.rs (Hepburn `shi/chi/tsu/fu/ji`, long `o`
collapse, `ん` -> `n`). If none of the registered `ja_typings` matches
either the collapsed or expanded expected romaji as an exact or prefix
match, the entry is flagged as suspicious.

Output:
- scripts/suspicious_ja_typings.json (list of flagged entries)
- summary on stdout
"""
from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Iterable

import pykakasi

ROOT = Path(__file__).resolve().parent.parent
QUESTIONS_PATH = ROOT / "data" / "questions_ja.json"
REPORT_PATH = ROOT / "scripts" / "suspicious_ja_typings.json"

KANJI_RE = re.compile(r"[一-鿿㐀-䶿]")

SEPARATOR_CHARS = set("・･（）()[]{}「」『』、。,.:;!?")
PRESERVE_ASCII = set(".|/")  # romaji.rs preserves '.' and '/'

PAIR_TABLE = {
    ("き", "ゃ"): "kya", ("き", "ゅ"): "kyu", ("き", "ょ"): "kyo",
    ("ぎ", "ゃ"): "gya", ("ぎ", "ゅ"): "gyu", ("ぎ", "ょ"): "gyo",
    ("し", "ゃ"): "sha", ("し", "ゅ"): "shu", ("し", "ょ"): "sho",
    ("じ", "ゃ"): "ja", ("じ", "ゅ"): "ju", ("じ", "ょ"): "jo",
    ("ち", "ゃ"): "cha", ("ち", "ゅ"): "chu", ("ち", "ょ"): "cho",
    ("ぢ", "ゃ"): "ja", ("ぢ", "ゅ"): "ju", ("ぢ", "ょ"): "jo",
    ("に", "ゃ"): "nya", ("に", "ゅ"): "nyu", ("に", "ょ"): "nyo",
    ("ひ", "ゃ"): "hya", ("ひ", "ゅ"): "hyu", ("ひ", "ょ"): "hyo",
    ("び", "ゃ"): "bya", ("び", "ゅ"): "byu", ("び", "ょ"): "byo",
    ("ぴ", "ゃ"): "pya", ("ぴ", "ゅ"): "pyu", ("ぴ", "ょ"): "pyo",
    ("み", "ゃ"): "mya", ("み", "ゅ"): "myu", ("み", "ょ"): "myo",
    ("り", "ゃ"): "rya", ("り", "ゅ"): "ryu", ("り", "ょ"): "ryo",
    ("ふ", "ぁ"): "fa", ("ふ", "ぃ"): "fi", ("ふ", "ぇ"): "fe", ("ふ", "ぉ"): "fo",
    ("て", "ぃ"): "ti", ("で", "ぃ"): "di",
    ("と", "ぅ"): "tu", ("ど", "ぅ"): "du",
    ("し", "ぇ"): "she", ("じ", "ぇ"): "je", ("ち", "ぇ"): "che",
    ("つ", "ぁ"): "tsa", ("つ", "ぃ"): "tsi", ("つ", "ぇ"): "tse", ("つ", "ぉ"): "tso",
    ("う", "ぁ"): "wa", ("う", "ぃ"): "wi", ("う", "ぇ"): "we", ("う", "ぉ"): "wo",
    ("ゔ", "ぁ"): "va", ("ゔ", "ぃ"): "vi", ("ゔ", "ぇ"): "ve", ("ゔ", "ぉ"): "vo",
    ("い", "ぇ"): "ye",
}

SINGLE_TABLE = {
    "あ": "a", "ぁ": "a", "い": "i", "ぃ": "i", "う": "u", "ぅ": "u",
    "え": "e", "ぇ": "e", "お": "o", "ぉ": "o",
    "か": "ka", "き": "ki", "く": "ku", "け": "ke", "こ": "ko",
    "が": "ga", "ぎ": "gi", "ぐ": "gu", "げ": "ge", "ご": "go",
    "さ": "sa", "し": "shi", "す": "su", "せ": "se", "そ": "so",
    "ざ": "za", "じ": "ji", "ず": "zu", "ぜ": "ze", "ぞ": "zo",
    "た": "ta", "ち": "chi", "つ": "tsu", "て": "te", "と": "to",
    "だ": "da", "ぢ": "ji", "づ": "zu", "で": "de", "ど": "do",
    "な": "na", "に": "ni", "ぬ": "nu", "ね": "ne", "の": "no",
    "は": "ha", "ひ": "hi", "ふ": "fu", "へ": "he", "ほ": "ho",
    "ば": "ba", "び": "bi", "ぶ": "bu", "べ": "be", "ぼ": "bo",
    "ぱ": "pa", "ぴ": "pi", "ぷ": "pu", "ぺ": "pe", "ぽ": "po",
    "ま": "ma", "み": "mi", "む": "mu", "め": "me", "も": "mo",
    "や": "ya", "ゃ": "ya", "ゆ": "yu", "ゅ": "yu", "よ": "yo", "ょ": "yo",
    "ら": "ra", "り": "ri", "る": "ru", "れ": "re", "ろ": "ro",
    "わ": "wa", "を": "o", "ん": "n", "ゔ": "vu",
}


def _normalize_kana(ch: str) -> str:
    code = ord(ch)
    if 0x30A1 <= code <= 0x30F6:  # katakana -> hiragana
        return chr(code - 0x60)
    return ch


def _geminate_prefix(roman: str) -> str | None:
    if roman.startswith("ch"):
        return "t"
    if roman:
        c = roman[0]
        if c.isalpha() and c not in "aiueo":
            return c
    return None


def _hiragana_to_hepburn_raw(text: str) -> str:
    chars = [_normalize_kana(c) for c in text]
    out: list[str] = []
    i = 0
    geminate = False
    n = len(chars)
    while i < n:
        c = chars[i]
        if c.isspace() or c == "　":
            if out and out[-1] != " ":
                out.append(" ")
            geminate = False
            i += 1
            continue
        if c == "ー":
            i += 1
            continue
        if c == "っ":
            geminate = True
            i += 1
            continue
        if c in {".", "/"}:
            out.append(c)
            geminate = False
            i += 1
            continue
        if c in SEPARATOR_CHARS:
            if out and out[-1] != " ":
                out.append(" ")
            geminate = False
            i += 1
            continue
        if c.isascii() and c.isalnum():
            out.append(c.lower())
            geminate = False
            i += 1
            continue
        # pair
        if i + 1 < n and (c, chars[i + 1]) in PAIR_TABLE:
            roman = PAIR_TABLE[(c, chars[i + 1])]
            if geminate:
                pre = _geminate_prefix(roman)
                if pre:
                    out.append(pre)
                geminate = False
            out.append(roman)
            i += 2
            continue
        if c in SINGLE_TABLE:
            roman = SINGLE_TABLE[c]
            if geminate:
                pre = _geminate_prefix(roman)
                if pre:
                    out.append(pre)
                geminate = False
            out.append(roman)
            i += 1
            continue
        # unknown
        geminate = False
        i += 1

    joined = "".join(out)
    return " ".join(joined.split())


def _collapse_long_o(s: str) -> str:
    out: list[str] = []
    i = 0
    n = len(s)
    while i < n:
        c = s[i]
        out.append(c)
        if c == "o" and i + 1 < n and s[i + 1] in ("o", "u"):
            i += 2
            continue
        i += 1
    return "".join(out)


def hepburn_variants(hira: str) -> list[str]:
    raw = _hiragana_to_hepburn_raw(hira)
    if not raw:
        return []
    collapsed = _collapse_long_o(raw)
    if raw == collapsed:
        return [collapsed]
    return [collapsed, raw]


_kks = pykakasi.kakasi()


def reading_of(ja: str) -> str:
    parts = _kks.convert(ja)
    return "".join(part["hira"] for part in parts)


def has_kanji(s: str) -> bool:
    return bool(KANJI_RE.search(s))


def _normalize_for_match(s: str) -> str:
    # ja_typings registered by humans may include hyphens / spaces; strip them for matching.
    return re.sub(r"[\s\-_]+", "", s.lower())


def matches_any(expected_variants: Iterable[str], actual: Iterable[str]) -> bool:
    norm_actual = [_normalize_for_match(a) for a in actual]
    for variant in expected_variants:
        ev = _normalize_for_match(variant)
        if not ev:
            continue
        for a in norm_actual:
            if not a:
                continue
            if a == ev or a.startswith(ev) or ev.startswith(a):
                return True
    return False


def main() -> int:
    with QUESTIONS_PATH.open(encoding="utf-8") as f:
        data = json.load(f)

    suspicious: list[dict] = []
    checked = 0
    skipped_no_kanji = 0

    for q in data:
        qid = q.get("id", "?")
        for idx, choice in enumerate(q.get("choices", [])):
            ja = choice.get("ja", "")
            typings = choice.get("ja_typings") or []
            if not ja or not has_kanji(ja):
                skipped_no_kanji += 1
                continue
            hira = reading_of(ja)
            expected = hepburn_variants(hira)
            if not expected:
                continue
            checked += 1
            if matches_any(expected, typings):
                continue
            suspicious.append({
                "question_id": qid,
                "choice_index": idx,
                "ja": ja,
                "ja_reading_hira": hira,
                "expected_typings": expected,
                "actual_typings": list(typings),
                "reason": "no ja_typings matches the kakasi-derived Hepburn reading (exact/prefix)",
            })

    REPORT_PATH.write_text(
        json.dumps(suspicious, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )

    print(f"checked kanji-bearing choices: {checked}")
    print(f"suspicious entries: {len(suspicious)}")
    print(f"report: {REPORT_PATH.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

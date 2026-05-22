#!/usr/bin/env python3
"""Fix ja_typings entries where katakana ー was written as a doubled vowel.

IME-wapuro strict spec (CLAUDE.md / type-globe-ja-typing skill) requires
the katakana long-vowel mark ー to be typed with the `-` key. Bulk
imports in earlier sessions registered the long vowel as a duplicated
vowel (`rukusooru` instead of `rukuso-ru`), and the existing
check_ja_typings.py stripped `-` during normalization so the bad form
passed through.

Strategy: for each (ja, typing) pair where `ja` contains ー, align ja
kana with the typing string and, at every ー position in ja, if the
corresponding character in the typing is a vowel that matches the
preceding character's last vowel, replace it with `-`.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Minimal IME-wapuro single-kana table (Hepburn allowed: shi/chi/tsu/fu/ji).
# Includes katakana and hiragana via katakana→hiragana normalization.
SINGLE = {
    "あ": "a", "い": "i", "う": "u", "え": "e", "お": "o",
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
    "や": "ya", "ゆ": "yu", "よ": "yo",
    "ら": "ra", "り": "ri", "る": "ru", "れ": "re", "ろ": "ro",
    "わ": "wa", "を": "wo", "ん": "n", "ゔ": "vu",
}
PAIR = {
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
    ("し", "ぇ"): "she", ("じ", "ぇ"): "je", ("ち", "ぇ"): "che",
    ("つ", "ぁ"): "tsa", ("つ", "ぃ"): "tsi", ("つ", "ぇ"): "tse", ("つ", "ぉ"): "tso",
    ("ゔ", "ぁ"): "va", ("ゔ", "ぃ"): "vi", ("ゔ", "ぇ"): "ve", ("ゔ", "ぉ"): "vo",
    ("て", "ぃ"): "thi", ("で", "ぃ"): "dhi",
}


def to_hira(ch: str) -> str:
    code = ord(ch)
    if 0x30A1 <= code <= 0x30F6:
        return chr(code - 0x60)
    return ch


def kana_romaji(c1: str, c2: str | None) -> tuple[str | None, int]:
    """Return (romaji, kana_chars_consumed) for c1 or c1+c2, or (None, 0)."""
    h1 = to_hira(c1)
    if c2 is not None:
        h2 = to_hira(c2)
        if (h1, h2) in PAIR:
            return PAIR[(h1, h2)], 2
    if h1 in SINGLE:
        return SINGLE[h1], 1
    return None, 0


def fix_typing(ja: str, typing: str) -> tuple[str, bool]:
    """Walk ja and typing in parallel; replace doubled vowel with '-' at each ー.

    Returns (new_typing, changed).
    """
    if "ー" not in ja:
        return typing, False
    new = list(typing)
    ti = 0  # cursor into typing
    ji = 0
    last_vowel: str | None = None
    changed = False
    while ji < len(ja):
        if ti >= len(new) and ja[ji] != "ー":
            break
        ch = ja[ji]
        if ch == "ー":
            # End-of-typing: ー was dropped at the tail (e.g. 'rangure' for
            # ラングレー). Append '-'.
            if ti >= len(new):
                new.append("-")
                changed = True
                ji += 1
                last_vowel = None
                continue
            # Expect '-' at current typing cursor. Several forms to normalize:
            #   ・new[ti] == '-' → already correct.
            #   ・new[ti] matches last_vowel (e.g. 'rukusoo' for ルクソー) → replace.
            #   ・last_vowel == 'o' and new[ti] == 'u' (e.g. 'sou' for ソー)  → replace.
            #   ・last_vowel == 'e' and new[ti] == 'i' (e.g. 'kei' for ケー)  → replace.
            #   ・new[ti] is start of next kana's romaji (ー dropped entirely) → insert '-'.
            if ti < len(new):
                cur = new[ti].lower()
                if cur == "-":
                    ti += 1
                elif last_vowel is not None and (
                    cur == last_vowel
                    or (last_vowel == "o" and cur == "u")
                    or (last_vowel == "e" and cur == "i")
                ):
                    new[ti] = "-"
                    changed = True
                    ti += 1
                else:
                    # Try insertion: peek next kana in ja and see if its romaji
                    # starts at ti. If so, ー was dropped — insert '-'.
                    look = ji + 1
                    inserted = False
                    while look < len(ja):
                        lc = ja[look]
                        if lc == "ー":
                            break
                        lc2 = ja[look + 1] if look + 1 < len(ja) else None
                        lr, _ = kana_romaji(lc, lc2)
                        if lr is not None:
                            alts = [lr] + HEPBURN_ALT.get(lr, [])
                            if any(
                                "".join(new[ti : ti + len(a)]).lower() == a
                                for a in alts
                            ):
                                new.insert(ti, "-")
                                changed = True
                                ti += 1
                                inserted = True
                            break
                        look += 1
                    if not inserted:
                        return typing, False
            ji += 1
            last_vowel = None
            continue
        # Try kana lookup.
        c2 = ja[ji + 1] if ji + 1 < len(ja) else None
        roman, consumed = kana_romaji(ch, c2)
        if roman is None:
            # Non-kana (kanji, ASCII, punct). Try to advance typing by matching
            # contiguous typing chars until we re-sync on the next kana or end.
            # Heuristic: find the next kana in ja and the next matching point
            # in typing by skipping kanji-equivalent chars in typing.
            # Easiest: skip ji forward by 1 and don't advance ti; if we hit
            # another kana, we'll try to find its romaji in typing.
            # But for stable alignment we look ahead in ja for the next kana
            # and find its romaji prefix in typing from ti onwards.
            ji += 1
            # Try to advance ti past whatever corresponds to this non-kana
            # by detecting the next kana's romaji.
            # Find next kana char(s).
            look = ji
            while look < len(ja):
                lc = ja[look]
                if lc == "ー":
                    break
                lc2 = ja[look + 1] if look + 1 < len(ja) else None
                lr, lcons = kana_romaji(lc, lc2)
                if lr is not None:
                    # Find lr in typing from ti onwards.
                    idx = typing.find(lr, ti)
                    if idx == -1:
                        # Try alternative spellings for Hepburn ↔ kunrei.
                        alts = HEPBURN_ALT.get(lr, [])
                        for alt in alts:
                            idx = typing.find(alt, ti)
                            if idx != -1:
                                break
                    if idx != -1:
                        ti = idx
                    # else: leave ti as-is; next iter may still align.
                    break
                look += 1
            continue
        # Match romaji against typing at ti.
        if typing[ti : ti + len(roman)].lower() == roman:
            ti += len(roman)
        else:
            # Try Hepburn alternatives.
            matched = False
            for alt in HEPBURN_ALT.get(roman, []):
                if typing[ti : ti + len(alt)].lower() == alt:
                    ti += len(alt)
                    matched = True
                    break
            if not matched:
                # Skip one char in typing and retry this kana once.
                ti += 1
                continue
        # Track last vowel.
        for v in reversed(roman):
            if v in "aiueo":
                last_vowel = v
                break
        ji += consumed
    return "".join(new), changed


HEPBURN_ALT = {
    "shi": ["si"], "chi": ["ti"], "tsu": ["tu"], "fu": ["hu"], "ji": ["zi"],
    "sha": ["sya"], "shu": ["syu"], "sho": ["syo"],
    "cha": ["tya"], "chu": ["tyu"], "cho": ["tyo"],
    "ja": ["zya"], "ju": ["zyu"], "jo": ["zyo"],
    "she": ["sye"], "je": ["zye"], "che": ["tye"],
    # IME-strict requires `thi`/`dhi` for ティ/ディ, but legacy data often
    # used kunrei `ti`/`di`. Accept both for alignment so we can still
    # repair the ー at the tail. The kunrei→IME-strict fix is a separate
    # cleanup pass.
    "thi": ["ti", "texi", "teli"],
    "dhi": ["di", "dexi", "deli"],
    # Yoon IME alt-paths (small-ya): kya ≡ kixya/kilya, etc.
    "kya": ["kixya", "kilya"], "kyu": ["kixyu", "kilyu"], "kyo": ["kixyo", "kilyo"],
    "gya": ["gixya", "gilya"], "gyu": ["gixyu", "gilyu"], "gyo": ["gixyo", "gilyo"],
    "sya": ["sixya", "silya"], "syu": ["sixyu", "silyu"], "syo": ["sixyo", "silyo"],
    "tya": ["tixya", "tilya"], "tyu": ["tixyu", "tilyu"], "tyo": ["tixyo", "tilyo"],
    "nya": ["nixya", "nilya"], "nyu": ["nixyu", "nilyu"], "nyo": ["nixyo", "nilyo"],
    "hya": ["hixya", "hilya"], "hyu": ["hixyu", "hilyu"], "hyo": ["hixyo", "hilyo"],
    "mya": ["mixya", "milya"], "myu": ["mixyu", "milyu"], "myo": ["mixyo", "milyo"],
    "rya": ["rixya", "rilya"], "ryu": ["rixyu", "rilyu", "riyu"], "ryo": ["rixyo", "rilyo"],
    "bya": ["bixya", "bilya"], "byu": ["bixyu", "bilyu"], "byo": ["bixyo", "bilyo"],
    "pya": ["pixya", "pilya"], "pyu": ["pixyu", "pilyu"], "pyo": ["pixyo", "pilyo"],
}


def process_file(path: Path) -> int:
    with path.open(encoding="utf-8") as f:
        data = json.load(f)
    total_fixed = 0
    for q in data:
        for choice in q.get("choices", []):
            ja = choice.get("ja", "")
            if "ー" not in ja:
                continue
            typings = choice.get("ja_typings") or []
            new_typings = []
            for t in typings:
                new_t, changed = fix_typing(ja, t)
                if changed:
                    total_fixed += 1
                new_typings.append(new_t)
            choice["ja_typings"] = new_typings
    text = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
    path.write_text(text, encoding="utf-8")
    return total_fixed


def main() -> int:
    files = [
        ROOT / "data" / "questions_ja.json",
        ROOT / "data" / "questions_en.json",
    ]
    for f in files:
        n = process_file(f)
        print(f"{f.relative_to(ROOT)}: fixed {n} typings")
    return 0


if __name__ == "__main__":
    sys.exit(main())

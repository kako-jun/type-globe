#!/usr/bin/env python3
"""Insert `-` into ja_typings at positions corresponding to ー in source ja.

We trust the human-curated ja_typings as authoritative (we do NOT use
pykakasi to determine readings). Each ー in `ja` adds exactly one `-`
to the strict romaji at the position right after the prior kana mora's
romaji.

Algorithm per (ja, typing):
  walk ja char by char; maintain pointer i into typing.
  - katakana / hiragana mora → consume known romaji from typing
  - ー → emit `-` (no advance in typing)
  - kanji / unknown → look ahead to next known kana segment; the romaji
    of the kanji block is whatever sits in typing between i and the
    location where the next kana's romaji begins
  - ASCII / separator → pass through

If alignment fails (typing romaji doesn't match expected kana romaji),
we leave the entry unchanged and warn.

Run from repo root:
  python3 scripts/migration/insert_dash_for_chouon.py
  (writes back to data/questions_ja.json, prints summary)
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
DATA = ROOT / "data" / "questions_ja.json"

# Hiragana / katakana mora -> hepburn romaji.
# Yo-on (small ャ ュ ョ) is handled via 2-char lookup taking precedence.
DIGRAPH = {
    "きゃ": "kya", "きゅ": "kyu", "きょ": "kyo",
    "ぎゃ": "gya", "ぎゅ": "gyu", "ぎょ": "gyo",
    "しゃ": "sha", "しゅ": "shu", "しょ": "sho",
    "じゃ": "ja", "じゅ": "ju", "じょ": "jo",
    "ちゃ": "cha", "ちゅ": "chu", "ちょ": "cho",
    "にゃ": "nya", "にゅ": "nyu", "にょ": "nyo",
    "ひゃ": "hya", "ひゅ": "hyu", "ひょ": "hyo",
    "びゃ": "bya", "びゅ": "byu", "びょ": "byo",
    "ぴゃ": "pya", "ぴゅ": "pyu", "ぴょ": "pyo",
    "みゃ": "mya", "みゅ": "myu", "みょ": "myo",
    "りゃ": "rya", "りゅ": "ryu", "りょ": "ryo",
    "ふぁ": "fa", "ふぃ": "fi", "ふぇ": "fe", "ふぉ": "fo", "ふゅ": "fyu",
    "うぁ": "wa", "うぃ": "wi", "うぇ": "we", "うぉ": "wo",
    "ヴぁ": "va", "ヴぃ": "vi", "ヴ": "vu", "ヴぇ": "ve", "ヴぉ": "vo",
    "てぃ": "thi", "でぃ": "di", "とぅ": "tu", "どぅ": "du",
    "ちぇ": "che", "しぇ": "she", "じぇ": "je", "つぁ": "tsa",
    "つぃ": "tsi", "つぇ": "tse", "つぉ": "tso",
    "いぇ": "ye",
}

MONO = {
    # gojuuon
    "あ": "a", "い": "i", "う": "u", "え": "e", "お": "o",
    "か": "ka", "き": "ki", "く": "ku", "け": "ke", "こ": "ko",
    "さ": "sa", "し": "shi", "す": "su", "せ": "se", "そ": "so",
    "た": "ta", "ち": "chi", "つ": "tsu", "て": "te", "と": "to",
    "な": "na", "に": "ni", "ぬ": "nu", "ね": "ne", "の": "no",
    "は": "ha", "ひ": "hi", "ふ": "fu", "へ": "he", "ほ": "ho",
    "ま": "ma", "み": "mi", "む": "mu", "め": "me", "も": "mo",
    "や": "ya", "ゆ": "yu", "よ": "yo",
    "ら": "ra", "り": "ri", "る": "ru", "れ": "re", "ろ": "ro",
    "わ": "wa", "を": "o", "ん": "n",
    "が": "ga", "ぎ": "gi", "ぐ": "gu", "げ": "ge", "ご": "go",
    "ざ": "za", "じ": "ji", "ず": "zu", "ぜ": "ze", "ぞ": "zo",
    "だ": "da", "ぢ": "ji", "づ": "zu", "で": "de", "ど": "do",
    "ば": "ba", "び": "bi", "ぶ": "bu", "べ": "be", "ぼ": "bo",
    "ぱ": "pa", "ぴ": "pi", "ぷ": "pu", "ぺ": "pe", "ぽ": "po",
    # small kana (when standalone)
    "ぁ": "a", "ぃ": "i", "ぅ": "u", "ぇ": "e", "ぉ": "o",
    "ゃ": "ya", "ゅ": "yu", "ょ": "yo",
}


def kata_to_hira(c: str) -> str:
    code = ord(c)
    if 0x30A1 <= code <= 0x30F6:
        return chr(code - 0x60)
    return c


def lookup_kana(ja: str, j: int):
    """Return (romaji, advance) for kana chunk starting at ja[j], or None."""
    if j + 1 < len(ja):
        digraph = kata_to_hira(ja[j]) + kata_to_hira(ja[j + 1])
        if digraph in DIGRAPH:
            return DIGRAPH[digraph], 2
    single = kata_to_hira(ja[j])
    if single in MONO:
        return MONO[single], 1
    return None


def is_kanji(c: str) -> bool:
    cp = ord(c)
    return 0x3400 <= cp <= 0x9FFF or 0xF900 <= cp <= 0xFAFF


def insert_dashes(ja: str, typing: str):
    """Return (new_typing, ok). ok=False means alignment failed."""
    # Idempotency: if the typing already contains `-`, assume it was
    # previously migrated and leave it alone.
    if "-" in typing:
        return typing, True
    t = typing.lower()
    out = []
    i = 0  # pointer in t
    j = 0  # pointer in ja
    while j < len(ja):
        c = ja[j]
        if c == "ー":
            out.append("-")
            j += 1
            continue
        if c == "っ" or c == "ッ":
            # sokuon doubles the next consonant. Just consume one typing char if it
            # matches a consonant-doubling pattern; otherwise advance once.
            if i < len(t) and t[i].isalpha() and t[i] not in "aiueo":
                # depends on the next mora's romaji. Cheapest: if t[i] == t[i+1],
                # take that as the sokuon's contribution.
                if i + 1 < len(t) and t[i] == t[i + 1]:
                    out.append(t[i])
                    i += 1
                elif i + 1 < len(t) and t[i] == 't' and t[i + 1] == 'c':
                    # っち = tchi (hepburn) - common
                    out.append(t[i])
                    i += 1
            j += 1
            continue
        kana = lookup_kana(ja, j)
        if kana is not None:
            r, advance = kana
            # ji/zi etc. — accept either spelling already in typing
            seg = t[i : i + len(r)]
            # allow kunrei variants registered in data: shi/si, chi/ti, tsu/tu, fu/hu, ji/zi
            if seg != r and not _equiv(seg, r):
                # try shorter match (single ji vs zi etc.)
                # also handle ん before vowel registered as nn
                if r == "n" and seg.startswith("nn"):
                    out.append("nn")
                    i += 2
                    j += advance
                    continue
                return None, False
            out.append(seg)
            i += len(seg)
            j += advance
            continue
        if c.isascii() and (c.isalnum() or c in "/."):
            if i < len(t) and t[i].lower() == c.lower():
                out.append(t[i])
                i += 1
            j += 1
            continue
        if is_kanji(c):
            # find next "anchor" in ja: next kana, ASCII, or ー
            k = j + 1
            while k < len(ja):
                ch = ja[k]
                if ch == "ー" or is_kanji(ch) is False and (lookup_kana(ja, k) is not None or (ch.isascii() and (ch.isalnum() or ch in "/."))):
                    if ch != "ー" or k == j + 1:
                        break
                    # we found ー but want to find what's after kanji block first
                    break
                k += 1
            if k >= len(ja):
                # rest of ja is kanji; consume rest of t
                out.append(t[i:])
                i = len(t)
                j = len(ja)
                continue
            # Determine anchor romaji
            if ja[k] == "ー":
                # this means kanji block is immediately followed by ー. Without
                # knowing kanji's romaji length, the ー position is ambiguous.
                return None, False
            anchor = lookup_kana(ja, k)
            if anchor is None:
                # ASCII anchor
                anchor_r = ja[k].lower()
            else:
                anchor_r = anchor[0]
            # Find anchor_r in t starting at i. To avoid spurious matches
            # inside the kanji's own romaji, prefer the earliest occurrence
            # after at least 1 char of advance.
            pos = t.find(anchor_r, i + 1)
            if pos < 0:
                return None, False
            out.append(t[i:pos])
            i = pos
            j = k
            continue
        # other punctuation (・, etc.) — usually emits a space in typing? Check.
        if c in "・、。 　":
            # if t has a space here, consume it; otherwise skip silently
            if i < len(t) and t[i] == " ":
                out.append(" ")
                i += 1
            j += 1
            continue
        # unknown
        j += 1
    out.append(t[i:])
    return "".join(out), True


def _equiv(seg: str, r: str) -> bool:
    """Accept kunrei <-> hepburn equivalents between registered typing and computed romaji."""
    pairs = [("shi", "si"), ("chi", "ti"), ("tsu", "tu"), ("fu", "hu"), ("ji", "zi"), ("ja", "za"), ("ju", "zu"), ("jo", "zo")]
    for a, b in pairs:
        if (seg == a and r == b) or (seg == b and r == a):
            return True
    return False


def main() -> int:
    data = json.loads(DATA.read_text(encoding="utf-8"))
    updated = 0
    failed = []
    for q in data:
        for ci, choice in enumerate(q.get("choices", [])):
            ja = choice.get("ja", "") or ""
            if "ー" not in ja:
                continue
            typings = choice.get("ja_typings", []) or []
            if not typings:
                continue
            new_typings = []
            ok_all = True
            for t in typings:
                new_t, ok = insert_dashes(ja, t)
                if not ok or "-" not in new_t:
                    ok_all = False
                    failed.append((q["id"], ci, ja, t))
                    new_typings.append(t)
                else:
                    new_typings.append(new_t)
            if ok_all:
                choice["ja_typings"] = new_typings
                updated += 1
    DATA.write_text(
        json.dumps(data, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    print(f"updated choices: {updated}")
    print(f"failed alignments: {len(failed)}")
    for f in failed[:20]:
        print("  ", f)
    if len(failed) > 20:
        print(f"  ... and {len(failed) - 20} more")
    return 0


if __name__ == "__main__":
    sys.exit(main())

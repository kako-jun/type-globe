"""Tests for pure helpers in scripts/check_ja_typings.py.

These cover only the pure conversion / matching utilities (no pykakasi
calls), so they run fast and deterministically. The expected values are
pinned to the behaviour documented by src/io/romaji.rs's #[cfg(test)]
block — Python and Rust must agree.
"""
from __future__ import annotations

import json
from pathlib import Path

from check_ja_typings import (
    _hiragana_to_hepburn_raw,
    has_kanji,
    hepburn_variants,
    matches_any,
)

ROOT = Path(__file__).resolve().parent.parent
QUESTIONS_PATH = ROOT / "data" / "questions_ja.json"


def test_hepburn_raw_basic():
    assert _hiragana_to_hepburn_raw("りんご") == "ringo"
    assert _hiragana_to_hepburn_raw("ふゆ") == "fuyu"


def test_hepburn_variants_long_o():
    # とうきょう: raw=toukyou, collapsed=tokyo → variants = [collapsed, raw]
    assert hepburn_variants("とうきょう") == ["tokyo", "toukyou"]
    # おおさか: raw=oosaka, collapsed=osaka
    assert hepburn_variants("おおさか") == ["osaka", "oosaka"]


def test_hepburn_raw_n_before_bmp():
    # ん always maps to plain 'n' — no m before b/m/p
    assert _hiragana_to_hepburn_raw("しんばし") == "shinbashi"
    assert _hiragana_to_hepburn_raw("てんぷら") == "tenpura"


def test_hepburn_raw_sokuon_and_youon():
    assert _hiragana_to_hepburn_raw("ろけっと") == "roketto"
    # がっこう raw = gakkou (collapsed = gakko via variants)
    assert _hiragana_to_hepburn_raw("がっこう") == "gakkou"
    assert hepburn_variants("がっこう") == ["gakko", "gakkou"]
    # ちぇっく: ち+ぇ → che, prefix for っ before "ch" is 't'? No — gemination of
    # ch uses 't' prefix, but next pair is く ("ku"), not ちぇ. Sequence is
    # ちぇ + っ + く → "che" + "kku"
    assert _hiragana_to_hepburn_raw("ちぇっく") == "chekku"


def test_hepburn_katakana_and_separator():
    # Katakana is normalized to hiragana; separator '・' and parens become spaces
    assert _hiragana_to_hepburn_raw("エル（Lawliet）") == "eru lawliet"
    assert _hiragana_to_hepburn_raw("エレン・イェーガー") == "eren yega"


def test_hepburn_preserves_decimal_and_slash():
    assert _hiragana_to_hepburn_raw("やく1.5おくkm") == "yaku1.5okukm"
    assert _hiragana_to_hepburn_raw("やく300km/s") == "yaku300km/s"


def test_has_kanji():
    assert has_kanji("テスト") is False
    assert has_kanji("駆動") is True
    assert has_kanji("ABC") is False
    assert has_kanji("X線") is True


def test_matches_any_bidirectional_prefix():
    # Either side may be a prefix of the other (exact match or one extends the other)
    assert matches_any(["tokyo"], ["tokyou"]) is True  # actual extends expected
    assert matches_any(["tokyou"], ["tokyo"]) is True  # expected extends actual
    assert matches_any(["tokyo"], ["tokyo"]) is True  # exact match
    # Diverging strings (share only a short prefix) do not match
    assert matches_any(["toukyou"], ["tokyo"]) is False
    assert matches_any(["tokyo"], ["nagoya"]) is False


def test_matches_any_normalizes_hyphen_space_underscore_and_case():
    # _normalize_for_match strips spaces/hyphens/underscores and lowercases
    assert matches_any(["tesutokudo"], ["TESUTO-KUDO"]) is True
    assert matches_any(["abc def"], ["abcdef"]) is True


def test_q00069_tdd_ja_typings_pinned():
    """Regression pin: テスト駆動開発 must have correct Hepburn typings.

    Catches the historical 'kido' mis-romanization (駆動 → ku-dou → kudo).
    """
    data = json.loads(QUESTIONS_PATH.read_text(encoding="utf-8"))
    q = next(q for q in data if q.get("id") == "q00069")
    tdd_choice = next(c for c in q["choices"] if c.get("ja") == "テスト駆動開発")
    typings = tdd_choice["ja_typings"]
    assert "tesutokudokaihatsu" in typings
    assert "tesutokudoukaihatsu" in typings
    assert not any(t.startswith("kido") for t in typings)

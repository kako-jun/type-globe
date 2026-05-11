"""
question_text.ja を漢字かな交じりへ書き換える。

入力: question_text.en (主) + question_text_reading.ja (補助ヒント)
出力: question_text.ja のみ書き換え。choices や reading は触らない。

事前に backfill_question_text_reading.py を必ず実行しておくこと。
読みが question_text_reading.ja に退避されている前提で動く。

ローカル LLM (デフォルト: Ollama gemma4:e4b) に問い合わせて自然な
日本語のクイズ問題文を生成する。フィルタ条件:

- ひらがな比率 >= --min-hiragana-ratio (デフォ 0.6)
- かつ 漢字数 <= --max-kanji (デフォ 1)
- 既に question_text_reading.ja が空の場合はスキップ (バックフィル未実行)

usage:
    uv run python3 scripts/restore_ja_question_texts_with_ollama.py data/questions_ja.json --dry-run
    uv run python3 scripts/restore_ja_question_texts_with_ollama.py data/questions_ja.json
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

KANJI_RE = re.compile(r"[一-鿿]")

PROMPT = """\
英語のクイズ問題文を、自然で読みやすい日本語のクイズ問題文に翻訳してください。

ルール:
- 漢字とかなをバランスよく使う (現代の自然な書き言葉)
- 文末は「？」で終える
- 出力は問題文一行のみ。説明・引用符・注釈・前置きは出力しない
- 専門用語や英語のキーワードはそのまま英字で残してよい (例: Python, lambda, NaN)
- 既存のひらがな読みは参考にしてよいが、表記は読みやすく整えること
- LaTeX 記法 ($...$ や \\log など) は使わず、プレーンテキストで書くこと
- 余計な丸括弧の英訳併記 (例:「ハッシュ衝突（hash collision）」) は付けないこと

英語: {en}
ひらがな読み参考: {reading}
日本語:"""

PROMPT_FALLBACK = """\
Translate this English quiz question to natural Japanese (mix kanji and kana, modern written style).
Output only the Japanese question, ending with ？. No explanation, no quotes, single line.
Keep technical English keywords as-is (e.g. Python, lambda, NaN). No LaTeX, no parenthetical English glosses.

English: {en}
Japanese:"""


def is_hiragana(c: str) -> bool:
    return "ぁ" <= c <= "ゟ"


def is_katakana(c: str) -> bool:
    return "ァ" <= c <= "ヿ"


def hira_ratio(s: str) -> float:
    h = sum(1 for c in s if is_hiragana(c))
    t = sum(1 for c in s if c.isalnum() or is_hiragana(c) or is_katakana(c) or KANJI_RE.match(c))
    return h / t if t else 0.0


def kanji_count(s: str) -> int:
    return len(KANJI_RE.findall(s))


def call_ollama(host: str, model: str, prompt: str, timeout: int) -> str:
    body = json.dumps(
        {
            "model": model,
            "stream": False,
            "options": {
                "temperature": 0.2,
                "num_predict": 120,
                "num_ctx": 1024,
            },
            "prompt": prompt,
        }
    ).encode("utf-8")
    req = urllib.request.Request(
        f"{host.rstrip('/')}/api/generate",
        data=body,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        payload = json.loads(resp.read())
    return payload.get("response", "").strip()


def clean_response(s: str) -> str:
    # Take only the first line (model sometimes adds explanations after).
    s = s.strip()
    s = s.splitlines()[0].strip() if s else s
    if not s:
        return s
    # Strip outer wrappers only when balanced — avoids leaving a stray
    # closing bracket like "たんこうぼん」とは何？" when the model emits
    # only one side.
    for opener, closer in (("「", "」"), ('"', '"'), ("'", "'"), ("`", "`")):
        if s.startswith(opener) and s.endswith(closer) and len(s) >= 2:
            s = s[1:-1].strip()
    # Strip LaTeX inline math wrappers ($...$) but keep the inner text.
    s = re.sub(r"\$([^$]+)\$", r"\1", s)
    # Strip residual LaTeX command backslashes (e.g. \log -> log).
    s = re.sub(r"\\([A-Za-z]+)", r"\1", s)
    # Normalise terminal punctuation to a full-width question mark.
    s = re.sub(r"[?？。．\.!！]+\s*$", "", s).rstrip()
    s = s + "？"
    return s


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("path", type=Path)
    ap.add_argument("--ollama-host", default="http://127.0.0.1:11434")
    ap.add_argument("--model", default="gemma4:e4b")
    ap.add_argument("--min-hiragana-ratio", type=float, default=0.6)
    ap.add_argument("--max-kanji", type=int, default=1)
    ap.add_argument("--limit", type=int, default=0, help="0 = no limit")
    ap.add_argument("--start-id", default=None, help="resume from this question id")
    ap.add_argument("--timeout", type=int, default=120)
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--verbose", action="store_true")
    args = ap.parse_args()

    data = json.loads(args.path.read_text(encoding="utf-8"))

    targets: list[dict] = []
    for q in data:
        qt = q.get("question_text") or {}
        qtr = q.get("question_text_reading") or {}
        ja = qt.get("ja", "")
        en = qt.get("en", "")
        ja_reading = qtr.get("ja", "")
        if not ja_reading:
            # backfill 未済 → 触らない (安全策)
            continue
        if not en:
            continue
        if hira_ratio(ja) < args.min_hiragana_ratio:
            continue
        if kanji_count(ja) > args.max_kanji:
            continue
        targets.append(q)

    if args.start_id is not None:
        skip = True
        new_targets = []
        for q in targets:
            if skip and q.get("id") == args.start_id:
                skip = False
            if not skip:
                new_targets.append(q)
        targets = new_targets

    if args.limit > 0:
        targets = targets[: args.limit]

    print(f"target: {len(targets)} questions")
    if args.dry_run:
        for q in targets[:20]:
            print(f"  {q.get('id')}: {q['question_text']['ja']!r} <- {q['question_text']['en']!r}")
        if len(targets) > 20:
            print(f"  ... and {len(targets) - 20} more")
        return 0

    converted = 0
    failed = 0
    started = time.monotonic()
    save_every = 25
    max_retries = 2
    for i, q in enumerate(targets, 1):
        en = q["question_text"]["en"]
        reading = q["question_text_reading"]["ja"]
        new_ja = ""
        raw = ""
        dt = 0.0
        ok = False
        last_err = None
        # Two prompt variants: primary Japanese-led, fallback English-led
        # (Gemma occasionally returns whitespace-only output for the JP prompt
        # on certain inputs — the EN-led prompt rescues those cases).
        for attempt in range(max_retries + 1):
            prompt = (PROMPT if attempt == 0 else PROMPT_FALLBACK).format(en=en, reading=reading)
            try:
                t0 = time.monotonic()
                raw = call_ollama(args.ollama_host, args.model, prompt, args.timeout)
                dt = time.monotonic() - t0
                new_ja = clean_response(raw)
                if new_ja and len(new_ja) >= 3:
                    ok = True
                    break
            except (urllib.error.URLError, TimeoutError, OSError) as e:
                last_err = e
                time.sleep(0.5)
                continue
            time.sleep(0.3)
        if not ok:
            failed += 1
            if last_err is not None:
                print(f"  [{i}/{len(targets)}] {q.get('id')} ERROR after retries: {last_err}", file=sys.stderr)
            else:
                print(f"  [{i}/{len(targets)}] {q.get('id')} EMPTY after retries: {raw!r}", file=sys.stderr)
            continue
        old_ja = q["question_text"]["ja"]
        q["question_text"]["ja"] = new_ja
        converted += 1
        if args.verbose or i <= 3 or i % 25 == 0:
            print(f"  [{i}/{len(targets)}] {q.get('id')} {dt:.1f}s  {old_ja!r} -> {new_ja!r}")
        if i % save_every == 0:
            args.path.write_text(
                json.dumps(data, ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
            elapsed = time.monotonic() - started
            rate = i / elapsed
            eta = (len(targets) - i) / rate if rate else 0
            print(f"    -- checkpoint saved ({i}/{len(targets)}, {rate:.2f} q/s, ETA {eta/60:.1f} min)")

    args.path.write_text(
        json.dumps(data, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    elapsed = time.monotonic() - started
    print(f"converted: {converted}  failed: {failed}  elapsed: {elapsed/60:.1f} min")
    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())

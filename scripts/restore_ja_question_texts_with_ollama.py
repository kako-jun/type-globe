"""
question_text.ja を漢字かな交じりへ書き換える。

入力: question_text.en (主) + question_text_reading.ja (補助ヒント)
出力: question_text.ja を書き換え。--include-choices 指定時は choices[].ja
も表示用の自然な日本語へ書き換える。reading / ja_typings は触らない。

事前に backfill_question_text_reading.py を必ず実行しておくこと。
読みが question_text_reading.ja に退避されている前提で動く。

ローカル LLM (デフォルト: Ollama gemma4:e4b) に問い合わせて自然な
日本語のクイズ問題文を生成する。フィルタ条件:

- デフォルトでは以下の疑わしい項目だけを対象にする
  - ひらがな比率 >= --min-hiragana-ratio (デフォ 0.6)
  - かつ 漢字数 <= --max-kanji (デフォ 1)
- --all 指定時はフィルタせず全件を対象にする
- 既に question_text_reading.ja が空の場合はスキップ (バックフィル未実行)

usage:
    uv run python3 scripts/restore_ja_question_texts_with_ollama.py data/questions_ja.json --dry-run
    uv run python3 scripts/restore_ja_question_texts_with_ollama.py data/questions_ja.json
    uv run python3 scripts/restore_ja_question_texts_with_ollama.py data/questions_ja.json --all --include-choices
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
ひらがな読みのクイズ問題文を、読みを変えずに自然な漢字かな交じり表記へ直してください。

ルール:
- ひらがな読み参考の語順・助詞・文末を変えない
- 「何ですか」「どれですか」など、読み参考に無い語を追加しない
- 読み参考に無い説明や言い換えをしない
- 漢字・カタカナ・英字への表記変換だけを行う
- 英語は曖昧な固有名詞・専門用語の表記判断にだけ使う
- 読み参考が「？」で終わるなら、出力も「？」で終える
- 出力は問題文一行のみ。説明・引用符・注釈・前置きは出力しない
- 専門用語や英語のキーワードは英字で残してよい (例: Python, lambda, NaN)
- LaTeX 記法 ($...$ や \\log など) は使わず、プレーンテキストで書くこと
- 余計な丸括弧の英訳併記 (例:「ハッシュ衝突（hash collision）」) は付けないこと

専門用語の例:
- きていじょうけん + base case -> 基底条件 (「規定以上条件」ではない)
- とれーと + trait -> トレイト
- れんけつりすと + linked list -> 連結リスト
- はばゆうせんたんさく + BFS -> 幅優先探索
- すいちょくすけーりんぐ + vertical scaling -> 垂直スケーリング

英語: {en}
ひらがな読み正本: {reading}
日本語:"""

PROMPT_FALLBACK = """\
Convert this Japanese hiragana reading into natural kanji-kana display text without changing its wording.
Do not add words, do not paraphrase, and do not translate freely.
Use the English only to disambiguate terms. Output one Japanese question line only.

English: {en}
Hiragana reading to preserve: {reading}
Japanese:"""

CHOICE_PROMPT = """\
ひらがな読みのクイズ回答候補を、読みを変えずに自然な漢字かな交じり表記へ直してください。

ルール:
- 読み参考の語順・助詞・意味を変えない
- 読み参考に無い語を追加しない
- 説明や言い換えをしない
- 漢字・カタカナ・英字への表記変換だけを行う
- 英語は曖昧な固有名詞・専門用語の表記判断にだけ使う
- 文末の句点や疑問符は付けない
- 出力は回答候補一つだけ。説明・引用符・前置きは出力しない
- コード、コマンド、HTMLタグ、数式、化学式、略語、固有の英字表記はそのまま残してよい
- 余計な英訳併記は付けないこと

専門用語の例:
- きていじょうけん + base case -> 基底条件
- とれーと + trait -> トレイト
- れんけつりすと + linked list -> 連結リスト
- はばゆうせんたんさく + BFS -> 幅優先探索

英語: {en}
ひらがな読み正本: {reading}
日本語:"""

CHOICE_PROMPT_FALLBACK = """\
Convert this Japanese hiragana answer reading into a natural kanji-kana display label without changing its wording.
Do not add words, do not paraphrase, and do not translate freely.
Use the English only to disambiguate terms. Output one answer label only.

English: {en}
Hiragana reading to preserve: {reading}
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
    return call_ollama_with_options(host, model, prompt, timeout, num_predict=120)


def call_ollama_with_options(
    host: str, model: str, prompt: str, timeout: int, *, num_predict: int
) -> str:
    body = json.dumps(
        {
            "model": model,
            "stream": False,
            "think": False,
            "options": {
                "temperature": 0.2,
                "num_predict": num_predict,
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
    if payload.get("error"):
        raise RuntimeError(payload["error"])
    if not payload.get("response") and payload.get("done_reason") == "length":
        raise RuntimeError(f"empty response before visible output; increase --num-predict (eval_count={payload.get('eval_count')})")
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


def clean_choice_response(s: str) -> str:
    s = s.strip()
    s = s.splitlines()[0].strip() if s else s
    if not s:
        return s
    for opener, closer in (("「", "」"), ('"', '"'), ("'", "'"), ("`", "`")):
        if s.startswith(opener) and s.endswith(closer) and len(s) >= 2:
            s = s[1:-1].strip()
    s = re.sub(r"\$([^$]+)\$", r"\1", s)
    s = re.sub(r"\\([A-Za-z]+)", r"\1", s)
    s = re.sub(r"[?？。．\.!！]+\s*$", "", s).strip()
    return s


def should_rewrite_display(s: str, min_hira: float, max_kanji: int, rewrite_all: bool) -> bool:
    if rewrite_all:
        return True
    return hira_ratio(s) >= min_hira and kanji_count(s) <= max_kanji


def generate_with_retries(
    *,
    host: str,
    model: str,
    prompts: list[str],
    timeout: int,
    num_predict: int,
    cleaner,
) -> tuple[str, str, float, object | None]:
    raw = ""
    dt = 0.0
    last_err = None
    for prompt in prompts:
        try:
            t0 = time.monotonic()
            raw = call_ollama_with_options(host, model, prompt, timeout, num_predict=num_predict)
            dt = time.monotonic() - t0
            cleaned = cleaner(raw)
            if cleaned and len(cleaned) >= 1:
                return cleaned, raw, dt, None
        except (urllib.error.URLError, TimeoutError, OSError, RuntimeError) as e:
            last_err = e
            time.sleep(0.5)
            continue
        time.sleep(0.3)
    return "", raw, dt, last_err


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("path", type=Path)
    ap.add_argument("--ollama-host", default="http://127.0.0.1:11434")
    ap.add_argument("--model", default="gemma4:e4b")
    ap.add_argument("--min-hiragana-ratio", type=float, default=0.6)
    ap.add_argument("--max-kanji", type=int, default=1)
    ap.add_argument("--all", action="store_true", help="rewrite all question_text.ja entries")
    ap.add_argument(
        "--include-choices",
        action="store_true",
        help="also rewrite choices[].ja display labels; ja_typings are preserved",
    )
    ap.add_argument("--limit", type=int, default=0, help="total item limit; 0 = no limit")
    ap.add_argument("--start-id", default=None, help="resume from this question id")
    ap.add_argument("--timeout", type=int, default=120)
    ap.add_argument("--num-predict", type=int, default=160)
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--verbose", action="store_true")
    args = ap.parse_args()

    data = json.loads(args.path.read_text(encoding="utf-8"))

    question_targets: list[dict] = []
    choice_targets: list[tuple[dict, int, dict]] = []
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
        if should_rewrite_display(ja, args.min_hiragana_ratio, args.max_kanji, args.all):
            question_targets.append(q)
        if args.include_choices:
            for choice_idx, choice in enumerate(q.get("choices", [])):
                choice_ja = choice.get("ja", "")
                choice_en = choice.get("en", "")
                if not choice_ja or not choice_en:
                    continue
                if should_rewrite_display(choice_ja, args.min_hiragana_ratio, args.max_kanji, args.all):
                    choice_targets.append((q, choice_idx, choice))

    if args.start_id is not None:
        skip = True
        new_question_targets = []
        for q in question_targets:
            if skip and q.get("id") == args.start_id:
                skip = False
            if not skip:
                new_question_targets.append(q)
        question_targets = new_question_targets

        skip = True
        new_choice_targets = []
        for q, choice_idx, choice in choice_targets:
            if skip and q.get("id") == args.start_id:
                skip = False
            if not skip:
                new_choice_targets.append((q, choice_idx, choice))
        choice_targets = new_choice_targets

    if args.limit > 0:
        question_targets = question_targets[: args.limit]
        remaining = args.limit - len(question_targets)
        choice_targets = choice_targets[: max(0, remaining)]

    total_targets = len(question_targets) + len(choice_targets)
    print(f"target: {len(question_targets)} questions, {len(choice_targets)} choices")
    if args.dry_run:
        for q in question_targets[:20]:
            print(f"  {q.get('id')}: {q['question_text']['ja']!r} <- {q['question_text']['en']!r}")
        if len(question_targets) > 20:
            print(f"  ... and {len(question_targets) - 20} more questions")
        for q, choice_idx, choice in choice_targets[:20]:
            print(f"  {q.get('id')} choice#{choice_idx}: {choice['ja']!r} <- {choice['en']!r}")
        if len(choice_targets) > 20:
            print(f"  ... and {len(choice_targets) - 20} more choices")
        return 0

    converted = 0
    failed = 0
    started = time.monotonic()
    save_every = 25
    for i, q in enumerate(question_targets, 1):
        en = q["question_text"]["en"]
        reading = q["question_text_reading"]["ja"]
        # Two prompt variants: primary Japanese-led, fallback English-led
        # (Gemma occasionally returns whitespace-only output for the JP prompt
        # on certain inputs — the EN-led prompt rescues those cases).
        prompts = [
            PROMPT.format(en=en, reading=reading),
            PROMPT_FALLBACK.format(en=en, reading=reading),
            PROMPT_FALLBACK.format(en=en, reading=reading),
        ]
        new_ja, raw, dt, last_err = generate_with_retries(
            host=args.ollama_host,
            model=args.model,
            prompts=prompts,
            timeout=args.timeout,
            num_predict=args.num_predict,
            cleaner=clean_response,
        )
        if not new_ja or len(new_ja) < 3:
            failed += 1
            if last_err is not None:
                print(
                    f"  [question {i}/{len(question_targets)}] {q.get('id')} ERROR after retries: {last_err}",
                    file=sys.stderr,
                )
            else:
                print(
                    f"  [question {i}/{len(question_targets)}] {q.get('id')} EMPTY after retries: {raw!r}",
                    file=sys.stderr,
                )
            continue
        old_ja = q["question_text"]["ja"]
        q["question_text"]["ja"] = new_ja
        converted += 1
        if args.verbose or i <= 3 or i % 25 == 0:
            print(f"  [question {i}/{len(question_targets)}] {q.get('id')} {dt:.1f}s  {old_ja!r} -> {new_ja!r}")
        if i % save_every == 0:
            args.path.write_text(
                json.dumps(data, ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
            elapsed = time.monotonic() - started
            done = i
            rate = done / elapsed
            eta = (total_targets - done) / rate if rate else 0
            print(f"    -- checkpoint saved ({done}/{total_targets}, {rate:.2f} item/s, ETA {eta/60:.1f} min)")

    offset = len(question_targets)
    for j, (q, choice_idx, choice) in enumerate(choice_targets, 1):
        en = choice["en"]
        reading = choice["ja"]
        prompts = [
            CHOICE_PROMPT.format(en=en, reading=reading),
            CHOICE_PROMPT_FALLBACK.format(en=en, reading=reading),
            CHOICE_PROMPT_FALLBACK.format(en=en, reading=reading),
        ]
        new_ja, raw, dt, last_err = generate_with_retries(
            host=args.ollama_host,
            model=args.model,
            prompts=prompts,
            timeout=args.timeout,
            num_predict=args.num_predict,
            cleaner=clean_choice_response,
        )
        if not new_ja:
            failed += 1
            if last_err is not None:
                print(
                    f"  [choice {j}/{len(choice_targets)}] {q.get('id')}#{choice_idx} ERROR after retries: {last_err}",
                    file=sys.stderr,
                )
            else:
                print(
                    f"  [choice {j}/{len(choice_targets)}] {q.get('id')}#{choice_idx} EMPTY after retries: {raw!r}",
                    file=sys.stderr,
                )
            continue
        old_ja = choice["ja"]
        choice["ja"] = new_ja
        converted += 1
        if args.verbose or j <= 3 or j % 50 == 0:
            print(f"  [choice {j}/{len(choice_targets)}] {q.get('id')}#{choice_idx} {dt:.1f}s  {old_ja!r} -> {new_ja!r}")
        done = offset + j
        if done % save_every == 0:
            args.path.write_text(
                json.dumps(data, ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
            elapsed = time.monotonic() - started
            rate = done / elapsed
            eta = (total_targets - done) / rate if rate else 0
            print(f"    -- checkpoint saved ({done}/{total_targets}, {rate:.2f} item/s, ETA {eta/60:.1f} min)")

    args.path.write_text(
        json.dumps(data, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    elapsed = time.monotonic() - started
    print(f"converted: {converted}  failed: {failed}  elapsed: {elapsed/60:.1f} min")
    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())

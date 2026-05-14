# サブエージェントによる問題データレビュー手順書

## 背景

`data/questions_ja.json` は今後も問題が増えていく。メインエージェントが全件読んでレビューするのは現実的でない。サブエージェントに分担させるしかないが、過去のラウンドで **15体並列で全員 600秒タイムアウト** という事故が起きた (round-2, 2026-05)。失敗の原因をふまえて、以下の手順を守る。

## 失敗の根本原因 (避けるべきアンチパターン)

サブエージェントは review プロンプトを受け取ると、以下の流れで詰む:

1. quality-spec / CLAUDE.md を読む
2. 入力 JSON を全件読む
3. 1問ずつ細かく検討 (IME の揺れ、漢字読み、ja↔en 対応、distractor 質)
4. メモを書きまくる
5. **出力ファイルを書かないまま watchdog タイムアウト**

特に IME 関連の細かい揺れ (`nn` / `n` など) について深く考え込む傾向が強い。`dhi` / `di` はそれぞれ ディ / ぢ で別物なので混同しない。

## サブエージェント呼び出し時の鉄則

### 1. 出力先行を強制する

プロンプト冒頭に以下を入れる:

> **MANDATORY FIRST ACTION:** Write `data/review/questions_<genre>.json` as an EXACT COPY of the input file. Do this BEFORE reading any content. Use `cp` via Bash or read+write the file unchanged. This is the safety baseline — even if you run out of time later, the output exists.

これだけで「最低でも入力のコピーがアウトプットされる」が保証される。

### 2. Patch リスト出力フォーマット

「全件 review した後で完全な corrected file を書く」は禁止。代わりに:

> Output format: write `data/review/patches_<genre>.json` containing ONLY the questions you decided to change. Schema:
> ```json
> [
>   {"id": "q0007", "reason": "kanji error 海王星 reading", "new_choice_2": {"ja": "...", "en": "...", "ja_typings": [...]}},
>   ...
> ]
> ```
> Skip questions that look fine. Don't include them in patches.

差分だけ書けば出力が小さくなり、トークン消費・処理時間が大幅に減る。

### 3. 厳格なトリアージ基準

> **What to fix (only these):**
> - Clear factual error in question_text or choice text (verifiable wrong fact)
> - Obviously wrong proper-noun kanji (e.g., 織田栄一郎 for ONE PIECE author)
> - Question logic contradiction (e.g., "技を使えない" but actually can use)
> - Fake people / nonexistent works in choices
> - ja↔en hard mismatch (different concept entirely)
>
> **What NOT to touch:**
> - IME romaji edge cases (nn vs n before consonants — runtime handles it)
> - Subtle awkward Japanese phrasing
> - Distractor quality nitpicks
> - Anything you're less than 80% sure about

「迷ったら触らない」を強調する。「明らかな誤りだけ」と何度も書く。

### 4. 時間予算とチェックポイント

> **Time budget: 5 minutes total. Checkpoints:**
> - At 1 min: output file written as copy of input (safety net)
> - At 3 min: must have at least 5 patches OR confirmed "no issues found"
> - At 5 min: stop reviewing, write final patches file regardless of completion
>
> If you find yourself analyzing an entry for more than 30 seconds, skip it.

### 5. チャンク粒度を小さく

50問のジャンルを1エージェントに渡さない。**10〜15問単位** に分割:

- science (50問) → 4エージェント (q0001-q0015, q0016-q0030, q0031-q0045, q0046-q0050)
- programming (80問) → 6エージェント
- it_terminology (35問) → 3エージェント

メイン側で `python3 -c "import json; data = json.load(open('...')); print(json.dumps(data[start:end]))"` で範囲指定して渡せる。

### 6. WebSearch 必須を解除

過去のラウンドで複数エージェントが WebSearch denied で迷走した。プロンプトに:

> WebSearch is optional. If denied, proceed using your general knowledge. For facts you can't verify, skip the entry (don't fix what you're unsure about).

### 7. メインエージェント側の集約フロー

サブエージェント完了後、メインがやる:

```python
# 全ジャンルの patches を集約
import json, glob
all_patches = []
for path in sorted(glob.glob('data/review/patches_*.json')):
    all_patches.extend(json.load(open(path)))

# 元データに patch を適用
d = json.load(open('data/questions_ja.json'))
by_id = {q['id']: q for q in d}
for patch in all_patches:
    q = by_id[patch['id']]
    # patch のキーに従って書き換え
    ...

# lint + test + commit
```

患者数だけ Python で適用できれば、メインのトークン消費を最小化できる。

## 推奨 サブエージェント プロンプト テンプレート

```
Review questions q{START}-q{END} from data/questions_ja.json for {genre}.

**MANDATORY FIRST ACTION (do this BEFORE reading data):**
Write `data/review/patches_{genre}_{START}_{END}.json` as `[]` (empty JSON array).

**Then read the questions in that range and identify ONLY:**
1. Clear factual errors (verifiable wrong fact about real-world thing)
2. Wrong kanji for proper nouns (verify via WebSearch if available, otherwise rely on memory)
3. Question logic contradictions
4. Fake/nonexistent items in choices
5. ja↔en hard mismatches (different concept)

**Skip:**
- IME romaji edge cases already covered by runtime normalize (nn/n before consonants, Hepburn/Kunrei, explicit small-kana paths). Do not conflate `dhi` and `di`.
- Awkward phrasing
- Distractor quality nitpicks
- Anything you're not confident about

**Output:** update `data/review/patches_{genre}_{START}_{END}.json` with this schema:
```json
[
  {
    "id": "q0007",
    "reason": "one sentence why",
    "patches": {
      "choices[2].ja": "fixed text",
      "choices[2].ja_typings": ["fixed_typing"]
    }
  }
]
```

**Time budget: 5 minutes max. If you've spent 30s on one entry, skip it.**

**If you run out of time:** keep whatever patches you've written so far. Don't try to finish everything.
```

## 適用例

```bash
# Round-3 で manga ジャンルだけ再レビュー (50問 → 4エージェント)
# Agent 1: q0446-q0458 (13問)
# Agent 2: q0459-q0471 (13問)
# Agent 3: q0472-q0484 (13問)
# Agent 4: q0485-q0495 (11問)
# 各 5分以内、patch ファイル 4本出力 → メインで集約適用
```

## 反省点ログ (Round 2, 2026-05)

- 15体並列で全 600秒タイムアウト
- 失敗 log に大量の指摘が書かれていたが、ファイル出力なし
- メインが log から手動抽出して bulk 適用 (366件) → でも全件カバーには至らず
- 教訓: **agent が思考を log に出すのは無意味。ファイル出力させる仕組みが必要**

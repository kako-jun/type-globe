# type-globe

ターミナル TUI のタイピングゲーム。**打つべき文字列が画面に出ない**のが本作の核。

## コア原理（絶対）

> プレイヤーは画面に表示されたものを打つのではなく、**クイズを解く**または**音声を聞き取る**ことで「何を打つべきか」を導出する。

百人一首の競技かるた（下の句は読まれない）と同じ。**反射神経ではなく知識・記憶・聴解**で勝負する。

この原理を破る仕様（=答えの文字列を入力前に画面表示する仕様）は**追加してはいけない**。`Tab` のスキップは「問題そのものを飛ばす」意味であり、答えを表示するスキップは存在しない。

## 二軸構成（クロスさせない）

| 提示方式 | ゲーム構造 |
|---|---|
| クイズ提示（4択 / 将来は画像） | 単発スコア / Time Attack 25 / ランキング |
| リスニング提示（TTS で読み上げ） | ハクスラ RPG（10問=1潜入） |

クロスのモード（リスニング × Time Attack 等）は作らない。

## ドキュメント

| ファイル | 内容 | 言語 |
|---|---|---|
| `README.md` | エンドユーザー向けの使い方（マスター） | 英語 |
| `docs/overview.md` | コンセプト・設計思想 | 英語 |
| `docs/spec.md` | モード仕様・画面・データ構造 | 英語 |
| `docs/roadmap.md` | フェーズ別タスク | 日本語 |
| `CLAUDE.md` | AI 向け内部ドキュメント（このファイル） | 日本語 |

## ソース構成（v0.2.0 ターゲット）

```
src/
├── main.rs
├── types.rs
├── config.rs
├── jiwa_core/           # 内製アニメーションモジュール
│   ├── typewriter.rs    # 1文字ずつ出現
│   ├── fade.rs          # TrueColor 段階補間
│   └── input.rs         # 演出と並行した入力受付
├── game/
│   ├── quiz.rs          # 「打って選択」+ Enter 確定
│   ├── time_attack.rs   # 5x5 パネル + CPU 対戦
│   └── hack.rs          # リスニング × RPG（10問サイクル）
├── audio/
│   └── tts.rs           # `tts` crate ラッパー、言語ルーティング
├── io/
│   ├── data_loader.rs
│   └── storage.rs       # current main: JSON, target v0.2.0: YAML
└── ui/
    ├── menu.rs
    ├── quiz.rs          # 3ペイン
    ├── hack.rs          # 4ペイン
    ├── time_attack.rs
    ├── records.rs
    └── help_line.rs     # 常時表示ヘルプ
```

## 主要な設計判断（v0.2.0 ターゲット）

- **Rust + ratatui + crossterm**：クロスプラットフォーム TUI
- **保存形式は移行途中**：現行 `main` は JSON 永続化（`player.json` / `records_<lang>.json` / `data/questions_<lang>.json`）だが、v0.2.0 ターゲットでは YAML + `serde_yaml` へ移行予定
- **TTS は実行時生成**：`tts` crate で OS ネイティブ TTS を呼ぶ。音声ファイルは同梱しない
- **`jiwa_core` は内製**：jiwa リポは未着手。type-globe で先に実装し、安定後に jiwa crate として切り出す（**(b) 内製→後で切り出し**方針）
- **演出スキップ不可**：問題文の reveal アニメーションは最後まで再生する（公平性）
- **演出と並行入力**：知っている人は出待ちせず先打ちできる
- **失敗概念なし（v0.2.0）**：HP 0 で死亡などは導入しない。10問やったら必ず帰還、ミスは EXP が減るだけ
- **音声リプレイ無制限・無ペナルティ**：タイム消費が自然なペナルティ
- **クイズは「打って選択」**：これは v0.2.0 のターゲット。現行 `main` に残る旧矢印/数字選択は follow-up Issue で除去する
- **CPM / WPM 併記**

## ブランド

- **type-globe**：オフライン版。本リポの主力
- **type-globe-online**：v0.3.0+ で展開予定の**ブランド名**。リポは分けない。`online` ラベル付きで Issue 管理
- mypace WebSocket 連携 / **Nostralgic Ranking 連携（世界ランキング）** / Nostr 投稿は `type-globe-online` 配下
- 用語ルール: ローカル自己ベストは **Records**、世界順位は **Ranking**（Nostralgic Ranking）。混同禁止

## v0.2.0「オフライン完成版」スコープ

`offline-first` ラベル付き Issue が対象。`online` は v0.3.0+ に送る。

詳細は `docs/roadmap.md` 参照。

## 廃止された旧仕様 / 段階的廃止

v0.1.x にあった以下を v0.2.0 で段階的に廃止する：

- 「画面に表示された文字列を打つ」表示タイピングモード（コア原理に反するため）: **削除済み**
- クイズの矢印キー / 数字キー（1-4）選択（反射要素を排除するため）: **未削除。follow-up Issue で除去**

## 問題生成

- 既存：`scripts/generate_questions.py`、`.claude/quiz-generation-policy.md`
- v0.2.0 ターゲット：100 → 1,000 問
- リスニング問題セット（`data/listening_<lang>.json`）も新設

## JA ローマ字入力ルール

- `data/questions_ja.json` の各 choice は `ja_typings: string[]` を持つ
- `ja_typings` は **小文字 ASCII のみ**
- 許可する揺れは **ヘボン式ベース + 長音の潰し/非潰し** のみ
  - 例: `tokyo` / `toukyou`
  - 例: `osaka` / `oosaka`
  - 例: `kyoto` / `kyouto`
- **Wapuro 系の子音揺れは不許可**
  - 不許可例: `si`, `ti`, `tu`, `hu`, `zi`
  - 正規形は `shi`, `chi`, `tsu`, `fu`, `ji`
- 外来語・固有名詞で **公式の英字綴りがあるなら、それも `ja_typings` に追加してよい**
  - 例: `エル（Lawliet）` → `lawliet`
- かな・カタカナ・ASCII だけの choice は自動生成でよい
  - `cargo run --bin backfill-ja-typing -- data/questions_ja.json`
- 漢字を含む choice は読みを人間が判断して `ja_typings` を手で入れる
- 検査は必ず実行する
  - `cargo run --bin lint-questions -- data/questions_ja.json data/questions_en.json`

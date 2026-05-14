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
│   ├── quiz.rs          # 「打って選択」+ exact match 自動確定
│   ├── time_attack.rs   # 5x5 パネル + CPU 対戦
│   └── rpg.rs           # リスニング × RPG（10問サイクル）
├── audio/
│   └── tts.rs           # `tts` crate ラッパー、言語ルーティング
├── io/
│   ├── data_loader.rs
│   └── storage.rs       # current main: JSON, target v0.2.0: YAML
└── ui/
    ├── menu.rs
    ├── quiz.rs          # 3ペイン
    ├── rpg.rs           # 4ペイン
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
- **クイズは「打って選択」+ 自動確定**：有効な prefix だけ受け付け、exact match になった瞬間に確定する。旧矢印/数字選択は follow-up Issue で除去する
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
- リスニング問題セット（`data/listening_<lang>.yaml`）も新設

## JA ローマ字入力ルール (v0.7.0 以降の IME strict 仕様)

type-globe は**タイピング練習として IME-wapuro 流儀を「正」**と定義する。
プレイヤーが IME 上で実際に打鍵して目的のかなになるキー列のみを正解とする。

- `data/questions_ja.json` の各 choice は `ja_typings: string[]` を持つ
- `ja_typings` は **小文字 ASCII のみ**
- 登録は **IME で実際に打鍵する形 (ヘボン式 + 長音保持)** に統一
  - カタカナ長音 `ー` は `-` キー必須: `サーバー` → `sa-ba-`, `ボール` → `bo-ru`
  - ひらがな長音 `おう` / `おお` / `うう` も保持: `東京` → `toukyou` (×`tokyo`), `大阪` → `oosaka` (×`osaka`), `空気` → `kuuki` (×`kuki`)
  - `ティ` は `thi`: `パーティ` → `pa-thi` (×`pa-ti`)
  - `ディ` は `dhi`: `ガンディー` → `gandhi-` (×`gandi-`)。`di` は IME では `ぢ`
  - ン+母音 / ヤ行: `nn` 二重: `観音` → `kannon`, `翻訳` → `honnyaku`
  - **ン+ナ行: `nn` 二重 (= 計3連 n)**: `せんのりきゅう` → `sennnorikyuu`, `ごめんなさい` → `gomennnasai` (× Hepburn の 2連 `sennorikyuu` は IME で せんおりきゅう になる)
  - ン+子音 / 末尾: `n` 単数: `ドラゴン` → `doragon`, `東京` → `toukyou`
- **collapsed 形 (`tokyo`, `osaka`, `boru` 等) は data に登録しない**(プレイヤーが入力しても不正解になる、IME 入力の strict 仕様に合わせる)
- **同じかな読みのローマ字 variant は複数登録しない**
  - runtime canonical で吸収されるもの: `ninnshou/ninshou`, `dairanntou/dairantou`, `texi/thi`, `si/shi` など
  - `ja_typings` に複数登録するのは、`日本` = `nihon` / `nippon` のような**かな読み自体の真の揺れ**だけ
- **Wapuro 系の子音揺れは data に登録しない**
  - 不許可例: `si`, `ti`, `tu`, `hu`, `zi`
  - 正規形は `shi`, `chi`, `tsu`, `fu`, `ji`
- 公式英字綴りは、**同じ読みの別ローマ字としては追加しない**
  - `ja` 表示が ASCII / 英字ブランドそのものなら、その ASCII 表記1つを登録する
  - `ja` 表示がかな・カタカナ・漢字なら、原則は IME 読み1つだけ。公式英字の追加はユーザーが明示した場合だけ
- 漢字読みの真の揺れ (`日本` = `nihon` も `nippon` も実在) は両方登録
- かな・カタカナ・ASCII だけの choice は自動生成でよい (`hiragana_to_hepburn` が IME 正解形で出す)
- 漢字を含む choice は読みを人間が判断して `ja_typings` を手で入れる
- 検査は必ず実行する
  - `cargo run --bin lint-questions -- data/questions_ja.json data/questions_en.json`
- データ全件の prefix 入力可否は `cargo test --release data_typings_are_prefix_typeable` が ~40ms で常時確認する

### ローマ字バリアントの runtime 吸収（#96 + v0.7.0 拡張）

`src/io/normalize.rs::canonical_romaji` が runtime で以下を吸収する。**データ側は1通りで登録**、プレイヤー入力は IME 別経路で打っても通る。

- ヘボン式 ↔ kunrei (単音): `shi/si`, `chi/ti`, `tsu/tu`, `ji/zi`, `fu/hu` (※ `wo/o` は IME で別キーで別の文字 (`wo`→を, `o`→お) なので **吸収しない**)
- ヘボン式 ↔ kunrei (拗音): `sha/sya`, `shu/syu`, `sho/syo`, `cha/tya`, `chu/tyu`, `cho/tyo`, `ja/zya`, `ju/zyu`, `jo/zyo`
- ティ系: `thi/texi/teli` → `thi`
- ディ系: `dhi/dexi/deli` → `dhi`
- ぢ系: `di/dji/dzi` → `di`
- ヅ collapse: `dzu` → `du`
- ファ系 (フ+小ぁ): `fa/fi/fe/fo` ≡ `huxa/hula/fuxa/fula/huxi/.../etc`
- ヴァ系: `va/vi/ve/vo` ≡ `vuxa/vula/vuxi/.../etc`
- ウェ系: `we/wi/wo` ≡ `uxe/ule/uxi/uli/uxo/ulo` (`uxa/ula → wa` は ウァ ≠ ワ なので **除外**)
- 拗音の明示的小ゃ (`Cixya/Cilya`): `kixya/kya`, `shixya/sha`, `chixya/cha`, `jixya/ja` 等を同一視
- 促音の明示的小っ: `ltu/xtu/ltsu/xtsu + 子音C → CC` (例: `rokeltsuto ≡ roketto`、`maltsucha ≡ matcha`)
- ン+子音/末尾の `nn`: 冗長な `nn` を `n` に畳む (ン+母音/y/n は残す)
- **3 連以上の n は正規化しない**: 2連 (ん+母音) と 3連 (ん+ナ行) は IME で別経路なので canonical でも別物
- 中黒 `・` (= IME `/`)、読点 `、` (= `,`)、句点 `。` (= `.`) は位置一致で保持する。直前の単独 `n` は IME commit boundary として `nn` に正規化する
- カタカナ ー の `-` は **畳まない** (データ側も `-` 入り)
- ひらがな長音も **畳まない** (データ側も `ou` / `oo` / `uu` 入り)

これにより「じが4個ある単語に 2^4=16通りを登録する」必要はない。

### かな読み揺れは複数登録（#98）

- 漢字 → かな の段階で**複数の自然な読みがある**ものは、`ja_typings` に**両方登録する**
  - 例: 日本 = `nihon` / `nippon` → 両方
  - 例: 七人 = `shichinin` / `nananin` → 両方
- ローマ字バリアントとは別軸の問題なので、#96 では救えない
- 数量読みなどで厳密形と口語形が揺れる場合は、データ爆発を避けるため既定は1つに絞る。広く定着した誤読・慣用読みを受け入れる場合だけ、個別判断で追加する

### 新規問題作成時の ja_typings 必須判断

新しい問題を作るときは、既存データから雰囲気で推測せず、必ずこの順で決める。

1. `ja` がかな・カタカナ・ASCIIだけなら `cargo run --bin backfill-ja-typing -- <file>` に任せる。手で variant を足さない。
2. 漢字入りなら、まず標準的な読みを1つ決め、その IME-wapuro 表記を1つだけ登録する。
3. `ja_typings` を2つ以上にしてよいのは、かな読み自体が複数ある場合だけ。
   - OK: `日本` = `nihon` / `nippon`
   - OK: `弩` = `ishiyumi` / `do`
   - NG: `tokyo` / `toukyou`, `oo` / `ou` 潰し、`thi` / `texi`, `ninnshou` / `ninshou`, スペース有無、記号有無、公式英字綴りだけの併記
4. 迷ったら複数登録しない。単一の IME 正解形で作り、必要になったときだけ個別に追加する。

## ja_typings 全件チェック手順（新規問題追加時・定期保守時）

**前提**: `scripts/check_ja_typings.py` は pykakasi で粗く比較するだけの**事前フィルタ**。最終判定はフリーザ様が per-entry で全件行う。スクリプトを信用しすぎて一括処理しないこと。

### 手順

1. **粗フィルタを実行**
   ```bash
   cd /home/d131/repos/2025/type-globe
   uv run --with pykakasi python3 scripts/check_ja_typings.py
   ```
   → `scripts/suspicious_ja_typings.json` に「pykakasi 読み ≠ 登録 ja_typings」の不一致が出力される

2. **per-entry で3カテゴリに判定**

   各 entry を `{question_id, choice_index, ja, ja_reading_hira (pykakasi), expected_typings, actual_typings}` で確認:

   | カテゴリ | 条件 | 行動 |
   |---|---|---|
   | **A 修正** | 既存 actual が明確な誤読、pykakasi が正しい | actual を削除、pykakasi 読みに置換 |
   | **B 追加** | 既存 actual も pykakasi 読みも**両方実在する正しい読み** | actual を残し、pykakasi 読みを追加（dedup） |
   | **C スキップ** | pykakasi が誤読、または出題者が意図的に変則表記を使った | 何もしない |

   **判定基準**:
   - 既存 actual が日本語として読み得ない → A
   - 両方読みうる（日本=にほん/にっぽん、型=かた/がた、行=いく/ゆく/ぎょう、人=にん/じん/ひと） → B
   - pykakasi が固有名詞・専門用語・造語を一般読みで誤読 → C
   - choice text に英数字・カタカナ混じりで pykakasi が混乱 → C
   - 出題者が意図的に英単語綴りで答えさせている（`define`, `null` 等）→ C
   - **迷ったら C**（変更しないのが最安全）

3. **prefix conflict のチェック**

   B で読みを追加した結果、同じ問題内の選択肢同士で prefix 衝突が起きる場合がある:
   - 例: choice0 に `sanjuhon`、choice1 に `sanjuhonni` が並ぶと、入力が確定しない
   - 衝突したら、その読みは追加せず C 扱いに格下げする

4. **適用と検証**
   ```bash
   cargo run --bin lint-questions data/questions_ja.json data/questions_en.json
   uv run --with pykakasi python3 scripts/check_ja_typings.py
   ```
   - lint-questions が通る（conflict 0）
   - 再 check の suspicious は C 件数まで減る

5. **`ja_reviewed: true` を立てる**

   per-entry 判定が完了した問題は `Question.ja_reviewed` を `true` にする。
   - 新規問題は default `false`
   - `cargo run --bin lint-questions data/questions_ja.json` が unreviewed 件数を表示する

6. **コミット**
   - 適用スクリプト（あれば）: `feat: ja_typings 判定スクリプト追加`
   - データ修正: `fix: ja_typings 誤読修正と読み揺れ追加（N件、A:n B:n C:n）`
   - docs: 必要があれば spec.md / CLAUDE.md 追記

### サブエージェント運用の注意

- **「迷ったら B（追加）」をデフォルトにすると寛容になりすぎる**（例: 既存=きそんが正しいのに、kison も正解として登録されてしまう）
- サブエージェントに任せるなら、**全 entry に対して明示的に A/B/C を返させる**プロンプトにする
- 「default = B」みたいなショートカットを許さない

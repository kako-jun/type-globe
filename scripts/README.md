# クイズ問題生成スクリプト

TypeGlobeのクイズ問題を大量に生成するためのスクリプトです。

## セットアップ

1. Python 3.8以上をインストール

2. 依存パッケージをインストール:
```bash
cd scripts
pip install -r requirements.txt
```

3. Anthropic API キーを設定:
```bash
export ANTHROPIC_API_KEY='your-api-key-here'
```

## 使い方

### 基本的な使い方

```bash
# 10,000問生成（デフォルト）
python generate_questions.py

# テストモード（各ジャンル1問ずつ生成）
python generate_questions.py --test

# 指定した数だけ生成
python generate_questions.py --count 1000

# 特定のジャンルだけ生成
python generate_questions.py --genre programming --count 500
```

### オプション

- `--test`: テストモード（各ジャンル1問ずつ生成して動作確認）
- `--count N`: N問生成（既存の問題数を除く）
- `--genre GENRE`: 特定のジャンルのみ生成

利用可能なジャンル:
- `programming`: プログラミング
- `web_development`: Web開発
- `technology`: テクノロジー
- `it_terminology`: IT用語
- `anime`: アニメ
- `manga`: 漫画
- `game`: ゲーム
- `vtuber_net_culture`: VTuber・ネット文化
- `general_knowledge`: 一般常識
- `geography`: 地理
- `history`: 歴史
- `science`: 科学
- `math`: 数学
- `language`: 言語
- `culture`: 文化

### カスタマイズ

スクリプト内の定数を編集することで、カスタマイズできます:

- `QUESTIONS_PER_BATCH`: 一度に生成する問題数（デフォルト: 50）
- `TOTAL_QUESTIONS`: 目標問題数（デフォルト: 10,000）
- `GENRES`: ジャンル定義と配分比率（weight）

### 問題配分

現在の配分:
- **IT・プログラミング系**: 約40%
  - プログラミング (Python, JavaScript, Rust, アルゴリズム等)
  - Web開発 (React, Vue.js, Node.js等)
  - テクノロジー (OS, ネットワーク, クラウド等)
  - IT用語

- **アニメ・ゲーム・漫画系**: 約30%
  - アニメ (人気作品, 声優, 監督等)
  - 漫画 (少年漫画, 漫画家等)
  - ゲーム (RPG, eスポーツ等)
  - VTuber・ネット文化

- **一般常識・学問**: 約30%
  - 一般常識 (時事, ビジネスマナー等)
  - 地理、歴史、科学、数学、言語、文化

### 問題の特徴

生成される問題は以下の仕様に準拠します:

1. **入力形式**
   - 日本語: ひらがな、カタカナ、アルファベットのみ（漢字不使用）
   - 英語: アルファベット、数字、記号のみ
   - 選択肢は20文字以内を推奨（タイピングしやすさ重視）

2. **オリジナリティ**
   - 既存クイズサイトの模倣ではなく、独自性のある問題
   - 応用的・実践的な問題を含む
   - 複数の知識を組み合わせた問題
   - 最新トレンドやトリビアも織り交ぜる

3. **難易度**
   - 初級から上級まで混在
   - プログラマー・アニメ/ゲームファン向けにやや偏重
   - 一般常識や高校レベルの学問も含む

## 出力

生成された問題は以下のファイルに保存されます:
- `data/questions_ja.json` - 日本語版
- `data/questions_en.json` - 英語版

## 注意事項

- API利用料金が発生します
- 10,000問生成には数時間かかる場合があります
- 500問ごとに中間保存されるため、途中で中断しても安全です
- 既存の問題は保持され、新しい問題が追加されます

## トラブルシューティング

### API キーエラー
```
エラー: ANTHROPIC_API_KEY環境変数が設定されていません
```
→ API キーを設定してください

### レート制限エラー
スクリプトは自動的に1秒間隔で実行されますが、それでもエラーが出る場合は、
`time.sleep(1)` の値を増やしてください。

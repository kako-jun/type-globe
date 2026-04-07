# TypeGlobe - Quiz & Typing Game

クイズとタイピングを融合させた教育的TUIゲーム。多言語対応、オフライン動作。

## ドキュメント

| ファイル | 内容 | 言語 |
|---|---|---|
| `README.md` | エンドユーザー向けの使い方 | 英語（暫定） |
| `docs/overview.md` | ゲームコンセプト・設計思想 | 英語 |
| `docs/spec.md` | ゲームモード仕様・データ構造 | 英語 |
| `docs/roadmap.md` | 完了済み・残タスク（内部運用メモ） | 日本語 |
| `CLAUDE.md` | AI向け内部ドキュメント | 日本語 |

## ソース構成

```
src/
├── main.rs              # エントリーポイント、ゲームモード分岐
├── types.rs             # データ型定義（Question, Player, Ranking, GameMode, Language）
├── config.rs            # 設定管理
├── game/
│   ├── quiz.rs          # クイズモードロジック（スコア計算、正解判定）
│   └── typing.rs        # タイピングモードロジック（WPM計算、正確性測定）
├── io/
│   ├── data_loader.rs   # JSON問題読み込み・パース
│   ├── storage.rs       # ランキング・プレイヤーデータ保存
│   └── typing_texts.rs  # タイピング練習テキスト
└── ui/
    ├── menu.rs          # 言語・モード選択メニュー
    ├── quiz.rs          # クイズUI描画
    └── typing.rs        # タイピングUI描画
```

## 主要な設計判断

- **Rust + ratatui + crossterm**: クロスプラットフォームTUI。軽量で高速な応答性能
- **ローカルJSON永続化**: オフラインファースト。問題・ランキング・プレイヤーデータはすべてローカルJSONファイル
- **ローマ字入力のみ**: 漢字変換を必要としない。4択クイズでは数字キー(1-4)にも対応
- **言語選択は起動時**: 日本語/英語を選択し、問題文・UIが切り替わる
- **ステルスモード**: CLI風偽装表示で仕事中にプレイ可能（フェーズ4で実装予定）
- **問題生成はAI**: scripts/generate_questions.py で生成。品質方針は .claude/quiz-generation-policy.md

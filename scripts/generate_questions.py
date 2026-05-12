#!/usr/bin/env python3
"""
TypeGlobe クイズ問題生成スクリプト
AI APIを使用して大量のクイズ問題を生成します
"""

import json
import os
import sys
import time
import argparse
from pathlib import Path
from typing import List, Dict, Any
import anthropic

# 設定
API_KEY = os.getenv("ANTHROPIC_API_KEY")
OUTPUT_DIR = Path(__file__).parent.parent / "data"
QUESTIONS_PER_BATCH = 50  # 一度に生成する問題数
TOTAL_QUESTIONS = 10000   # 目標問題数（デフォルト）

# ジャンル定義（配分比率付き）
GENRES = {
    # === IT・プログラミング系（重点分野：40%） ===
    "programming": {
        "ja": "プログラミング",
        "topics": ["Python基礎", "JavaScript基礎", "Rust言語", "アルゴリズム", "データ構造", "デザインパターン", "関数型プログラミング", "オブジェクト指向"],
        "weight": 2.5
    },
    "web_development": {
        "ja": "Web開発",
        "topics": ["HTML/CSS", "React", "Vue.js", "Node.js", "TypeScript", "WebAPI", "フロントエンド", "バックエンド"],
        "weight": 2.0
    },
    "technology": {
        "ja": "テクノロジー",
        "topics": ["コンピュータ基礎", "OS", "ネットワーク", "データベース", "クラウド", "Docker", "Git", "セキュリティ"],
        "weight": 2.0
    },
    "it_terminology": {
        "ja": "IT用語",
        "topics": ["略語", "技術用語", "コマンド", "プロトコル", "ツール", "フレームワーク", "ライブラリ", "API"],
        "weight": 1.5
    },

    # === アニメ・ゲーム・漫画系（重点分野：30%） ===
    "anime": {
        "ja": "アニメ",
        "topics": ["人気作品", "声優", "アニメ史", "監督・制作会社", "キャラクター", "主題歌", "劇場版", "深夜アニメ"],
        "weight": 2.0
    },
    "manga": {
        "ja": "漫画",
        "topics": ["少年漫画", "少女漫画", "青年漫画", "漫画家", "名作", "連載雑誌", "漫画賞", "漫画用語"],
        "weight": 1.5
    },
    "game": {
        "ja": "ゲーム",
        "topics": ["RPG", "アクション", "レトロゲーム", "ゲームハード", "eスポーツ", "ゲーム会社", "名作ゲーム", "ゲーム用語"],
        "weight": 2.0
    },
    "vtuber_net_culture": {
        "ja": "VTuber・ネット文化",
        "topics": ["VTuber", "ニコニコ動画", "ネットミーム", "配信文化", "SNS", "ネットスラング", "動画サイト", "インフルエンサー"],
        "weight": 1.0
    },

    # === 一般常識・学問（30%） ===
    "general_knowledge": {
        "ja": "一般常識",
        "topics": ["時事", "ビジネスマナー", "敬語", "冠婚葬祭", "生活の知恵", "法律", "経済", "政治"],
        "weight": 1.5
    },
    "geography": {
        "ja": "地理",
        "topics": ["国の首都", "世界遺産", "国旗", "都道府県", "河川・山脈"],
        "weight": 1.0
    },
    "history": {
        "ja": "歴史",
        "topics": ["日本史", "世界史", "歴史上の人物", "歴史的事件", "文化史"],
        "weight": 1.0
    },
    "science": {
        "ja": "科学",
        "topics": ["物理", "化学", "生物", "地学", "天文学"],
        "weight": 1.0
    },
    "math": {
        "ja": "数学",
        "topics": ["算数", "数学基礎", "図形", "確率・統計", "数学史"],
        "weight": 0.8
    },
    "language": {
        "ja": "言語",
        "topics": ["英単語", "慣用句", "四字熟語", "ことわざ", "語源"],
        "weight": 1.0
    },
    "culture": {
        "ja": "文化",
        "topics": ["音楽", "美術", "映画", "文学", "スポーツ"],
        "weight": 0.7
    }
}


def generate_questions_batch(client: anthropic.Anthropic, genre: str, topic: str, start_id: int, count: int) -> List[Dict[str, Any]]:
    """AIを使って問題を生成"""

    prompt = f"""以下の条件で、クイズ問題を{count}問生成してください。

【条件】
- ジャンル: {genre} ({GENRES[genre]['ja']})
- トピック: {topic}
- 4択問題
- 日本語と英語の両方を含める
- 難易度: 初級から上級まで混在
- 問題は正確で、事実に基づいた内容にする

【オリジナリティ要件】
既存のクイズサイトにありがちな問題ではなく、独自性のある問題を作成してください:
- 単純な定義問題ばかりではなく、応用的・実践的な問題を含める
- 複数の知識を組み合わせた問題を作る
- ユニークな切り口や視点を持つ問題を含める
- 最新のトレンドや話題も取り入れる（{genre}のトピックに関連する範囲で）
- トリビア的な面白い知識も織り交ぜる

【重要な入力仕様】
ユーザーはフリック入力やキーボードで回答します。漢字変換は不要です。
- 日本語の問題文は2系統で出力する
  - `question_text.ja`: 画面表示用。漢字かな交じりを優先する
  - `question_text_reading.ja`: 読み保持用。ひらがな主体で出力する
- 日本語の選択肢: ひらがな、カタカナ、アルファベットのみ使用（漢字は使わない）
  例: ✓「とうきょう」「トウキョウ」「Tokyo」
      ✗「東京」（漢字は不可）
- 英語の選択肢: アルファベット、数字、記号のみ
- 選択肢は簡潔に（タイピングしやすいよう、各選択肢は20文字以内を推奨）
- 固有名詞や専門用語はカタカナまたはアルファベット表記

【出力形式】
以下のJSON配列形式で出力してください（他の説明文は不要）:

[
  {{
    "id": "q{start_id:05d}",
    "genre": "{genre}",
    "question_text": {{
      "ja": "日本語の問題文（表示用。漢字かな交じり）",
      "en": "English question text"
    }},
    "question_text_reading": {{
      "ja": "にほんごのもんだいぶん",
      "en": "English question text"
    }},
    "choices": [
      {{"ja": "せんたくし1", "en": "Choice 1"}},
      {{"ja": "せんたくし2", "en": "Choice 2"}},
      {{"ja": "せんたくし3", "en": "Choice 3"}},
      {{"ja": "せんたくし4", "en": "Choice 4"}}
    ],
    "correct_answer_index": 0,
    "image_path": null
  }}
]

正解のインデックス(correct_answer_index)は0-3の範囲で、ランダムに配置してください。
問題番号は{start_id}から始めて、連番にしてください。"""

    try:
        response = client.messages.create(
            model="claude-sonnet-4-5-20250929",
            max_tokens=8000,
            messages=[{"role": "user", "content": prompt}]
        )

        # レスポンスからJSONを抽出
        content = response.content[0].text

        # JSONブロックを抽出（```json ``` で囲まれている場合を考慮）
        if "```json" in content:
            content = content.split("```json")[1].split("```")[0].strip()
        elif "```" in content:
            content = content.split("```")[1].split("```")[0].strip()

        questions = json.loads(content)
        print(f"  ✓ {len(questions)}問生成: {genre}/{topic}")
        return questions

    except Exception as e:
        print(f"  ✗ エラー: {e}")
        return []


def load_existing_questions() -> Dict[str, List[Dict]]:
    """既存の問題を読み込み"""
    questions = {"ja": [], "en": []}

    for lang in ["ja", "en"]:
        file_path = OUTPUT_DIR / f"questions_{lang}.json"
        if file_path.exists():
            with open(file_path, "r", encoding="utf-8") as f:
                questions[lang] = json.load(f)

    return questions


def save_questions(questions: List[Dict[str, Any]]):
    """問題をJSONファイルに保存"""
    OUTPUT_DIR.mkdir(exist_ok=True)

    # 既存の問題と統合（重複を避ける）
    existing = load_existing_questions()

    # 言語別にファイルを保存（互換性のため）
    for lang in ["ja", "en"]:
        all_questions = existing[lang] + questions

        # IDで重複を除去
        unique_questions = {}
        for q in all_questions:
            unique_questions[q["id"]] = q

        questions_list = list(unique_questions.values())
        questions_list.sort(key=lambda x: x["id"])

        file_path = OUTPUT_DIR / f"questions_{lang}.json"
        with open(file_path, "w", encoding="utf-8") as f:
            json.dump(questions_list, f, ensure_ascii=False, indent=2)

        print(f"保存完了: {file_path} ({len(questions_list)}問)")


def main():
    """メイン処理"""
    parser = argparse.ArgumentParser(description="TypeGlobeクイズ問題生成")
    parser.add_argument("--test", action="store_true", help="テストモード（各ジャンル1問ずつ生成）")
    parser.add_argument("--count", type=int, help="生成する問題数（既存を除く）")
    parser.add_argument("--genre", type=str, help="特定のジャンルのみ生成")
    args = parser.parse_args()

    print("=== TypeGlobe クイズ問題生成 ===\n")

    if not API_KEY:
        print("エラー: ANTHROPIC_API_KEY環境変数が設定されていません")
        print("以下のコマンドで設定してください:")
        print("  export ANTHROPIC_API_KEY='your-api-key'")
        return

    client = anthropic.Anthropic(api_key=API_KEY)

    # 既存の問題数を確認
    existing = load_existing_questions()
    current_count = len(existing["ja"])
    print(f"既存の問題数: {current_count}問\n")

    # 生成数の決定
    if args.test:
        print("【テストモード】各ジャンル1問ずつ生成します\n")
        target_total = current_count + len(GENRES)
    elif args.count:
        target_total = current_count + args.count
    else:
        target_total = TOTAL_QUESTIONS

    if current_count >= target_total:
        print(f"既に{target_total}問以上の問題があります。")
        return

    remaining = target_total - current_count
    print(f"生成目標: {remaining}問\n")

    all_new_questions = []
    question_id = current_count + 1

    # ジャンルのフィルタリング
    genres_to_generate = GENRES
    if args.genre:
        if args.genre in GENRES:
            genres_to_generate = {args.genre: GENRES[args.genre]}
        else:
            print(f"エラー: ジャンル '{args.genre}' は存在しません")
            print(f"利用可能なジャンル: {', '.join(GENRES.keys())}")
            return

    # 重み付けを考慮して各ジャンルの問題数を計算
    total_weight = sum(info.get("weight", 1.0) for info in genres_to_generate.values())
    genre_questions = {}

    for genre, info in genres_to_generate.items():
        if args.test:
            # テストモードでは各ジャンル1問
            genre_questions[genre] = 1
        else:
            weight = info.get("weight", 1.0)
            genre_questions[genre] = int(remaining * weight / total_weight)

    print("各ジャンルの生成数:")
    for genre, count in genre_questions.items():
        print(f"  {GENRES[genre]['ja']:20s}: {count:4d}問 (weight: {GENRES[genre].get('weight', 1.0)})")

    for genre, info in genres_to_generate.items():
        print(f"\n[{info['ja']} / {genre}]")
        topics = info["topics"]
        genre_total = genre_questions[genre]

        if args.test:
            # テストモードでは最初のトピックだけ
            topics = topics[:1]
            questions_per_topic = 1
        else:
            questions_per_topic = max(1, genre_total // len(topics))

        for topic in topics:
            # バッチ処理
            batches = (questions_per_topic + QUESTIONS_PER_BATCH - 1) // QUESTIONS_PER_BATCH

            for batch_idx in range(batches):
                count = min(QUESTIONS_PER_BATCH, questions_per_topic - batch_idx * QUESTIONS_PER_BATCH)
                if count <= 0:
                    break

                questions = generate_questions_batch(client, genre, topic, question_id, count)

                if questions:
                    all_new_questions.extend(questions)
                    question_id += len(questions)

                    # 定期的に保存（データ損失を防ぐ）
                    if not args.test and len(all_new_questions) % 500 == 0:
                        save_questions(all_new_questions)
                        print(f"\n  → 中間保存: {len(all_new_questions)}問")

                # API制限対策
                time.sleep(1)

    # 最終保存
    if all_new_questions:
        save_questions(all_new_questions)
        print(f"\n✅ 完了! 新規生成: {len(all_new_questions)}問")
        print(f"📊 合計: {current_count + len(all_new_questions)}問")
    else:
        print("\n⚠️  新しい問題は生成されませんでした")


if __name__ == "__main__":
    main()

# TypeGlobe - Specification

## Game Modes in Detail

### Quiz Mode
- Displays a question with 4 choices.
- Input: arrow keys (up/down) to navigate, Enter to confirm, or number keys (1-4) to select directly.
- Scoring based on correctness. Progress bar shows completion.
- Final results screen shows score summary.

### Typing Mode
- Displays a word or sentence for the player to type.
- Real-time character coloring: correct characters in green, incorrect in red.
- Measures WPM (Words Per Minute) and accuracy in real-time.
- Statistics shown upon completion.

### Quiz + Typing Mode
- Combines quiz and typing: the player must type the full text of the correct choice (in romaji).
- Scoring considers both correctness and typing speed/accuracy.

### Time Attack 25
- 5x5 panel grid, inspired by the Japanese TV quiz show "Attack 25."
- CPU opponent. Players claim panels by answering correctly.
- Total elapsed time (including thinking time) determines the final ranking.

### RPG Mode (TypeQuest)
- Correct answers earn EXP.
- Level-up system with titles/achievements tied to levels.
- Player progress is persisted locally.

### Stealth Mode
- Disguises the game UI as a CLI tool output.
- Fake displays such as "Analyzing logs..." or "Monitoring data stream..."
- Toggle between stealth and normal display.

## Data Structures

### Question JSON (`data/questions_ja.json`)

```json
[
  {
    "id": "q001",
    "genre": "science",
    "question_text": {
      "ja": "Mizu no kagaku-shiki wa?",
      "en": "What is the chemical formula for water?"
    },
    "choices": [
      {"ja": "CO2", "en": "CO2"},
      {"ja": "H2O", "en": "H2O"},
      {"ja": "O2", "en": "O2"},
      {"ja": "N2", "en": "N2"}
    ],
    "correct_answer_index": 1,
    "image_path": null
  }
]
```

### Player JSON (`player.json`)

```json
{
  "player_name": "User",
  "language": "ja",
  "rpg_stats": {
    "level": 1,
    "exp": 0
  }
}
```

### Ranking JSON (`ranking_xx.json`)

```json
{
  "quiz_mode": [
    {"name": "Player1", "score": 1500}
  ],
  "time_attack": [
    {"name": "PlayerX", "time_seconds": 180}
  ]
}
```

## Source Architecture

```
src/
├── main.rs              # Entry point, game mode branching
├── types.rs             # Data types (Question, Player, Ranking, GameMode, Language)
├── config.rs            # Application-wide configuration
├── game/
│   ├── mod.rs
│   ├── quiz.rs          # Quiz mode logic (scoring, answer checking)
│   └── typing.rs        # Typing mode logic (WPM calculation, accuracy)
├── io/
│   ├── mod.rs
│   ├── data_loader.rs   # JSON question loading and parsing
│   ├── storage.rs       # Ranking/player data persistence
│   └── typing_texts.rs  # Typing exercise text data
└── ui/
    ├── mod.rs
    ├── menu.rs           # Language/mode selection menu
    ├── quiz.rs           # Quiz mode TUI rendering
    └── typing.rs         # Typing mode TUI rendering
```

## Supporting Files

| Path | Description |
|---|---|
| `data/questions_ja.json` | 100 quiz questions (Japanese) |
| `scripts/generate_questions.py` | AI-assisted question generation script |
| `.claude/quiz-generation-policy.md` | Quality guidelines for question generation |

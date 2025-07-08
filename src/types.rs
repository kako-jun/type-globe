use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Question {
    pub id: String,
    pub genre: String,
    pub question_text: HashMap<String, String>,
    pub choices: Vec<HashMap<String, String>>,
    pub correct_answer_index: usize,
    pub image_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Player {
    pub player_name: String,
    pub language: String,
    pub rpg_stats: RpgStats,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RpgStats {
    pub level: u32,
    pub exp: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScoreEntry {
    pub name: String,
    pub score: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TimeEntry {
    pub name: String,
    pub time_seconds: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Ranking {
    pub quiz_mode: Vec<ScoreEntry>,
    pub typing_mode: Vec<ScoreEntry>,
    pub quiz_typing_mode: Vec<ScoreEntry>,
    pub time_attack: Vec<TimeEntry>,
    pub rpg_mode: Vec<ScoreEntry>,
}

#[derive(Debug, Clone)]
pub enum GameMode {
    Quiz,
    Typing,
    QuizTyping,
    TimeAttack,
    Rpg,
    Stealth,
}

#[derive(Debug, Clone)]
pub enum Language {
    Japanese,
    English,
}

impl Language {
    pub fn code(&self) -> &str {
        match self {
            Language::Japanese => "ja",
            Language::English => "en",
        }
    }
}

impl Default for Player {
    fn default() -> Self {
        Player {
            player_name: "Player".to_string(),
            language: "ja".to_string(),
            rpg_stats: RpgStats { level: 1, exp: 0 },
        }
    }
}

impl Default for Ranking {
    fn default() -> Self {
        Ranking {
            quiz_mode: Vec::new(),
            typing_mode: Vec::new(),
            quiz_typing_mode: Vec::new(),
            time_attack: Vec::new(),
            rpg_mode: Vec::new(),
        }
    }
}
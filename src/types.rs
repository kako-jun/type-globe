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

/// Self-best history per language. Local file only — global ordering of
/// players (the actual *ranking*) is reserved for the v0.3.0+ Nostralgic
/// Ranking integration in `type-globe-online`. Per kako-jun rule:
/// "Ranking" is exclusively the world-comparison feature; offline
/// self-bests are "Records".
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Records {
    pub quiz_mode: Vec<ScoreEntry>,
    pub time_attack_25: Vec<TimeEntry>,
    pub hack_and_slash_rpg: Vec<ScoreEntry>,
}

#[derive(Debug, Clone)]
pub enum GameMode {
    Quiz,
    TimeAttack25,
    HackAndSlashRpg,
    Records,
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

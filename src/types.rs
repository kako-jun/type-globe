use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Question {
    pub id: String,
    pub genre: String,
    pub question_text: HashMap<String, String>,
    pub choices: Vec<Choice>,
    pub correct_answer_index: usize,
    pub image_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Choice {
    #[serde(flatten)]
    pub labels: HashMap<String, String>,
    #[serde(default)]
    pub ja_typings: Vec<String>,
}

/// Answer-form classification per `docs/spec.md`. Drives the RPG
/// boss placement (#33-#37: prompts 1-7 word, 8-9 phrase, 10 sentence)
/// and gives the renderer a hint for enemy size / visuals. `Question`
/// will gain this field when the YAML migration lands; `ListeningPrompt`
/// uses it from day one (#29).
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AnswerKind {
    Word,
    Phrase,
    Sentence,
}

/// One audio-only listening prompt. The TTS layer turns `text` into
/// audio at runtime (#28) — no audio files are shipped. The on-disk
/// shape mirrors the v0.2.0 YAML target in `docs/spec.md` so the eventual
/// JSON→YAML migration is a 1:1 rename.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListeningPrompt {
    pub id: String,
    pub text: String,
    pub kind: AnswerKind,
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

/// One row in a Records list. `ts` is RFC3339 format (e.g. "2025-05-11T12:34:56Z").
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScoreEntry {
    pub name: String,
    pub score: u32,
    #[serde(default)]
    pub cpm: u32,
    #[serde(default)]
    pub wpm: u32,
    #[serde(default)]
    pub ts: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TimeEntry {
    pub name: String,
    pub time_seconds: u32,
    #[serde(default)]
    pub ts: String,
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
    pub rpg: Vec<ScoreEntry>,
}

const RECORDS_TOP_N: usize = 10;

impl Records {
    /// Insert into `quiz_mode`, sort by score descending (ts descending as
    /// tiebreaker), and keep only the top 10.
    pub fn push_quiz(&mut self, entry: ScoreEntry) {
        self.quiz_mode.push(entry);
        self.quiz_mode
            .sort_by(|a, b| b.score.cmp(&a.score).then(b.ts.cmp(&a.ts)));
        self.quiz_mode.truncate(RECORDS_TOP_N);
    }

    /// Insert into `rpg`, sort by score descending (ts descending as
    /// tiebreaker), and keep only the top 10.
    #[allow(dead_code)]
    pub fn push_rpg(&mut self, entry: ScoreEntry) {
        self.rpg.push(entry);
        self.rpg
            .sort_by(|a, b| b.score.cmp(&a.score).then(b.ts.cmp(&a.ts)));
        self.rpg.truncate(RECORDS_TOP_N);
    }

    /// Insert into `time_attack_25`, sort by time ascending (shorter = better,
    /// ts descending as tiebreaker), and keep only the top 10.
    #[allow(dead_code)]
    pub fn push_ta25(&mut self, entry: TimeEntry) {
        self.time_attack_25.push(entry);
        self.time_attack_25
            .sort_by(|a, b| a.time_seconds.cmp(&b.time_seconds).then(b.ts.cmp(&a.ts)));
        self.time_attack_25.truncate(RECORDS_TOP_N);
    }
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

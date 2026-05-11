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

#[cfg(test)]
mod tests {
    use super::*;

    fn score_entry(name: &str, score: u32) -> ScoreEntry {
        ScoreEntry {
            name: name.into(),
            score,
            cpm: 0,
            wpm: 0,
            ts: "2025-01-01T00:00:00Z".into(),
        }
    }

    fn time_entry(name: &str, time_seconds: u32) -> TimeEntry {
        TimeEntry {
            name: name.into(),
            time_seconds,
            ts: "2025-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn push_quiz_11_entries_truncates_to_10() {
        let mut records = Records::default();
        for i in 0..11 {
            records.push_quiz(score_entry(&format!("p{i}"), i as u32 * 10));
        }
        assert_eq!(records.quiz_mode.len(), 10);
    }

    #[test]
    fn push_quiz_sorted_score_descending() {
        let mut records = Records::default();
        records.push_quiz(score_entry("low", 100));
        records.push_quiz(score_entry("high", 500));
        records.push_quiz(score_entry("mid", 300));
        assert_eq!(records.quiz_mode[0].score, 500);
        assert_eq!(records.quiz_mode[1].score, 300);
        assert_eq!(records.quiz_mode[2].score, 100);
    }

    #[test]
    fn push_quiz_lowest_score_is_dropped() {
        let mut records = Records::default();
        for i in 0..10 {
            records.push_quiz(score_entry(&format!("p{i}"), (i as u32 + 1) * 100));
        }
        // Score 50 is below the minimum (100), should be discarded
        records.push_quiz(score_entry("loser", 50));
        assert_eq!(records.quiz_mode.len(), 10);
        assert!(records.quiz_mode.iter().all(|e| e.score >= 100));
    }

    #[test]
    fn push_rpg_11_entries_truncates_to_10() {
        let mut records = Records::default();
        for i in 0..11 {
            records.push_rpg(score_entry(&format!("p{i}"), i as u32 * 10));
        }
        assert_eq!(records.rpg.len(), 10);
    }

    #[test]
    fn push_rpg_sorted_score_descending() {
        let mut records = Records::default();
        records.push_rpg(score_entry("low", 200));
        records.push_rpg(score_entry("high", 800));
        records.push_rpg(score_entry("mid", 500));
        assert_eq!(records.rpg[0].score, 800);
        assert_eq!(records.rpg[1].score, 500);
        assert_eq!(records.rpg[2].score, 200);
    }

    #[test]
    fn push_ta25_11_entries_truncates_to_10() {
        let mut records = Records::default();
        for i in 0..11 {
            records.push_ta25(time_entry(&format!("p{i}"), (i as u32 + 1) * 10));
        }
        assert_eq!(records.time_attack_25.len(), 10);
    }

    #[test]
    fn push_ta25_sorted_time_ascending() {
        let mut records = Records::default();
        records.push_ta25(time_entry("slow", 120));
        records.push_ta25(time_entry("fast", 40));
        records.push_ta25(time_entry("mid", 80));
        assert_eq!(records.time_attack_25[0].time_seconds, 40);
        assert_eq!(records.time_attack_25[1].time_seconds, 80);
        assert_eq!(records.time_attack_25[2].time_seconds, 120);
    }

    #[test]
    fn push_ta25_slowest_is_dropped() {
        let mut records = Records::default();
        for i in 0..10 {
            records.push_ta25(time_entry(&format!("p{i}"), (i as u32 + 1) * 10));
        }
        // Time 9999 is above the maximum kept (100s), should be discarded
        records.push_ta25(time_entry("tortoise", 9999));
        assert_eq!(records.time_attack_25.len(), 10);
        assert!(records.time_attack_25.iter().all(|e| e.time_seconds <= 100));
    }
}

#[derive(Debug, Clone)]
pub enum GameMode {
    Quiz,
    TimeAttack25,
    Rpg,
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

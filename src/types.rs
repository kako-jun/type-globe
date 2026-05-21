use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Question {
    pub id: String,
    pub genre: String,
    pub question_text: HashMap<String, String>,
    /// Per-language reading form (hiragana for ja, identical to display for en).
    /// Used by TTS / RPG audio paths. Quiz mode does not read the prompt aloud,
    /// so quiz entries may leave this empty; `get_question_reading_text` falls
    /// back to `question_text` automatically.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub question_text_reading: HashMap<String, String>,
    pub choices: Vec<Choice>,
    pub correct_answer_index: usize,
    pub image_path: Option<String>,
    /// True when every kanji-containing `ja_typings` in this question has been
    /// per-entry reviewed (see CLAUDE.md "ja_typings 全件チェック手順").
    /// Newly generated questions default to false; the lint binary reports
    /// the unreviewed count so the backlog can be drained over time.
    #[serde(default)]
    pub ja_reviewed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Choice {
    #[serde(flatten)]
    pub labels: HashMap<String, String>,
    #[serde(default)]
    pub ja_typings: Vec<String>,
}

/// Answer-form classification per `docs/spec.md`. Used for ordinary
/// listening-enemy flavor / sizing and as a hint for future run pacing.
/// Boss encounters may override the plain dictation format with their
/// own presentation rules. `Question` will gain this field when the YAML
/// migration lands; `ListeningPrompt` uses it from day one (#29).
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AnswerKind {
    Word,
    Phrase,
    Sentence,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BossTier {
    Miniboss,
    Boss,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BossHintRevealMode {
    Manual,
    Timed,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct BossHintStep {
    /// Short label shown in the hint stack, e.g. "Part of speech".
    pub label: String,
    /// Human-readable text rendered on screen.
    pub text_display: String,
    /// Optional TTS-specific reading form. Falls back to `text_display`
    /// when omitted, so a hint can stay text-only or share one string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_reading: Option<String>,
    /// For timed reveal mode only. Manual mode ignores this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_reveal_after_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ListeningBossSpec {
    pub tier: BossTier,
    pub reveal_mode: BossHintRevealMode,
    #[serde(deserialize_with = "deserialize_non_empty_boss_hints")]
    pub hints: Vec<BossHintStep>,
}

fn deserialize_non_empty_boss_hints<'de, D>(deserializer: D) -> Result<Vec<BossHintStep>, D::Error>
where
    D: Deserializer<'de>,
{
    let hints = Vec::<BossHintStep>::deserialize(deserializer)?;
    if hints.is_empty() {
        return Err(serde::de::Error::custom(
            "boss.hints must contain at least one hint step",
        ));
    }
    Ok(hints)
}

/// One audio-only listening prompt. The TTS layer turns `text_reading`
/// into audio at runtime (#28) — no audio files are shipped.
/// `text_reading` is hiragana-only (JA) or plain English, used for TTS
/// and romaji conversion. `text_display` is the human-readable form
/// (kanji/katakana for JA; identical to `text_reading` for EN).
/// `text_display` is shown on the result screen after the player answers.
/// TODO(#33): wire `text_display` into the result/log pane of the RPG UI.
#[derive(Debug, Clone)]
pub struct ListeningPrompt {
    pub id: String,
    pub text_reading: String,
    pub text_display: String,
    pub kind: AnswerKind,
    /// Optional boss-encounter override for layered reverse-Akinator
    /// presentation. Ordinary listening prompts leave this unset.
    pub boss: Option<ListeningBossSpec>,
}

#[derive(Debug, Deserialize)]
struct ListeningPromptWire {
    id: String,
    text_reading: String,
    text_display: String,
    kind: AnswerKind,
    #[serde(default)]
    boss: Option<ListeningBossSpec>,
}

impl<'de> Deserialize<'de> for ListeningPrompt {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ListeningPromptWire::deserialize(deserializer)?;
        if wire.boss.is_some() && wire.kind != AnswerKind::Word {
            return Err(serde::de::Error::custom(
                "boss prompts must stay kind=word until the listening input model supports spaces during active play",
            ));
        }
        Ok(Self {
            id: wire.id,
            text_reading: wire.text_reading,
            text_display: wire.text_display,
            kind: wire.kind,
            boss: wire.boss,
        })
    }
}

impl Serialize for ListeningPrompt {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.boss.is_some() && self.kind != AnswerKind::Word {
            return Err(serde::ser::Error::custom(
                "boss prompts must stay kind=word until the listening input model supports spaces during active play",
            ));
        }

        #[derive(Serialize)]
        struct ListeningPromptWireRef<'a> {
            id: &'a str,
            text_reading: &'a str,
            text_display: &'a str,
            kind: AnswerKind,
            #[serde(default, skip_serializing_if = "Option::is_none")]
            boss: &'a Option<ListeningBossSpec>,
        }

        ListeningPromptWireRef {
            id: &self.id,
            text_reading: &self.text_reading,
            text_display: &self.text_display,
            kind: self.kind,
            boss: &self.boss,
        }
        .serialize(serializer)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Player {
    pub player_name: String,
    pub language: String,
    #[serde(default)]
    pub rpg_stats: RpgStats,
}

/// Per-player RPG progression. Phase 1 (#32) only persists the data —
/// EXP/level updates and title-unlock logic land in Phase 2 (#34/#35).
/// `#[serde(default)]` on every field keeps legacy `player.yaml` files
/// loadable when new columns are added later in the same Phase 1 wave.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RpgStats {
    #[serde(default = "RpgStats::default_level")]
    pub level: u32,
    #[serde(default)]
    pub exp: u32,
    #[serde(default = "RpgStats::default_hp_max")]
    pub hp_max: u32,
    #[serde(default)]
    pub titles_unlocked: Vec<String>,
}

impl RpgStats {
    fn default_level() -> u32 {
        1
    }

    fn default_hp_max() -> u32 {
        100
    }
}

impl Default for RpgStats {
    fn default() -> Self {
        RpgStats {
            level: Self::default_level(),
            exp: 0,
            hp_max: Self::default_hp_max(),
            titles_unlocked: Vec::new(),
        }
    }
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
    use serde_yaml::Value;

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

    #[test]
    fn listening_prompt_without_boss_metadata_still_deserializes() {
        let yaml = r#"
id: l-en-001
text_reading: apple
text_display: apple
kind: word
"#;
        let prompt: ListeningPrompt = serde_yaml::from_str(yaml).expect("prompt");
        assert!(prompt.boss.is_none());
    }

    #[test]
    fn listening_prompt_with_boss_metadata_deserializes() {
        let yaml = r#"
id: l-en-boss-010
text_reading: ticket
text_display: ticket
kind: word
boss:
  tier: boss
  reveal_mode: manual
  hints:
    - label: Part of speech
      text_display: proper noun
    - label: First letter
      text_display: t
      text_reading: tee
      auto_reveal_after_ms: 2500
"#;
        let prompt: ListeningPrompt = serde_yaml::from_str(yaml).expect("prompt");
        let boss = prompt.boss.expect("boss metadata");
        assert_eq!(boss.tier, BossTier::Boss);
        assert_eq!(boss.reveal_mode, BossHintRevealMode::Manual);
        assert_eq!(boss.hints.len(), 2);
        assert_eq!(boss.hints[1].text_reading.as_deref(), Some("tee"));
    }

    #[test]
    fn boss_prompt_rejects_non_word_answer_kind() {
        let yaml = r#"
id: l-en-boss-011
text_reading: tokyo station
text_display: Tokyo Station
kind: phrase
boss:
  tier: boss
  reveal_mode: manual
  hints:
    - label: Category
      text_display: station
"#;
        let err = serde_yaml::from_str::<ListeningPrompt>(yaml).expect_err("phrase boss rejected");
        assert!(err.to_string().contains("kind=word"));
    }

    #[test]
    fn boss_metadata_requires_explicit_reveal_mode() {
        let yaml = r#"
tier: miniboss
hints:
  - label: Category
    text_display: place
"#;
        let err = serde_yaml::from_str::<ListeningBossSpec>(yaml).expect_err("missing reveal_mode");
        assert!(err.to_string().contains("reveal_mode"));
    }

    #[test]
    fn boss_metadata_rejects_empty_hints() {
        let yaml = r#"
tier: boss
reveal_mode: manual
hints: []
"#;
        let err = serde_yaml::from_str::<ListeningBossSpec>(yaml).expect_err("empty hints");
        assert!(err.to_string().contains("at least one hint"));
    }

    #[test]
    fn boss_metadata_serializes_cleanly() {
        let prompt = ListeningPrompt {
            id: "boss".into(),
            text_reading: "ticket".into(),
            text_display: "ticket".into(),
            kind: AnswerKind::Word,
            boss: Some(ListeningBossSpec {
                tier: BossTier::Miniboss,
                reveal_mode: BossHintRevealMode::Timed,
                hints: vec![BossHintStep {
                    label: "Used when".into(),
                    text_display: "discussing trains".into(),
                    text_reading: None,
                    auto_reveal_after_ms: Some(3000),
                }],
            }),
        };
        let value = serde_yaml::to_value(prompt).expect("yaml value");
        let boss = value
            .get("boss")
            .and_then(Value::as_mapping)
            .expect("boss mapping");
        assert!(boss.contains_key(Value::from("tier")));
        assert!(boss.contains_key(Value::from("hints")));
    }

    #[test]
    fn invalid_boss_prompt_is_rejected_on_serialize_too() {
        let prompt = ListeningPrompt {
            id: "boss-bad".into(),
            text_reading: "tokyo station".into(),
            text_display: "Tokyo Station".into(),
            kind: AnswerKind::Phrase,
            boss: Some(ListeningBossSpec {
                tier: BossTier::Boss,
                reveal_mode: BossHintRevealMode::Manual,
                hints: vec![BossHintStep {
                    label: "Category".into(),
                    text_display: "station".into(),
                    text_reading: None,
                    auto_reveal_after_ms: None,
                }],
            }),
        };
        let err = serde_yaml::to_string(&prompt).expect_err("serialize rejects invalid boss");
        assert!(err.to_string().contains("kind=word"));
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
            rpg_stats: RpgStats::default(),
        }
    }
}

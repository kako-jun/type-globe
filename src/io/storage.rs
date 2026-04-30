use crate::types::{Player, Records};
use std::fs;
use std::path::Path;

pub struct Storage;

impl Storage {
    pub fn ensure_data_directory(data_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        if !Path::new(data_dir).exists() {
            fs::create_dir_all(data_dir)?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn load_player_data(file_path: &str) -> Result<Player, Box<dyn std::error::Error>> {
        if !Path::new(file_path).exists() {
            return Ok(Player::default());
        }

        let content = fs::read_to_string(file_path)?;
        let player: Player = serde_json::from_str(&content)?;
        Ok(player)
    }

    #[allow(dead_code)]
    pub fn save_player_data(
        file_path: &str,
        player: &Player,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(player)?;
        fs::write(file_path, content)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn load_records(file_path: &str) -> Result<Records, Box<dyn std::error::Error>> {
        if !Path::new(file_path).exists() {
            return Ok(Records::default());
        }

        let content = fs::read_to_string(file_path)?;
        let records: Records = serde_json::from_str(&content)?;
        Ok(records)
    }

    #[allow(dead_code)]
    pub fn save_records(
        file_path: &str,
        records: &Records,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(records)?;
        fs::write(file_path, content)?;
        Ok(())
    }

    pub fn save_sample_questions(
        file_path: &str,
        questions: &[crate::types::Question],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(questions)?;
        fs::write(file_path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ScoreEntry;
    use std::env::temp_dir;

    fn unique_path(prefix: &str) -> String {
        // Nanosecond clock as a quick unique suffix — good enough for the
        // single-process test runner; std lacks a built-in tempfile helper.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = temp_dir();
        format!("{}/type-globe-{prefix}-{nanos}.json", dir.display())
    }

    #[test]
    fn load_records_returns_empty_when_file_absent() {
        let path = unique_path("missing");
        let records = Storage::load_records(&path).expect("load");
        assert!(records.quiz_mode.is_empty());
        assert!(records.time_attack_25.is_empty());
        assert!(records.hack_and_slash_rpg.is_empty());
    }

    #[test]
    fn save_then_load_records_round_trip() {
        let path = unique_path("roundtrip");
        let mut records = Records::default();
        records.quiz_mode.push(ScoreEntry {
            name: "Alice".into(),
            score: 1500,
            cpm: 230,
            wpm: 46,
            ts: 17_280_000,
        });
        Storage::save_records(&path, &records).expect("save");

        let loaded = Storage::load_records(&path).expect("load");
        assert_eq!(loaded.quiz_mode.len(), 1);
        assert_eq!(loaded.quiz_mode[0].name, "Alice");
        assert_eq!(loaded.quiz_mode[0].score, 1500);
        assert_eq!(loaded.quiz_mode[0].cpm, 230);
        assert_eq!(loaded.quiz_mode[0].wpm, 46);
        assert_eq!(loaded.quiz_mode[0].ts, 17_280_000);

        let _ = std::fs::remove_file(&path);
    }
}

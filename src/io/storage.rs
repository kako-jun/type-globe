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
        let player: Player = serde_yaml::from_str(&content)?;
        Ok(player)
    }

    #[allow(dead_code)]
    pub fn save_player_data(
        file_path: &str,
        player: &Player,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_yaml::to_string(player)?;
        fs::write(file_path, content)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn load_records(file_path: &str) -> Result<Records, Box<dyn std::error::Error>> {
        if !Path::new(file_path).exists() {
            return Ok(Records::default());
        }

        let content = fs::read_to_string(file_path)?;
        let records: Records = serde_yaml::from_str(&content)?;
        Ok(records)
    }

    #[allow(dead_code)]
    pub fn save_records(
        file_path: &str,
        records: &Records,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_yaml::to_string(records)?;
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
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = temp_dir();
        format!("{}/type-globe-{prefix}-{nanos}.yaml", dir.display())
    }

    #[test]
    fn load_records_returns_empty_when_file_absent() {
        let path = unique_path("missing");
        let records = Storage::load_records(&path).expect("load");
        assert!(records.quiz_mode.is_empty());
        assert!(records.time_attack_25.is_empty());
        assert!(records.rpg.is_empty());
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
            ts: "2025-05-11T00:00:00Z".into(),
        });
        Storage::save_records(&path, &records).expect("save");

        let loaded = Storage::load_records(&path).expect("load");
        assert_eq!(loaded.quiz_mode.len(), 1);
        assert_eq!(loaded.quiz_mode[0].name, "Alice");
        assert_eq!(loaded.quiz_mode[0].score, 1500);
        assert_eq!(loaded.quiz_mode[0].cpm, 230);
        assert_eq!(loaded.quiz_mode[0].wpm, 46);
        assert_eq!(loaded.quiz_mode[0].ts, "2025-05-11T00:00:00Z");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_then_load_records_rpg_round_trip() {
        let path = unique_path("rpg-roundtrip");
        let mut records = Records::default();
        records.rpg.push(ScoreEntry {
            name: "Bob".into(),
            score: 2500,
            cpm: 310,
            wpm: 62,
            ts: "2025-05-11T10:00:00Z".into(),
        });
        Storage::save_records(&path, &records).expect("save");

        let loaded = Storage::load_records(&path).expect("load");
        assert_eq!(loaded.rpg.len(), 1);
        assert_eq!(loaded.rpg[0].name, "Bob");
        assert_eq!(loaded.rpg[0].score, 2500);
        assert_eq!(loaded.rpg[0].cpm, 310);
        assert_eq!(loaded.rpg[0].wpm, 62);
        assert_eq!(loaded.rpg[0].ts, "2025-05-11T10:00:00Z");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_then_load_records_quiz_mode_round_trip() {
        let path = unique_path("quiz-roundtrip");
        let mut records = Records::default();
        records.quiz_mode.push(ScoreEntry {
            name: "Carol".into(),
            score: 1800,
            cpm: 270,
            wpm: 54,
            ts: "2025-05-11T11:00:00Z".into(),
        });
        Storage::save_records(&path, &records).expect("save");

        let loaded = Storage::load_records(&path).expect("load");
        assert_eq!(loaded.quiz_mode.len(), 1);
        assert_eq!(loaded.quiz_mode[0].name, "Carol");
        assert_eq!(loaded.quiz_mode[0].score, 1800);
        assert_eq!(loaded.quiz_mode[0].ts, "2025-05-11T11:00:00Z");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_player_data_returns_default_when_file_absent() {
        let path = unique_path("player-missing");
        let _ = std::fs::remove_file(&path);
        let player = Storage::load_player_data(&path).expect("load");
        assert_eq!(player.player_name, "Player");
        assert_eq!(player.language, "ja");
        assert_eq!(player.rpg_stats.level, 1);
        assert_eq!(player.rpg_stats.exp, 0);
        assert_eq!(player.rpg_stats.hp_max, 100);
        assert!(player.rpg_stats.titles_unlocked.is_empty());
    }

    #[test]
    fn save_then_load_player_data_round_trip() {
        let path = unique_path("player-roundtrip");
        let player = crate::types::Player {
            player_name: "Goku".into(),
            language: "en".into(),
            rpg_stats: crate::types::RpgStats {
                level: 5,
                exp: 1234,
                hp_max: 150,
                titles_unlocked: vec!["Apprentice".into(), "Veteran".into()],
            },
        };

        Storage::save_player_data(&path, &player).expect("save");

        let loaded = Storage::load_player_data(&path).expect("load");
        assert_eq!(loaded.player_name, "Goku");
        assert_eq!(loaded.language, "en");
        assert_eq!(loaded.rpg_stats.level, 5);
        assert_eq!(loaded.rpg_stats.exp, 1234);
        assert_eq!(loaded.rpg_stats.hp_max, 150);
        assert_eq!(
            loaded.rpg_stats.titles_unlocked,
            vec!["Apprentice".to_string(), "Veteran".to_string()]
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_player_data_accepts_legacy_yaml_without_new_fields() {
        // Legacy player.yaml from before #32 had `rpg_stats: { level, exp }`
        // only. New fields must default rather than refuse to deserialize.
        let path = unique_path("player-legacy");
        std::fs::write(
            &path,
            "player_name: Legacy\nlanguage: ja\nrpg_stats:\n  level: 3\n  exp: 200\n",
        )
        .expect("write legacy");

        let loaded = Storage::load_player_data(&path).expect("load");
        assert_eq!(loaded.player_name, "Legacy");
        assert_eq!(loaded.rpg_stats.level, 3);
        assert_eq!(loaded.rpg_stats.exp, 200);
        assert_eq!(loaded.rpg_stats.hp_max, 100);
        assert!(loaded.rpg_stats.titles_unlocked.is_empty());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_records_file_absent_returns_default() {
        let path = unique_path("absent-yaml");
        // Ensure the file does not exist
        let _ = std::fs::remove_file(&path);
        let records = Storage::load_records(&path).expect("load");
        assert!(records.quiz_mode.is_empty());
        assert!(records.time_attack_25.is_empty());
        assert!(records.rpg.is_empty());
    }
}

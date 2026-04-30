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

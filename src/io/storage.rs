use crate::types::{Player, Ranking};
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

    pub fn load_player_data(file_path: &str) -> Result<Player, Box<dyn std::error::Error>> {
        if !Path::new(file_path).exists() {
            return Ok(Player::default());
        }

        let content = fs::read_to_string(file_path)?;
        let player: Player = serde_json::from_str(&content)?;
        Ok(player)
    }

    pub fn save_player_data(file_path: &str, player: &Player) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(player)?;
        fs::write(file_path, content)?;
        Ok(())
    }

    pub fn load_ranking(file_path: &str) -> Result<Ranking, Box<dyn std::error::Error>> {
        if !Path::new(file_path).exists() {
            return Ok(Ranking::default());
        }

        let content = fs::read_to_string(file_path)?;
        let ranking: Ranking = serde_json::from_str(&content)?;
        Ok(ranking)
    }

    pub fn save_ranking(file_path: &str, ranking: &Ranking) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(ranking)?;
        fs::write(file_path, content)?;
        Ok(())
    }

    pub fn save_sample_questions(file_path: &str, questions: &[crate::types::Question]) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(questions)?;
        fs::write(file_path, content)?;
        Ok(())
    }
}
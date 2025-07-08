use crate::types::Language;

pub struct Config {
    pub data_dir: String,
    pub default_language: Language,
    pub questions_file_pattern: String,
    pub player_data_file: String,
    pub ranking_file_pattern: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            data_dir: "data".to_string(),
            default_language: Language::Japanese,
            questions_file_pattern: "questions_{}.json".to_string(),
            player_data_file: "player.json".to_string(),
            ranking_file_pattern: "ranking_{}.json".to_string(),
        }
    }
}

impl Config {
    pub fn questions_file_path(&self, language: &Language) -> String {
        format!("{}/{}", self.data_dir, self.questions_file_pattern.replace("{}", language.code()))
    }

    pub fn player_data_file_path(&self) -> String {
        format!("{}/{}", self.data_dir, self.player_data_file)
    }

    pub fn ranking_file_path(&self, language: &Language) -> String {
        format!("{}/{}", self.data_dir, self.ranking_file_pattern.replace("{}", language.code()))
    }
}
use crate::types::Language;

pub struct Config {
    pub data_dir: String,
    #[allow(dead_code)]
    pub default_language: Language,
    pub questions_file_pattern: String,
    #[allow(dead_code)]
    pub player_data_file: String,
    #[allow(dead_code)]
    pub records_file_pattern: String,
    pub listening_file_pattern: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            data_dir: "data".to_string(),
            default_language: Language::Japanese,
            questions_file_pattern: "questions_{}.json".to_string(),
            player_data_file: "player.yaml".to_string(),
            records_file_pattern: "records_{}.yaml".to_string(),
            listening_file_pattern: "listening_{}.yaml".to_string(),
        }
    }
}

impl Config {
    pub fn questions_file_path(&self, language: &Language) -> String {
        format!(
            "{}/{}",
            self.data_dir,
            self.questions_file_pattern.replace("{}", language.code())
        )
    }

    #[allow(dead_code)]
    pub fn player_data_file_path(&self) -> String {
        format!("{}/{}", self.data_dir, self.player_data_file)
    }

    #[allow(dead_code)]
    pub fn records_file_path(&self, language: &Language) -> String {
        format!(
            "{}/{}",
            self.data_dir,
            self.records_file_pattern.replace("{}", language.code())
        )
    }

    pub fn listening_file_path(&self, language: &Language) -> String {
        format!(
            "{}/{}",
            self.data_dir,
            self.listening_file_pattern.replace("{}", language.code())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listening_file_path_uses_language_code() {
        let cfg = Config::default();
        assert_eq!(
            cfg.listening_file_path(&Language::Japanese),
            "data/listening_ja.yaml"
        );
        assert_eq!(
            cfg.listening_file_path(&Language::English),
            "data/listening_en.yaml"
        );
    }

    #[test]
    fn questions_file_path_uses_language_code() {
        let cfg = Config::default();
        assert_eq!(
            cfg.questions_file_path(&Language::Japanese),
            "data/questions_ja.json"
        );
    }
}

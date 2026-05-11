use crate::io::romaji::{hiragana_to_hepburn, hiragana_to_hepburn_variants};
use crate::types::{Choice, Language, ListeningPrompt, Question};
use std::fs;
use std::path::Path;

// Embedded question banks — shipped inside the binary so `cargo install`
// users don't need to supply external data files.
const BUNDLED_QUESTIONS_JA: &str = include_str!("../../data/questions_ja.json");
const BUNDLED_QUESTIONS_EN: &str = include_str!("../../data/questions_en.json");
const BUNDLED_LISTENING_JA: &str = include_str!("../../data/listening_ja.yaml");
const BUNDLED_LISTENING_EN: &str = include_str!("../../data/listening_en.yaml");

pub struct DataLoader;

impl DataLoader {
    /// Return the bundled question JSON for `language`, or `None` for
    /// languages that have no bundled data.
    fn bundled_questions_json(language: &Language) -> Option<&'static str> {
        match language {
            Language::Japanese => Some(BUNDLED_QUESTIONS_JA),
            Language::English => Some(BUNDLED_QUESTIONS_EN),
        }
    }

    /// Return the bundled listening YAML for `language`, or `None` for
    /// languages that have no bundled data.
    fn bundled_listening_yaml(language: &Language) -> Option<&'static str> {
        match language {
            Language::Japanese => Some(BUNDLED_LISTENING_JA),
            Language::English => Some(BUNDLED_LISTENING_EN),
        }
    }

    pub fn load_questions(file_path: &str) -> Result<Vec<Question>, Box<dyn std::error::Error>> {
        // Prefer the on-disk file (allows users to add/override questions).
        if Path::new(file_path).exists() {
            let content = fs::read_to_string(file_path)?;
            let questions: Vec<Question> = serde_json::from_str(&content)?;
            return Ok(questions);
        }

        // Fall back to bundled data keyed by the language suffix in the path.
        let lang = if file_path.contains("_en") {
            Language::English
        } else if file_path.contains("_ja") {
            Language::Japanese
        } else {
            return Ok(Vec::new());
        };
        if let Some(json) = Self::bundled_questions_json(&lang) {
            let questions: Vec<Question> = serde_json::from_str(json)?;
            return Ok(questions);
        }

        Ok(Vec::new())
    }

    /// Load a listening-prompt bank for a single language (#29). The
    /// caller picks the file via `Config::listening_file_path`. Empty
    /// vector when the file is absent — same convention as
    /// `load_questions` so the Listening UI can degrade to a "no
    /// prompts shipped" message instead of a hard error.
    pub fn load_listening_prompts(
        file_path: &str,
    ) -> Result<Vec<ListeningPrompt>, Box<dyn std::error::Error>> {
        // Prefer the on-disk file.
        if Path::new(file_path).exists() {
            let content = fs::read_to_string(file_path)?;
            let prompts: Vec<ListeningPrompt> = serde_yaml::from_str(&content)?;
            return Ok(prompts);
        }

        // Fall back to bundled data.
        let lang = if file_path.contains("_en") {
            Language::English
        } else if file_path.contains("_ja") {
            Language::Japanese
        } else {
            return Ok(Vec::new());
        };
        if let Some(yaml) = Self::bundled_listening_yaml(&lang) {
            let prompts: Vec<ListeningPrompt> = serde_yaml::from_str(yaml)?;
            return Ok(prompts);
        }

        Ok(Vec::new())
    }

    #[allow(dead_code)]
    pub fn filter_questions_by_genre(questions: &[Question], genre: Option<&str>) -> Vec<Question> {
        match genre {
            Some(g) => questions.iter().filter(|q| q.genre == g).cloned().collect(),
            None => questions.to_vec(),
        }
    }

    pub fn get_question_text(question: &Question, language: &Language) -> String {
        question
            .question_text
            .get(language.code())
            .cloned()
            .unwrap_or_else(|| {
                question
                    .question_text
                    .values()
                    .next()
                    .cloned()
                    .unwrap_or_default()
            })
    }

    pub fn get_choice_text(choice: &Choice, language: &Language) -> String {
        choice
            .labels
            .get(language.code())
            .cloned()
            .unwrap_or_else(|| choice.labels.values().next().cloned().unwrap_or_default())
    }

    pub fn get_choice_typing_texts(choice: &Choice, language: &Language) -> Vec<String> {
        match language {
            Language::Japanese => {
                let mut variants = Vec::new();
                for typing in &choice.ja_typings {
                    variants.push(typing.to_lowercase());
                }
                let displayed = Self::get_choice_text(choice, language);
                if displayed.is_ascii() {
                    variants.push(displayed.to_lowercase());
                } else {
                    variants.extend(
                        hiragana_to_hepburn_variants(&displayed)
                            .into_iter()
                            .filter(|candidate| !candidate.is_empty()),
                    );
                    let canonical = hiragana_to_hepburn(&displayed);
                    if !canonical.is_empty() {
                        variants.push(canonical);
                    }
                }
                variants.sort();
                variants.dedup();
                variants
            }
            Language::English => vec![Self::get_choice_text(choice, language)],
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AnswerKind;

    #[test]
    fn shipped_listening_data_loads_for_both_languages() {
        // The listening foundation epic ships these files in `data/`; if a
        // future change accidentally breaks the schema (e.g. missing
        // `kind`, typo'd serde rename) this test fails at PR time rather
        // than at runtime when the player picks Listening Practice.
        let ja = DataLoader::load_listening_prompts("data/listening_ja.yaml").expect("ja loads");
        let en = DataLoader::load_listening_prompts("data/listening_en.yaml").expect("en loads");
        assert!(!ja.is_empty(), "ja prompts non-empty");
        assert!(!en.is_empty(), "en prompts non-empty");
        // Foundation flow only exposes word-kind prompts (Space is
        // reserved for replay) — fail loudly if no word survives the
        // filter, which would brick the Listening Practice mode.
        assert!(
            ja.iter().any(|p| p.kind == AnswerKind::Word),
            "at least one word-kind ja prompt must ship"
        );
        assert!(
            en.iter().any(|p| p.kind == AnswerKind::Word),
            "at least one word-kind en prompt must ship"
        );
    }

    #[test]
    fn load_listening_prompts_returns_empty_when_missing() {
        let prompts = DataLoader::load_listening_prompts("data/__does_not_exist__.yaml")
            .expect("missing file is not an error");
        assert!(prompts.is_empty());
    }

    #[test]
    fn ja_choice_typing_prefers_explicit_field() {
        let choice = Choice {
            labels: std::collections::HashMap::from([
                ("ja".to_string(), "東京".to_string()),
                ("en".to_string(), "Tokyo".to_string()),
            ]),
            ja_typings: vec!["tokyo".to_string()],
        };
        assert_eq!(
            DataLoader::get_choice_typing_texts(&choice, &Language::Japanese),
            vec!["tokyo".to_string()]
        );
    }

    #[test]
    fn ja_choice_typing_includes_long_vowel_aliases() {
        let choice = Choice {
            labels: std::collections::HashMap::from([
                ("ja".to_string(), "とうきょう".to_string()),
                ("en".to_string(), "Tokyo".to_string()),
            ]),
            ja_typings: vec!["tokyo".to_string()],
        };
        assert_eq!(
            DataLoader::get_choice_typing_texts(&choice, &Language::Japanese),
            vec!["tokyo".to_string(), "toukyou".to_string()]
        );
    }
}

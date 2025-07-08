use crate::types::{Question, Language};
use std::fs;
use std::path::Path;

pub struct DataLoader;

impl DataLoader {
    pub fn load_questions(file_path: &str) -> Result<Vec<Question>, Box<dyn std::error::Error>> {
        if !Path::new(file_path).exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(file_path)?;
        let questions: Vec<Question> = serde_json::from_str(&content)?;
        Ok(questions)
    }

    pub fn filter_questions_by_genre(questions: &[Question], genre: Option<&str>) -> Vec<Question> {
        match genre {
            Some(g) => questions.iter()
                .filter(|q| q.genre == g)
                .cloned()
                .collect(),
            None => questions.to_vec(),
        }
    }

    pub fn get_question_text(question: &Question, language: &Language) -> String {
        question.question_text
            .get(language.code())
            .cloned()
            .unwrap_or_else(|| {
                question.question_text
                    .values()
                    .next()
                    .cloned()
                    .unwrap_or_default()
            })
    }

    pub fn get_choice_text(choice: &std::collections::HashMap<String, String>, language: &Language) -> String {
        choice
            .get(language.code())
            .cloned()
            .unwrap_or_else(|| {
                choice
                    .values()
                    .next()
                    .cloned()
                    .unwrap_or_default()
            })
    }

    pub fn create_sample_questions() -> Vec<Question> {
        use std::collections::HashMap;

        vec![
            Question {
                id: "q001".to_string(),
                genre: "science".to_string(),
                question_text: {
                    let mut map = HashMap::new();
                    map.insert("ja".to_string(), "水の化学式は？".to_string());
                    map.insert("en".to_string(), "What is the chemical formula for water?".to_string());
                    map
                },
                choices: vec![
                    {
                        let mut map = HashMap::new();
                        map.insert("ja".to_string(), "CO2".to_string());
                        map.insert("en".to_string(), "CO2".to_string());
                        map
                    },
                    {
                        let mut map = HashMap::new();
                        map.insert("ja".to_string(), "H2O".to_string());
                        map.insert("en".to_string(), "H2O".to_string());
                        map
                    },
                    {
                        let mut map = HashMap::new();
                        map.insert("ja".to_string(), "O2".to_string());
                        map.insert("en".to_string(), "O2".to_string());
                        map
                    },
                    {
                        let mut map = HashMap::new();
                        map.insert("ja".to_string(), "N2".to_string());
                        map.insert("en".to_string(), "N2".to_string());
                        map
                    },
                ],
                correct_answer_index: 1,
                image_path: None,
            },
            Question {
                id: "q002".to_string(),
                genre: "geography".to_string(),
                question_text: {
                    let mut map = HashMap::new();
                    map.insert("ja".to_string(), "日本の首都は？".to_string());
                    map.insert("en".to_string(), "What is the capital of Japan?".to_string());
                    map
                },
                choices: vec![
                    {
                        let mut map = HashMap::new();
                        map.insert("ja".to_string(), "大阪".to_string());
                        map.insert("en".to_string(), "Osaka".to_string());
                        map
                    },
                    {
                        let mut map = HashMap::new();
                        map.insert("ja".to_string(), "東京".to_string());
                        map.insert("en".to_string(), "Tokyo".to_string());
                        map
                    },
                    {
                        let mut map = HashMap::new();
                        map.insert("ja".to_string(), "京都".to_string());
                        map.insert("en".to_string(), "Kyoto".to_string());
                        map
                    },
                    {
                        let mut map = HashMap::new();
                        map.insert("ja".to_string(), "名古屋".to_string());
                        map.insert("en".to_string(), "Nagoya".to_string());
                        map
                    },
                ],
                correct_answer_index: 1,
                image_path: None,
            },
        ]
    }
}
use crate::io::DataLoader;
use crate::types::{Language, Question};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct QuizGame {
    questions: Vec<Question>,
    current_question_index: usize,
    score: u32,
    correct_answers: u32,
    total_answers: u32,
    start_time: Option<Instant>,
    language: Language,
}

#[derive(Debug, Clone)]
pub struct QuizResult {
    pub is_correct: bool,
    pub correct_answer_index: usize,
    #[allow(dead_code)]
    pub selected_answer_index: usize,
    #[allow(dead_code)]
    pub time_taken: Duration,
}

impl QuizGame {
    pub fn new(questions: Vec<Question>, language: Language) -> Self {
        Self {
            questions,
            current_question_index: 0,
            score: 0,
            correct_answers: 0,
            total_answers: 0,
            start_time: None,
            language,
        }
    }

    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }

    pub fn get_current_question(&self) -> Option<&Question> {
        self.questions.get(self.current_question_index)
    }

    pub fn get_question_text(&self, question: &Question) -> String {
        DataLoader::get_question_text(question, &self.language)
    }

    pub fn get_choice_texts(&self, question: &Question) -> Vec<String> {
        question
            .choices
            .iter()
            .map(|choice| DataLoader::get_choice_text(choice, &self.language))
            .collect()
    }

    /// Resolve the typed text against the current question's choices and
    /// answer with the matching index. Per `docs/spec.md`, only an **exact**
    /// match counts — prefix matches do nothing (so `mov` does not auto-pick
    /// `move`). A non-matching string yields an incorrect answer.
    pub fn answer_question_typed(&mut self, typed: &str) -> Option<QuizResult> {
        let index = self.get_current_question().and_then(|question| {
            self.get_choice_texts(question)
                .iter()
                .position(|choice| choice == typed)
        });
        // usize::MAX guarantees a non-match against any valid index.
        self.answer_question(index.unwrap_or(usize::MAX))
    }

    /// Index-based answer recorder. Prefer `answer_question_typed` from the
    /// UI layer — `usize::MAX` is the documented "no-match" sentinel.
    pub(crate) fn answer_question(&mut self, answer_index: usize) -> Option<QuizResult> {
        let question_start_time = Instant::now();

        if let Some(question) = self.get_current_question() {
            let correct_answer_index = question.correct_answer_index;
            let is_correct = answer_index == correct_answer_index;

            if is_correct {
                self.correct_answers += 1;
                self.score += self.calculate_score_for_question();
            }

            self.total_answers += 1;

            let result = QuizResult {
                is_correct,
                correct_answer_index,
                selected_answer_index: answer_index,
                time_taken: question_start_time.elapsed(),
            };

            self.current_question_index += 1;

            Some(result)
        } else {
            None
        }
    }

    pub fn is_game_finished(&self) -> bool {
        self.current_question_index >= self.questions.len()
    }

    pub fn get_final_score(&self) -> u32 {
        self.score
    }

    /// TODO(#24): wired up by typed-selection scoring.
    #[allow(dead_code)]
    pub fn get_accuracy(&self) -> f32 {
        if self.total_answers == 0 {
            0.0
        } else {
            self.correct_answers as f32 / self.total_answers as f32
        }
    }

    pub fn get_total_time(&self) -> Option<Duration> {
        self.start_time.map(|start| start.elapsed())
    }

    pub fn get_progress(&self) -> (usize, usize) {
        (self.current_question_index, self.questions.len())
    }

    fn calculate_score_for_question(&self) -> u32 {
        100
    }

    pub fn skip_question(&mut self) -> bool {
        if self.current_question_index < self.questions.len() {
            self.current_question_index += 1;
            self.total_answers += 1;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_question(choices: &[&str], correct: usize) -> Question {
        let mut question_text = HashMap::new();
        question_text.insert("ja".to_string(), "ダミー".to_string());
        question_text.insert("en".to_string(), "dummy".to_string());

        let choices = choices
            .iter()
            .map(|text| {
                let mut h = HashMap::new();
                h.insert("ja".to_string(), text.to_string());
                h.insert("en".to_string(), text.to_string());
                h
            })
            .collect();

        Question {
            id: "q-test".into(),
            genre: "test".into(),
            question_text,
            choices,
            correct_answer_index: correct,
            image_path: None,
        }
    }

    #[test]
    fn typed_exact_match_is_correct() {
        let question = make_question(&["borrow", "move", "ref", "clone"], 1);
        let mut game = QuizGame::new(vec![question], Language::English);
        game.start();
        let result = game.answer_question_typed("move").expect("result");
        assert!(result.is_correct);
        assert_eq!(result.selected_answer_index, 1);
    }

    #[test]
    fn typed_prefix_does_not_auto_confirm() {
        let question = make_question(&["borrow", "move", "ref", "clone"], 1);
        let mut game = QuizGame::new(vec![question], Language::English);
        game.start();
        let result = game.answer_question_typed("mov").expect("result");
        assert!(!result.is_correct);
        // usize::MAX sentinel: never matches a real choice index.
        assert_eq!(result.selected_answer_index, usize::MAX);
    }

    #[test]
    fn typed_wrong_choice_is_incorrect_but_recorded() {
        let question = make_question(&["borrow", "move", "ref", "clone"], 1);
        let mut game = QuizGame::new(vec![question], Language::English);
        game.start();
        let result = game.answer_question_typed("clone").expect("result");
        assert!(!result.is_correct);
        assert_eq!(result.selected_answer_index, 3);
    }

    #[test]
    fn typed_empty_string_is_incorrect() {
        let question = make_question(&["borrow", "move", "ref", "clone"], 1);
        let mut game = QuizGame::new(vec![question], Language::English);
        game.start();
        let result = game.answer_question_typed("").expect("result");
        assert!(!result.is_correct);
        assert_eq!(result.selected_answer_index, usize::MAX);
    }

    #[test]
    fn typed_is_case_sensitive() {
        // Documents the current contract: matching is byte-exact, no folding.
        let question = make_question(&["borrow", "move", "ref", "clone"], 1);
        let mut game = QuizGame::new(vec![question], Language::English);
        game.start();
        let result = game.answer_question_typed("Move").expect("result");
        assert!(!result.is_correct);
    }

    #[test]
    fn typed_trailing_whitespace_is_not_trimmed() {
        // Documents the current contract: whitespace is significant.
        let question = make_question(&["borrow", "move", "ref", "clone"], 1);
        let mut game = QuizGame::new(vec![question], Language::English);
        game.start();
        let result = game.answer_question_typed("move ").expect("result");
        assert!(!result.is_correct);
    }

    #[test]
    fn typed_matches_phrase_choice() {
        let question = make_question(
            &[
                "George Washington",
                "Abraham Lincoln",
                "Thomas Jefferson",
                "John Adams",
            ],
            0,
        );
        let mut game = QuizGame::new(vec![question], Language::English);
        game.start();
        let result = game
            .answer_question_typed("George Washington")
            .expect("result");
        assert!(result.is_correct);
        assert_eq!(result.selected_answer_index, 0);
    }
}

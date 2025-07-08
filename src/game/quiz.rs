use crate::types::{Question, Language};
use crate::io::DataLoader;
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
    pub selected_answer_index: usize,
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
        question.choices
            .iter()
            .map(|choice| DataLoader::get_choice_text(choice, &self.language))
            .collect()
    }

    pub fn answer_question(&mut self, answer_index: usize) -> Option<QuizResult> {
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
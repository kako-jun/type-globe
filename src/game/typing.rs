use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct TypingGame {
    target_text: String,
    typed_text: String,
    start_time: Option<Instant>,
    end_time: Option<Instant>,
    errors: u32,
}

#[derive(Debug, Clone)]
pub struct TypingResult {
    pub wpm: f32,
    pub accuracy: f32,
    pub total_time: Duration,
    pub total_characters: usize,
    pub correct_characters: usize,
    pub errors: u32,
}

impl TypingGame {
    pub fn new(target_text: String) -> Self {
        Self {
            target_text,
            typed_text: String::new(),
            start_time: None,
            end_time: None,
            errors: 0,
        }
    }

    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }

    pub fn type_character(&mut self, ch: char) -> bool {
        if self.start_time.is_none() {
            self.start();
        }

        if self.is_finished() {
            return false;
        }

        self.typed_text.push(ch);

        let expected_char = self.target_text.chars().nth(self.typed_text.len() - 1);
        if expected_char != Some(ch) {
            self.errors += 1;
        }

        if self.typed_text.len() >= self.target_text.len() {
            self.end_time = Some(Instant::now());
        }

        true
    }

    pub fn backspace(&mut self) -> bool {
        if !self.typed_text.is_empty() {
            self.typed_text.pop();
            true
        } else {
            false
        }
    }

    pub fn is_finished(&self) -> bool {
        self.typed_text.len() >= self.target_text.len()
    }

    pub fn get_target_text(&self) -> &str {
        &self.target_text
    }

    pub fn get_typed_text(&self) -> &str {
        &self.typed_text
    }

    pub fn get_current_position(&self) -> usize {
        self.typed_text.len()
    }

    pub fn get_progress(&self) -> f32 {
        if self.target_text.is_empty() {
            1.0
        } else {
            self.typed_text.len() as f32 / self.target_text.len() as f32
        }
    }

    pub fn calculate_wpm(&self) -> f32 {
        if let (Some(start), Some(end)) = (self.start_time, self.end_time) {
            let duration = end.duration_since(start);
            let minutes = duration.as_secs_f32() / 60.0;
            
            if minutes > 0.0 {
                let words = self.typed_text.len() as f32 / 5.0; // Standard: 5 characters = 1 word
                words / minutes
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    pub fn calculate_accuracy(&self) -> f32 {
        if self.typed_text.is_empty() {
            return 100.0;
        }

        let correct_chars = self.count_correct_characters();
        (correct_chars as f32 / self.typed_text.len() as f32) * 100.0
    }

    pub fn get_result(&self) -> Option<TypingResult> {
        if !self.is_finished() {
            return None;
        }

        let total_time = if let (Some(start), Some(end)) = (self.start_time, self.end_time) {
            end.duration_since(start)
        } else {
            Duration::new(0, 0)
        };

        Some(TypingResult {
            wpm: self.calculate_wpm(),
            accuracy: self.calculate_accuracy(),
            total_time,
            total_characters: self.typed_text.len(),
            correct_characters: self.count_correct_characters(),
            errors: self.errors,
        })
    }

    fn count_correct_characters(&self) -> usize {
        self.typed_text
            .chars()
            .zip(self.target_text.chars())
            .filter(|(typed, target)| typed == target)
            .count()
    }

    pub fn get_character_status(&self) -> Vec<CharacterStatus> {
        let mut status = Vec::new();
        let target_chars: Vec<char> = self.target_text.chars().collect();
        let typed_chars: Vec<char> = self.typed_text.chars().collect();

        for (i, &target_char) in target_chars.iter().enumerate() {
            let char_status = if i < typed_chars.len() {
                if typed_chars[i] == target_char {
                    CharacterStatus::Correct
                } else {
                    CharacterStatus::Incorrect
                }
            } else if i == typed_chars.len() {
                CharacterStatus::Current
            } else {
                CharacterStatus::Untyped
            };

            status.push(char_status);
        }

        status
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CharacterStatus {
    Correct,
    Incorrect,
    Current,
    Untyped,
}
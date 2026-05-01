use crate::io::DataLoader;
use crate::types::{Language, Question};
use rand::seq::SliceRandom;
use std::time::{Duration, Instant};

/// Number of questions a single Quiz run is locked to. Per `docs/spec.md`
/// (Quiz Mode header) and Issue #26's acceptance criteria.
pub const QUIZ_RUN_LENGTH: usize = 10;

#[derive(Debug)]
pub struct QuizGame {
    questions: Vec<Question>,
    current_question_index: usize,
    /// Index of the most recently answered question, recorded *before*
    /// `current_question_index` advances. The result screen needs this so
    /// it can render the choices and `correct_answer_index` of the
    /// question the player just answered, not the next one — otherwise
    /// two consecutive questions whose `correct_answer_index` happens to
    /// match (e.g. both `1`) make the "Wrong. Answer: X" line read off
    /// the next question entirely.
    last_answered_index: Option<usize>,
    score: u32,
    correct_answers: u32,
    total_answers: u32,
    /// Cumulative count of characters from *correctly* answered choice texts.
    /// CPM / WPM are derived from this and `start_time`. Wrong answers are
    /// not counted because the player never finished typing the correct
    /// choice — including them would inflate WPM for fast guessers.
    typed_correct_chars: u32,
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
    /// Construct a quiz with the supplied question list as-is. Used by
    /// tests and any caller that has already curated its own ordering.
    pub fn new(questions: Vec<Question>, language: Language) -> Self {
        Self {
            questions,
            current_question_index: 0,
            last_answered_index: None,
            score: 0,
            correct_answers: 0,
            total_answers: 0,
            typed_correct_chars: 0,
            start_time: None,
            language,
        }
    }

    /// Build a fresh run by sampling up to `QUIZ_RUN_LENGTH` distinct
    /// questions out of `pool`. If the pool is shorter than the run length
    /// the whole pool is used (no padding, no repeats). Order is shuffled
    /// so two consecutive runs don't see the same questions in the same
    /// sequence.
    pub fn from_pool(pool: &[Question], language: Language) -> Self {
        let mut rng = rand::thread_rng();
        let take = pool.len().min(QUIZ_RUN_LENGTH);
        let sampled: Vec<Question> = pool.choose_multiple(&mut rng, take).cloned().collect();
        Self::new(sampled, language)
    }

    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }

    pub fn get_current_question(&self) -> Option<&Question> {
        self.questions.get(self.current_question_index)
    }

    /// The question the player most recently submitted an answer for.
    /// Required by the result screen, which has to render the choices /
    /// correct_answer_index of *that* question even though
    /// `current_question_index` has already advanced to the next one.
    pub fn get_answered_question(&self) -> Option<&Question> {
        self.last_answered_index.and_then(|i| self.questions.get(i))
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

    /// All currently acceptable typed strings for the active question.
    /// Used by the UI for live prefix validation so a mistyped suffix
    /// is rejected immediately instead of forcing a Backspace recovery.
    pub fn current_typing_candidates(&self) -> Vec<String> {
        let Some(question) = self.get_current_question() else {
            return Vec::new();
        };
        let mut candidates: Vec<String> = question
            .choices
            .iter()
            .flat_map(|choice| DataLoader::get_choice_typing_texts(choice, &self.language))
            .map(|candidate| candidate.to_lowercase())
            .collect();
        candidates.sort();
        candidates.dedup();
        candidates
    }

    /// Whether `typed` is still a valid prefix of at least one answer
    /// candidate for the active question. Empty input is always valid.
    pub fn is_valid_typed_prefix(&self, typed: &str) -> bool {
        if typed.is_empty() {
            return true;
        }
        let typed = typed.to_lowercase();
        self.current_typing_candidates()
            .iter()
            .any(|candidate| candidate.starts_with(&typed))
    }

    /// Resolve the typed text against the current question's choices and
    /// answer with the matching index. Per `docs/spec.md`, only an **exact**
    /// match counts — prefix matches do nothing (so `mov` does not auto-pick
    /// `move`). A non-matching string yields an incorrect answer.
    pub fn answer_question_typed(&mut self, typed: &str) -> Option<QuizResult> {
        let typed = typed.to_lowercase();
        let matched = self.get_current_question().and_then(|question| {
            question
                .choices
                .iter()
                .enumerate()
                .find_map(|(idx, choice)| {
                    DataLoader::get_choice_typing_texts(choice, &self.language)
                        .into_iter()
                        .find(|candidate| candidate.to_lowercase() == typed)
                        .map(|candidate| (idx, candidate.chars().count() as u32))
                })
        });
        // usize::MAX guarantees a non-match against any valid index.
        let (index, typed_chars) = matched.unwrap_or((usize::MAX, 0));
        self.answer_question(index, typed_chars)
    }

    /// Index-based answer recorder. Prefer `answer_question_typed` from the
    /// UI layer — `usize::MAX` is the documented "no-match" sentinel.
    pub(crate) fn answer_question(
        &mut self,
        answer_index: usize,
        typed_chars: u32,
    ) -> Option<QuizResult> {
        let question_start_time = Instant::now();

        let snapshot = self.get_current_question().map(|question| {
            let correct_answer_index = question.correct_answer_index;
            let is_correct = answer_index == correct_answer_index;
            (correct_answer_index, is_correct)
        });

        if let Some((correct_answer_index, is_correct)) = snapshot {
            if is_correct {
                self.correct_answers += 1;
                self.score += self.calculate_score_for_question();
                self.typed_correct_chars = self.typed_correct_chars.saturating_add(typed_chars);
            }

            self.total_answers += 1;

            let result = QuizResult {
                is_correct,
                correct_answer_index,
                selected_answer_index: answer_index,
                time_taken: question_start_time.elapsed(),
            };

            // Record before advancing so the result screen can render
            // the question that was just answered, not the next one.
            self.last_answered_index = Some(self.current_question_index);
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

    pub fn get_correct_count(&self) -> u32 {
        self.correct_answers
    }

    pub fn get_total_time(&self) -> Option<Duration> {
        self.start_time.map(|start| start.elapsed())
    }

    /// Characters per minute over the full run so far. Returns 0 when the
    /// timer hasn't started or when no time has elapsed.
    pub fn get_cpm(&self) -> u32 {
        let secs = self
            .get_total_time()
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        if secs <= 0.0 {
            0
        } else {
            ((self.typed_correct_chars as f64) * 60.0 / secs).round() as u32
        }
    }

    /// Words per minute, where 1 word = 5 characters (the standard
    /// typing-test convention). Same denominator as `get_cpm`.
    pub fn get_wpm(&self) -> u32 {
        self.get_cpm() / 5
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
    use crate::types::Choice;
    use std::collections::HashMap;

    fn make_question(choices: &[&str], correct: usize) -> Question {
        let mut question_text = HashMap::new();
        question_text.insert("ja".to_string(), "ダミー".to_string());
        question_text.insert("en".to_string(), "dummy".to_string());

        let choices = choices
            .iter()
            .map(|text| {
                let mut labels = HashMap::new();
                labels.insert("ja".to_string(), text.to_string());
                labels.insert("en".to_string(), text.to_string());
                Choice {
                    labels,
                    ja_typings: Vec::new(),
                }
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
    fn typed_is_case_insensitive() {
        let question = make_question(&["borrow", "move", "ref", "clone"], 1);
        let mut game = QuizGame::new(vec![question], Language::English);
        game.start();
        let result = game.answer_question_typed("Move").expect("result");
        assert!(result.is_correct);
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

    #[test]
    fn japanese_mode_accepts_explicit_romaji() {
        let mut question = make_question(&["東京", "大阪", "京都", "名古屋"], 0);
        question.choices[0].ja_typings = vec!["tokyo".into(), "toukyou".into()];
        question.choices[1].ja_typings = vec!["osaka".into(), "oosaka".into()];
        question.choices[2].ja_typings = vec!["kyoto".into(), "kyouto".into()];
        question.choices[3].ja_typings = vec!["nagoya".into()];
        let mut game = QuizGame::new(vec![question], Language::Japanese);
        game.start();
        let result = game.answer_question_typed("tokyo").expect("result");
        assert!(result.is_correct);
    }

    #[test]
    fn japanese_mode_accepts_uppercase_romaji() {
        let mut question = make_question(&["東京", "大阪", "京都", "名古屋"], 0);
        question.choices[0].ja_typings = vec!["tokyo".into(), "toukyou".into()];
        let mut game = QuizGame::new(vec![question], Language::Japanese);
        game.start();
        let result = game.answer_question_typed("TOKYO").expect("result");
        assert!(result.is_correct);
    }

    #[test]
    fn japanese_mode_accepts_long_vowel_alias() {
        let mut question = make_question(&["とうきょう", "おおさか", "きょうと", "なごや"], 0);
        question.choices[0].ja_typings = vec!["tokyo".into(), "toukyou".into()];
        let mut game = QuizGame::new(vec![question], Language::Japanese);
        game.start();
        let result = game.answer_question_typed("toukyou").expect("result");
        assert!(result.is_correct);
    }

    #[test]
    fn valid_prefix_accepts_partial_match() {
        let question = make_question(&["borrow", "move", "ref", "clone"], 1);
        let game = QuizGame::new(vec![question], Language::English);
        assert!(game.is_valid_typed_prefix("mo"));
    }

    #[test]
    fn valid_prefix_rejects_wrong_branch() {
        let question = make_question(&["borrow", "move", "ref", "clone"], 1);
        let game = QuizGame::new(vec![question], Language::English);
        assert!(!game.is_valid_typed_prefix("mx"));
    }

    #[test]
    fn valid_prefix_handles_japanese_romaji_variants() {
        let mut question = make_question(&["東京", "大阪", "京都", "名古屋"], 0);
        question.choices[0].ja_typings = vec!["tokyo".into(), "toukyou".into()];
        let game = QuizGame::new(vec![question], Language::Japanese);
        assert!(game.is_valid_typed_prefix("tok"));
        assert!(game.is_valid_typed_prefix("tou"));
        assert!(!game.is_valid_typed_prefix("tax"));
    }

    #[test]
    fn from_pool_caps_at_run_length() {
        // 30 distinct questions in the pool; from_pool must hand back
        // exactly QUIZ_RUN_LENGTH = 10.
        let pool: Vec<Question> = (0..30)
            .map(|i| {
                let mut q = make_question(&["a", "b", "c", "d"], 0);
                q.id = format!("q{i:02}");
                q
            })
            .collect();
        let game = QuizGame::from_pool(&pool, Language::English);
        assert_eq!(game.get_progress(), (0, QUIZ_RUN_LENGTH));
    }

    #[test]
    fn from_pool_returns_whole_pool_when_smaller_than_run_length() {
        let pool: Vec<Question> = (0..3)
            .map(|i| {
                let mut q = make_question(&["a", "b", "c", "d"], 0);
                q.id = format!("q{i}");
                q
            })
            .collect();
        let game = QuizGame::from_pool(&pool, Language::English);
        assert_eq!(game.get_progress(), (0, 3));
    }

    #[test]
    fn from_pool_does_not_repeat_questions() {
        // 10 distinct questions, take 10 — every id must be present once.
        let pool: Vec<Question> = (0..QUIZ_RUN_LENGTH)
            .map(|i| {
                let mut q = make_question(&["a", "b", "c", "d"], 0);
                q.id = format!("q{i}");
                q
            })
            .collect();
        let game = QuizGame::from_pool(&pool, Language::English);
        let ids: std::collections::BTreeSet<&str> =
            game.questions.iter().map(|q| q.id.as_str()).collect();
        assert_eq!(ids.len(), QUIZ_RUN_LENGTH);
    }

    #[test]
    fn cpm_and_wpm_are_zero_before_any_correct_answer() {
        let q = make_question(&["a", "b", "c", "d"], 0);
        let mut game = QuizGame::new(vec![q], Language::English);
        game.start();
        assert_eq!(game.get_cpm(), 0);
        assert_eq!(game.get_wpm(), 0);
    }

    #[test]
    fn cpm_counts_only_correct_answers() {
        // Two questions, answer the first wrong and the second right —
        // typed_correct_chars must equal len("right") == 5.
        let q1 = make_question(&["right", "wrong1", "wrong2", "wrong3"], 0);
        let q2 = make_question(&["right", "wrong1", "wrong2", "wrong3"], 0);
        let mut game = QuizGame::new(vec![q1, q2], Language::English);
        game.start();
        game.answer_question_typed("wrong1");
        game.answer_question_typed("right");
        assert_eq!(game.typed_correct_chars, 5);
    }

    #[test]
    fn cpm_counts_typed_variant_length_for_japanese() {
        let mut question = make_question(&["東京", "大阪", "京都", "名古屋"], 0);
        question.choices[0].ja_typings = vec!["tokyo".into(), "toukyou".into()];
        let mut game = QuizGame::new(vec![question], Language::Japanese);
        game.start();
        game.answer_question_typed("toukyou");
        assert_eq!(game.typed_correct_chars, 7);
    }

    #[test]
    fn get_answered_question_points_at_just_answered_question() {
        // Regression for the H2O / Tokyo result-screen bug: `answer_question`
        // advances `current_question_index`, but the result screen needs the
        // question the player *just* submitted. With two questions whose
        // `correct_answer_index` happen to coincide (both `1`), looking up
        // `current_question.choices[result.correct_answer_index]` after the
        // answer would yield Q2's "Tokyo" while the run was actually on Q1
        // ("H2O"). `get_answered_question` must keep returning Q1 here.
        let q1 = make_question(&["CO2", "H2O", "O2", "N2"], 1);
        let q2 = make_question(&["Osaka", "Tokyo", "Kyoto", "Nagoya"], 1);
        let mut game = QuizGame::new(vec![q1, q2], Language::English);
        game.start();

        // Answer Q1 correctly. After this call, current_question_index = 1
        // but the result screen still wants Q1.
        let result = game.answer_question_typed("H2O").expect("result");
        assert!(result.is_correct);

        let answered = game
            .get_answered_question()
            .expect("just-answered question is set");
        assert_eq!(
            answered.choices[result.correct_answer_index].labels["en"],
            "H2O"
        );

        // And `get_current_question` correctly points at Q2 for the
        // next round — the two getters are not interchangeable.
        let current = game.get_current_question().expect("next question");
        assert_eq!(current.choices[1].labels["en"], "Tokyo");
    }

    #[test]
    fn get_answered_question_is_none_before_any_answer() {
        let q = make_question(&["a", "b", "c", "d"], 0);
        let game = QuizGame::new(vec![q], Language::English);
        assert!(game.get_answered_question().is_none());
    }

    #[test]
    fn correct_count_tracks_only_correct_answers() {
        let q = make_question(&["right", "a", "b", "c"], 0);
        let mut game = QuizGame::new(vec![q], Language::English);
        game.start();
        game.answer_question_typed("right");
        assert_eq!(game.get_correct_count(), 1);
        assert!((game.get_accuracy() - 1.0).abs() < f32::EPSILON);
    }
}

//! Listening-mode game logic (#30 / #31).
//!
//! Single-prompt session for the v0.2.0 "listening foundation" epic.
//! The full RPG run (10 prompts, HP/EXP, boss placement)
//! lives in #32-#37 and will compose this module rather than replacing
//! it.
//!
//! Design notes:
//! - Pure: no audio I/O lives here. The UI owns the `TtsEngine` and
//!   calls `speak()` whenever `wants_replay()` flips true.
//! - The blind-input judge (`is_correct_listening_input`, #31) is a
//!   free function so the unit tests don't need a session at all.

use crate::io::romaji::hiragana_to_hepburn_variants;
use crate::types::{Language, ListeningPrompt};
use rand::seq::SliceRandom;

pub fn acceptable_listening_inputs(language: &Language, expected: &str) -> Vec<String> {
    match language {
        Language::Japanese => hiragana_to_hepburn_variants(expected),
        Language::English => vec![expected.trim().to_lowercase()],
    }
}

pub fn is_valid_listening_prefix(language: &Language, typed: &str, expected: &str) -> bool {
    if typed.is_empty() {
        return true;
    }
    let typed = typed.to_lowercase();
    acceptable_listening_inputs(language, expected)
        .iter()
        .any(|candidate| candidate.starts_with(&typed))
}

/// Decide whether `typed` matches `expected` for a listening prompt.
///
/// Per `docs/spec.md` and issue #31, the judge:
/// - is **case-insensitive** (the player can't see the prompt, so
///   forcing them to remember capitalisation is hostile to the spirit
///   of the mode);
/// - **trims surrounding whitespace** (a stray Space / Enter buffering
///   space at either end shouldn't lose them the prompt);
/// - keeps **internal spacing exact** (two-word phrases must be typed
///   with the right number of spaces — that's part of the listening
///   skill).
pub fn is_correct_listening_input(language: &Language, typed: &str, expected: &str) -> bool {
    let typed = typed.trim().to_lowercase();
    acceptable_listening_inputs(language, expected)
        .iter()
        .any(|candidate| candidate == &typed)
}

/// One play-through of a single listening prompt. Tracks the active
/// prompt, the player's typed buffer, and whether the round is over.
/// The UI advances state by calling `submit()` on Enter and `replay()`
/// on Space — the latter is just a flag the UI clears once it has
/// asked the TTS engine to speak again.
pub struct ListeningSession {
    prompt: ListeningPrompt,
    language: Language,
    input: String,
    submitted: Option<SubmissionResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmissionResult {
    pub is_correct: bool,
    /// The prompt's expected text, captured at submission time so the
    /// UI can reveal it on the result screen without holding the
    /// `ListeningPrompt` reference.
    pub expected: String,
}

impl ListeningSession {
    pub fn new(prompt: ListeningPrompt, language: Language) -> Self {
        Self {
            prompt,
            language,
            input: String::new(),
            submitted: None,
        }
    }

    /// Pick a random prompt from `pool`. Returns `None` when the pool
    /// is empty so the caller can show a "no listening data" message
    /// instead of panicking.
    pub fn from_pool(pool: &[ListeningPrompt], language: Language) -> Option<Self> {
        let mut rng = rand::thread_rng();
        pool.choose(&mut rng)
            .cloned()
            .map(|prompt| Self::new(prompt, language))
    }

    pub fn prompt(&self) -> &ListeningPrompt {
        &self.prompt
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn push_char(&mut self, c: char) {
        if self.submitted.is_some() {
            return;
        }
        self.input.push(c);
    }

    pub fn pop_char(&mut self) {
        if self.submitted.is_some() {
            return;
        }
        self.input.pop();
    }

    /// Apply the blind-input judge. Once submitted the session is
    /// frozen — further `push_char` / `pop_char` are no-ops.
    pub fn submit(&mut self) -> &SubmissionResult {
        let is_correct =
            is_correct_listening_input(&self.language, &self.input, &self.prompt.text_reading);
        self.submitted = Some(SubmissionResult {
            is_correct,
            expected: self.prompt.text_reading.clone(),
        });
        // Safe: we just assigned `Some(_)`.
        self.submitted.as_ref().expect("just submitted")
    }

    pub fn result(&self) -> Option<&SubmissionResult> {
        self.submitted.as_ref()
    }

    /// Whether the session has been submitted. The single-prompt UI
    /// currently uses `result()` instead, but the run-loop in #32-#37
    /// needs this as the "advance to next prompt" gate.
    #[allow(dead_code)]
    pub fn is_finished(&self) -> bool {
        self.submitted.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AnswerKind;

    fn p(text: &str) -> ListeningPrompt {
        ListeningPrompt {
            id: "test".into(),
            text_reading: text.into(),
            text_display: text.into(),
            kind: AnswerKind::Word,
        }
    }

    #[test]
    fn judge_accepts_exact_match() {
        assert!(is_correct_listening_input(
            &Language::English,
            "apple",
            "apple"
        ));
    }

    #[test]
    fn judge_is_case_insensitive() {
        assert!(is_correct_listening_input(
            &Language::English,
            "Apple",
            "apple"
        ));
        assert!(is_correct_listening_input(
            &Language::English,
            "APPLE",
            "apple"
        ));
    }

    #[test]
    fn judge_trims_surrounding_whitespace() {
        assert!(is_correct_listening_input(
            &Language::English,
            "  apple ",
            "apple"
        ));
        assert!(is_correct_listening_input(
            &Language::English,
            "apple\n",
            "apple"
        ));
    }

    #[test]
    fn judge_keeps_internal_spacing_exact() {
        // Listening is *part of* the skill: drop a space and it's wrong.
        assert!(is_correct_listening_input(
            &Language::English,
            "George Washington",
            "George Washington"
        ));
        assert!(!is_correct_listening_input(
            &Language::English,
            "GeorgeWashington",
            "George Washington"
        ));
        assert!(!is_correct_listening_input(
            &Language::English,
            "George  Washington",
            "George Washington"
        ));
    }

    #[test]
    fn judge_rejects_different_word() {
        assert!(!is_correct_listening_input(
            &Language::English,
            "orange",
            "apple"
        ));
    }

    #[test]
    fn prefix_accepts_partial_match() {
        assert!(is_valid_listening_prefix(
            &Language::English,
            "app",
            "apple"
        ));
    }

    #[test]
    fn prefix_rejects_wrong_branch() {
        assert!(!is_valid_listening_prefix(
            &Language::English,
            "apx",
            "apple"
        ));
    }

    #[test]
    fn judge_romanizes_japanese_prompts() {
        assert!(is_correct_listening_input(
            &Language::Japanese,
            "tokyo",
            "とうきょう"
        ));
        assert!(is_correct_listening_input(
            &Language::Japanese,
            "toukyou",
            "とうきょう"
        ));
        assert!(is_correct_listening_input(
            &Language::Japanese,
            "shinbashi",
            "しんばし"
        ));
    }

    #[test]
    fn prefix_accepts_japanese_variants() {
        assert!(is_valid_listening_prefix(
            &Language::Japanese,
            "tok",
            "とうきょう"
        ));
        assert!(is_valid_listening_prefix(
            &Language::Japanese,
            "tou",
            "とうきょう"
        ));
        assert!(!is_valid_listening_prefix(
            &Language::Japanese,
            "tax",
            "とうきょう"
        ));
    }

    #[test]
    fn session_records_correct_submission() {
        let mut s = ListeningSession::new(p("apple"), Language::English);
        for c in "apple".chars() {
            s.push_char(c);
        }
        let r = s.submit().clone();
        assert!(r.is_correct);
        assert_eq!(r.expected, "apple");
        assert!(s.is_finished());
    }

    #[test]
    fn session_freezes_after_submit() {
        let mut s = ListeningSession::new(p("apple"), Language::English);
        s.push_char('a');
        s.submit();
        s.push_char('b');
        assert_eq!(s.input(), "a");
    }

    #[test]
    fn session_records_incorrect_submission() {
        let mut s = ListeningSession::new(p("apple"), Language::English);
        for c in "orange".chars() {
            s.push_char(c);
        }
        let r = s.submit().clone();
        assert!(!r.is_correct);
        assert_eq!(r.expected, "apple");
    }

    #[test]
    fn from_pool_returns_none_on_empty() {
        let pool: Vec<ListeningPrompt> = Vec::new();
        assert!(ListeningSession::from_pool(&pool, Language::English).is_none());
    }

    #[test]
    fn from_pool_picks_one_when_available() {
        let pool = vec![p("apple"), p("river")];
        let s = ListeningSession::from_pool(&pool, Language::English).expect("pool non-empty");
        assert!(matches!(
            s.prompt().text_reading.as_str(),
            "apple" | "river"
        ));
    }
}

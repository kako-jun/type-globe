//! Question-data integrity checks.
//!
//! With typed selection (#24), the only ambiguity that matters is when one
//! choice is a strict prefix of another in the same question and language.
//! Hitting Enter at the prefix point picks the shorter answer, so the player
//! can never reach the longer one without realising the shorter one
//! pre-empts it.
//!
//! Per `docs/spec.md`: "no two choices in a question may share a prefix that
//! would make a typed answer ambiguous before `Enter`." (Enforced here.)
//!
//! Scope: this module flags prefix conflicts only. Choice-count enforcement
//! (e.g. exactly 4 choices), correct-index range checks, and other shape
//! validation are out of scope and left to a separate validator.

use crate::types::Question;

/// One detected prefix conflict between two choices of a single question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefixConflict {
    pub question_id: String,
    pub language: String,
    pub shorter_index: usize,
    pub shorter_text: String,
    pub longer_index: usize,
    pub longer_text: String,
}

/// Walk every question and report all prefix conflicts. The result is
/// deterministic: questions in input order, language code ascending,
/// (shorter, longer) index pairs ascending.
pub fn find_prefix_conflicts(questions: &[Question]) -> Vec<PrefixConflict> {
    let mut conflicts = Vec::new();
    for question in questions {
        // BTreeSet's iterator is already in ascending order, so the resulting
        // Vec is sorted — no extra sort needed.
        let languages: Vec<&String> = question
            .choices
            .iter()
            .flat_map(|c| c.keys())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        for language in languages {
            let texts: Vec<(usize, &str)> = question
                .choices
                .iter()
                .enumerate()
                .filter_map(|(i, c)| c.get(language).map(|t| (i, t.as_str())))
                .collect();
            for (a_pos, (a_idx, a_text)) in texts.iter().enumerate() {
                for (b_idx, b_text) in texts.iter().skip(a_pos + 1) {
                    if let Some((shorter, shorter_idx, longer, longer_idx)) =
                        prefix_pair(a_text, *a_idx, b_text, *b_idx)
                    {
                        conflicts.push(PrefixConflict {
                            question_id: question.id.clone(),
                            language: language.clone(),
                            shorter_index: shorter_idx,
                            shorter_text: shorter.to_string(),
                            longer_index: longer_idx,
                            longer_text: longer.to_string(),
                        });
                    }
                }
            }
        }
    }
    conflicts
}

/// Return `(shorter, shorter_idx, longer, longer_idx)` if one of `a` / `b` is
/// a strict prefix of the other; `None` if they're equal or unrelated.
fn prefix_pair<'a>(
    a: &'a str,
    a_idx: usize,
    b: &'a str,
    b_idx: usize,
) -> Option<(&'a str, usize, &'a str, usize)> {
    if a == b {
        return None;
    }
    if b.starts_with(a) {
        Some((a, a_idx, b, b_idx))
    } else if a.starts_with(b) {
        Some((b, b_idx, a, a_idx))
    } else {
        None
    }
}

/// Format a single conflict for stderr or a build report. Choice texts are
/// rendered with `{:?}` so embedded quotes / control chars are escaped.
pub fn format_conflict(c: &PrefixConflict) -> String {
    format!(
        "[prefix conflict] question {} ({}): choice #{} {:?} is a prefix of choice #{} {:?}",
        c.question_id, c.language, c.shorter_index, c.shorter_text, c.longer_index, c.longer_text
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn question_with_choices(id: &str, choices_per_lang: &[(&str, &[&str])]) -> Question {
        let mut question_text = HashMap::new();
        question_text.insert("ja".to_string(), "ダミー".to_string());
        question_text.insert("en".to_string(), "dummy".to_string());

        // Build choices indexed by position; each position holds the same
        // choice across all languages (so callers pass parallel slices).
        let max_choices = choices_per_lang
            .iter()
            .map(|(_, choices)| choices.len())
            .max()
            .unwrap_or(0);

        let choices = (0..max_choices)
            .map(|i| {
                let mut h = HashMap::new();
                for (lang, slice) in choices_per_lang {
                    if let Some(text) = slice.get(i) {
                        h.insert(lang.to_string(), text.to_string());
                    }
                }
                h
            })
            .collect();

        Question {
            id: id.into(),
            genre: "test".into(),
            question_text,
            choices,
            correct_answer_index: 0,
            image_path: None,
        }
    }

    #[test]
    fn detects_prefix_conflict() {
        let q = question_with_choices("q-1", &[("en", &["move", "movement", "borrow", "ref"])]);
        let conflicts = find_prefix_conflicts(&[q]);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].question_id, "q-1");
        assert_eq!(conflicts[0].language, "en");
        assert_eq!(conflicts[0].shorter_text, "move");
        assert_eq!(conflicts[0].longer_text, "movement");
        assert_eq!(conflicts[0].shorter_index, 0);
        assert_eq!(conflicts[0].longer_index, 1);
    }

    #[test]
    fn detects_prefix_regardless_of_choice_order() {
        let q = question_with_choices("q-rev", &[("en", &["movement", "move", "borrow", "ref"])]);
        let conflicts = find_prefix_conflicts(&[q]);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].shorter_text, "move");
        assert_eq!(conflicts[0].longer_text, "movement");
    }

    #[test]
    fn equal_choices_are_not_a_prefix_conflict() {
        // Equal choices are a different bug (no two choices should be
        // identical) but the prefix linter's job is prefix relations only.
        let q = question_with_choices("q-eq", &[("en", &["a", "a", "b", "c"])]);
        let conflicts = find_prefix_conflicts(&[q]);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn unrelated_choices_pass() {
        let q = question_with_choices(
            "q-ok",
            &[("en", &["sort()", "order()", "arrange()", "organize()"])],
        );
        assert!(find_prefix_conflicts(&[q]).is_empty());
    }

    #[test]
    fn checks_each_language_independently() {
        let q = question_with_choices(
            "q-multi",
            &[
                ("en", &["clean", "cleaner", "x", "y"]),
                ("ja", &["きれい", "綺麗", "X", "Y"]),
            ],
        );
        let conflicts = find_prefix_conflicts(&[q]);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].language, "en");
    }

    #[test]
    fn reports_all_conflicts_within_a_question() {
        let q = question_with_choices("q-many", &[("en", &["a", "ab", "abc", "z"])]);
        let conflicts = find_prefix_conflicts(&[q]);
        // a < ab, a < abc, ab < abc → 3 conflicts.
        assert_eq!(conflicts.len(), 3);
    }

    #[test]
    fn empty_input_is_empty_output() {
        assert!(find_prefix_conflicts(&[]).is_empty());
    }

    #[test]
    fn format_conflict_matches_expected_shape() {
        let c = PrefixConflict {
            question_id: "q-fmt".into(),
            language: "en".into(),
            shorter_index: 1,
            shorter_text: "move".into(),
            longer_index: 2,
            longer_text: "movement".into(),
        };
        assert_eq!(
            format_conflict(&c),
            "[prefix conflict] question q-fmt (en): choice #1 \"move\" is a prefix of choice #2 \"movement\""
        );
    }

    #[test]
    fn format_conflict_escapes_embedded_quotes() {
        let c = PrefixConflict {
            question_id: "q-q".into(),
            language: "en".into(),
            shorter_index: 0,
            shorter_text: r#"say "hi""#.into(),
            longer_index: 1,
            longer_text: r#"say "hi" loud"#.into(),
        };
        // {:?} debug-formatted strings escape embedded quotes as \", so the
        // output is unambiguous when grepped or piped.
        assert!(format_conflict(&c).contains(r#"\"hi\""#));
    }

    #[test]
    fn detects_multi_byte_prefix() {
        // Multi-byte UTF-8 characters must not split — `日` is a strict
        // prefix of `日本` at the byte level too, since UTF-8 is
        // self-synchronizing.
        let q = question_with_choices("q-utf8", &[("ja", &["日", "日本", "x", "y"])]);
        let conflicts = find_prefix_conflicts(&[q]);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].shorter_text, "日");
        assert_eq!(conflicts[0].longer_text, "日本");
    }

    #[test]
    fn skips_languages_missing_from_a_choice() {
        // If a choice carries only `en`, the `ja` pass for that choice is
        // simply skipped — no panic, no false positive.
        let q = Question {
            id: "q-asym".into(),
            genre: "test".into(),
            question_text: {
                let mut m = HashMap::new();
                m.insert("en".to_string(), "dummy".to_string());
                m.insert("ja".to_string(), "ダミー".to_string());
                m
            },
            choices: vec![
                {
                    let mut h = HashMap::new();
                    h.insert("en".to_string(), "let".to_string());
                    // No `ja` entry on purpose.
                    h
                },
                {
                    let mut h = HashMap::new();
                    h.insert("en".to_string(), "let mut".to_string());
                    h.insert("ja".to_string(), "可変let".to_string());
                    h
                },
            ],
            correct_answer_index: 0,
            image_path: None,
        };
        let conflicts = find_prefix_conflicts(&[q]);
        // Conflict reported only for `en`; `ja` has just one populated
        // choice so nothing to compare against.
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].language, "en");
    }

    // Bundled data must stay free of prefix conflicts; the build-time
    // linter (#60) re-enforces this in CI on the same data files. Reads
    // the JSON directly with serde_json so the assertion logic compiles
    // unchanged when this module is `#[path]`-included into the
    // `lint-questions` binary, where `crate::io::DataLoader` is absent.
    fn assert_data_clean(path: &str) {
        if !std::path::Path::new(path).exists() {
            return;
        }
        let text = std::fs::read_to_string(path).expect("read questions json");
        let questions: Vec<Question> = serde_json::from_str(&text).expect("parse questions json");
        let conflicts = find_prefix_conflicts(&questions);
        assert!(
            conflicts.is_empty(),
            "shipped data ({}) has prefix conflicts:\n{}",
            path,
            conflicts
                .iter()
                .map(format_conflict)
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    #[test]
    fn shipped_question_data_is_clean_ja() {
        assert_data_clean("data/questions_ja.json");
    }

    #[test]
    fn shipped_question_data_is_clean_en() {
        assert_data_clean("data/questions_en.json");
    }
}

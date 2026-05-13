//! Build-time linter for bundled question data.
//!
//! Loads each JSON path passed on the command line, runs the prefix-conflict
//! validator (`io::validator::find_prefix_conflicts`), prints every conflict
//! to stderr, and exits with code 1 if any were found. CI runs this on the
//! shipped `data/questions_*.json` so that a regression in the data fails
//! the build, not just a runtime warning. (#60, spec.md "build-time linter")
//!
//! The library tests already enforce the same invariant
//! (`shipped_question_data_is_clean_*`); this binary covers ad-hoc data
//! files that are not part of the test fixture, e.g. a contributor's
//! work-in-progress JSON.

#[path = "../types.rs"]
#[allow(dead_code)]
mod types;

#[path = "../io/romaji.rs"]
#[allow(dead_code)]
mod romaji;

// `validator` (included below) references `super::normalize::canonical_romaji`
// when run as part of the library. As a standalone binary we re-include the
// module here so the same path resolves; the `dead_code` allow is needed
// because this binary only uses a subset of normalize's surface.
#[path = "../io/normalize.rs"]
#[allow(dead_code)]
mod normalize;

#[path = "../io/validator.rs"]
mod validator;

use std::process::ExitCode;
use types::Question;
use validator::{find_prefix_conflicts, format_conflict};

fn main() -> ExitCode {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: lint-questions <path-to-questions.json> [more.json ...]");
        return ExitCode::from(2);
    }

    let mut total_conflicts = 0usize;
    let mut load_errors = 0usize;
    let mut typing_errors = 0usize;

    for path in &paths {
        match load(path) {
            Ok(questions) => {
                let conflicts = find_prefix_conflicts(&questions);
                for c in &conflicts {
                    eprintln!("{}: {}", path, format_conflict(c));
                }
                total_conflicts += conflicts.len();
                let errors = find_ja_typing_errors(path, &questions);
                for error in &errors {
                    eprintln!("{path}: {error}");
                }
                typing_errors += errors.len();
                if path.contains("questions_ja") {
                    let unreviewed: Vec<&str> = questions
                        .iter()
                        .filter(|q| !q.ja_reviewed)
                        .map(|q| q.id.as_str())
                        .collect();
                    if !unreviewed.is_empty() {
                        eprintln!(
                            "{path}: {} unreviewed question(s) (ja_reviewed=false). First 10: {:?}",
                            unreviewed.len(),
                            &unreviewed[..unreviewed.len().min(10)]
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("{path}: load error: {e}");
                load_errors += 1;
            }
        }
    }

    if total_conflicts > 0 || typing_errors > 0 || load_errors > 0 {
        eprintln!(
            "lint-questions: {} conflict(s), {} ja_typing error(s), {} load error(s) across {} file(s)",
            total_conflicts,
            typing_errors,
            load_errors,
            paths.len()
        );
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn load(path: &str) -> Result<Vec<Question>, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(path)?;
    let questions: Vec<Question> = serde_json::from_str(&text)?;
    Ok(questions)
}

fn find_ja_typing_errors(path: &str, questions: &[Question]) -> Vec<String> {
    if !path.contains("questions_ja") {
        return Vec::new();
    }

    let mut errors = Vec::new();

    for question in questions {
        for (choice_idx, choice) in question.choices.iter().enumerate() {
            let Some(ja) = choice.labels.get("ja") else {
                continue;
            };
            if choice.ja_typings.is_empty() && contains_han(ja) {
                errors.push(format!(
                    "[ja_typings missing] question {} choice #{} has non-kana ja={:?}",
                    question.id, choice_idx, ja
                ));
                continue;
            }
            for ja_typing in &choice.ja_typings {
                if !ja_typing.is_ascii() {
                    errors.push(format!(
                        "[ja_typings non-ascii] question {} choice #{} has ja_typing={:?}",
                        question.id, choice_idx, ja_typing
                    ));
                }
            }

            let actual = normalized_variants(choice.ja_typings.clone());
            if actual.len() != choice.ja_typings.len() {
                errors.push(format!(
                    "[ja_typings duplicate-or-unsorted] question {} choice #{} has {:?}",
                    question.id, choice_idx, choice.ja_typings
                ));
            }

            // Pure-kana labels: ja_typings must match what `hiragana_to_hepburn`
            // emits (IME-wapuro 正解形)。これで `gomennasai` のような IME 流儀で
            // 別の読みになってしまう登録を自動検出する。漢字ラベルは読みが取れ
            // ないので skip (人手レビュー対象)。
            if !choice.ja_typings.is_empty() && !contains_han(ja) {
                let expected = expected_ja_typings(ja);
                if let Some(reason) = ja_typing_mismatch_reason(choice, &actual, &expected) {
                    errors.push(format!(
                        "[ja_typings mismatch] question {} choice #{} ja={:?} expected {:?} but got {:?}: {}",
                        question.id, choice_idx, ja, expected, actual, reason
                    ));
                }
            }
        }
    }

    errors
}

fn expected_ja_typings(ja: &str) -> Vec<String> {
    if ja.is_ascii() {
        vec![ja.to_ascii_lowercase()]
    } else {
        normalized_variants(romaji::hiragana_to_hepburn_variants(ja))
    }
}

fn normalized_variants(mut variants: Vec<String>) -> Vec<String> {
    for variant in &mut variants {
        *variant = variant.to_lowercase();
    }
    variants.sort();
    variants.dedup();
    variants
}

fn ja_typing_mismatch_reason(
    choice: &types::Choice,
    actual: &[String],
    expected: &[String],
) -> Option<String> {
    let missing: Vec<String> = expected
        .iter()
        .filter(|variant| !actual.contains(variant))
        .cloned()
        .collect();
    if !missing.is_empty() {
        return Some(format!("missing generated variants {missing:?}"));
    }

    let allowed_extras = allowed_extra_typings(choice);
    let unexpected: Vec<String> = actual
        .iter()
        .filter(|variant| !expected.contains(variant))
        .filter(|variant| !allowed_extras.contains(*variant))
        .cloned()
        .collect();
    if !unexpected.is_empty() {
        return Some(format!(
            "unexpected extras {unexpected:?}; only lowercase official en spelling is allowed"
        ));
    }

    None
}

fn allowed_extra_typings(choice: &types::Choice) -> Vec<String> {
    let mut allowed = Vec::new();

    for key in ["en", "ja"] {
        let Some(label) = choice.labels.get(key) else {
            continue;
        };

        if label.is_ascii() {
            allowed.push(label.to_ascii_lowercase());
        }

        allowed.extend(extract_ascii_parenthetical_aliases(label));
    }

    normalized_variants(allowed)
}

fn extract_ascii_parenthetical_aliases(label: &str) -> Vec<String> {
    let mut aliases = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;

    for ch in label.chars() {
        match ch {
            '(' | '（' => {
                depth += 1;
                if depth == 1 {
                    current.clear();
                } else {
                    current.push(ch);
                }
            }
            ')' | '）' => {
                if depth == 0 {
                    continue;
                }
                if depth == 1 {
                    let alias = current.trim();
                    if !alias.is_empty() && alias.is_ascii() {
                        aliases.push(alias.to_ascii_lowercase());
                    }
                    current.clear();
                } else {
                    current.push(ch);
                }
                depth -= 1;
            }
            _ if depth > 0 => current.push(ch),
            _ => {}
        }
    }

    aliases
}

fn contains_han(s: &str) -> bool {
    s.chars()
        .any(|c| matches!(c as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF))
}

#[cfg(test)]
mod tests {
    use super::{
        allowed_extra_typings, expected_ja_typings, extract_ascii_parenthetical_aliases,
        ja_typing_mismatch_reason,
    };
    use crate::types::Choice;
    use std::collections::HashMap;

    #[test]
    fn allows_official_english_spelling_as_extra_variant() {
        let mut labels = HashMap::new();
        labels.insert("ja".to_string(), "エレン・イェーガー".to_string());
        labels.insert("en".to_string(), "Eren Yeager".to_string());
        let choice = Choice {
            labels,
            ja_typings: vec!["eren yeager".to_string(), "eren ye-ga-".to_string()],
        };
        let expected = expected_ja_typings(choice.labels.get("ja").unwrap());
        let actual = vec!["eren ye-ga-".to_string(), "eren yeager".to_string()];
        assert_eq!(ja_typing_mismatch_reason(&choice, &actual, &expected), None);
    }

    #[test]
    fn rejects_extra_variant_that_is_not_official_english_spelling() {
        let mut labels = HashMap::new();
        labels.insert("ja".to_string(), "エレン・イェーガー".to_string());
        labels.insert("en".to_string(), "Eren Yeager".to_string());
        let choice = Choice {
            labels,
            ja_typings: vec!["eren ye-ga-".to_string(), "eren jaeger".to_string()],
        };
        let expected = expected_ja_typings(choice.labels.get("ja").unwrap());
        let actual = vec!["eren jaeger".to_string(), "eren ye-ga-".to_string()];
        assert!(ja_typing_mismatch_reason(&choice, &actual, &expected).is_some());
    }

    #[test]
    fn extracts_ascii_aliases_from_parentheses() {
        assert_eq!(
            extract_ascii_parenthetical_aliases("エル（Lawliet）"),
            vec!["lawliet".to_string()]
        );
        assert_eq!(
            extract_ascii_parenthetical_aliases("L (Lawliet)"),
            vec!["lawliet".to_string()]
        );
    }

    #[test]
    fn allows_parenthetical_ascii_alias_as_extra_variant() {
        let mut labels = HashMap::new();
        labels.insert("ja".to_string(), "エル（Lawliet）".to_string());
        labels.insert("en".to_string(), "L (Lawliet)".to_string());
        let choice = Choice {
            labels,
            ja_typings: vec!["eru lawliet".to_string(), "lawliet".to_string()],
        };
        assert_eq!(
            allowed_extra_typings(&choice),
            vec!["l (lawliet)".to_string(), "lawliet".to_string()]
        );
        let expected = expected_ja_typings(choice.labels.get("ja").unwrap());
        let actual = vec!["eru lawliet".to_string(), "lawliet".to_string()];
        assert_eq!(ja_typing_mismatch_reason(&choice, &actual, &expected), None);
    }
}

//! Deterministically review `ja_typings` for kana / ASCII choices and
//! auto-confirm questions whose every choice is machine-verifiable.
//!
//! For any choice whose label has no ambiguous reading — pure ASCII or
//! kana, plus the hand-curated kanji table — the canonical IME-strict
//! typing is a *function* of the label (`romaji::derive_ja_typings`). So we
//! can verify the stored typing without a human and, for questions where
//! *every* choice is derivable, flip `ja_reviewed` to `true` with high
//! confidence: the typings provably match the labels' canonical romaji.
//!
//! Kanji-bearing labels (compounds, proper nouns, number readings) have no
//! mechanical reading and are left untouched (`ja_reviewed` unchanged) for
//! the LLM-judge pass (#134).
//!
//! Modes:
//! - `verify` (CI gate): report any derivable choice whose stored typing is
//!   not canonical, and exit non-zero. No mutation.
//! - `apply`: rewrite derivable choices to their canonical typings and set
//!   `ja_reviewed = true` on every all-derivable question, then write back.

#[path = "../io/romaji.rs"]
mod romaji;

use serde_json::Value;
use std::fs;
use std::process::ExitCode;

/// Result of checking a single choice's stored typings against the
/// canonical derivation.
#[derive(Debug, PartialEq)]
enum ChoiceCheck {
    /// Label is derivable and the stored typings already equal canonical.
    Verified,
    /// Label is derivable but the stored typings differ from canonical.
    Mismatch { canonical: Vec<String> },
    /// Label is derivable but the canonical typing itself would fail the
    /// IME-strict form linter (e.g. a label with a `:` whose canonical
    /// gains a space). We refuse to write a typing the linter rejects, so
    /// such a choice is left untouched and surfaced for human attention.
    Unsafe { canonical: Vec<String> },
    /// Label is kanji-bearing with no mechanical reading — left for review.
    NotDerivable,
}

/// `true` unless a canonical variant contains whitespace that `ja` itself
/// does not. Mirrors `lint_ja_typings.py`'s S1 rule: a space in the typing
/// is only legal when the label has one too. The romaji engine turns some
/// separators (`:`, `(`, `)`, …) into spaces, which would otherwise produce
/// a typing the form linter rejects — so the auto-reviewer must never emit
/// or confirm one.
fn canonical_is_lint_safe(ja: &str, canonical: &[String]) -> bool {
    let ja_has_ws = ja.chars().any(|c| c.is_whitespace() || c == '　');
    if ja_has_ws {
        return true;
    }
    !canonical
        .iter()
        .any(|t| t.chars().any(|c| c.is_whitespace() || c == '　'))
}

fn check_choice(ja: &str, stored: &[String]) -> ChoiceCheck {
    match romaji::derive_ja_typings(ja) {
        None => ChoiceCheck::NotDerivable,
        Some(canonical) if !canonical_is_lint_safe(ja, &canonical) => {
            ChoiceCheck::Unsafe { canonical }
        }
        Some(canonical) => {
            if stored == canonical.as_slice() {
                ChoiceCheck::Verified
            } else {
                ChoiceCheck::Mismatch { canonical }
            }
        }
    }
}

#[derive(Default)]
struct Report {
    questions: usize,
    /// Questions where every choice is derivable (auto-confirmable).
    all_derivable: usize,
    /// Questions with at least one kanji-bearing choice (left for #134).
    has_kanji: usize,
    /// Derivable choices whose stored typings were not canonical.
    mismatches: Vec<String>,
    /// Derivable choices whose canonical typing would fail the form linter
    /// (left untouched, never auto-confirmed).
    needs_attention: Vec<String>,
    /// `apply`: choices whose typings were rewritten to canonical.
    fixed_choices: usize,
    /// `apply`: questions flipped from ja_reviewed=false to true.
    confirmed_now: usize,
}

fn stored_typings(choice: &Value) -> Vec<String> {
    choice
        .get("ja_typings")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Process one question. In `apply` mode mutates `question` in place
/// (canonicalises derivable typings, sets `ja_reviewed=true` when every
/// choice is derivable). Accumulates findings into `report`.
fn process_question(question: &mut Value, apply: bool, report: &mut Report) {
    report.questions += 1;
    let qid = question
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("?")
        .to_string();

    let Some(choices) = question.get_mut("choices").and_then(Value::as_array_mut) else {
        return;
    };

    let mut all_derivable = true;
    for (idx, choice) in choices.iter_mut().enumerate() {
        let Some(ja) = choice.get("ja").and_then(Value::as_str).map(str::to_string) else {
            all_derivable = false;
            continue;
        };
        let stored = stored_typings(choice);
        match check_choice(&ja, &stored) {
            ChoiceCheck::Verified => {}
            ChoiceCheck::NotDerivable => all_derivable = false,
            ChoiceCheck::Unsafe { canonical } => {
                // Canonical itself is not form-linter safe — never rewrite
                // or confirm; leave the stored typing for a human to fix.
                all_derivable = false;
                report.needs_attention.push(format!(
                    "{qid}#{idx} {ja}: canonical {canonical:?} would fail form lint (stored {stored:?})"
                ));
            }
            ChoiceCheck::Mismatch { canonical } => {
                report.mismatches.push(format!(
                    "{qid}#{idx} {ja}: stored {stored:?} != canonical {canonical:?}"
                ));
                if apply {
                    if let Some(obj) = choice.as_object_mut() {
                        obj.insert(
                            "ja_typings".to_string(),
                            Value::Array(canonical.into_iter().map(Value::String).collect()),
                        );
                        report.fixed_choices += 1;
                    }
                }
            }
        }
    }

    if all_derivable {
        report.all_derivable += 1;
        if apply {
            let was_reviewed = question
                .get("ja_reviewed")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !was_reviewed {
                report.confirmed_now += 1;
            }
            if let Some(obj) = question.as_object_mut() {
                obj.insert("ja_reviewed".to_string(), Value::Bool(true));
            }
        }
    } else {
        report.has_kanji += 1;
    }
}

fn main() -> ExitCode {
    let mode = std::env::args().nth(1).unwrap_or_default();
    let path = std::env::args().nth(2);
    let apply = match mode.as_str() {
        "verify" => false,
        "apply" => true,
        _ => {
            eprintln!("usage: review-ja-typings <verify|apply> <path-to-questions_ja.json>");
            return ExitCode::from(2);
        }
    };
    let Some(path) = path else {
        eprintln!("usage: review-ja-typings <verify|apply> <path-to-questions_ja.json>");
        return ExitCode::from(2);
    };

    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("{path}: read error: {err}");
            return ExitCode::from(1);
        }
    };
    let mut json: Value = match serde_json::from_str(&text) {
        Ok(json) => json,
        Err(err) => {
            eprintln!("{path}: parse error: {err}");
            return ExitCode::from(1);
        }
    };
    let Some(questions) = json.as_array_mut() else {
        eprintln!("{path}: top-level JSON must be an array");
        return ExitCode::from(1);
    };

    let mut report = Report::default();
    for question in questions.iter_mut() {
        process_question(question, apply, &mut report);
    }

    if apply {
        let formatted = match serde_json::to_string_pretty(&json) {
            Ok(text) => text + "\n",
            Err(err) => {
                eprintln!("{path}: serialize error: {err}");
                return ExitCode::from(1);
            }
        };
        if let Err(err) = fs::write(&path, formatted) {
            eprintln!("{path}: write error: {err}");
            return ExitCode::from(1);
        }
    }

    println!("questions                 : {}", report.questions);
    println!("all-derivable (kana/ASCII): {}", report.all_derivable);
    println!("has-kanji (left for #134) : {}", report.has_kanji);
    println!("non-canonical typings     : {}", report.mismatches.len());
    println!(
        "needs attention (unsafe)  : {}",
        report.needs_attention.len()
    );
    for m in report.needs_attention.iter() {
        println!("  ! {m}");
    }
    if apply {
        println!("typings canonicalised     : {}", report.fixed_choices);
        println!("ja_reviewed confirmed now : {}", report.confirmed_now);
    } else {
        for m in report.mismatches.iter().take(20) {
            println!("  {m}");
        }
        if report.mismatches.len() > 20 {
            println!("  ... and {} more", report.mismatches.len() - 20);
        }
    }

    // `verify` fails the build when any derivable choice is non-canonical;
    // `apply` always succeeds (it just fixed them).
    if !apply && !report.mismatches.is_empty() {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ascii_label_verified_when_lowercased() {
        assert_eq!(
            check_choice("H2O", &["h2o".to_string()]),
            ChoiceCheck::Verified
        );
    }

    #[test]
    fn ascii_label_mismatch_when_not_lowercase() {
        assert_eq!(
            check_choice("H2O", &["H2O".to_string()]),
            ChoiceCheck::Mismatch {
                canonical: vec!["h2o".to_string()]
            }
        );
    }

    #[test]
    fn kana_label_verified_against_canonical() {
        // ソウル -> souru (the legacy `soru` variant is not canonical).
        assert_eq!(
            check_choice("ソウル", &["souru".to_string()]),
            ChoiceCheck::Verified
        );
    }

    #[test]
    fn kana_label_with_stale_variant_is_mismatch() {
        match check_choice("ソウル", &["souru".to_string(), "soru".to_string()]) {
            ChoiceCheck::Mismatch { canonical } => assert_eq!(canonical, vec!["souru".to_string()]),
            other => panic!("expected mismatch, got {other:?}"),
        }
    }

    #[test]
    fn kanji_label_is_not_derivable() {
        // 陽子 has an ambiguous reading (kakasi guesses the name "Yoko");
        // it must be left for review, never auto-verified.
        assert_eq!(
            check_choice("陽子", &["youshi".to_string()]),
            ChoiceCheck::NotDerivable
        );
    }

    #[test]
    fn colon_label_is_unsafe_not_mismatch() {
        // イド:インヴェイデッド: the engine turns `:` into a space, so the
        // canonical typing would gain whitespace the label lacks and fail
        // the form linter's S1 rule. The reviewer must flag it Unsafe and
        // leave the stored typing alone — never rewrite it to a space form.
        match check_choice("イド:インヴェイデッド", &["idoinveideddo".to_string()]) {
            ChoiceCheck::Unsafe { canonical } => {
                assert!(canonical.iter().any(|t| t.contains(' ')));
            }
            other => panic!("expected Unsafe, got {other:?}"),
        }
    }

    #[test]
    fn unsafe_choice_blocks_question_confirmation() {
        let mut q = json!({
            "id": "qcolon",
            "choices": [
                {"ja": "イド:インヴェイデッド", "ja_typings": ["idoinveideddo"]},
                {"ja": "ソウル", "ja_typings": ["souru"]}
            ],
            "ja_reviewed": false
        });
        let mut report = Report::default();
        process_question(&mut q, true, &mut report);

        assert_eq!(report.needs_attention.len(), 1);
        assert_eq!(report.all_derivable, 0, "unsafe choice must block confirm");
        assert_eq!(report.fixed_choices, 0, "unsafe typing left untouched");
        assert_eq!(q["ja_reviewed"], json!(false));
        assert_eq!(
            q["choices"][0]["ja_typings"],
            json!(["idoinveideddo"]),
            "stored typing for the unsafe choice is preserved"
        );
    }

    #[test]
    fn canonical_lint_safe_allows_space_when_label_has_space() {
        // A label that itself contains whitespace may legitimately produce a
        // spaced typing (e.g. "O(log n)") — that is not unsafe.
        assert!(canonical_is_lint_safe("a b", &["a b".to_string()]));
        assert!(!canonical_is_lint_safe("a:b", &["a b".to_string()]));
    }

    #[test]
    fn apply_confirms_all_kana_question_and_canonicalises() {
        let mut q = json!({
            "id": "qtest",
            "choices": [
                {"ja": "ソウル", "ja_typings": ["souru", "soru"]},
                {"ja": "H2O", "ja_typings": ["h2o"]}
            ],
            "ja_reviewed": false
        });
        let mut report = Report::default();
        process_question(&mut q, true, &mut report);

        assert_eq!(report.all_derivable, 1);
        assert_eq!(report.has_kanji, 0);
        assert_eq!(report.fixed_choices, 1, "the stale `soru` variant is fixed");
        assert_eq!(report.confirmed_now, 1);
        assert_eq!(q["ja_reviewed"], json!(true));
        assert_eq!(q["choices"][0]["ja_typings"], json!(["souru"]));
    }

    #[test]
    fn apply_leaves_kanji_question_unconfirmed() {
        let mut q = json!({
            "id": "qkanji",
            "choices": [
                {"ja": "陽子", "ja_typings": ["youshi"]},
                {"ja": "ソウル", "ja_typings": ["souru"]}
            ],
            "ja_reviewed": false
        });
        let mut report = Report::default();
        process_question(&mut q, true, &mut report);

        assert_eq!(report.all_derivable, 0);
        assert_eq!(report.has_kanji, 1);
        assert_eq!(
            q["ja_reviewed"],
            json!(false),
            "kanji question must stay unreviewed for #134"
        );
    }

    #[test]
    fn verify_mode_does_not_mutate() {
        let mut q = json!({
            "id": "qtest",
            "choices": [{"ja": "ソウル", "ja_typings": ["souru", "soru"]}],
            "ja_reviewed": false
        });
        let mut report = Report::default();
        process_question(&mut q, false, &mut report);

        assert_eq!(report.mismatches.len(), 1, "mismatch is reported");
        assert_eq!(report.fixed_choices, 0, "verify never fixes");
        assert_eq!(q["ja_reviewed"], json!(false), "verify never flips flag");
        assert_eq!(
            q["choices"][0]["ja_typings"],
            json!(["souru", "soru"]),
            "verify never rewrites typings"
        );
    }
}

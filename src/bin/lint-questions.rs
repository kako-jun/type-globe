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

    for path in &paths {
        match load(path) {
            Ok(questions) => {
                let conflicts = find_prefix_conflicts(&questions);
                for c in &conflicts {
                    eprintln!("{}: {}", path, format_conflict(c));
                }
                total_conflicts += conflicts.len();
            }
            Err(e) => {
                eprintln!("{}: load error: {}", path, e);
                load_errors += 1;
            }
        }
    }

    if total_conflicts > 0 || load_errors > 0 {
        eprintln!(
            "lint-questions: {} conflict(s), {} load error(s) across {} file(s)",
            total_conflicts,
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

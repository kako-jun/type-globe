#[path = "../io/romaji.rs"]
mod romaji;

use serde_json::Value;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: backfill-ja-typing <path-to-questions_ja.json>");
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

    for question in questions {
        let Some(choices) = question.get_mut("choices").and_then(Value::as_array_mut) else {
            continue;
        };
        for choice in choices {
            let Some(obj) = choice.as_object_mut() else {
                continue;
            };
            let Some(ja) = obj.get("ja").and_then(Value::as_str) else {
                continue;
            };
            obj.insert(
                "ja_typings".to_string(),
                Value::Array(
                    derive_ja_typings(ja)
                        .into_iter()
                        .map(Value::String)
                        .collect(),
                ),
            );
        }
    }

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

    ExitCode::SUCCESS
}

fn derive_ja_typings(ja: &str) -> Vec<String> {
    match ja {
        "酸素" => vec!["sanso".to_string()],
        "鉄" => vec!["tetsu".to_string()],
        _ if ja.is_ascii() => vec![ja.to_ascii_lowercase()],
        _ => romaji::hiragana_to_hepburn_variants(ja),
    }
}

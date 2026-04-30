mod config;
mod game;
mod io;
mod jiwa_core;
mod types;
mod ui;

use config::Config;
use io::{DataLoader, Storage};
use std::io::{stdin, stdout, Write};
use types::{GameMode, Question};
use ui::{MenuUI, QuizUI, RecordsUI};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();
    let mut menu = MenuUI::new();

    Storage::ensure_data_directory(&config.data_dir)?;

    loop {
        let (language, mode) = match menu.run() {
            Ok(result) => result,
            Err(_) => return Ok(()),
        };

        match mode {
            GameMode::Quiz => {
                let questions_file = config.questions_file_path(&language);

                if !std::path::Path::new(&questions_file).exists() {
                    println!("問題ファイルが見つかりません。サンプル問題を作成しています...");
                    let sample_questions = DataLoader::create_sample_questions();
                    Storage::save_sample_questions(&questions_file, &sample_questions)?;
                    println!("サンプル問題を作成しました: {questions_file}");
                }

                let questions = load_questions_with_warnings(&questions_file)?;
                if questions.is_empty() {
                    println!("問題が見つかりません。");
                    return Ok(());
                }

                // 10-question sampled run + result screen + Records save are
                // all owned by QuizUI now (#26). main.rs only supplies the
                // pool and the records file path.
                let records_path = config.records_file_path(&language);
                let mut quiz_ui = QuizUI::from_pool(&questions, language.clone(), records_path);
                let _final_score = quiz_ui.run()?;

                menu.return_to_mode_selection(language);
            }
            GameMode::TimeAttack25 => {
                show_return_to_menu_message("Time Attack 25 is not implemented yet.")?;
                menu.return_to_mode_selection(language);
            }
            GameMode::HackAndSlashRpg => {
                show_return_to_menu_message("Listening RPG is not implemented yet.")?;
                menu.return_to_mode_selection(language);
            }
            GameMode::Records => {
                let records_path = config.records_file_path(&language);
                let mut records_ui = RecordsUI::load(&records_path)?;
                records_ui.run()?;
                menu.return_to_mode_selection(language);
            }
        }
    }
}

/// Load a question bank and warn (non-fatally) on any prefix conflicts in
/// the data. Routing every question-loading code path through this helper
/// keeps future modes (Time Attack 25, Records) from silently bypassing the
/// `docs/spec.md` integrity check (#27).
fn load_questions_with_warnings(path: &str) -> Result<Vec<Question>, Box<dyn std::error::Error>> {
    let questions = DataLoader::load_questions(path)?;
    for c in io::find_prefix_conflicts(&questions) {
        eprintln!("warning: {}", io::format_conflict(&c));
    }
    Ok(questions)
}

fn show_return_to_menu_message(message: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("{message}");
    println!("Press Enter to return to the menu.");
    stdout().flush()?;

    let mut input = String::new();
    stdin().read_line(&mut input)?;
    Ok(())
}

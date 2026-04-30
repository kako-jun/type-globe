mod config;
mod game;
mod io;
mod types;
mod ui;

use config::Config;
use io::{DataLoader, Storage};
use std::io::{stdin, stdout, Write};
use types::GameMode;
use ui::{MenuUI, QuizUI};

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

                let questions = DataLoader::load_questions(&questions_file)?;
                if questions.is_empty() {
                    println!("問題が見つかりません。");
                    return Ok(());
                }

                // Warn (don't block) on prefix conflicts in the question
                // bank. The shipped data is checked in unit tests; this
                // catches user-supplied or freshly added questions at run
                // time. See `docs/spec.md` and Issue #27.
                let conflicts = io::find_prefix_conflicts(&questions);
                for c in &conflicts {
                    eprintln!("warning: {}", io::format_conflict(c));
                }

                let mut quiz_ui = QuizUI::new(questions, language.clone());
                let final_score = quiz_ui.run()?;

                show_return_to_menu_message(&format!("Quiz finished. Final score: {final_score}"))?;
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
            GameMode::Ranking => {
                show_return_to_menu_message("Ranking is not implemented yet.")?;
                menu.return_to_mode_selection(language);
            }
        }
    }
}

fn show_return_to_menu_message(message: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("{message}");
    println!("Press Enter to return to the menu.");
    stdout().flush()?;

    let mut input = String::new();
    stdin().read_line(&mut input)?;
    Ok(())
}

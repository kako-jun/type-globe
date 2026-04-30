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

    Storage::ensure_data_directory(&config.data_dir)?;

    loop {
        let mut menu = MenuUI::new();
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

                let mut quiz_ui = QuizUI::new(questions, language);
                let final_score = quiz_ui.run()?;

                println!("ゲーム終了！最終スコア: {final_score}");
                return Ok(());
            }
            GameMode::TimeAttack25 => {
                show_unimplemented_mode_message("Time Attack 25")?;
            }
            GameMode::HackAndSlashRpg => {
                show_unimplemented_mode_message("Listening RPG")?;
            }
            GameMode::Ranking => {
                show_unimplemented_mode_message("Ranking")?;
            }
        }
    }
}

fn show_unimplemented_mode_message(mode_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("{mode_name} is not implemented yet.");
    println!("Press Enter to return to the menu.");
    stdout().flush()?;

    let mut input = String::new();
    stdin().read_line(&mut input)?;
    Ok(())
}

mod types;
mod config;
mod io;
mod game;
mod ui;

use config::Config;
use io::{DataLoader, Storage};
use types::GameMode;
use ui::{MenuUI, QuizUI};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();
    
    Storage::ensure_data_directory(&config.data_dir)?;

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
                println!("サンプル問題を作成しました: {}", questions_file);
            }

            let questions = DataLoader::load_questions(&questions_file)?;
            if questions.is_empty() {
                println!("問題が見つかりません。");
                return Ok(());
            }
            
            let mut quiz_ui = QuizUI::new(questions, language);
            let final_score = quiz_ui.run()?;
            
            println!("ゲーム終了！最終スコア: {}", final_score);
        }
        GameMode::TimeAttack25 => {
            println!("タイムアタック25モードを開始します...");
        }
        GameMode::HackAndSlashRpg => {
            println!("リスニングRPGモードを開始します...");
        }
        GameMode::Ranking => {
            println!("ランキング画面を開始します...");
        }
    }

    Ok(())
}

mod types;
mod config;
mod io;
mod game;
mod ui;

use config::Config;
use io::{DataLoader, Storage, TypingTexts};
use types::{GameMode, Language};
use ui::{MenuUI, QuizUI, TypingUI};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();
    
    Storage::ensure_data_directory(&config.data_dir)?;

    let mut menu = MenuUI::new();
    let (language, mode) = menu.run()?;

    let questions_file = config.questions_file_path(&language);
    
    if !std::path::Path::new(&questions_file).exists() {
        println!("問題ファイルが見つかりません。サンプル問題を作成しています...");
        let sample_questions = DataLoader::create_sample_questions();
        Storage::save_sample_questions(&questions_file, &sample_questions)?;
        println!("サンプル問題を作成しました: {}", questions_file);
    }

    match mode {
        GameMode::Quiz => {
            let questions = DataLoader::load_questions(&questions_file)?;
            if questions.is_empty() {
                println!("問題が見つかりません。");
                return Ok(());
            }
            
            let mut quiz_ui = QuizUI::new(questions, language);
            let final_score = quiz_ui.run()?;
            
            println!("ゲーム終了！最終スコア: {}", final_score);
        }
        GameMode::Typing => {
            let typing_text = TypingTexts::get_random_text(&language);
            let mut typing_ui = TypingUI::new(typing_text);
            let result = typing_ui.run()?;
            
            println!("タイピング完了！");
            println!("WPM: {:.1}", result.wpm);
            println!("正確性: {:.1}%", result.accuracy);
            println!("時間: {:.1}秒", result.total_time.as_secs_f32());
            println!("エラー数: {}", result.errors);
        }
        GameMode::QuizTyping => {
            println!("クイズ+タイピングモードを開始します...");
        }
        GameMode::TimeAttack => {
            println!("タイムアタック25モードを開始します...");
        }
        GameMode::Rpg => {
            println!("RPGモードを開始します...");
        }
        GameMode::Stealth => {
            println!("ステルスモードを開始します...");
        }
    }

    Ok(())
}

mod audio;
mod config;
mod game;
mod io;
mod jiwa_core;
mod types;
mod ui;

use audio::TtsEngine;
use clap::{Parser, Subcommand};
use config::Config;
use game::ListeningSession;
use io::{DataLoader, Storage};
use std::io::{stdin, stdout, Write};
use types::{AnswerKind, GameMode, Language, ListeningPrompt, Question};
use ui::{tts_unavailable_message, ListenUI, MenuUI, QuizUI, RecordsUI};

// ---------------------------------------------------------------------------
// CLI definition (#48)
// ---------------------------------------------------------------------------

/// type-globe — terminal typing game
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// クイズモードを即開始
    Quiz {
        /// 言語を指定（ja / en）。省略時はメニューで選択
        #[arg(long, value_parser = parse_language)]
        lang: Option<Language>,

        /// 出題順を固定するシード値（スタブ: 受け取るが未実装）
        #[arg(long)]
        seed: Option<u64>,

        /// 特定の問題IDから開始（スタブ: 受け取るが未実装）
        #[arg(long)]
        question: Option<String>,
    },

    /// ハクスラRPGモードを即開始
    Rpg {
        /// 言語を指定（ja / en）。省略時はメニューで選択
        #[arg(long, value_parser = parse_language)]
        lang: Option<Language>,

        /// 出題順を固定するシード値（スタブ: 受け取るが未実装）
        #[arg(long)]
        seed: Option<u64>,

        /// 指定フロアから開始（スタブ: 受け取るが未実装）
        #[arg(long)]
        floor: Option<u32>,

        /// TTS 読み上げをスキップする
        #[arg(long)]
        no_tts: bool,
    },

    /// Time Attack 25 を即開始
    Ta25 {
        /// 言語を指定（ja / en）。省略時はメニューで選択
        #[arg(long, value_parser = parse_language)]
        lang: Option<Language>,

        /// 出題順を固定するシード値（スタブ: 受け取るが未実装）
        #[arg(long)]
        seed: Option<u64>,
    },

    /// ランキングを表示
    Ranking {
        /// 言語を指定（ja / en）。省略時はメニューで選択
        #[arg(long, value_parser = parse_language)]
        lang: Option<Language>,
    },
}

fn parse_language(s: &str) -> Result<Language, String> {
    match s {
        "ja" => Ok(Language::Japanese),
        "en" => Ok(Language::English),
        other => Err(format!("不明な言語コード: '{other}'. ja または en を指定してください")),
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let config = Config::default();

    Storage::ensure_data_directory(&config.data_dir)?;

    match cli.command {
        // ---- サブコマンドなし: 従来どおりメインメニューへ ----
        None => run_menu_loop(&config),

        // ---- quiz サブコマンド ----
        Some(Commands::Quiz { lang, seed, question }) => {
            // TODO(#48): --seed は未実装。引数を受け取るのみ。
            if seed.is_some() {
                eprintln!("note: --seed は現在未実装です（スタブ）");
            }
            // TODO(#48): --question は未実装。引数を受け取るのみ。
            if question.is_some() {
                eprintln!("note: --question は現在未実装です（スタブ）");
            }

            let language = resolve_language_or_select(lang)?;
            run_quiz_mode(&config, &language)?;
            Ok(())
        }

        // ---- rpg サブコマンド ----
        Some(Commands::Rpg { lang, seed, floor, no_tts }) => {
            // TODO(#48): --seed は未実装。引数を受け取るのみ。
            if seed.is_some() {
                eprintln!("note: --seed は現在未実装です（スタブ）");
            }
            // TODO(#48): --floor は未実装。引数を受け取るのみ。
            if floor.is_some() {
                eprintln!("note: --floor は現在未実装です（スタブ）");
            }

            let language = resolve_language_or_select(lang)?;
            run_listening_practice(&config, &language, no_tts)?;
            Ok(())
        }

        // ---- ta25 サブコマンド ----
        Some(Commands::Ta25 { lang, seed }) => {
            // TODO(#48): --seed は未実装。引数を受け取るのみ。
            if seed.is_some() {
                eprintln!("note: --seed は現在未実装です（スタブ）");
            }

            let _language = resolve_language_or_select(lang)?;
            show_return_to_menu_message("Time Attack 25 is not implemented yet.")?;
            Ok(())
        }

        // ---- ranking サブコマンド ----
        Some(Commands::Ranking { lang }) => {
            let language = resolve_language_or_select(lang)?;
            let records_path = config.records_file_path(&language);
            let mut records_ui = RecordsUI::load(&records_path)?;
            records_ui.run()?;
            Ok(())
        }
    }
}

/// サブコマンドで --lang が省略された場合、簡易選択プロンプトを表示する。
fn resolve_language_or_select(lang: Option<Language>) -> Result<Language, Box<dyn std::error::Error>> {
    if let Some(l) = lang {
        return Ok(l);
    }
    // 簡易プロンプト（メニュー TUI を経由しない）
    loop {
        print!("言語を選択してください (ja/en): ");
        stdout().flush()?;
        let mut input = String::new();
        stdin().read_line(&mut input)?;
        match input.trim() {
            "ja" => return Ok(Language::Japanese),
            "en" => return Ok(Language::English),
            _ => println!("ja または en を入力してください。"),
        }
    }
}

// ---------------------------------------------------------------------------
// メニューループ（サブコマンドなし時の従来フロー）
// ---------------------------------------------------------------------------

fn run_menu_loop(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let mut menu = MenuUI::new();

    loop {
        let (language, mode) = match menu.run() {
            Ok(result) => result,
            Err(_) => return Ok(()),
        };

        match mode {
            GameMode::Quiz => {
                run_quiz_mode(config, &language)?;
                menu.return_to_mode_selection(language);
            }
            GameMode::TimeAttack25 => {
                show_return_to_menu_message("Time Attack 25 is not implemented yet.")?;
                menu.return_to_mode_selection(language);
            }
            GameMode::HackAndSlashRpg => {
                run_listening_practice(config, &language, false)?;
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

// ---------------------------------------------------------------------------
// モード実装ヘルパー
// ---------------------------------------------------------------------------

fn run_quiz_mode(config: &Config, language: &Language) -> Result<(), Box<dyn std::error::Error>> {
    let questions_file = config.questions_file_path(language);

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

    let records_path = config.records_file_path(language);
    let mut quiz_ui = QuizUI::from_pool(&questions, language.clone(), records_path);
    let _final_score = quiz_ui.run()?;
    Ok(())
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

/// One round of listening practice (#28-#31). v0.2.0 foundation only —
/// the 10-prompt run loop is #32-#37. Foundation restricts the pool to
/// `word`-kind prompts because Space is reserved for replay (per
/// `docs/spec.md`); phrase / sentence input mapping is part of the
/// run-loop work and is intentionally out of scope here.
///
/// `skip_tts`: when `true` (set via `rpg --no-tts`), the TTS engine is
/// not initialised and the session runs silently. Useful for debugging
/// in environments where TTS is unavailable or undesirable.
fn run_listening_practice(
    config: &Config,
    language: &Language,
    skip_tts: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = config.listening_file_path(language);
    let prompts = DataLoader::load_listening_prompts(&path)?;
    let pool: Vec<ListeningPrompt> = prompts
        .into_iter()
        .filter(|p| p.kind == AnswerKind::Word)
        .collect();

    if pool.is_empty() {
        show_return_to_menu_message(
            "No listening prompts available for this language. Add `data/listening_<lang>.json`.",
        )?;
        return Ok(());
    }

    let session = match ListeningSession::from_pool(&pool, language.clone()) {
        Some(s) => s,
        None => {
            show_return_to_menu_message("Failed to pick a listening prompt.")?;
            return Ok(());
        }
    };

    if skip_tts {
        // --no-tts: TTS を初期化せずサイレント実行
        let mut ui = ListenUI::new_without_tts(session, language.clone());
        let _ = ui.run()?;
        return Ok(());
    }

    let tts = match TtsEngine::new() {
        Ok(t) => t,
        Err(err) => {
            show_return_to_menu_message(&tts_unavailable_message(err.as_ref()))?;
            return Ok(());
        }
    };

    let mut ui = ListenUI::new(session, tts, language.clone());
    let _ = ui.run()?;
    Ok(())
}

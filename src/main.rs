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
use std::time::Duration;
use types::{AnswerKind, GameMode, Language, ListeningPrompt, Question};
use ui::{tts_unavailable_message, DemoInputSource, ListenUI, MenuUI, QuizUI, RecordsUI};

// ---------------------------------------------------------------------------
// CLI definition (#48)
// ---------------------------------------------------------------------------

/// type-globe — terminal typing game
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    // ----- Auto-demo flags (#106) -----
    // These are top-level (not under a subcommand) so existing demos and
    // onboarding scripts can do `type-globe --demo --lang ja` without
    // having to remember which subcommand owns demo mode. They are
    // ignored when any subcommand is supplied.
    /// 自動デモモードで起動する（無人ループ展示・宣伝動画用）。1問ごとに
    /// `--demo-wait-ms` 待機したあと正解を自動入力する。
    #[arg(long)]
    demo: bool,

    /// デモで連続出題する問題数（default 10）。
    #[arg(long, default_value_t = 10)]
    demo_count: u32,

    /// 各問の開始から自動打鍵を始めるまでの待機時間 (ms, default 1000)。
    #[arg(long, default_value_t = 1000)]
    demo_wait_ms: u64,

    /// 1秒あたりの自動打鍵数 (default 20)。
    #[arg(long, default_value_t = 20)]
    demo_type_cps: u32,

    /// 終端させずにデモを永続ループする (Esc / Ctrl+C で中断)。
    #[arg(long)]
    demo_loop: bool,

    /// デモモードの言語指定 (ja / en)。`--demo` 時のみ参照される。
    #[arg(long, value_parser = parse_language)]
    lang: Option<Language>,

    /// デモモードのジャンル絞り込み。指定したジャンルの問題だけから出題する。
    #[arg(long)]
    genre: Option<String>,
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
        other => Err(format!(
            "不明な言語コード: '{other}'. ja または en を指定してください"
        )),
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let config = Config::default();

    Storage::ensure_data_directory(&config.data_dir)?;

    // --demo は最優先。サブコマンド経路を通さず、専用の auto-demo
    // ループに直行する。--demo 指定時はサブコマンドを無視する仕様。
    //
    // 言語選択は --lang が未指定なら Japanese を採用する。demo は
    // 無人ループ展示・OBS 録画・CI 動画生成など非対話用途が中心で、
    // TTY/stdin が無い環境で `resolve_language_or_select` の対話
    // プロンプトに詰まると死ぬため、ここでは絶対にプロンプトを出さない
    // (R-1)。日本語デフォルトは type-globe の主用途と既存データ量、
    // および日本人ユーザー優先方針に従う。
    if cli.demo {
        let language = cli.lang.clone().unwrap_or(Language::Japanese);
        return run_quiz_demo(
            &config,
            &language,
            cli.genre.as_deref(),
            DemoOptions {
                count: cli.demo_count,
                wait_ms: cli.demo_wait_ms,
                type_cps: cli.demo_type_cps,
                loop_forever: cli.demo_loop,
            },
        );
    }

    match cli.command {
        // ---- サブコマンドなし: 従来どおりメインメニューへ ----
        None => run_menu_loop(&config),

        // ---- quiz サブコマンド ----
        Some(Commands::Quiz {
            lang,
            seed,
            question,
        }) => {
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
        Some(Commands::Rpg {
            lang,
            seed,
            floor,
            no_tts,
        }) => {
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
        Some(Commands::Ta25 { lang: _, seed }) => {
            // TODO(#48): --seed は未実装。引数を受け取るのみ。
            if seed.is_some() {
                eprintln!("note: --seed は現在未実装です（スタブ）");
            }

            // Ta25 は未実装のため言語選択プロンプトを出さずに即メッセージ表示。
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
fn resolve_language_or_select(
    lang: Option<Language>,
) -> Result<Language, Box<dyn std::error::Error>> {
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
            GameMode::Rpg => {
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

// ---------------------------------------------------------------------------
// Auto-demo runner (#106)
// ---------------------------------------------------------------------------

/// Tunables that the CLI surface exposes for the auto-demo. Bundled into
/// a struct so future modes (listening demo, RPG demo) can take the
/// same configuration without growing per-call argument lists.
#[derive(Debug, Clone)]
struct DemoOptions {
    count: u32,
    wait_ms: u64,
    type_cps: u32,
    loop_forever: bool,
}

/// Run the quiz under the auto-demo driver. Loads the question pool
/// (optionally filtered by genre), then either runs one demo session
/// or loops until the user aborts with Esc / Ctrl+C.
fn run_quiz_demo(
    config: &Config,
    language: &Language,
    genre: Option<&str>,
    options: DemoOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let questions_file = config.questions_file_path(language);
    let mut questions = load_questions_with_warnings(&questions_file)?;

    if let Some(g) = genre {
        questions = DataLoader::filter_questions_by_genre(&questions, Some(g));
        if questions.is_empty() {
            eprintln!("error: --genre '{g}' に一致する問題がありません。");
            return Ok(());
        }
    }

    if questions.is_empty() {
        println!("問題が見つかりません。");
        return Ok(());
    }

    let records_path = config.records_file_path(language);
    let count = options.count.max(1) as usize;
    let wait = Duration::from_millis(options.wait_ms);

    // M-3: consecutive `no_target_abort` counter so a broken question
    // pool (e.g. every sample lacks `ja_typings`) can't kiosk-spin the
    // demo forever. Three sessions in a row failing to derive a typing
    // target is enough evidence the data — not transient flakiness —
    // is the problem; bail out with a final message.
    const MAX_CONSECUTIVE_NO_TARGET_ABORTS: u32 = 3;
    let mut consecutive_aborts: u32 = 0;

    loop {
        let demo = DemoInputSource::new(options.type_cps, wait);
        let mut quiz_ui =
            QuizUI::from_pool_with_count(&questions, language.clone(), records_path.clone(), count);
        // Demo path discards the score — the operator only cares that
        // the run completes and the screen looks right. Errors are
        // surfaced so a broken terminal doesn't get swallowed in loop
        // mode.
        let outcome = quiz_ui.run_with_demo(demo)?;

        // M-2: emit warnings *after* the alt screen has been torn down
        // (this is the first safe place — `run_with_demo` already
        // restored the terminal before returning).
        for w in &outcome.warnings {
            eprintln!("{w}");
        }

        // S-1: an explicit Esc / Ctrl+C from the user must break the
        // outer `--demo-loop` too. Without this, hitting Esc inside a
        // looping kiosk demo would immediately restart the next run.
        if outcome.user_aborted {
            break;
        }

        // M-3: if the session aborted because no typing target could
        // be found, count it; bail after a small streak rather than
        // looping forever on broken data.
        if outcome.no_target_abort {
            consecutive_aborts += 1;
            if consecutive_aborts >= MAX_CONSECUTIVE_NO_TARGET_ABORTS {
                eprintln!(
                    "demo stopped: aborted {consecutive_aborts} sessions in a row because the chosen question had no typing target. \
                     Check that `data/questions_{}.json` has `ja_typings` populated for the relevant questions.",
                    match language { Language::Japanese => "ja", Language::English => "en" }
                );
                break;
            }
        } else {
            consecutive_aborts = 0;
        }

        if !options.loop_forever {
            break;
        }
    }

    Ok(())
}

fn run_quiz_mode(config: &Config, language: &Language) -> Result<(), Box<dyn std::error::Error>> {
    let questions_file = config.questions_file_path(language);

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    // --- TC-01: "ja" → Language::Japanese ---
    #[test]
    fn parse_language_ja_returns_japanese() {
        assert!(matches!(parse_language("ja"), Ok(Language::Japanese)));
    }

    // --- TC-02: "en" → Language::English ---
    #[test]
    fn parse_language_en_returns_english() {
        assert!(matches!(parse_language("en"), Ok(Language::English)));
    }

    // --- TC-03: invalid inputs return Err containing the input value ---
    #[test]
    fn parse_language_zh_returns_err_containing_input() {
        let err = parse_language("zh").unwrap_err();
        assert!(
            err.contains("zh"),
            "error message should contain input 'zh': {err}"
        );
    }

    #[test]
    fn parse_language_uppercase_ja_returns_err_containing_input() {
        let err = parse_language("JA").unwrap_err();
        assert!(
            err.contains("JA"),
            "error message should contain input 'JA': {err}"
        );
    }

    #[test]
    fn parse_language_spelled_out_returns_err_containing_input() {
        let err = parse_language("japanese").unwrap_err();
        assert!(
            err.contains("japanese"),
            "error message should contain input 'japanese': {err}"
        );
    }

    // --- TC-04: empty string returns Err without panicking ---
    #[test]
    fn parse_language_empty_string_returns_err_without_panic() {
        let result = parse_language("");
        assert!(result.is_err(), "empty string must not parse successfully");
    }

    // --- TC-27: --seed u64::MAX does not cause a parse error ---
    #[test]
    fn cli_seed_u64_max_parses_without_error() {
        let args = ["type-globe", "quiz", "--seed", "18446744073709551615"];
        let cli = Cli::parse_from(args);
        match cli.command {
            Some(Commands::Quiz { seed, .. }) => {
                assert_eq!(seed, Some(u64::MAX));
            }
            other => panic!("expected Quiz subcommand, got {other:?}"),
        }
    }

    // --- TC-28: --floor u32::MAX does not cause a parse error ---
    #[test]
    fn cli_floor_u32_max_parses_without_error() {
        let args = ["type-globe", "rpg", "--floor", "4294967295"];
        let cli = Cli::parse_from(args);
        match cli.command {
            Some(Commands::Rpg { floor, .. }) => {
                assert_eq!(floor, Some(u32::MAX));
            }
            other => panic!("expected Rpg subcommand, got {other:?}"),
        }
    }

    // --- TC-29: --seed -1 causes clap to return an error (negative not accepted for u64) ---
    #[test]
    fn cli_seed_negative_one_fails_to_parse() {
        let args = ["type-globe", "quiz", "--seed", "-1"];
        let result = Cli::try_parse_from(args);
        assert!(result.is_err(), "--seed -1 should be rejected by clap");
    }
}

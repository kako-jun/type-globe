mod audio;
mod config;
mod game;
mod io;
mod types;
mod ui;

use audio::SpeechBackendHandle;
use clap::{Parser, Subcommand, ValueEnum};
use config::Config;
use game::{
    enemy::enemy_for_ordinal,
    rpg::{apply_exp_gain, apply_exp_loss, exp_gain_for_hit, MISS_EXP_PENALTY},
    title::newly_unlocked_titles,
    ListeningRpgRun, ListeningSession, RpgEncounterKind, RpgRunPhase, Ta25LocalGame,
    RPG_RUN_LENGTH, TA25_RUN_LENGTH,
};
use io::{DataLoader, Storage};
use std::io::{stdin, stdout, Error as StdIoError, ErrorKind, Write};
use std::time::{Duration, Instant};
use types::{GameMode, Language, Player, Question};
use ui::{
    tts_unavailable_message, BossListenUI, DemoInputSource, ListenUI, MenuUI, QuizUI, RecordsUI,
    TimeAttack25UI,
};

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

        /// 読み上げ backend を選ぶ（system / local-command）
        #[arg(long, value_enum, default_value_t = SpeechBackendCli::System)]
        speech_backend: SpeechBackendCli,

        /// local-command backend の起動コマンド（未指定時は OFFLINE_VOICE_RUNTIME_COMMAND）
        #[arg(long)]
        speech_command: Option<String>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum SpeechBackendCli {
    System,
    LocalCommand,
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
            speech_backend,
            speech_command,
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
            run_listening_rpg(&config, &language, no_tts, speech_backend, speech_command)?;
            Ok(())
        }

        // ---- ta25 サブコマンド ----
        Some(Commands::Ta25 { lang, seed }) => {
            // TODO(#48): --seed は未実装。引数を受け取るのみ。
            if seed.is_some() {
                eprintln!("note: --seed は現在未実装です（スタブ）");
            }

            let language = resolve_language_or_select(lang)?;
            run_ta25_mode(&config, &language)?;
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
                run_ta25_mode(config, &language)?;
                menu.return_to_mode_selection(language);
            }
            GameMode::Rpg => {
                run_listening_rpg(config, &language, false, SpeechBackendCli::System, None)?;
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
            // N-1: exit non-zero so kiosk / CI wrappers can detect the
            // empty-genre case and surface it instead of recording a
            // "successful" empty demo run.
            eprintln!("error: --genre '{g}' に一致する問題がありません。");
            std::process::exit(1);
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

fn run_ta25_mode(config: &Config, language: &Language) -> Result<(), Box<dyn std::error::Error>> {
    let questions_file = config.questions_file_path(language);
    let questions = load_questions_with_warnings(&questions_file)?;
    if questions.len() < TA25_RUN_LENGTH {
        show_return_to_menu_message(&format!(
            "Time Attack 25 needs at least {TA25_RUN_LENGTH} questions for this language.\n\
             Current pool: {}",
            questions.len()
        ))?;
        return Ok(());
    }

    let Some(game) = Ta25LocalGame::from_pool(&questions, language.clone(), "You") else {
        show_return_to_menu_message("Failed to build the local TA25 prototype run.")?;
        return Ok(());
    };
    let records_path = config.records_file_path(language);
    let mut ui = TimeAttack25UI::new(game, records_path);
    ui.run()?;
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

/// Listening RPG prototype run. Ships a fixed 10-encounter structure:
/// regular listening on 1-4 / 6-9, timed miniboss on 5, manual boss on
/// 10. Persistence (HP / EXP / titles / records) still lands in the
/// later RPG issues, but #113 wires the boss beats into the actual run.
///
/// `skip_tts`: when `true` (set via `rpg --no-tts`), no speech backend is
/// initialised and the session runs silently. Useful for debugging in
/// environments where speech is unavailable or undesirable.
fn run_listening_rpg(
    config: &Config,
    language: &Language,
    skip_tts: bool,
    speech_backend: SpeechBackendCli,
    speech_command: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = config.listening_file_path(language);
    let prompts = DataLoader::load_listening_prompts(&path)?;
    if prompts.is_empty() {
        show_return_to_menu_message(
            "No listening prompts available for this language. Add `data/listening_<lang>.yaml`.",
        )?;
        return Ok(());
    }

    // #32: load persistent RPG progression up-front, save it on exit.
    // Phase 1 deliberately does *not* mutate level / exp — the load →
    // save round-trip is the acceptance criterion. EXP/level wiring lands
    // in Phase 2 (#34).
    let player_path = config.player_file_path();
    let mut player = match Storage::load_player_data(&player_path) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("warning: failed to load player.yaml ({err}); starting from defaults");
            Player::default()
        }
    };
    // Remember the language the player most recently played, so the
    // next launch can prefer it. Selection itself stays where it is —
    // this is only persistence.
    player.language = language.code().to_string();

    let mut run = match ListeningRpgRun::build(&prompts) {
        Ok(run) => run,
        Err(err) => {
            show_return_to_menu_message(&err)?;
            // Even on early-exit, persist whatever language switch the
            // player made. Failure here is non-fatal (warn only).
            // Phase 2: 検討 — closure/scopeguard で defer 化
            if let Err(err) = Storage::save_player_data(&player_path, &player) {
                eprintln!("warning: failed to save player.yaml: {err}");
            }
            return Ok(());
        }
    };

    let mut speech = if skip_tts {
        None
    } else {
        match build_speech_backend(speech_backend, speech_command) {
            Ok(speech) => Some(speech),
            Err(err) => {
                show_return_to_menu_message(&tts_unavailable_message(err.as_ref()))?;
                // Phase 2: 検討 — closure/scopeguard で defer 化
                if let Err(err) = Storage::save_player_data(&player_path, &player) {
                    eprintln!("warning: failed to save player.yaml: {err}");
                }
                return Ok(());
            }
        }
    };

    // ----- Phase-driven run loop (#33) ----------------------------------
    // Town → Diving → Encounter(1..=10) → Return → Town. UI behaviour
    // matches the previous straight-line for-loop; the explicit state
    // machine is what Phase 2 (#34/#37) will hang EXP / enemy art off of.
    run.enter_diving();
    let mut correct = 0usize;
    let mut aborted = false;

    'run: loop {
        match run.phase() {
            RpgRunPhase::Town | RpgRunPhase::Return => break 'run,
            RpgRunPhase::Diving | RpgRunPhase::Encounter(_) => {}
        }

        // `advance_to_next_encounter` transitions Diving → Encounter(1)
        // and Encounter(N) → Encounter(N+1) / Return. When it returns
        // `None` the run is done.
        let encounter = match run.advance_to_next_encounter() {
            Some(encounter) => encounter.clone(),
            None => break 'run,
        };

        // #37: pick the cosmetic enemy spec for this beat (regular pool
        // cycles by ordinal; 5 = miniboss, 10 = boss).
        let enemy = enemy_for_ordinal(encounter.ordinal);

        // #34: measure how long the player takes to land an exact match,
        // so the speed bonus has something concrete to feed off. The
        // session itself doesn't (yet) expose elapsed time, so we wrap
        // the UI call in an `Instant`.
        let encounter_started_at = Instant::now();

        let session = ListeningSession::new(encounter.prompt.clone(), language.clone());
        let result = match encounter.kind {
            RpgEncounterKind::Regular => {
                let mut ui = if let Some(engine) = speech.take() {
                    ListenUI::new(session, engine, language.clone())
                } else {
                    ListenUI::new_without_tts(session, language.clone())
                };
                ui.set_run_progress(encounter.ordinal, RPG_RUN_LENGTH);
                ui.set_battle_log(run.battle_log().to_vec());
                ui.set_enemy_display(enemy.display);
                let result = ui.run()?;
                speech = ui.take_speech();
                result
            }
            RpgEncounterKind::Miniboss | RpgEncounterKind::Boss => {
                let spec = encounter
                    .prompt
                    .boss
                    .clone()
                    .expect("boss encounters are built only from boss prompts");
                let mut ui = BossListenUI::new(
                    session,
                    spec,
                    speech.take(),
                    language.clone(),
                    encounter.ordinal,
                );
                ui.set_battle_log(run.battle_log().to_vec());
                ui.set_enemy_display(enemy.display);
                let result = ui.run()?;
                speech = ui.take_speech();
                result
            }
        };

        // Esc / Ctrl+C inside the UI returns None — abort cleanly but
        // still persist player state on the way out.
        let Some(result) = result else {
            aborted = true;
            break 'run;
        };

        // #34 / #36: richer battle-log entries. The data layer carries
        // the strings; ListenUI surfaces the tail on the next encounter's
        // play pane.
        //
        // TODO(phase3): structured log を導入して 1 ビートあたり大量行
        // (連続レベルアップ + 複数称号アンロック) でも `▸ Hit / Expected:`
        // ペアが崩れない構造にする。現状は flat な Vec<String> なので、
        // 1 beat で 10+ lines emit すると BATTLE_LOG_MAX (64) の tail に
        // Hit/Expected の片割れだけ残るリスクがある。Phase 3 では
        // BattleLogEntry { kind, lines } のような構造体を導入し、UI 側で
        // entry 単位に表示 (古い entry まるごとを drop) する。
        if result.is_correct {
            correct += 1;
            let elapsed = encounter_started_at.elapsed().as_secs_f64();
            let gain = exp_gain_for_hit(elapsed);
            let events = apply_exp_gain(&mut player.rpg_stats, gain);
            run.push_battle_log(format!("▸ {} defeated! +{gain} EXP", enemy.display));
            // #35: surface every level-up and any new titles each one
            // unlocked. We emit titles after the level-up line so the
            // log reads "Lv up → title unlocked".
            for event in events {
                run.push_battle_log(format!(
                    "🎉 Level up! Lv {} → {}",
                    event.old_level, event.new_level
                ));
                let unlocked =
                    newly_unlocked_titles(event.new_level, &player.rpg_stats.titles_unlocked);
                for title in unlocked {
                    player.rpg_stats.titles_unlocked.push(title.key.to_string());
                    run.push_battle_log(format!("🏆 Title unlocked: {}", title.display));
                }
            }
        } else {
            apply_exp_loss(&mut player.rpg_stats, MISS_EXP_PENALTY);
            run.push_battle_log(format!(
                "▸ Stumble against {}. -{MISS_EXP_PENALTY} EXP",
                enemy.display
            ));
        }
        run.push_battle_log(format!("  Expected: {}", encounter.prompt.text_display));
    }

    // Run finished naturally: transition Return → Town. (If aborted, we
    // skip this so future Phase-2 introspection can tell apart "ended"
    // and "bailed".)
    if !aborted {
        run.return_to_town();
    }

    // #32: persist progression on the way back to the menu. Phase 1
    // does not mutate level/exp, so this is effectively just a
    // language-write today, but it cements the load→save round-trip.
    // Phase 2: 検討 — closure/scopeguard で defer 化
    if let Err(err) = Storage::save_player_data(&player_path, &player) {
        eprintln!("warning: failed to save player.yaml: {err}");
    }

    if !aborted {
        // q1: surface the tail of the rolling battle log so the player
        // can see the last few encounters — most importantly the boss
        // (#10) Hit/Missed line, which otherwise never appears in any UI
        // because the boss UI exits immediately after its Result phase.
        const SUMMARY_LOG_TAIL: usize = 6;
        let log = run.battle_log();
        let start = log.len().saturating_sub(SUMMARY_LOG_TAIL);
        let tail = if log.is_empty() {
            String::new()
        } else {
            format!("\n\nRecent log:\n{}", log[start..].join("\n"))
        };
        show_return_to_menu_message(&format!(
            "Listening RPG run complete.\nCorrect: {correct}/{RPG_RUN_LENGTH}\nBoss structure: regular 1-4 / miniboss 5 / regular 6-9 / boss 10.{tail}"
        ))?;
    }
    Ok(())
}

fn build_speech_backend(
    backend: SpeechBackendCli,
    command: Option<String>,
) -> Result<SpeechBackendHandle, Box<dyn std::error::Error>> {
    match backend {
        SpeechBackendCli::System => SpeechBackendHandle::system(),
        SpeechBackendCli::LocalCommand => {
            let command = command
                .or_else(|| std::env::var("OFFLINE_VOICE_RUNTIME_COMMAND").ok())
                .ok_or_else(|| {
                    StdIoError::new(
                        ErrorKind::InvalidInput,
                        "--speech-backend local-command requires --speech-command or OFFLINE_VOICE_RUNTIME_COMMAND",
                    )
                })?;
            SpeechBackendHandle::local_command(command)
        }
    }
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

    #[test]
    fn rpg_speech_backend_local_command_parses() {
        let args = [
            "type-globe",
            "rpg",
            "--lang",
            "en",
            "--speech-backend",
            "local-command",
            "--speech-command",
            "ovr-qwen-daemon",
        ];
        let cli = Cli::parse_from(args);
        match cli.command {
            Some(Commands::Rpg {
                speech_backend,
                speech_command,
                ..
            }) => {
                assert_eq!(speech_backend, SpeechBackendCli::LocalCommand);
                assert_eq!(speech_command.as_deref(), Some("ovr-qwen-daemon"));
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

    #[test]
    fn missing_questions_file_returns_empty_vec() {
        let questions = load_questions_with_warnings("data/__missing__.json").expect("loads");
        assert!(questions.is_empty());
    }
}

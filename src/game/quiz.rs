use crate::io::normalize::canonical_romaji;
use crate::io::DataLoader;
use crate::types::{Language, Question};
use rand::seq::SliceRandom;
use std::time::{Duration, Instant};

/// Number of questions a single Quiz run is locked to. Per `docs/spec.md`
/// (Quiz Mode header) and Issue #26's acceptance criteria.
pub const QUIZ_RUN_LENGTH: usize = 10;

#[derive(Debug)]
pub struct QuizGame {
    questions: Vec<Question>,
    current_question_index: usize,
    score: u32,
    correct_answers: u32,
    total_answers: u32,
    /// Cumulative count of characters from *correctly* answered choice texts.
    /// CPM / WPM are derived from this and `start_time`. Wrong answers are
    /// not counted because the player never finished typing the correct
    /// choice — including them would inflate WPM for fast guessers.
    typed_correct_chars: u32,
    start_time: Option<Instant>,
    /// Elapsed time at the moment the run ended (last question answered or
    /// skipped). Once set, `get_total_time` returns this fixed value so the
    /// final result screen shows the time of the last keystroke, not a
    /// timer that keeps ticking afterwards.
    frozen_time: Option<Duration>,
    language: Language,
}

/// Per-answer outcome. Issue #70 removed the result interstitial, so
/// production code only reads `is_correct` (via `Option::is_some` on the
/// return value, since wrong answers are now blocked at the input layer).
/// The remaining fields are kept for tests and possible future reuse.
#[derive(Debug, Clone)]
pub struct QuizResult {
    #[cfg_attr(not(test), allow(dead_code))]
    pub is_correct: bool,
    #[allow(dead_code)]
    pub correct_answer_index: usize,
    #[cfg_attr(not(test), allow(dead_code))]
    pub selected_answer_index: usize,
    #[allow(dead_code)]
    pub time_taken: Duration,
}

impl QuizGame {
    /// Construct a quiz with the supplied question list as-is. Used by
    /// tests and any caller that has already curated its own ordering.
    pub fn new(questions: Vec<Question>, language: Language) -> Self {
        Self {
            questions,
            current_question_index: 0,
            score: 0,
            correct_answers: 0,
            total_answers: 0,
            typed_correct_chars: 0,
            start_time: None,
            frozen_time: None,
            language,
        }
    }

    /// Build a fresh run by sampling up to `QUIZ_RUN_LENGTH` distinct
    /// questions out of `pool`. If the pool is shorter than the run length
    /// the whole pool is used (no padding, no repeats). Order is shuffled
    /// so two consecutive runs don't see the same questions in the same
    /// sequence.
    pub fn from_pool(pool: &[Question], language: Language) -> Self {
        Self::from_pool_with_count(pool, language, QUIZ_RUN_LENGTH)
    }

    /// Same as [`from_pool`] but with a caller-specified run length.
    /// Used by the auto-demo (#106) where the operator can dial the
    /// session length up or down with `--demo-count`.
    pub fn from_pool_with_count(pool: &[Question], language: Language, count: usize) -> Self {
        let mut rng = rand::thread_rng();
        let take = pool.len().min(count.max(1));
        let sampled: Vec<Question> = pool.choose_multiple(&mut rng, take).cloned().collect();
        Self::new(sampled, language)
    }

    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }

    pub fn get_current_question(&self) -> Option<&Question> {
        self.questions.get(self.current_question_index)
    }

    pub fn get_question_text(&self, question: &Question) -> String {
        DataLoader::get_question_text(question, &self.language)
    }

    pub fn get_choice_texts(&self, question: &Question) -> Vec<String> {
        question
            .choices
            .iter()
            .map(|choice| DataLoader::get_choice_text(choice, &self.language))
            .collect()
    }

    /// Typed strings that count as the **correct** answer for the active
    /// question. Per Issue #70 the player can only advance by typing the
    /// correct choice — wrong choices' typings are not accepted, so the
    /// candidate list is intentionally narrow.
    ///
    /// Returned strings are the **pre-canonical** (lowercased) candidates
    /// — the spelling as it appears in `ja_typings`, not the canonicalised
    /// form. Callers comparing against player input in JA mode should
    /// normalise both sides via `canonical_romaji` (see #96); the raw form
    /// is still useful for display and length-based metrics.
    pub fn current_correct_typing_candidates(&self) -> Vec<String> {
        let Some(question) = self.get_current_question() else {
            return Vec::new();
        };
        let Some(choice) = question.choices.get(question.correct_answer_index) else {
            return Vec::new();
        };
        let mut candidates: Vec<String> =
            DataLoader::get_choice_typing_texts(choice, &self.language)
                .into_iter()
                .map(|candidate| candidate.to_lowercase())
                .collect();
        candidates.sort();
        candidates.dedup();
        candidates
    }

    /// Whether `typed` is still a valid prefix of the **correct** answer
    /// for the active question. Empty input is always valid. Anything that
    /// diverges from the correct prefix is rejected like a mistype, which
    /// is the contract Issue #70 asks for.
    ///
    /// In JA mode comparison is done on the *canonical* romaji form (#96),
    /// so the player can type either Hepburn (`shi`/`chi`/`tsu`/`wo`) or
    /// kunrei (`si`/`ti`/`tu`/`o`) regardless of which variant was
    /// registered in `ja_typings`. In other languages the rewrites would
    /// be active harm (e.g. `wolf` would match the `olf` prefix because
    /// `wo`→`o`), so non-JA modes fall back to plain lowercase comparison.
    pub fn is_valid_correct_typed_prefix(&self, typed: &str) -> bool {
        if typed.is_empty() {
            return true;
        }
        let typed_lower = typed.to_lowercase();
        let typed_key = self.canonical_key(typed);
        // 二段判定: canonical (例 `shi`/`si` 等価) と、生 lowercase の両側で
        // prefix を見る。後者は「multi-char rewrite の途中」を救う:
        // 例: データ `ratenmoji` を打鍵中、`ratenmoj` (ji 未完成) は
        // canonical 比較だと `ratenmozi` と整合せず弾かれるが、生比較なら
        // `ratenmoji`.starts_with(`ratenmoj`) で通る。
        self.current_correct_typing_candidates()
            .iter()
            .any(|candidate| {
                let cand_lower = candidate.to_lowercase();
                cand_lower.starts_with(&typed_lower)
                    || self.canonical_key(candidate).starts_with(&typed_key)
            })
    }

    /// Resolve the typed text against the current question's choices and
    /// answer with the matching index. Per `docs/spec.md`, only an **exact**
    /// match counts — prefix matches do nothing (so `mov` does not auto-pick
    /// `move`). A non-matching string yields an incorrect answer.
    ///
    /// Equality is checked on the canonical romaji form (#96): a player
    /// typing `tsuru` against a `turu` candidate (or vice versa) is treated
    /// as an exact match. CPM accounting uses the *player's* typed length,
    /// which reflects actual keystrokes regardless of which variant the
    /// data file happened to register.
    pub fn answer_question_typed(&mut self, typed: &str) -> Option<QuizResult> {
        let typed_key = self.canonical_key(typed);
        // ja_typings / choice labels are ASCII in practice, so char count
        // and byte count coincide; we use char count for safety.
        let typed_chars = typed.chars().count() as u32;
        let matched = self.get_current_question().and_then(|question| {
            question
                .choices
                .iter()
                .enumerate()
                .find_map(|(idx, choice)| {
                    DataLoader::get_choice_typing_texts(choice, &self.language)
                        .into_iter()
                        .find(|candidate| self.canonical_key(candidate) == typed_key)
                        .map(|_| (idx, typed_chars))
                })
        });
        // usize::MAX guarantees a non-match against any valid index.
        let (index, typed_chars) = matched.unwrap_or((usize::MAX, 0));
        self.answer_question(index, typed_chars)
    }

    /// Lower-case the input and, in JA mode only, apply the canonical-romaji
    /// rewrites from #96 so Hepburn/kunrei variants compare equal. Non-JA
    /// modes get plain lowercase to avoid harmful collapses like `wolf`→`olf`.
    fn canonical_key(&self, s: &str) -> String {
        let lower = s.to_lowercase();
        if matches!(self.language, Language::Japanese) {
            canonical_romaji(&lower)
        } else {
            lower
        }
    }

    /// Index-based answer recorder. Prefer `answer_question_typed` from the
    /// UI layer — `usize::MAX` is the documented "no-match" sentinel.
    pub(crate) fn answer_question(
        &mut self,
        answer_index: usize,
        typed_chars: u32,
    ) -> Option<QuizResult> {
        let question_start_time = Instant::now();

        let snapshot = self.get_current_question().map(|question| {
            let correct_answer_index = question.correct_answer_index;
            let is_correct = answer_index == correct_answer_index;
            (correct_answer_index, is_correct)
        });

        if let Some((correct_answer_index, is_correct)) = snapshot {
            if is_correct {
                self.correct_answers += 1;
                self.score += self.calculate_score_for_question();
                self.typed_correct_chars = self.typed_correct_chars.saturating_add(typed_chars);
            }

            self.total_answers += 1;

            let result = QuizResult {
                is_correct,
                correct_answer_index,
                selected_answer_index: answer_index,
                time_taken: question_start_time.elapsed(),
            };

            self.current_question_index += 1;

            self.maybe_freeze_time();

            Some(result)
        } else {
            None
        }
    }

    pub fn is_game_finished(&self) -> bool {
        self.current_question_index >= self.questions.len()
    }

    pub fn get_final_score(&self) -> u32 {
        self.score
    }

    pub fn get_accuracy(&self) -> f32 {
        if self.total_answers == 0 {
            0.0
        } else {
            self.correct_answers as f32 / self.total_answers as f32
        }
    }

    pub fn get_correct_count(&self) -> u32 {
        self.correct_answers
    }

    pub fn get_total_time(&self) -> Option<Duration> {
        if let Some(d) = self.frozen_time {
            return Some(d);
        }
        self.start_time.map(|start| start.elapsed())
    }

    /// Snapshot the current elapsed time once the run is finished, so the
    /// summary screen reports the time of the final correct keystroke
    /// rather than a timer that keeps running while the result is on
    /// screen. Idempotent — only fires the first time it's called after
    /// `is_game_finished()` returns true.
    fn maybe_freeze_time(&mut self) {
        if self.frozen_time.is_some() {
            return;
        }
        if !self.is_game_finished() {
            return;
        }
        if let Some(start) = self.start_time {
            self.frozen_time = Some(start.elapsed());
        }
    }

    /// Characters per minute over the full run so far. Returns 0 when the
    /// timer hasn't started or when no time has elapsed.
    pub fn get_cpm(&self) -> u32 {
        let secs = self
            .get_total_time()
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        if secs <= 0.0 {
            0
        } else {
            ((self.typed_correct_chars as f64) * 60.0 / secs).round() as u32
        }
    }

    /// Words per minute, where 1 word = 5 characters (the standard
    /// typing-test convention). Same denominator as `get_cpm`.
    pub fn get_wpm(&self) -> u32 {
        self.get_cpm() / 5
    }

    pub fn get_progress(&self) -> (usize, usize) {
        (self.current_question_index, self.questions.len())
    }

    fn calculate_score_for_question(&self) -> u32 {
        100
    }

    pub fn skip_question(&mut self) -> bool {
        if self.current_question_index < self.questions.len() {
            self.current_question_index += 1;
            self.total_answers += 1;
            self.maybe_freeze_time();
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Choice;
    use std::collections::HashMap;

    fn make_question(choices: &[&str], correct: usize) -> Question {
        let mut question_text = HashMap::new();
        question_text.insert("ja".to_string(), "ダミー".to_string());
        question_text.insert("en".to_string(), "dummy".to_string());

        let choices = choices
            .iter()
            .map(|text| {
                let mut labels = HashMap::new();
                labels.insert("ja".to_string(), text.to_string());
                labels.insert("en".to_string(), text.to_string());
                Choice {
                    labels,
                    ja_typings: Vec::new(),
                }
            })
            .collect();

        Question {
            id: "q-test".into(),
            genre: "test".into(),
            question_text,
            question_text_reading: HashMap::new(),
            choices,
            correct_answer_index: correct,
            image_path: None,
            ja_reviewed: false,
        }
    }

    #[test]
    fn typed_exact_match_is_correct() {
        let question = make_question(&["borrow", "move", "ref", "clone"], 1);
        let mut game = QuizGame::new(vec![question], Language::English);
        game.start();
        let result = game.answer_question_typed("move").expect("result");
        assert!(result.is_correct);
        assert_eq!(result.selected_answer_index, 1);
    }

    #[test]
    fn typed_prefix_does_not_auto_confirm() {
        let question = make_question(&["borrow", "move", "ref", "clone"], 1);
        let mut game = QuizGame::new(vec![question], Language::English);
        game.start();
        let result = game.answer_question_typed("mov").expect("result");
        assert!(!result.is_correct);
        // usize::MAX sentinel: never matches a real choice index.
        assert_eq!(result.selected_answer_index, usize::MAX);
    }

    #[test]
    fn typed_wrong_choice_is_incorrect_but_recorded() {
        let question = make_question(&["borrow", "move", "ref", "clone"], 1);
        let mut game = QuizGame::new(vec![question], Language::English);
        game.start();
        let result = game.answer_question_typed("clone").expect("result");
        assert!(!result.is_correct);
        assert_eq!(result.selected_answer_index, 3);
    }

    #[test]
    fn typed_empty_string_is_incorrect() {
        let question = make_question(&["borrow", "move", "ref", "clone"], 1);
        let mut game = QuizGame::new(vec![question], Language::English);
        game.start();
        let result = game.answer_question_typed("").expect("result");
        assert!(!result.is_correct);
        assert_eq!(result.selected_answer_index, usize::MAX);
    }

    #[test]
    fn typed_is_case_insensitive() {
        let question = make_question(&["borrow", "move", "ref", "clone"], 1);
        let mut game = QuizGame::new(vec![question], Language::English);
        game.start();
        let result = game.answer_question_typed("Move").expect("result");
        assert!(result.is_correct);
    }

    #[test]
    fn typed_trailing_whitespace_is_not_trimmed() {
        // Documents the current contract: whitespace is significant.
        let question = make_question(&["borrow", "move", "ref", "clone"], 1);
        let mut game = QuizGame::new(vec![question], Language::English);
        game.start();
        let result = game.answer_question_typed("move ").expect("result");
        assert!(!result.is_correct);
    }

    #[test]
    fn typed_matches_phrase_choice() {
        let question = make_question(
            &[
                "George Washington",
                "Abraham Lincoln",
                "Thomas Jefferson",
                "John Adams",
            ],
            0,
        );
        let mut game = QuizGame::new(vec![question], Language::English);
        game.start();
        let result = game
            .answer_question_typed("George Washington")
            .expect("result");
        assert!(result.is_correct);
        assert_eq!(result.selected_answer_index, 0);
    }

    #[test]
    fn japanese_mode_accepts_explicit_romaji() {
        let mut question = make_question(&["東京", "大阪", "京都", "名古屋"], 0);
        question.choices[0].ja_typings = vec!["tokyo".into(), "toukyou".into()];
        question.choices[1].ja_typings = vec!["osaka".into(), "oosaka".into()];
        question.choices[2].ja_typings = vec!["kyoto".into(), "kyouto".into()];
        question.choices[3].ja_typings = vec!["nagoya".into()];
        let mut game = QuizGame::new(vec![question], Language::Japanese);
        game.start();
        let result = game.answer_question_typed("tokyo").expect("result");
        assert!(result.is_correct);
    }

    #[test]
    fn japanese_mode_accepts_uppercase_romaji() {
        let mut question = make_question(&["東京", "大阪", "京都", "名古屋"], 0);
        question.choices[0].ja_typings = vec!["tokyo".into(), "toukyou".into()];
        let mut game = QuizGame::new(vec![question], Language::Japanese);
        game.start();
        let result = game.answer_question_typed("TOKYO").expect("result");
        assert!(result.is_correct);
    }

    #[test]
    fn japanese_tokyo_tou_prefix_is_valid() {
        // Regression for the user-reported bug: typing "tou" toward
        // Tokyo (とうきょう) was rejected as a mistype even though
        // "toukyou" is a legitimate alias.
        let mut question = make_question(&["とうきょう", "おおさか", "きょうと", "なごや"], 0);
        question.choices[0].ja_typings = vec!["tokyo".into(), "toukyou".into()];
        let game = QuizGame::new(vec![question], Language::Japanese);
        let cands = game.current_correct_typing_candidates();
        eprintln!("candidates: {cands:?}");
        assert!(game.is_valid_correct_typed_prefix("t"), "t");
        assert!(game.is_valid_correct_typed_prefix("to"), "to");
        assert!(game.is_valid_correct_typed_prefix("tou"), "tou");
        assert!(game.is_valid_correct_typed_prefix("touk"), "touk");
    }

    #[test]
    fn japanese_tokyo_kanji_label_tou_prefix_is_valid() {
        // Same bug, but with kanji labels (legacy data path).
        let mut question = make_question(&["東京", "大阪", "京都", "名古屋"], 0);
        question.choices[0].ja_typings = vec!["tokyo".into(), "toukyou".into()];
        let game = QuizGame::new(vec![question], Language::Japanese);
        let cands = game.current_correct_typing_candidates();
        eprintln!("kanji candidates: {cands:?}");
        assert!(
            game.is_valid_correct_typed_prefix("tou"),
            "'tou' should be a valid prefix of toukyou"
        );
    }

    #[test]
    fn japanese_mode_accepts_long_vowel_alias() {
        let mut question = make_question(&["とうきょう", "おおさか", "きょうと", "なごや"], 0);
        question.choices[0].ja_typings = vec!["tokyo".into(), "toukyou".into()];
        let mut game = QuizGame::new(vec![question], Language::Japanese);
        game.start();
        let result = game.answer_question_typed("toukyou").expect("result");
        assert!(result.is_correct);
    }

    #[test]
    fn valid_prefix_accepts_partial_match_of_correct_choice() {
        let question = make_question(&["borrow", "move", "ref", "clone"], 1);
        let game = QuizGame::new(vec![question], Language::English);
        assert!(game.is_valid_correct_typed_prefix("mo"));
    }

    #[test]
    fn valid_prefix_rejects_wrong_branch() {
        let question = make_question(&["borrow", "move", "ref", "clone"], 1);
        let game = QuizGame::new(vec![question], Language::English);
        assert!(!game.is_valid_correct_typed_prefix("mx"));
    }

    #[test]
    fn valid_prefix_rejects_wrong_choice_text() {
        // Per Issue #70 only the correct choice's typings are valid; typing
        // a wrong choice's full text must be rejected as a mistype.
        let question = make_question(&["borrow", "move", "ref", "clone"], 1);
        let game = QuizGame::new(vec![question], Language::English);
        assert!(!game.is_valid_correct_typed_prefix("b"));
        assert!(!game.is_valid_correct_typed_prefix("borrow"));
    }

    #[test]
    fn valid_prefix_handles_mid_rewrite_partial_input() {
        // Multi-char rewrite (ji → zi) の途中で打鍵されると、canonical 比較
        // だけだと `ratenmoj` が `ratenmozi` の prefix にならず弾かれてしまう。
        // 生 lowercase との二段判定で救う回帰テスト。
        let mut question = make_question(&["ラテン文字", "漢字", "クメール文字", "タイ文字"], 0);
        question.choices[0].ja_typings = vec!["ratenmoji".into()];
        let game = QuizGame::new(vec![question], Language::Japanese);
        assert!(game.is_valid_correct_typed_prefix("ratenmo"));
        assert!(game.is_valid_correct_typed_prefix("ratenmoj"));
        assert!(game.is_valid_correct_typed_prefix("ratenmoji"));
        // kunrei (zi) 経路も通る。
        assert!(game.is_valid_correct_typed_prefix("ratenmoz"));
        assert!(game.is_valid_correct_typed_prefix("ratenmozi"));
    }

    #[test]
    fn valid_prefix_handles_all_multi_char_rewrite_mid_states() {
        // ji→zi 以外の multi-char rewrite ルールも同じ「途中状態 mistype」
        // クラスのバグを抱えていた。candidate を LEFT(Hepburn) 形で登録した
        // ときに、各 rewrite の境界で 1 文字目を打った瞬間 (canonical では
        // 整合しない) でも prefix が通ることを担保する。
        let cases: &[(&str, &[&str])] = &[
            // shi → si:  `s` 打鍵時 canonical(`sus`).starts_with(`sus`) は通る
            // が、`sush` まで打つと canonical(`sush`)=`sush` vs canonical(`sushi`)=`susi`
            // で破綻する。raw fallback が救う。
            ("sushi", &["s", "su", "sus", "sush", "sushi"]),
            // chi → ti
            ("kachi", &["k", "ka", "kac", "kach", "kachi"]),
            // tsu → tu
            ("katsu", &["k", "ka", "kat", "kats", "katsu"]),
            // fu → hu : `f` 単体で破綻していた
            ("fuji", &["f", "fu", "fuj", "fuji"]),
            // texi → thi
            ("texi", &["t", "te", "tex", "texi"]),
            // dexi → di
            ("dexi", &["d", "de", "dex", "dexi"]),
            // dhi → di
            ("dhi", &["d", "dh", "dhi"]),
            // dzu → du
            ("dzu", &["d", "dz", "dzu"]),
        ];
        for (registered, prefixes) in cases {
            let mut question =
                make_question(&["ラテン文字", "漢字", "クメール文字", "タイ文字"], 0);
            question.choices[0].ja_typings = vec![(*registered).into()];
            let game = QuizGame::new(vec![question], Language::Japanese);
            for p in *prefixes {
                assert!(
                    game.is_valid_correct_typed_prefix(p),
                    "candidate={registered:?} should accept partial prefix {p:?}"
                );
            }
        }
    }

    #[test]
    fn user_reported_partial_typing_cases() {
        // ユーザー報告された具体的なつまずきが現行 validator で通ることを担保。
        let cases: &[(&str, &[&str])] = &[
            // 美内すずえ Hepburn 部分入力 (217ba7c)
            (
                "miuchisuzue",
                &["m", "mi", "miu", "miuc", "miuch", "miuchi", "miuchisuzue"],
            ),
            // 千利休 IME-wapuro 3連 n (v0.7.0 ん 厳密化)
            (
                "sennnorikyuu",
                &["s", "se", "sen", "senn", "sennn", "sennno", "sennnorikyuu"],
            ),
            // スクウェア・エニックス: IME 別経路 (ウェ系 `uxe`/`ule`) と、
            // 中黒 `・` の IME ショートカット `/` 打鍵 (canonical で剥がす)
            // を両方検証する。
            (
                "sukuweaenikkusu",
                &[
                    "sukuwe",
                    "sukuwea",
                    "sukuuxea",
                    "sukuulea",
                    "sukuwea/enikkusu",
                ],
            ),
            // ろけっと 明示的小っ
            ("roketto", &["rokeltsuto", "rokextuto"]),
            // ジェンナー (Wapuro 3連 n は v0.7.0 で正解形)
            (
                "jennna-",
                &["j", "je", "jen", "jenn", "jennn", "jennna", "jennna-"],
            ),
        ];
        for (registered, inputs) in cases {
            let mut question = make_question(&["A", "B", "C", "D"], 0);
            question.choices[0].ja_typings = vec![(*registered).into()];
            let game = QuizGame::new(vec![question], Language::Japanese);
            for input in *inputs {
                assert!(
                    game.is_valid_correct_typed_prefix(input),
                    "candidate={registered:?} should accept input {input:?}"
                );
            }
        }
    }

    /// 全 ja_typings に対して「先頭1文字→2文字→...→全文字」と打鍵していった
    /// ときに、各 prefix が validator を通ることを確認する coverage テスト。
    /// 既存データの IME 整合性 (ん 厳密化、digraph 別経路、促音 等) を一括で
    /// 担保する。release ビルドで ~40ms と軽いので常時実行する。
    #[test]
    fn data_typings_are_prefix_typeable() {
        let raw = include_str!("../../data/questions_ja.json");
        let questions: Vec<Question> = serde_json::from_str(raw).expect("parse questions_ja.json");
        let mut failures: Vec<String> = Vec::new();
        for question in questions {
            let correct_idx = question.correct_answer_index;
            let typings = question.choices[correct_idx].ja_typings.clone();
            for typing in typings {
                let game = QuizGame::new(vec![question.clone()], Language::Japanese);
                // 1文字ずつ伸ばして各 prefix を検証。
                let chars: Vec<char> = typing.chars().collect();
                for end in 1..=chars.len() {
                    let prefix: String = chars[..end].iter().collect();
                    if !game.is_valid_correct_typed_prefix(&prefix) {
                        failures.push(format!(
                            "{} (ja={:?}, typing={:?}) rejected at prefix {:?}",
                            question.id,
                            question.choices[correct_idx]
                                .labels
                                .get("ja")
                                .cloned()
                                .unwrap_or_default(),
                            typing,
                            prefix
                        ));
                        break;
                    }
                }
            }
        }
        assert!(
            failures.is_empty(),
            "data prefix-typeability failures ({}):\n  {}",
            failures.len(),
            failures.join("\n  ")
        );
    }

    #[test]
    fn valid_prefix_handles_japanese_romaji_variants() {
        let mut question = make_question(&["東京", "大阪", "京都", "名古屋"], 0);
        question.choices[0].ja_typings = vec!["tokyo".into(), "toukyou".into()];
        let game = QuizGame::new(vec![question], Language::Japanese);
        assert!(game.is_valid_correct_typed_prefix("tok"));
        assert!(game.is_valid_correct_typed_prefix("tou"));
        assert!(!game.is_valid_correct_typed_prefix("tax"));
    }

    #[test]
    fn valid_prefix_accepts_long_vowel_dash_against_strict_data() {
        // Issue #93: カタカナ ー は IME 入力で `-` キー必須。データも
        // `bo-ru` 形で登録するので、入力も `-` を含む形のみ受理する。
        let mut question = make_question(&["サーバー", "クライアント", "DB", "API"], 0);
        question.choices[0].ja_typings = vec!["sa-ba-".into()];
        let game = QuizGame::new(vec![question], Language::Japanese);
        assert!(game.is_valid_correct_typed_prefix("sa-"));
        assert!(game.is_valid_correct_typed_prefix("sa-b"));
        assert!(game.is_valid_correct_typed_prefix("sa-ba"));
        assert!(game.is_valid_correct_typed_prefix("sa-ba-"));
        // `-` 抜きの collapsed 形は不正解扱い。
        assert!(!game.is_valid_correct_typed_prefix("sab"));
        assert!(!game.is_valid_correct_typed_prefix("saba"));
        // 子音直後の `-` も無効。
        assert!(!game.is_valid_correct_typed_prefix("s-"));
    }

    #[test]
    fn answer_question_typed_accepts_strict_long_vowel_dash() {
        // exact match 経路でも `-` を含む strict 形のみ受理。
        let mut question = make_question(&["サーバー", "クライアント", "DB", "API"], 0);
        question.choices[0].ja_typings = vec!["sa-ba-".into()];
        let mut game = QuizGame::new(vec![question], Language::Japanese);
        game.start();
        let result = game.answer_question_typed("sa-ba-").expect("result");
        assert!(result.is_correct);
        assert_eq!(result.selected_answer_index, 0);
    }

    #[test]
    fn answer_question_typed_rejects_dash_collapsed_form() {
        // `-` 抜きの collapsed 形は不正解 (selected_answer_index = usize::MAX)。
        let mut question = make_question(&["サーバー", "クライアント", "DB", "API"], 0);
        question.choices[0].ja_typings = vec!["sa-ba-".into()];
        let mut game = QuizGame::new(vec![question], Language::Japanese);
        game.start();
        let result = game.answer_question_typed("saba").expect("result");
        assert!(!result.is_correct);
        assert_eq!(result.selected_answer_index, usize::MAX);
    }

    #[test]
    fn final_time_is_frozen_at_last_correct_answer() {
        // Two questions; after answering both, get_total_time() must return
        // a stable value even after additional time elapses.
        let q1 = make_question(&["right", "a", "b", "c"], 0);
        let q2 = make_question(&["right", "a", "b", "c"], 0);
        let mut game = QuizGame::new(vec![q1, q2], Language::English);
        game.start();
        game.answer_question_typed("right");
        game.answer_question_typed("right");
        let frozen = game.get_total_time().unwrap();
        std::thread::sleep(Duration::from_millis(20));
        let after_sleep = game.get_total_time().unwrap();
        assert_eq!(frozen, after_sleep);
    }

    #[test]
    fn final_time_is_frozen_when_last_question_is_skipped() {
        let q = make_question(&["right", "a", "b", "c"], 0);
        let mut game = QuizGame::new(vec![q], Language::English);
        game.start();
        assert!(game.skip_question());
        let frozen = game.get_total_time().unwrap();
        std::thread::sleep(Duration::from_millis(20));
        let after_sleep = game.get_total_time().unwrap();
        assert_eq!(frozen, after_sleep);
    }

    #[test]
    fn from_pool_caps_at_run_length() {
        // 30 distinct questions in the pool; from_pool must hand back
        // exactly QUIZ_RUN_LENGTH = 10.
        let pool: Vec<Question> = (0..30)
            .map(|i| {
                let mut q = make_question(&["a", "b", "c", "d"], 0);
                q.id = format!("q{i:02}");
                q
            })
            .collect();
        let game = QuizGame::from_pool(&pool, Language::English);
        assert_eq!(game.get_progress(), (0, QUIZ_RUN_LENGTH));
    }

    #[test]
    fn from_pool_returns_whole_pool_when_smaller_than_run_length() {
        let pool: Vec<Question> = (0..3)
            .map(|i| {
                let mut q = make_question(&["a", "b", "c", "d"], 0);
                q.id = format!("q{i}");
                q
            })
            .collect();
        let game = QuizGame::from_pool(&pool, Language::English);
        assert_eq!(game.get_progress(), (0, 3));
    }

    #[test]
    fn from_pool_does_not_repeat_questions() {
        // 10 distinct questions, take 10 — every id must be present once.
        let pool: Vec<Question> = (0..QUIZ_RUN_LENGTH)
            .map(|i| {
                let mut q = make_question(&["a", "b", "c", "d"], 0);
                q.id = format!("q{i}");
                q
            })
            .collect();
        let game = QuizGame::from_pool(&pool, Language::English);
        let ids: std::collections::BTreeSet<&str> =
            game.questions.iter().map(|q| q.id.as_str()).collect();
        assert_eq!(ids.len(), QUIZ_RUN_LENGTH);
    }

    #[test]
    fn cpm_and_wpm_are_zero_before_any_correct_answer() {
        let q = make_question(&["a", "b", "c", "d"], 0);
        let mut game = QuizGame::new(vec![q], Language::English);
        game.start();
        assert_eq!(game.get_cpm(), 0);
        assert_eq!(game.get_wpm(), 0);
    }

    #[test]
    fn cpm_counts_only_correct_answers() {
        // Two questions, answer the first wrong and the second right —
        // typed_correct_chars must equal len("right") == 5.
        let q1 = make_question(&["right", "wrong1", "wrong2", "wrong3"], 0);
        let q2 = make_question(&["right", "wrong1", "wrong2", "wrong3"], 0);
        let mut game = QuizGame::new(vec![q1, q2], Language::English);
        game.start();
        game.answer_question_typed("wrong1");
        game.answer_question_typed("right");
        assert_eq!(game.typed_correct_chars, 5);
    }

    #[test]
    fn cpm_counts_typed_variant_length_for_japanese() {
        let mut question = make_question(&["東京", "大阪", "京都", "名古屋"], 0);
        question.choices[0].ja_typings = vec!["tokyo".into(), "toukyou".into()];
        let mut game = QuizGame::new(vec![question], Language::Japanese);
        game.start();
        game.answer_question_typed("toukyou");
        assert_eq!(game.typed_correct_chars, 7);
    }

    #[test]
    fn issue_96_accepts_hepburn_when_data_has_kunrei() {
        // ja_typings に kunrei が登録されていても shi/chi/tsu の Hepburn が通る
        let mut question = make_question(&["はし", "つる", "ちば", "おおさか"], 0);
        question.choices[0].ja_typings = vec!["hasi".into()];
        let game = QuizGame::new(vec![question], Language::Japanese);
        assert!(game.is_valid_correct_typed_prefix("ha"));
        assert!(game.is_valid_correct_typed_prefix("has"));
        assert!(game.is_valid_correct_typed_prefix("hashi"));
        assert!(game.is_valid_correct_typed_prefix("hasi"));
    }

    #[test]
    fn issue_96_accepts_kunrei_when_data_has_hepburn() {
        let mut question = make_question(&["ちば", "はし", "つる", "おおさか"], 0);
        question.choices[0].ja_typings = vec!["chiba".into()];
        let game = QuizGame::new(vec![question], Language::Japanese);
        assert!(game.is_valid_correct_typed_prefix("chiba"));
        assert!(game.is_valid_correct_typed_prefix("tiba"));
    }

    #[test]
    fn wo_and_o_are_distinct_in_japanese_mode() {
        // を と お は IME 入力 (wo vs o) で別キーストローク。strict 仕様では
        // データが `kawawo` (を 入り) なら入力も `wo` 必須、`o` だけでは不正解。
        let mut question = make_question(&["かわを", "はし", "つる", "ちば"], 0);
        question.choices[0].ja_typings = vec!["kawawo".into()];
        let game = QuizGame::new(vec![question], Language::Japanese);
        assert!(game.is_valid_correct_typed_prefix("kawawo"));
        assert!(!game.is_valid_correct_typed_prefix("kawao"));
    }

    #[test]
    fn issue_96_accepts_thi_variants() {
        let mut question = make_question(&["パティ", "はし", "つる", "ちば"], 0);
        question.choices[0].ja_typings = vec!["pathi".into()];
        let game = QuizGame::new(vec![question], Language::Japanese);
        assert!(game.is_valid_correct_typed_prefix("pathi"));
        assert!(game.is_valid_correct_typed_prefix("patexi"));
        assert!(game.is_valid_correct_typed_prefix("pateli"));
    }

    #[test]
    fn issue_96_answer_typed_accepts_variants() {
        // exact match レベルでも変換が効いていることを確認
        let mut question = make_question(&["つる", "はし", "ちば", "おおさか"], 0);
        question.choices[0].ja_typings = vec!["tsuru".into()];
        let mut game = QuizGame::new(vec![question], Language::Japanese);
        game.start();
        let result = game.answer_question_typed("turu").expect("result");
        assert!(result.is_correct);
    }

    #[test]
    fn wolf_is_not_accepted_by_olf_in_en_mode() {
        // M1 regression: canonical_romaji rewrites `wo`→`o`, which would
        // make `olf` accidentally match `wolf` if applied in EN mode.
        // EN mode must use plain lowercase comparison.
        let question = make_question(&["wolf", "fox", "bear", "deer"], 0);
        let mut game = QuizGame::new(vec![question], Language::English);
        game.start();
        assert!(!game.is_valid_correct_typed_prefix("olf"));
        let result = game.answer_question_typed("olf").expect("result");
        assert!(!result.is_correct);
        assert_eq!(result.selected_answer_index, usize::MAX);
    }

    #[test]
    fn en_mode_prefix_still_works_normally() {
        let question = make_question(&["wolf", "fox", "bear", "deer"], 0);
        let game = QuizGame::new(vec![question], Language::English);
        assert!(game.is_valid_correct_typed_prefix("w"));
        assert!(game.is_valid_correct_typed_prefix("wo"));
        assert!(game.is_valid_correct_typed_prefix("wolf"));
    }

    // ----- from_pool_with_count (#106 auto-demo --demo-count) -----

    #[test]
    fn from_pool_with_count_zero_falls_back_to_one() {
        // `--demo-count 0` is nonsense but must not produce an empty run
        // (the demo loop would never call `start()` and we'd freeze on a
        // 0-of-0 progress bar). `count.max(1)` is the documented floor.
        let pool: Vec<Question> = (0..5)
            .map(|i| {
                let mut q = make_question(&["a", "b", "c", "d"], 0);
                q.id = format!("q{i}");
                q
            })
            .collect();
        let game = QuizGame::from_pool_with_count(&pool, Language::English, 0);
        assert_eq!(game.get_progress(), (0, 1));
    }

    #[test]
    fn from_pool_with_count_one_takes_single_question() {
        let pool: Vec<Question> = (0..5)
            .map(|i| {
                let mut q = make_question(&["a", "b", "c", "d"], 0);
                q.id = format!("q{i}");
                q
            })
            .collect();
        let game = QuizGame::from_pool_with_count(&pool, Language::English, 1);
        assert_eq!(game.get_progress(), (0, 1));
    }

    #[test]
    fn from_pool_with_count_exact_match() {
        let pool: Vec<Question> = (0..5)
            .map(|i| {
                let mut q = make_question(&["a", "b", "c", "d"], 0);
                q.id = format!("q{i}");
                q
            })
            .collect();
        let game = QuizGame::from_pool_with_count(&pool, Language::English, 5);
        assert_eq!(game.get_progress(), (0, 5));
    }

    #[test]
    fn from_pool_with_count_exceeds_pool_is_clamped() {
        // Requesting more than the pool offers must clamp to pool size
        // rather than padding / repeating (mirrors `from_pool`'s contract).
        let pool: Vec<Question> = (0..3)
            .map(|i| {
                let mut q = make_question(&["a", "b", "c", "d"], 0);
                q.id = format!("q{i}");
                q
            })
            .collect();
        let game = QuizGame::from_pool_with_count(&pool, Language::English, 1000);
        assert_eq!(game.get_progress(), (0, 3));
    }

    #[test]
    fn correct_count_tracks_only_correct_answers() {
        let q = make_question(&["right", "a", "b", "c"], 0);
        let mut game = QuizGame::new(vec![q], Language::English);
        game.start();
        game.answer_question_typed("right");
        assert_eq!(game.get_correct_count(), 1);
        assert!((game.get_accuracy() - 1.0).abs() < f32::EPSILON);
    }
}

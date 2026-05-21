use crate::types::{AnswerKind, BossTier, ListeningPrompt, RpgStats};
use rand::seq::SliceRandom;

pub const RPG_RUN_LENGTH: usize = 10;
const REGULAR_ENCOUNTER_COUNT: usize = 8;

// --- #34: EXP / level-up pure functions --------------------------------------
//
// Phase 2 keeps the math in `game::rpg` (data layer) so unit tests can
// exercise it without booting any UI. `run_listening_rpg` in `main.rs`
// is the only caller; it threads `RpgStats` through these functions and
// surfaces emitted events to the battle log + title-unlock pipeline.

/// Base EXP per correct hit, before the speed bonus.
pub const BASE_EXP_PER_HIT: u32 = 10;
/// Maximum bonus EXP awarded for an instant (≤ 0s) correct answer.
pub const MAX_SPEED_BONUS: u32 = 5;
/// Time-to-correct beyond which the speed bonus is 0.
pub const SPEED_BONUS_WINDOW_SECS: f64 = 5.0;
/// EXP penalty per missed encounter. Per CLAUDE.md the v0.2.0 RPG has no
/// failure state; missed answers only chip away at progression.
///
/// CLAUDE.md「失敗概念なし」を踏まえ、miss penalty は base hit gain の
/// 1/10 (10 EXP gain vs 1 EXP loss)。プレイテスト後に再調整可能な
/// バランス値。
pub const MISS_EXP_PENALTY: u32 = 1;

/// Required EXP to advance *from* `level` to `level + 1`. Issue #34 picks
/// the linear `level * 100` schedule (Lv 1→2: 100, Lv 2→3: 200, …). The
/// `max(100)` clamp protects the (presently unreachable) `level == 0`
/// case from collapsing to a zero threshold.
pub fn next_level_exp(level: u32) -> u32 {
    level.saturating_mul(100).max(100)
}

/// EXP awarded for a correct answer that took `elapsed_secs` seconds.
/// Bonus is a linear ramp from `MAX_SPEED_BONUS` (≤ 0s) down to 0 at
/// `SPEED_BONUS_WINDOW_SECS`, rounded to the nearest integer. NaN /
/// negative inputs are treated as "instant" — never as a penalty.
pub fn exp_gain_for_hit(elapsed_secs: f64) -> u32 {
    let bonus = if !elapsed_secs.is_finite() || elapsed_secs <= 0.0 {
        MAX_SPEED_BONUS
    } else if elapsed_secs >= SPEED_BONUS_WINDOW_SECS {
        0
    } else {
        let ratio = 1.0 - (elapsed_secs / SPEED_BONUS_WINDOW_SECS);
        (ratio * MAX_SPEED_BONUS as f64).round() as u32
    };
    BASE_EXP_PER_HIT + bonus
}

/// A single level-up event emitted by `apply_exp_gain`. UI / battle-log
/// callers turn these into `🎉 Level up!` lines and feed `new_level` to
/// the title-unlock pipeline (#35).
///
/// `old_level` is the level immediately before this event fired, so a
/// caller emitting `Lv {old} → {new}` lines does not have to reconstruct
/// it from `new_level - 1` (which is fragile if the schedule ever skips
/// levels — Phase 3 may grant +N levels in a single beat).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelUpEvent {
    pub old_level: u32,
    pub new_level: u32,
}

/// Add `gain` EXP to `stats` and roll the level forward as many times as
/// the running total allows. Returns a `LevelUpEvent` per level crossed,
/// in ascending order. Pure: only mutates `stats`. Saturating on u32
/// overflow so cosmically large EXP totals can't panic the run loop.
pub fn apply_exp_gain(stats: &mut RpgStats, gain: u32) -> Vec<LevelUpEvent> {
    stats.exp = stats.exp.saturating_add(gain);
    let mut events = Vec::new();
    // Hard cap on `events.len()` is unnecessary in practice (one beat
    // never grants enough EXP to cross multiple levels at sane Lv values)
    // but the `saturating_add` on `stats.level` keeps the loop sound even
    // if EXP grows past u32::MAX semantics.
    loop {
        let threshold = next_level_exp(stats.level);
        if stats.exp < threshold {
            break;
        }
        stats.exp -= threshold;
        let old_level = stats.level;
        stats.level = stats.level.saturating_add(1);
        events.push(LevelUpEvent {
            old_level,
            new_level: stats.level,
        });
    }
    events
}

/// Subtract `loss` EXP from `stats`, saturating at 0 (no negative EXP /
/// level loss per CLAUDE.md "失敗概念なし").
pub fn apply_exp_loss(stats: &mut RpgStats, loss: u32) {
    stats.exp = stats.exp.saturating_sub(loss);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RpgEncounterKind {
    Regular,
    Miniboss,
    Boss,
}

#[derive(Debug, Clone)]
pub struct RpgEncounter {
    pub ordinal: usize,
    pub kind: RpgEncounterKind,
    pub prompt: ListeningPrompt,
}

/// Macro-phase of a single RPG run, used by `main::run_listening_rpg`
/// to drive the loop instead of a bare `for encounter in run.encounters()`.
///
/// Phase 1 (#33) deliberately keeps this simple:
///   Town → Diving → Encounter(1) → ... → Encounter(10) → Return → Town
///
/// `Combat / Defeat` are not modelled — the v0.2.0 RPG has *no failure
/// state* (CLAUDE.md: "失敗概念なし"). Players always finish all 10
/// encounters and return to town; missed answers only affect future EXP
/// accounting (Phase 2). The single `Encounter(N)` variant is enough to
/// represent both regular and boss beats; the encounter table itself
/// (regular / miniboss / boss) is held by `ListeningRpgRun::encounters`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RpgRunPhase {
    Town,
    Diving,
    /// 1..=`RPG_RUN_LENGTH`. Ordinal matches `RpgEncounter::ordinal`.
    Encounter(usize),
    Return,
}

/// Maximum number of battle-log lines kept on `ListeningRpgRun` in
/// Phase 1 (#36). The UI clips to the visible pane height; this bound
/// just prevents unbounded growth across a 10-encounter run.
pub const BATTLE_LOG_MAX: usize = 64;

#[derive(Debug, Clone)]
pub struct ListeningRpgRun {
    encounters: Vec<RpgEncounter>,
    phase: RpgRunPhase,
    /// Per-run rolling battle log surfaced by the listening UIs (#36).
    /// Phase 1 only carries Hit / Missed / Plays lines; richer events
    /// (damage numbers, EXP gain, level-ups) come in Phase 2.
    battle_log: Vec<String>,
}

impl ListeningRpgRun {
    pub fn build(prompts: &[ListeningPrompt]) -> Result<Self, String> {
        let mut rng = rand::thread_rng();

        let mut regulars: Vec<ListeningPrompt> = prompts
            .iter()
            .filter(|prompt| prompt.boss.is_none() && prompt.kind == AnswerKind::Word)
            .cloned()
            .collect();
        let mut minibosses: Vec<ListeningPrompt> = prompts
            .iter()
            .filter(|prompt| {
                prompt
                    .boss
                    .as_ref()
                    .map(|spec| spec.tier == BossTier::Miniboss)
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
        let mut bosses: Vec<ListeningPrompt> = prompts
            .iter()
            .filter(|prompt| {
                prompt
                    .boss
                    .as_ref()
                    .map(|spec| spec.tier == BossTier::Boss)
                    .unwrap_or(false)
            })
            .cloned()
            .collect();

        if regulars.len() < REGULAR_ENCOUNTER_COUNT {
            return Err(format!(
                "Listening RPG needs at least {REGULAR_ENCOUNTER_COUNT} regular prompts, found {}.",
                regulars.len()
            ));
        }
        if minibosses.is_empty() {
            return Err(
                "Listening RPG needs at least one miniboss prompt with `boss.tier: miniboss`."
                    .into(),
            );
        }
        if bosses.is_empty() {
            return Err(
                "Listening RPG needs at least one boss prompt with `boss.tier: boss`.".into(),
            );
        }

        regulars.shuffle(&mut rng);
        minibosses.shuffle(&mut rng);
        bosses.shuffle(&mut rng);

        let mut encounters = Vec::with_capacity(RPG_RUN_LENGTH);
        let mut regular_iter = regulars.into_iter();

        for ordinal in 1..=4 {
            encounters.push(RpgEncounter {
                ordinal,
                kind: RpgEncounterKind::Regular,
                prompt: regular_iter
                    .next()
                    .expect("validated regular prompt count before building run"),
            });
        }

        encounters.push(RpgEncounter {
            ordinal: 5,
            kind: RpgEncounterKind::Miniboss,
            prompt: minibosses
                .into_iter()
                .next()
                .expect("validated miniboss availability before building run"),
        });

        for ordinal in 6..=9 {
            encounters.push(RpgEncounter {
                ordinal,
                kind: RpgEncounterKind::Regular,
                prompt: regular_iter
                    .next()
                    .expect("validated regular prompt count before building run"),
            });
        }

        encounters.push(RpgEncounter {
            ordinal: 10,
            kind: RpgEncounterKind::Boss,
            prompt: bosses
                .into_iter()
                .next()
                .expect("validated boss availability before building run"),
        });

        Ok(Self {
            encounters,
            phase: RpgRunPhase::Town,
            battle_log: Vec::new(),
        })
    }

    #[cfg(test)]
    pub fn encounters(&self) -> &[RpgEncounter] {
        &self.encounters
    }

    pub fn phase(&self) -> RpgRunPhase {
        self.phase
    }

    /// Town → Diving. Called once when a run is starting to descend.
    pub fn enter_diving(&mut self) {
        self.phase = RpgRunPhase::Diving;
    }

    /// Advance from Diving (or Encounter(N-1)) to Encounter(N).
    /// Returns the encounter to play, or `None` once all 10 are done
    /// (in which case the phase becomes `Return`).
    ///
    /// Phase 1 keeps this dumb on purpose — the loop driver in
    /// `main.rs` calls `advance_to_next_encounter` between beats and
    /// trusts the returned `Option<&RpgEncounter>` for control flow.
    pub fn advance_to_next_encounter(&mut self) -> Option<&RpgEncounter> {
        let next_ordinal: usize = match self.phase {
            RpgRunPhase::Town | RpgRunPhase::Return => return None,
            RpgRunPhase::Diving => 1,
            RpgRunPhase::Encounter(n) => n + 1,
        };

        if next_ordinal > self.encounters.len() {
            self.phase = RpgRunPhase::Return;
            return None;
        }

        self.phase = RpgRunPhase::Encounter(next_ordinal);
        self.encounters.get(next_ordinal - 1)
    }

    /// Force the run back into `Town`. Called once the `Return` beat
    /// finishes (or by callers that need to abort cleanly).
    pub fn return_to_town(&mut self) {
        self.phase = RpgRunPhase::Town;
    }

    /// Append a line to the run's rolling battle log (#36). Older
    /// entries are dropped past `BATTLE_LOG_MAX` to keep memory bounded.
    pub fn push_battle_log<S: Into<String>>(&mut self, entry: S) {
        self.battle_log.push(entry.into());
        let len = self.battle_log.len();
        if len > BATTLE_LOG_MAX {
            self.battle_log.drain(0..(len - BATTLE_LOG_MAX));
        }
    }

    /// Read-only view over the battle log. UIs slice the tail of this
    /// to fill their log pane.
    pub fn battle_log(&self) -> &[String] {
        &self.battle_log
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        AnswerKind, BossHintRevealMode, BossHintStep, ListeningBossSpec, ListeningPrompt,
    };

    fn regular(id: &str) -> ListeningPrompt {
        ListeningPrompt {
            id: id.into(),
            text_reading: id.into(),
            text_display: id.into(),
            kind: AnswerKind::Word,
            boss: None,
        }
    }

    fn boss(id: &str, tier: BossTier) -> ListeningPrompt {
        ListeningPrompt {
            id: id.into(),
            text_reading: id.into(),
            text_display: id.into(),
            kind: AnswerKind::Word,
            boss: Some(ListeningBossSpec {
                tier,
                reveal_mode: match tier {
                    BossTier::Miniboss => BossHintRevealMode::Timed,
                    BossTier::Boss => BossHintRevealMode::Manual,
                },
                hints: vec![BossHintStep {
                    label: "hint".into(),
                    text_display: "hint".into(),
                    text_reading: None,
                    auto_reveal_after_ms: None,
                }],
            }),
        }
    }

    #[test]
    fn build_places_miniboss_and_boss_at_slots_five_and_ten() {
        let mut prompts = vec![
            boss("mini", BossTier::Miniboss),
            boss("boss", BossTier::Boss),
        ];
        for idx in 0..8 {
            prompts.push(regular(&format!("r{idx}")));
        }

        let run = ListeningRpgRun::build(&prompts).expect("run builds");
        assert_eq!(run.encounters().len(), RPG_RUN_LENGTH);
        assert_eq!(run.encounters()[4].kind, RpgEncounterKind::Miniboss);
        assert_eq!(run.encounters()[9].kind, RpgEncounterKind::Boss);
        assert!(run.encounters()[..4]
            .iter()
            .all(|encounter| encounter.kind == RpgEncounterKind::Regular));
        assert!(run.encounters()[5..9]
            .iter()
            .all(|encounter| encounter.kind == RpgEncounterKind::Regular));
    }

    #[test]
    fn build_rejects_missing_boss_prompt() {
        let prompts: Vec<ListeningPrompt> = (0..8).map(|idx| regular(&format!("r{idx}"))).collect();
        let err = ListeningRpgRun::build(&prompts).expect_err("boss prompt required");
        assert!(err.contains("miniboss") || err.contains("boss"));
    }

    fn full_pool() -> Vec<ListeningPrompt> {
        let mut prompts = vec![
            boss("mini", BossTier::Miniboss),
            boss("boss", BossTier::Boss),
        ];
        for idx in 0..8 {
            prompts.push(regular(&format!("r{idx}")));
        }
        prompts
    }

    #[test]
    fn new_run_starts_in_town_with_empty_log() {
        let run = ListeningRpgRun::build(&full_pool()).expect("build");
        assert_eq!(run.phase(), RpgRunPhase::Town);
        assert!(run.battle_log().is_empty());
    }

    #[test]
    fn state_machine_visits_all_ten_encounters_then_returns() {
        let mut run = ListeningRpgRun::build(&full_pool()).expect("build");
        // Town → can't advance directly.
        assert!(run.advance_to_next_encounter().is_none());

        run.enter_diving();
        assert_eq!(run.phase(), RpgRunPhase::Diving);

        for expected in 1..=RPG_RUN_LENGTH {
            let encounter = run.advance_to_next_encounter().expect("encounter present");
            assert_eq!(encounter.ordinal, expected);
            assert_eq!(run.phase(), RpgRunPhase::Encounter(expected));
        }

        // After the 10th encounter the next advance flips into Return.
        assert!(run.advance_to_next_encounter().is_none());
        assert_eq!(run.phase(), RpgRunPhase::Return);

        run.return_to_town();
        assert_eq!(run.phase(), RpgRunPhase::Town);
    }

    #[test]
    fn battle_log_records_and_caps_entries() {
        let mut run = ListeningRpgRun::build(&full_pool()).expect("build");
        for i in 0..(BATTLE_LOG_MAX + 5) {
            run.push_battle_log(format!("entry {i}"));
        }
        assert_eq!(run.battle_log().len(), BATTLE_LOG_MAX);
        // Oldest dropped, newest preserved.
        assert_eq!(
            run.battle_log().last().map(String::as_str),
            Some(format!("entry {}", BATTLE_LOG_MAX + 4).as_str())
        );
    }

    // --- #34: EXP / level math --------------------------------------------

    #[test]
    fn next_level_exp_is_monotonic_and_positive() {
        let mut prev = 0;
        for level in 1..=20u32 {
            let n = next_level_exp(level);
            assert!(n > 0, "Lv {level} threshold must be positive");
            assert!(n >= prev, "Lv {level} threshold must be ≥ Lv {}", level - 1);
            prev = n;
        }
        assert_eq!(next_level_exp(1), 100);
        assert_eq!(next_level_exp(2), 200);
        assert_eq!(next_level_exp(10), 1000);
    }

    #[test]
    fn next_level_exp_clamps_level_zero() {
        // Defensive: protects against accidental Lv 0 RpgStats from a
        // hand-edited save.
        assert_eq!(next_level_exp(0), 100);
    }

    #[test]
    fn exp_gain_for_hit_boundaries() {
        assert_eq!(
            exp_gain_for_hit(0.0),
            BASE_EXP_PER_HIT + MAX_SPEED_BONUS,
            "instant answer = full bonus"
        );
        assert_eq!(
            exp_gain_for_hit(SPEED_BONUS_WINDOW_SECS),
            BASE_EXP_PER_HIT,
            "at the window edge the bonus must be 0"
        );
        assert_eq!(
            exp_gain_for_hit(10.0),
            BASE_EXP_PER_HIT,
            "past the window the bonus must stay 0 (no negative bonus)"
        );
        // Mid-window: 2.5s = halfway → bonus ≈ MAX/2 (rounded)
        let mid = exp_gain_for_hit(2.5);
        assert!(
            (BASE_EXP_PER_HIT + 2..=BASE_EXP_PER_HIT + 3).contains(&mid),
            "mid-window gain ≈ {BASE_EXP_PER_HIT} + ~{}, got {mid}",
            MAX_SPEED_BONUS / 2
        );
    }

    #[test]
    fn exp_gain_for_hit_negative_and_nan_are_treated_as_instant() {
        assert_eq!(exp_gain_for_hit(-1.0), BASE_EXP_PER_HIT + MAX_SPEED_BONUS);
        assert_eq!(
            exp_gain_for_hit(f64::NAN),
            BASE_EXP_PER_HIT + MAX_SPEED_BONUS
        );
    }

    #[test]
    fn apply_exp_gain_accumulates_without_level_up() {
        let mut stats = RpgStats::default();
        let events = apply_exp_gain(&mut stats, 50);
        assert!(events.is_empty());
        assert_eq!(stats.level, 1);
        assert_eq!(stats.exp, 50);
    }

    #[test]
    fn apply_exp_gain_emits_one_level_up_event_with_carry() {
        let mut stats = RpgStats::default();
        // Lv 1 needs 100 EXP. Granting 150 should level up once with 50 carry.
        let events = apply_exp_gain(&mut stats, 150);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].old_level, 1);
        assert_eq!(events[0].new_level, 2);
        assert_eq!(stats.level, 2);
        assert_eq!(stats.exp, 50);
    }

    #[test]
    fn apply_exp_gain_emits_multiple_level_ups_in_order() {
        let mut stats = RpgStats::default();
        // Lv 1→2 needs 100, Lv 2→3 needs 200 → total 300 to reach Lv 3.
        let events = apply_exp_gain(&mut stats, 350);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].old_level, 1);
        assert_eq!(events[0].new_level, 2);
        assert_eq!(events[1].old_level, 2);
        assert_eq!(events[1].new_level, 3);
        assert_eq!(stats.level, 3);
        assert_eq!(stats.exp, 50);
    }

    #[test]
    fn apply_exp_gain_saturates_on_u32_overflow() {
        let mut stats = RpgStats {
            exp: u32::MAX - 10,
            ..RpgStats::default()
        };
        // Should not panic; level rolls forward and EXP stays bounded.
        let _ = apply_exp_gain(&mut stats, u32::MAX);
        assert!(stats.level > 1, "should have leveled up at least once");
    }

    #[test]
    fn apply_exp_loss_saturates_at_zero() {
        let mut stats = RpgStats {
            exp: 3,
            ..RpgStats::default()
        };
        apply_exp_loss(&mut stats, 10);
        assert_eq!(stats.exp, 0);
        assert_eq!(stats.level, 1, "loss never decreases level");
    }

    #[test]
    fn apply_exp_loss_subtracts_normally() {
        let mut stats = RpgStats {
            exp: 50,
            ..RpgStats::default()
        };
        apply_exp_loss(&mut stats, 1);
        assert_eq!(stats.exp, 49);
    }

    #[test]
    fn build_ignores_non_word_regular_prompts() {
        let mut prompts = vec![
            boss("mini", BossTier::Miniboss),
            boss("boss", BossTier::Boss),
        ];
        for idx in 0..8 {
            prompts.push(regular(&format!("r{idx}")));
        }
        prompts.push(ListeningPrompt {
            id: "phrase".into(),
            text_reading: "good morning".into(),
            text_display: "good morning".into(),
            kind: AnswerKind::Phrase,
            boss: None,
        });

        let run = ListeningRpgRun::build(&prompts).expect("run builds");
        assert!(run
            .encounters()
            .iter()
            .filter(|encounter| encounter.kind == RpgEncounterKind::Regular)
            .all(|encounter| encounter.prompt.kind == AnswerKind::Word));
    }
}

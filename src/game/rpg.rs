use crate::types::{AnswerKind, BossTier, ListeningPrompt};
use rand::seq::SliceRandom;

pub const RPG_RUN_LENGTH: usize = 10;
const REGULAR_ENCOUNTER_COUNT: usize = 8;

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
    Encounter(u32),
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

    #[allow(dead_code)]
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
        let next_ordinal = match self.phase {
            RpgRunPhase::Town | RpgRunPhase::Return => return None,
            RpgRunPhase::Diving => 1,
            RpgRunPhase::Encounter(n) => n + 1,
        };

        if (next_ordinal as usize) > self.encounters.len() {
            self.phase = RpgRunPhase::Return;
            return None;
        }

        self.phase = RpgRunPhase::Encounter(next_ordinal);
        self.encounters.get((next_ordinal - 1) as usize)
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

        for expected in 1..=RPG_RUN_LENGTH as u32 {
            let encounter = run.advance_to_next_encounter().expect("encounter present");
            assert_eq!(encounter.ordinal as u32, expected);
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

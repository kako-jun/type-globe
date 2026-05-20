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

#[derive(Debug, Clone)]
pub struct ListeningRpgRun {
    encounters: Vec<RpgEncounter>,
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
            return Err("Listening RPG needs at least one miniboss prompt with `boss.tier: miniboss`.".into());
        }
        if bosses.is_empty() {
            return Err("Listening RPG needs at least one boss prompt with `boss.tier: boss`.".into());
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

        Ok(Self { encounters })
    }

    pub fn encounters(&self) -> &[RpgEncounter] {
        &self.encounters
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
        let mut prompts = vec![boss("mini", BossTier::Miniboss), boss("boss", BossTier::Boss)];
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

    #[test]
    fn build_ignores_non_word_regular_prompts() {
        let mut prompts = vec![boss("mini", BossTier::Miniboss), boss("boss", BossTier::Boss)];
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

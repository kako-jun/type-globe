#![allow(dead_code)]

use crate::types::{BossHintRevealMode, BossTier, ListeningBossSpec};
use crate::ui::HelpEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossLayoutVariant {
    HintStack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BossUiPlan {
    pub title: &'static str,
    pub variant: BossLayoutVariant,
    pub reveal_mode: BossHintRevealMode,
    pub show_hint_history_in_main_pane: bool,
    pub show_events_in_log_pane: bool,
    pub next_hint_key: Option<&'static str>,
    pub previous_hint_key: Option<&'static str>,
    pub help_entries: Vec<HelpEntry>,
}

impl BossUiPlan {
    pub fn from_spec(spec: &ListeningBossSpec) -> Self {
        let title = match spec.tier {
            BossTier::Miniboss => "Miniboss Listening",
            BossTier::Boss => "Boss Listening",
        };
        let (next_hint_key, previous_hint_key, help_entries) = match spec.reveal_mode {
            BossHintRevealMode::Manual => (
                Some("Tab"),
                Some("S-Tab"),
                vec![
                    HelpEntry::new("Esc", "Quit"),
                    HelpEntry::new("Space", "Replay"),
                    HelpEntry::new("Tab", "Next hint"),
                    HelpEntry::new("S-Tab", "Prev hint"),
                    HelpEntry::new("Bksp", "Erase"),
                ],
            ),
            BossHintRevealMode::Timed => (
                None,
                Some("S-Tab"),
                vec![
                    HelpEntry::new("Esc", "Quit"),
                    HelpEntry::new("Space", "Replay"),
                    HelpEntry::new("S-Tab", "Prev hint"),
                    HelpEntry::new("Bksp", "Erase"),
                ],
            ),
        };

        Self {
            title,
            variant: BossLayoutVariant::HintStack,
            reveal_mode: spec.reveal_mode,
            show_hint_history_in_main_pane: true,
            show_events_in_log_pane: true,
            next_hint_key,
            previous_hint_key,
            help_entries,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BossHintStep;

    fn boss_spec(tier: BossTier, reveal_mode: BossHintRevealMode) -> ListeningBossSpec {
        ListeningBossSpec {
            tier,
            reveal_mode,
            hints: vec![BossHintStep {
                label: "Part of speech".into(),
                text_display: "noun".into(),
                text_reading: None,
                auto_reveal_after_ms: None,
            }],
        }
    }

    #[test]
    fn manual_boss_plan_exposes_next_and_previous_hint_keys() {
        let plan = BossUiPlan::from_spec(&boss_spec(BossTier::Boss, BossHintRevealMode::Manual));
        assert_eq!(plan.next_hint_key, Some("Tab"));
        assert_eq!(plan.previous_hint_key, Some("S-Tab"));
        assert!(plan.help_entries.iter().any(|entry| entry.key == "Tab"));
    }

    #[test]
    fn timed_miniboss_plan_has_no_next_hint_key() {
        let plan = BossUiPlan::from_spec(&boss_spec(BossTier::Miniboss, BossHintRevealMode::Timed));
        assert_eq!(plan.next_hint_key, None);
        assert_eq!(plan.previous_hint_key, Some("S-Tab"));
        assert!(plan.help_entries.iter().all(|entry| entry.key != "Tab"));
    }
}

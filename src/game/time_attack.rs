use crate::io::{
    normalize::{canonical_romaji, punctuation_skip_variant},
    DataLoader,
};
use crate::types::{Language, Question};
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

pub const TA25_RUN_LENGTH: usize = 25;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Ta25SeatColor {
    Red,
    Blue,
    Green,
    Yellow,
}

impl Ta25SeatColor {
    pub const ALL: [Self; 4] = [Self::Red, Self::Blue, Self::Green, Self::Yellow];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Blue => "blue",
            Self::Green => "green",
            Self::Yellow => "yellow",
        }
    }

    pub fn panel_style_name(self) -> &'static str {
        match self {
            Self::Red => "RED",
            Self::Blue => "BLUE",
            Self::Green => "GREEN",
            Self::Yellow => "YELLOW",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Ta25SeatKind {
    Human,
    Cpu,
    Empty,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Ta25Seat {
    pub color: Ta25SeatColor,
    pub kind: Ta25SeatKind,
    /// Stable logical participant ID. In local prototypes this can be a
    /// simple internal token; online play can later map it to a remote ID.
    pub player_id: String,
    pub display_name: String,
}

impl Ta25Seat {
    fn human(color: Ta25SeatColor, player_id: &str, display_name: &str) -> Self {
        Self {
            color,
            kind: Ta25SeatKind::Human,
            player_id: player_id.to_string(),
            display_name: display_name.to_string(),
        }
    }

    fn cpu(color: Ta25SeatColor, ordinal: usize) -> Self {
        Self {
            color,
            kind: Ta25SeatKind::Cpu,
            player_id: format!("cpu-{}", color.as_str()),
            display_name: format!("CPU {ordinal}"),
        }
    }

    fn empty(color: Ta25SeatColor) -> Self {
        Self {
            color,
            kind: Ta25SeatKind::Empty,
            // Sentinel only, not a real stable participant identity.
            player_id: format!("empty-{}", color.as_str()),
            display_name: "(empty)".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Ta25Roster {
    pub seats: [Ta25Seat; 4],
}

impl Ta25Roster {
    /// Explicit empty four-seat roster. Reserved for future seat-claim
    /// flows; the current `add_human()` API intentionally models only the
    /// local prototype path (red stays local starter, then blue/green/yellow
    /// are replaced in order).
    pub fn all_empty() -> Self {
        Self {
            seats: std::array::from_fn(|index| Ta25Seat::empty(Ta25SeatColor::ALL[index])),
        }
    }

    /// Canonical local prototype setup for TA25:
    /// one human seat plus three CPU seats.
    pub fn standard_local(human_name: &str) -> Self {
        let mut roster = Self::all_empty();
        roster.seats[0] = Ta25Seat::human(Ta25SeatColor::Red, "local-human", human_name);
        roster.seats[1] = Ta25Seat::cpu(Ta25SeatColor::Blue, 1);
        roster.seats[2] = Ta25Seat::cpu(Ta25SeatColor::Green, 2);
        roster.seats[3] = Ta25Seat::cpu(Ta25SeatColor::Yellow, 3);
        roster
    }

    /// Canonical join path for extra humans. The seat-allocation rule is:
    /// red stays the local starter, then blue -> green -> yellow are replaced
    /// in that order as humans join. Duplicate player IDs are rejected.
    #[allow(dead_code)]
    pub fn add_human(&mut self, player_id: &str, display_name: &str) -> bool {
        if self
            .seats
            .iter()
            .any(|seat| seat.kind == Ta25SeatKind::Human && seat.player_id == player_id)
        {
            return false;
        }

        let Some(seat) = self
            .seats
            .iter_mut()
            .skip(1)
            .find(|seat| seat.kind != Ta25SeatKind::Human)
        else {
            return false;
        };
        *seat = Ta25Seat::human(seat.color, player_id, display_name);
        true
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn summary_line(&self) -> String {
        self.seats
            .iter()
            .map(|seat| {
                format!(
                    "{}={}({})",
                    seat.color.as_str(),
                    seat.display_name,
                    seat.kind.as_str()
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn seat_for_color(&self, color: Ta25SeatColor) -> Option<&Ta25Seat> {
        self.seats.iter().find(|seat| seat.color == color)
    }
}

impl Ta25SeatKind {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Cpu => "cpu",
            Self::Empty => "empty",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ta25CpuPlan {
    pub seat_color: Ta25SeatColor,
    pub answer_at: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct Ta25LocalGame {
    roster: Ta25Roster,
    questions: Vec<Question>,
    board: [Option<Ta25SeatColor>; TA25_RUN_LENGTH],
    current_question_index: usize,
    language: Language,
    started_at: Instant,
    round_started_at: Instant,
    current_cpu_plans: Vec<Ta25CpuPlan>,
    last_action: String,
    finished_elapsed: Option<Duration>,
}

impl Ta25LocalGame {
    pub fn from_pool(pool: &[Question], language: Language, human_name: &str) -> Option<Self> {
        if pool.len() < TA25_RUN_LENGTH {
            return None;
        }

        let mut rng = rand::thread_rng();
        let questions = pool
            .choose_multiple(&mut rng, TA25_RUN_LENGTH)
            .cloned()
            .collect::<Vec<_>>();
        let now = Instant::now();
        let roster = Ta25Roster::standard_local(human_name);
        let mut game = Self {
            roster,
            questions,
            board: [None; TA25_RUN_LENGTH],
            current_question_index: 0,
            language,
            started_at: now,
            round_started_at: now,
            current_cpu_plans: Vec::new(),
            last_action: "Round 1 started. Human is red; CPUs fill blue, green, yellow.".into(),
            finished_elapsed: None,
        };
        game.roll_cpu_plans();
        Some(game)
    }

    pub fn roster(&self) -> &Ta25Roster {
        &self.roster
    }

    pub fn current_question(&self) -> Option<&Question> {
        self.questions.get(self.current_question_index)
    }

    pub fn current_question_number(&self) -> usize {
        (self.current_question_index + 1).min(self.questions.len())
    }

    pub fn total_questions(&self) -> usize {
        self.questions.len()
    }

    pub fn current_panel_number(&self) -> usize {
        self.current_question_number()
    }

    pub fn board(&self) -> &[Option<Ta25SeatColor>; TA25_RUN_LENGTH] {
        &self.board
    }

    pub fn question_text(&self) -> String {
        self.current_question()
            .map(|question| DataLoader::get_question_text(question, &self.language))
            .unwrap_or_else(|| "TA25 complete.".to_string())
    }

    pub fn choice_texts(&self) -> Vec<String> {
        self.current_question()
            .map(|question| {
                question
                    .choices
                    .iter()
                    .map(|choice| DataLoader::get_choice_text(choice, &self.language))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn current_correct_typing_candidates(&self) -> Vec<String> {
        let Some(question) = self.current_question() else {
            return Vec::new();
        };
        let Some(choice) = question.choices.get(question.correct_answer_index) else {
            return Vec::new();
        };
        let mut candidates = DataLoader::get_choice_typing_texts(choice, &self.language)
            .into_iter()
            .map(|candidate| candidate.to_lowercase())
            .collect::<Vec<_>>();
        // Accept skipping displayed punctuation (・, :, parens, &, …): a
        // player who omits a separator still matches. Typing it also works
        // because the base candidate keeps it. See `punctuation_skip_variant`.
        for variant in candidates
            .iter()
            .filter_map(|c| punctuation_skip_variant(c))
            .collect::<Vec<_>>()
        {
            candidates.push(variant);
        }
        candidates.sort();
        candidates.dedup();
        candidates
    }

    pub fn is_valid_human_prefix(&self, typed: &str) -> bool {
        if typed.is_empty() {
            return true;
        }
        let typed_lower = typed.to_lowercase();
        let typed_key = self.canonical_key(typed);
        self.current_correct_typing_candidates()
            .iter()
            .any(|candidate| {
                let candidate_lower = candidate.to_lowercase();
                candidate_lower.starts_with(&typed_lower)
                    || self.canonical_key(candidate).starts_with(&typed_key)
            })
    }

    pub fn is_complete_human_answer(&self, typed: &str) -> bool {
        let typed_key = self.canonical_key(typed);
        self.current_correct_typing_candidates()
            .iter()
            .any(|candidate| self.canonical_key(candidate) == typed_key)
    }

    pub fn submit_human_answer(&mut self, typed: &str, now: Instant) -> bool {
        if self.poll_cpu(now) {
            return false;
        }
        if !self.is_complete_human_answer(typed) {
            return false;
        }
        self.claim_current_panel(Some(Ta25SeatColor::Red), now, "You answered first.");
        true
    }

    pub fn forfeit_current_round(&mut self, now: Instant) {
        if self.is_finished() {
            return;
        }

        if let Some(plan) = self.fastest_cpu_plan() {
            let seat_name = self
                .roster
                .seat_for_color(plan.seat_color)
                .map(|seat| seat.display_name.as_str())
                .unwrap_or("CPU");
            self.claim_current_panel(
                Some(plan.seat_color),
                now,
                &format!("{seat_name} took the panel after your skip."),
            );
            return;
        }

        self.claim_current_panel(None, now, "Round skipped. No CPU claimed the panel.");
    }

    pub fn poll_cpu(&mut self, now: Instant) -> bool {
        if self.is_finished() {
            return false;
        }

        let elapsed = now.saturating_duration_since(self.round_started_at);
        let Some(plan) = self
            .current_cpu_plans
            .iter()
            .filter_map(|plan| plan.answer_at.map(|answer_at| (*plan, answer_at)))
            .filter(|(_, answer_at)| *answer_at <= elapsed)
            .min_by_key(|(_, answer_at)| *answer_at)
            .map(|(plan, _)| plan)
        else {
            return false;
        };

        let seat_name = self
            .roster
            .seat_for_color(plan.seat_color)
            .map(|seat| seat.display_name.as_str())
            .unwrap_or("CPU");
        self.claim_current_panel(
            Some(plan.seat_color),
            now,
            &format!("{seat_name} buzzed in first."),
        );
        true
    }

    pub fn is_finished(&self) -> bool {
        self.finished_elapsed.is_some() || self.current_question_index >= self.questions.len()
    }

    pub fn elapsed(&self, now: Instant) -> Duration {
        self.finished_elapsed
            .unwrap_or_else(|| now.saturating_duration_since(self.started_at))
    }

    pub fn counts_by_color(&self) -> [(Ta25SeatColor, u32); 4] {
        let mut counts = [(Ta25SeatColor::Red, 0); 4];
        for (index, color) in Ta25SeatColor::ALL.into_iter().enumerate() {
            counts[index] = (
                color,
                self.board
                    .iter()
                    .filter(|owner| **owner == Some(color))
                    .count() as u32,
            );
        }
        counts
    }

    pub fn leader_summary(&self) -> String {
        let counts = self.counts_by_color();
        let best = counts.iter().map(|(_, count)| *count).max().unwrap_or(0);
        if best == 0 {
            return "No leader yet".to_string();
        }
        let leaders = counts
            .iter()
            .filter(|(_, count)| *count == best)
            .filter_map(|(color, _)| self.roster.seat_for_color(*color))
            .map(|seat| seat.display_name.as_str())
            .collect::<Vec<_>>();
        if leaders.is_empty() {
            "No leader yet".to_string()
        } else if leaders.len() == 1 {
            format!("Leader: {} ({best})", leaders[0])
        } else {
            format!("Tie: {} ({best})", leaders.join(", "))
        }
    }

    pub fn last_action(&self) -> &str {
        &self.last_action
    }

    fn canonical_key(&self, s: &str) -> String {
        let lower = s.to_lowercase();
        if matches!(self.language, Language::Japanese) {
            canonical_romaji(&lower)
        } else {
            lower
        }
    }

    fn fastest_cpu_plan(&self) -> Option<Ta25CpuPlan> {
        self.current_cpu_plans
            .iter()
            .filter_map(|plan| plan.answer_at.map(|answer_at| (*plan, answer_at)))
            .min_by_key(|(_, answer_at)| *answer_at)
            .map(|(plan, _)| plan)
    }

    fn claim_current_panel(
        &mut self,
        owner: Option<Ta25SeatColor>,
        now: Instant,
        action_summary: &str,
    ) {
        if self.current_question_index >= self.questions.len() {
            return;
        }

        self.board[self.current_question_index] = owner;
        let panel_number = self.current_question_index + 1;
        self.last_action = match owner {
            Some(color) => format!(
                "{action_summary} Panel {panel_number:02} -> {}.",
                color.panel_style_name()
            ),
            None => format!("{action_summary} Panel {panel_number:02} stays blank."),
        };
        self.current_question_index += 1;
        if self.current_question_index >= self.questions.len() {
            self.finished_elapsed = Some(now.saturating_duration_since(self.started_at));
            self.current_cpu_plans.clear();
            return;
        }

        self.round_started_at = now;
        self.roll_cpu_plans();
    }

    fn roll_cpu_plans(&mut self) {
        let mut rng = rand::thread_rng();
        self.current_cpu_plans = self
            .roster
            .seats
            .iter()
            .filter(|seat| seat.kind == Ta25SeatKind::Cpu)
            .map(|seat| {
                let (min_ms, max_ms, success_rate) = match seat.color {
                    Ta25SeatColor::Blue => (1800_u64, 3200_u64, 72_u8),
                    Ta25SeatColor::Green => (2400_u64, 4200_u64, 58_u8),
                    Ta25SeatColor::Yellow => (3000_u64, 5200_u64, 46_u8),
                    Ta25SeatColor::Red => (2000_u64, 3600_u64, 65_u8),
                };
                let answer_at = if rng.gen_range(0_u8..100_u8) < success_rate {
                    Some(Duration::from_millis(rng.gen_range(min_ms..=max_ms)))
                } else {
                    None
                };
                Ta25CpuPlan {
                    seat_color: seat.color,
                    answer_at,
                }
            })
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Choice;
    use std::collections::HashMap;

    fn sample_question(id: usize) -> Question {
        Question {
            id: format!("q-{id:02}"),
            genre: "test".into(),
            question_text: HashMap::from([
                ("ja".to_string(), format!("問題{id}")),
                ("en".to_string(), format!("Question {id}")),
            ]),
            question_text_reading: HashMap::new(),
            choices: vec![
                Choice {
                    labels: HashMap::from([
                        ("ja".to_string(), "a".to_string()),
                        ("en".to_string(), "alpha".to_string()),
                    ]),
                    ja_typings: vec!["a".to_string()],
                },
                Choice {
                    labels: HashMap::from([
                        ("ja".to_string(), "b".to_string()),
                        ("en".to_string(), "bravo".to_string()),
                    ]),
                    ja_typings: vec!["b".to_string()],
                },
            ],
            correct_answer_index: 0,
            image_path: None,
            ja_reviewed: true,
        }
    }

    #[test]
    fn standard_local_is_one_human_plus_three_cpu() {
        let roster = Ta25Roster::standard_local("You");
        assert_eq!(roster.seats[0].color, Ta25SeatColor::Red);
        assert_eq!(roster.seats[0].kind, Ta25SeatKind::Human);
        assert_eq!(roster.seats[0].display_name, "You");

        let cpu_kinds = roster
            .seats
            .iter()
            .skip(1)
            .map(|seat| seat.kind)
            .collect::<Vec<_>>();
        assert_eq!(
            cpu_kinds,
            vec![Ta25SeatKind::Cpu, Ta25SeatKind::Cpu, Ta25SeatKind::Cpu]
        );
    }

    #[test]
    fn standard_local_uses_all_four_colors_once() {
        let roster = Ta25Roster::standard_local("You");
        let colors = roster
            .seats
            .iter()
            .map(|seat| seat.color)
            .collect::<Vec<_>>();
        assert_eq!(colors, Ta25SeatColor::ALL);
    }

    #[test]
    fn adding_human_replaces_next_cpu_in_canonical_order() {
        let mut roster = Ta25Roster::standard_local("You");
        assert!(roster.add_human("guest-1", "Guest 1"));
        let blue = roster
            .seats
            .iter()
            .find(|seat| seat.color == Ta25SeatColor::Blue)
            .expect("blue seat");
        assert_eq!(blue.kind, Ta25SeatKind::Human);
        assert_eq!(blue.player_id, "guest-1");
        assert_eq!(blue.display_name, "Guest 1");
    }

    #[test]
    fn duplicate_human_id_is_rejected() {
        let mut roster = Ta25Roster::standard_local("You");
        assert!(!roster.add_human("local-human", "Guest 1"));
    }

    #[test]
    fn adding_humans_consumes_blue_green_yellow_in_order() {
        let mut roster = Ta25Roster::standard_local("You");
        assert!(roster.add_human("guest-1", "Guest 1"));
        assert!(roster.add_human("guest-2", "Guest 2"));
        assert!(roster.add_human("guest-3", "Guest 3"));
        assert!(!roster.add_human("guest-4", "Guest 4"));

        let humans = roster
            .seats
            .iter()
            .filter(|seat| seat.kind == Ta25SeatKind::Human)
            .map(|seat| (seat.color, seat.player_id.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            humans,
            vec![
                (Ta25SeatColor::Red, "local-human"),
                (Ta25SeatColor::Blue, "guest-1"),
                (Ta25SeatColor::Green, "guest-2"),
                (Ta25SeatColor::Yellow, "guest-3"),
            ]
        );
    }

    #[test]
    fn all_empty_exposes_empty_seats() {
        let roster = Ta25Roster::all_empty();
        assert!(roster
            .seats
            .iter()
            .all(|seat| seat.kind == Ta25SeatKind::Empty));
    }

    #[test]
    fn summary_line_mentions_each_seat() {
        let roster = Ta25Roster::standard_local("You");
        assert_eq!(
            roster.summary_line(),
            "red=You(human), blue=CPU 1(cpu), green=CPU 2(cpu), yellow=CPU 3(cpu)"
        );
    }

    #[test]
    fn prototype_requires_full_25_question_pool() {
        let pool = (1..25).map(sample_question).collect::<Vec<_>>();
        assert!(Ta25LocalGame::from_pool(&pool, Language::English, "You").is_none());
    }

    #[test]
    fn prototype_builds_fixed_25_question_run() {
        let pool = (1..40).map(sample_question).collect::<Vec<_>>();
        let game = Ta25LocalGame::from_pool(&pool, Language::English, "You").expect("game");
        assert_eq!(game.total_questions(), TA25_RUN_LENGTH);
        assert_eq!(game.roster().seats[0].display_name, "You");
    }

    #[test]
    fn human_completion_claims_current_panel_for_red() {
        let pool = (1..40).map(sample_question).collect::<Vec<_>>();
        let mut game = Ta25LocalGame::from_pool(&pool, Language::English, "You").expect("game");
        let now = Instant::now();
        assert!(game.submit_human_answer("alpha", now));
        assert_eq!(game.board()[0], Some(Ta25SeatColor::Red));
    }

    #[test]
    fn forfeit_without_cpu_plan_leaves_blank_panel() {
        let pool = (1..40).map(sample_question).collect::<Vec<_>>();
        let mut game = Ta25LocalGame::from_pool(&pool, Language::English, "You").expect("game");
        game.current_cpu_plans = vec![
            Ta25CpuPlan {
                seat_color: Ta25SeatColor::Blue,
                answer_at: None,
            },
            Ta25CpuPlan {
                seat_color: Ta25SeatColor::Green,
                answer_at: None,
            },
        ];
        game.forfeit_current_round(Instant::now());
        assert_eq!(game.board()[0], None);
    }

    #[test]
    fn cpu_deadline_beats_human_submission_when_already_due() {
        let pool = (1..40).map(sample_question).collect::<Vec<_>>();
        let mut game = Ta25LocalGame::from_pool(&pool, Language::English, "You").expect("game");
        let now = Instant::now();
        game.round_started_at = now - Duration::from_millis(250);
        game.current_cpu_plans = vec![Ta25CpuPlan {
            seat_color: Ta25SeatColor::Blue,
            answer_at: Some(Duration::from_millis(120)),
        }];
        assert!(!game.submit_human_answer("alpha", now));
        assert_eq!(game.board()[0], Some(Ta25SeatColor::Blue));
    }

    #[test]
    fn leader_summary_is_empty_before_any_panel_is_claimed() {
        let pool = (1..40).map(sample_question).collect::<Vec<_>>();
        let game = Ta25LocalGame::from_pool(&pool, Language::English, "You").expect("game");
        assert_eq!(game.leader_summary(), "No leader yet");
    }
}

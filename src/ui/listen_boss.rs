use crate::audio::{TtsEngine, TtsRequest, TtsRequestKind};
use crate::game::listening::{acceptable_listening_inputs, is_valid_listening_prefix};
use crate::game::{ListeningSession, SubmissionResult, RPG_RUN_LENGTH};
use crate::types::{BossHintRevealMode, BossTier, Language, ListeningBossSpec};
use crate::ui::{HelpEntry, HelpLine, InputChannel, PaneFrame, RecvOutcome};
use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::time::{Duration, Instant};

const STYLE_TITLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
const STYLE_NORMAL: Style = Style::new().fg(Color::White);
const STYLE_DIM: Style = Style::new().fg(Color::DarkGray);
const STYLE_CORRECT: Style = Style::new().fg(Color::Green).add_modifier(Modifier::BOLD);
const STYLE_INCORRECT: Style = Style::new().fg(Color::Red).add_modifier(Modifier::BOLD);
const STYLE_INPUT_ECHO: Style = Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);
const STYLE_LABEL: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
const STYLE_HINT_ACTIVE: Style = Style::new().fg(Color::White).add_modifier(Modifier::BOLD);
const STYLE_HINT_LOCKED: Style = Style::new().fg(Color::DarkGray);
const INPUT_REJECT_FLASH_MS: u64 = 180;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Playing,
    Result,
}

pub struct BossListenUI {
    session: ListeningSession,
    spec: ListeningBossSpec,
    plan: BossUiPlan,
    language: Language,
    tts: Option<TtsEngine>,
    phase: Phase,
    encounter_index: usize,
    started_at: Instant,
    plays: u32,
    revealed_hints: usize,
    focused_hint: usize,
    logs: Vec<String>,
    rejected_char: Option<char>,
    reject_flash_until: Option<Instant>,
    /// Snapshot of the parent run's rolling battle log (#36). Surfaced as
    /// extra context lines below the in-encounter event log so the player
    /// can see Hit/Missed history coming into the boss/miniboss fight.
    battle_log: Vec<String>,
}

/// Maximum visible battle-log tail rendered above the per-encounter log.
const BOSS_LOG_TAIL_MAX: usize = 6;

impl BossListenUI {
    pub fn new(
        session: ListeningSession,
        spec: ListeningBossSpec,
        tts: Option<TtsEngine>,
        language: Language,
        encounter_index: usize,
    ) -> Self {
        Self {
            session,
            plan: BossUiPlan::from_spec(&spec),
            spec,
            language,
            tts,
            phase: Phase::Playing,
            encounter_index,
            started_at: Instant::now(),
            plays: 0,
            revealed_hints: 1,
            focused_hint: 0,
            logs: Vec::new(),
            rejected_char: None,
            reject_flash_until: None,
            battle_log: Vec::new(),
        }
    }

    /// Seed the log pane with the rolling battle log from the parent
    /// `ListeningRpgRun` (#36). Pass the *full* log; the UI clips to the
    /// visible tail.
    pub fn set_battle_log(&mut self, log: Vec<String>) {
        self.battle_log = log;
    }

    pub fn take_tts(&mut self) -> Option<TtsEngine> {
        self.tts.take()
    }

    pub fn run(&mut self) -> Result<Option<SubmissionResult>, Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        self.voice_hint(0, true);
        let result = self.run_app(&mut terminal);

        if let Some(tts) = self.tts.as_mut() {
            let _ = tts.stop();
        }
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        result
    }

    fn run_app(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<Option<SubmissionResult>, Box<dyn std::error::Error>> {
        const REDRAW: Duration = Duration::from_millis(30);
        let input = InputChannel::spawn();

        loop {
            self.tick();
            terminal.draw(|f| self.ui(f))?;
            match input.recv_until(REDRAW) {
                RecvOutcome::Key(key) => {
                    if self.handle_key(key) {
                        break;
                    }
                }
                RecvOutcome::Timeout => {}
                RecvOutcome::Disconnected => break,
            }
        }

        Ok(self.session.result().cloned())
    }

    fn tick(&mut self) {
        if self.phase != Phase::Playing || self.plan.reveal_mode != BossHintRevealMode::Timed {
            return;
        }

        let now = Instant::now();
        let mut visible = 1;
        for hint in self.spec.hints.iter().skip(1) {
            if hint
                .auto_reveal_after_ms
                .map(|delay| now >= self.started_at + Duration::from_millis(delay))
                .unwrap_or(false)
            {
                visible += 1;
            }
        }

        while self.revealed_hints < visible {
            let next = self.revealed_hints;
            self.revealed_hints += 1;
            self.focused_hint = next;
            self.logs.push(format!(
                "Hint {} opened automatically.",
                self.focused_hint + 1
            ));
            self.voice_hint(self.focused_hint, false);
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if matches!(key.code, KeyCode::Esc) {
            return true;
        }
        if matches!(key.code, KeyCode::Char('c')) && key.modifiers.contains(KeyModifiers::CONTROL) {
            return true;
        }

        match self.phase {
            Phase::Result => return matches!(key.code, KeyCode::Enter),
            Phase::Playing => {}
        }

        match key.code {
            KeyCode::Char(' ')
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.voice_hint(self.focused_hint, true);
            }
            KeyCode::Tab
                if self.plan.reveal_mode == BossHintRevealMode::Manual
                    && self.revealed_hints < self.spec.hints.len() =>
            {
                self.revealed_hints += 1;
                self.focused_hint = self.revealed_hints - 1;
                self.logs
                    .push(format!("Hint {} opened by player.", self.focused_hint + 1));
                self.voice_hint(self.focused_hint, false);
            }
            KeyCode::BackTab if self.focused_hint > 0 => {
                self.focused_hint -= 1;
                self.logs.push(format!(
                    "Focus moved back to hint {}.",
                    self.focused_hint + 1
                ));
            }
            KeyCode::Backspace => {
                self.session.pop_char();
                self.clear_reject_flash();
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.handle_playing_char(c);
            }
            _ => {}
        }

        false
    }

    fn voice_hint(&mut self, index: usize, replay: bool) {
        if let Some(hint) = self.spec.hints.get(index) {
            if let Some(tts) = self.tts.as_mut() {
                let _ = tts.speak_request(TtsRequest {
                    text: hint.text_reading.as_deref().unwrap_or(&hint.text_display),
                    lang: &self.language,
                    kind: TtsRequestKind::BossHint {
                        layer: (index + 1) as u8,
                    },
                });
            }
            self.plays += 1;
            self.logs.push(if replay {
                format!("Replayed hint {}.", index + 1)
            } else {
                format!("Hint {} voiced.", index + 1)
            });
        }
    }

    fn handle_playing_char(&mut self, c: char) {
        let mut attempted = self.session.input().to_string();
        attempted.push(c);
        if !is_valid_listening_prefix(
            &self.language,
            &attempted,
            &self.session.prompt().text_reading,
        ) {
            self.note_rejected_char(c);
            return;
        }

        self.session.push_char(c);
        self.clear_reject_flash();

        let typed = self.session.input().to_lowercase();
        if acceptable_listening_inputs(&self.language, &self.session.prompt().text_reading)
            .iter()
            .any(|candidate| candidate == &typed)
        {
            self.session.submit();
            self.phase = Phase::Result;
            self.logs.push("Boss defeated.".into());
            if let Some(tts) = self.tts.as_mut() {
                let _ = tts.stop();
                let _ = tts.speak_request(TtsRequest {
                    text: &self.session.prompt().text_reading,
                    lang: &self.language,
                    kind: TtsRequestKind::BossReveal,
                });
                self.plays += 1;
            }
        }
    }

    fn ui(&self, f: &mut Frame) {
        let frame = PaneFrame::rpg(f.area());
        self.render_main_pane(f, frame.main);
        self.render_status_pane(f, frame.side);
        self.render_input_echo(f, frame.input_echo);
        if let Some(log) = frame.log {
            self.render_log_pane(f, log);
        }
        self.render_help_line(f, frame.help_line);
    }

    fn render_main_pane(&self, f: &mut Frame, area: Rect) {
        let title = format!(
            " {} {}/{} ",
            self.plan.title, self.encounter_index, RPG_RUN_LENGTH
        );
        let mut lines = Vec::new();

        for (index, hint) in self.spec.hints.iter().enumerate() {
            if index < self.revealed_hints {
                let style = if index == self.focused_hint {
                    STYLE_HINT_ACTIVE
                } else {
                    STYLE_NORMAL
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("[{}] ", index + 1), STYLE_LABEL),
                    Span::styled(format!("{}: ", hint.label), style),
                    Span::styled(hint.text_display.clone(), style),
                ]));
            } else {
                lines.push(Line::from(Span::styled(
                    format!("[{}] ???", index + 1),
                    STYLE_HINT_LOCKED,
                )));
            }
        }

        if self.phase == Phase::Result {
            if let Some(result) = self.session.result() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    if result.is_correct {
                        "Correct!"
                    } else {
                        "Wrong."
                    },
                    if result.is_correct {
                        STYLE_CORRECT
                    } else {
                        STYLE_INCORRECT
                    },
                )));
                lines.push(Line::from(vec![
                    Span::styled("Expected: ", STYLE_LABEL),
                    Span::styled(self.session.prompt().text_display.clone(), STYLE_NORMAL),
                ]));
            }
        }

        let para = Paragraph::new(lines).alignment(Alignment::Left).block(
            Block::default()
                .title(title)
                .title_style(STYLE_TITLE)
                .borders(Borders::ALL)
                .padding(Padding::uniform(1)),
        );
        f.render_widget(para, area);
    }

    fn render_status_pane(&self, f: &mut Frame, area: Rect) {
        let elapsed = self.started_at.elapsed();
        let mins = elapsed.as_secs() / 60;
        let secs = elapsed.as_secs() % 60;
        let lines = vec![
            Line::from(Span::styled(
                match self.spec.tier {
                    BossTier::Miniboss => "Miniboss",
                    BossTier::Boss => "Boss",
                },
                STYLE_LABEL,
            )),
            Line::from(Span::styled(
                match self.plan.reveal_mode {
                    BossHintRevealMode::Manual => "Reveal: manual",
                    BossHintRevealMode::Timed => "Reveal: timed",
                },
                STYLE_DIM,
            )),
            Line::from(""),
            Line::from(format!(
                "Hint   : {}/{}",
                self.focused_hint + 1,
                self.spec.hints.len()
            )),
            Line::from(format!(
                "Open   : {}/{}",
                self.revealed_hints,
                self.spec.hints.len()
            )),
            Line::from(format!("Plays  : {}", self.plays)),
            Line::from(format!("Time   : {mins}:{secs:02}")),
        ];
        let para = Paragraph::new(lines).alignment(Alignment::Left).block(
            Block::default()
                .title(" Status ")
                .borders(Borders::ALL)
                .padding(Padding::uniform(1)),
        );
        f.render_widget(para, area);
    }

    fn render_input_echo(&self, f: &mut Frame, area: Rect) {
        if area.height == 0 {
            return;
        }

        let prompt = if self.should_shake_input_echo() {
            " > "
        } else {
            "> "
        };
        let flash_active = self.reject_flash_is_active();
        let mut spans = vec![
            Span::styled(
                prompt.to_string(),
                if flash_active {
                    STYLE_INCORRECT
                } else {
                    STYLE_DIM
                },
            ),
            Span::styled(self.session.input().to_string(), STYLE_CORRECT),
        ];
        if flash_active {
            if let Some(c) = self.rejected_char {
                spans.push(Span::styled(c.to_string(), STYLE_INCORRECT));
            }
            spans.push(Span::styled("_", STYLE_INCORRECT));
        } else {
            spans.push(Span::styled("_", STYLE_INPUT_ECHO));
        }

        f.render_widget(
            Paragraph::new(Line::from(spans)).alignment(Alignment::Left),
            area,
        );
    }

    fn render_log_pane(&self, f: &mut Frame, area: Rect) {
        let mut lines: Vec<Line<'static>> = Vec::new();

        // Surface the tail of the parent run's battle log first (#36 s1),
        // so boss/miniboss encounters see prior Hit/Missed history.
        if !self.battle_log.is_empty() {
            let start = self.battle_log.len().saturating_sub(BOSS_LOG_TAIL_MAX);
            for entry in &self.battle_log[start..] {
                lines.push(Line::from(Span::styled(entry.clone(), STYLE_DIM)));
            }
        }

        if self.logs.is_empty() && self.battle_log.is_empty() {
            lines.push(Line::from(Span::styled("(no events)", STYLE_DIM)));
        } else {
            let inner_height = area.height.saturating_sub(2) as usize;
            let remaining = inner_height.saturating_sub(lines.len()).max(1);
            let start = self.logs.len().saturating_sub(remaining);
            for entry in &self.logs[start..] {
                lines.push(Line::from(Span::styled(format!("▸ {entry}"), STYLE_NORMAL)));
            }
        }

        let para = Paragraph::new(lines)
            .alignment(Alignment::Left)
            .block(Block::default().title(" Log ").borders(Borders::ALL));
        f.render_widget(para, area);
    }

    fn render_help_line(&self, f: &mut Frame, area: Rect) {
        let help = match self.phase {
            Phase::Playing => HelpLine::new(self.plan.help_entries.clone()),
            Phase::Result => HelpLine::new(vec![HelpEntry::new("Enter", "Next")]),
        };
        help.render(f, area);
    }

    fn note_rejected_char(&mut self, c: char) {
        self.rejected_char = Some(c);
        self.reject_flash_until =
            Some(Instant::now() + Duration::from_millis(INPUT_REJECT_FLASH_MS));
    }

    fn clear_reject_flash(&mut self) {
        self.rejected_char = None;
        self.reject_flash_until = None;
    }

    fn reject_flash_is_active(&self) -> bool {
        self.reject_flash_until
            .map(|until| Instant::now() < until)
            .unwrap_or(false)
    }

    fn should_shake_input_echo(&self) -> bool {
        self.reject_flash_until
            .map(|until| {
                let remaining_ticks =
                    until.saturating_duration_since(Instant::now()).as_millis() / 45;
                self.reject_flash_is_active() && remaining_ticks % 2 == 0
            })
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AnswerKind, BossHintStep, ListeningPrompt};

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

    fn boss_session() -> ListeningSession {
        ListeningSession::new(
            ListeningPrompt {
                id: "boss".into(),
                text_reading: "language".into(),
                text_display: "language".into(),
                kind: AnswerKind::Word,
                boss: Some(boss_spec(BossTier::Boss, BossHintRevealMode::Manual)),
            },
            Language::English,
        )
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

    #[test]
    fn take_tts_is_none_when_built_without_tts() {
        let mut ui = BossListenUI::new(
            boss_session(),
            boss_spec(BossTier::Boss, BossHintRevealMode::Manual),
            None,
            Language::English,
            10,
        );
        assert!(ui.take_tts().is_none());
    }
}

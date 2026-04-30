use crate::types::{GameMode, Language};
use crate::ui::{HelpEntry, HelpLine};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::io;

const STYLE_TITLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
const STYLE_SELECTED: Style = Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);
const STYLE_NORMAL: Style = Style::new();
const STYLE_LABEL: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);

struct LanguageOption {
    label: &'static str,
    description: [&'static str; 2],
}

struct ModeOption {
    label: &'static str,
    description: [&'static str; 2],
}

const LANGUAGE_OPTIONS: [LanguageOption; 2] = [
    LanguageOption {
        label: "日本語 / Japanese",
        description: [
            "Use Japanese prompts and localized records.",
            "問題文とランキング表示を日本語にします。",
        ],
    },
    LanguageOption {
        label: "English",
        description: [
            "Use English prompts and localized records.",
            "English questions and ranking labels are used.",
        ],
    },
];

const MODE_OPTIONS: [ModeOption; 4] = [
    ModeOption {
        label: "Quiz",
        description: [
            "The standard play mode: answer quiz prompts and build core skill.",
            "基本プレイ。問題に答えて type-globe の土台を鍛えます。",
        ],
    },
    ModeOption {
        label: "Time Attack 25",
        description: [
            "A Quiz variant with panel capture and head-to-head pressure.",
            "Quiz 派生。対戦とパネル奪取で 25 マスを奪い合います。",
        ],
    },
    ModeOption {
        label: "Listening RPG",
        description: [
            "A separate ruleset: hear the prompt, then survive a 10-battle run.",
            "別ルール系。聞いて打つ 10 戦のリスニングRPGです。",
        ],
    },
    ModeOption {
        label: "Records / Ranking",
        description: [
            "Browse cross-mode records for Quiz, Time Attack 25, and Listening RPG.",
            "3 モード分の結果を横断して見るランキング画面です。",
        ],
    },
];

pub struct MenuUI {
    selected_language: usize,
    selected_mode: usize,
    step: MenuStep,
    should_quit: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum MenuStep {
    LanguageSelection,
    ModeSelection,
}

impl MenuUI {
    pub fn new() -> Self {
        Self {
            selected_language: 0,
            selected_mode: 0,
            step: MenuStep::LanguageSelection,
            should_quit: false,
        }
    }

    pub fn run(&mut self) -> Result<(Language, GameMode), Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.run_app(&mut terminal);

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        result
    }

    pub fn return_to_mode_selection(&mut self, language: Language) {
        self.selected_language = match language {
            Language::Japanese => 0,
            Language::English => 1,
        };
        self.step = MenuStep::ModeSelection;
        self.should_quit = false;
    }

    fn run_app(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<(Language, GameMode), Box<dyn std::error::Error>> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                if let Some(result) = self.handle_key(key) {
                    return Ok(result);
                }
                if self.should_quit {
                    return Err("User quit".into());
                }
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<(Language, GameMode)> {
        const MODE_COUNT: usize = 4;

        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Up | KeyCode::Char('k') => match self.step {
                MenuStep::LanguageSelection => {
                    self.selected_language = self.selected_language.saturating_sub(1);
                }
                MenuStep::ModeSelection => {
                    self.selected_mode = self.selected_mode.saturating_sub(1);
                }
            },
            KeyCode::Down | KeyCode::Char('j') => match self.step {
                MenuStep::LanguageSelection => {
                    if self.selected_language < 1 {
                        self.selected_language += 1;
                    }
                }
                MenuStep::ModeSelection => {
                    if self.selected_mode + 1 < MODE_COUNT {
                        self.selected_mode += 1;
                    }
                }
            },
            KeyCode::Enter => match self.step {
                MenuStep::LanguageSelection => {
                    self.step = MenuStep::ModeSelection;
                }
                MenuStep::ModeSelection => {
                    let language = match self.selected_language {
                        0 => Language::Japanese,
                        1 => Language::English,
                        _ => Language::Japanese,
                    };
                    let mode = match self.selected_mode {
                        0 => GameMode::Quiz,
                        1 => GameMode::TimeAttack25,
                        2 => GameMode::HackAndSlashRpg,
                        3 => GameMode::Ranking,
                        _ => GameMode::Quiz,
                    };
                    return Some((language, mode));
                }
            },
            KeyCode::Esc if self.step == MenuStep::ModeSelection => {
                self.step = MenuStep::LanguageSelection;
            }
            _ => {}
        }
        None
    }

    fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(1),
            ])
            .split(f.area());

        self.render_title(f, chunks[0]);

        match self.step {
            MenuStep::LanguageSelection => self.render_language_selection(f, chunks[1]),
            MenuStep::ModeSelection => self.render_mode_selection(f, chunks[1]),
        }

        self.help_line().render(f, chunks[2]);
    }

    fn render_title(&self, f: &mut Frame, area: Rect) {
        let title = Paragraph::new("type-globe - The answer is never shown")
            .style(STYLE_TITLE)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, area);
    }

    fn render_language_selection(&self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = LANGUAGE_OPTIONS
            .iter()
            .enumerate()
            .map(|(i, language)| {
                let style = if i == self.selected_language {
                    STYLE_SELECTED
                } else {
                    STYLE_NORMAL
                };
                ListItem::new(Line::from(Span::styled(language.label, style)))
            })
            .collect();

        let [list_area, detail_area] = split_selection_area(area);
        let language_list = List::new(items)
            .block(
                Block::default()
                    .title("言語を選択してください / Select Language")
                    .borders(Borders::ALL),
            )
            .highlight_style(STYLE_SELECTED);

        let mut state = ListState::default();
        state.select(Some(self.selected_language));
        f.render_stateful_widget(language_list, list_area, &mut state);

        self.render_detail_panel(
            f,
            detail_area,
            "Language",
            LANGUAGE_OPTIONS[self.selected_language].description,
        );
    }

    fn render_mode_selection(&self, f: &mut Frame, area: Rect) {
        let [list_area, detail_area] = split_selection_area(area);
        let items: Vec<ListItem> = MODE_OPTIONS
            .iter()
            .enumerate()
            .map(|(i, mode)| {
                let style = if i == self.selected_mode {
                    STYLE_SELECTED
                } else {
                    STYLE_NORMAL
                };
                ListItem::new(Line::from(Span::styled(mode.label, style)))
            })
            .collect();

        let mode_list = List::new(items)
            .block(
                Block::default()
                    .title("ゲームモードを選択してください / Select Game Mode")
                    .borders(Borders::ALL),
            )
            .highlight_style(STYLE_SELECTED);

        let mut state = ListState::default();
        state.select(Some(self.selected_mode));
        f.render_stateful_widget(mode_list, list_area, &mut state);

        self.render_detail_panel(
            f,
            detail_area,
            "Mode",
            MODE_OPTIONS[self.selected_mode].description,
        );
    }

    fn help_line(&self) -> HelpLine {
        match self.step {
            MenuStep::LanguageSelection => HelpLine::new(vec![
                HelpEntry::new("↑↓", "Select"),
                HelpEntry::new("Enter", "Confirm"),
                HelpEntry::new("q", "Quit"),
            ]),
            MenuStep::ModeSelection => HelpLine::new(vec![
                HelpEntry::new("↑↓", "Select"),
                HelpEntry::new("Enter", "Confirm"),
                HelpEntry::new("Esc", "Back"),
                HelpEntry::new("q", "Quit"),
            ]),
        }
    }

    fn render_detail_panel(&self, f: &mut Frame, area: Rect, title: &str, description: [&str; 2]) {
        let lines = vec![
            Line::from(Span::styled(title, STYLE_LABEL)),
            Line::default(),
            Line::from(description[0]),
            Line::from(description[1]),
        ];

        let detail = Paragraph::new(lines)
            .wrap(ratatui::widgets::Wrap { trim: true })
            .block(
                Block::default()
                    .title("説明 / Details")
                    .borders(Borders::ALL),
            );
        f.render_widget(detail, area);
    }
}

fn split_selection_area(area: Rect) -> [Rect; 2] {
    let constraints = if area.width >= 80 {
        [Constraint::Percentage(42), Constraint::Percentage(58)]
    } else {
        [Constraint::Percentage(50), Constraint::Percentage(50)]
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    [chunks[0], chunks[1]]
}

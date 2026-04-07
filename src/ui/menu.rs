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
use crate::types::{GameMode, Language};

const STYLE_TITLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
const STYLE_SELECTED: Style = Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);
const STYLE_NORMAL: Style = Style::new();
const STYLE_HELP: Style = Style::new().fg(Color::Gray);

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

    fn run_app(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<(Language, GameMode), Box<dyn std::error::Error>> {
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
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                match self.step {
                    MenuStep::LanguageSelection => {
                        self.selected_language = self.selected_language.saturating_sub(1);
                    }
                    MenuStep::ModeSelection => {
                        self.selected_mode = self.selected_mode.saturating_sub(1);
                    }
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                match self.step {
                    MenuStep::LanguageSelection => {
                        if self.selected_language < 1 {
                            self.selected_language += 1;
                        }
                    }
                    MenuStep::ModeSelection => {
                        if self.selected_mode < 5 {
                            self.selected_mode += 1;
                        }
                    }
                }
            }
            KeyCode::Enter => {
                match self.step {
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
                            1 => GameMode::Typing,
                            2 => GameMode::QuizTyping,
                            3 => GameMode::TimeAttack,
                            4 => GameMode::Rpg,
                            5 => GameMode::Stealth,
                            _ => GameMode::Quiz,
                        };
                        return Some((language, mode));
                    }
                }
            }
            KeyCode::Esc => {
                if self.step == MenuStep::ModeSelection {
                    self.step = MenuStep::LanguageSelection;
                }
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
                Constraint::Length(3),
            ])
            .split(f.area());

        self.render_title(f, chunks[0]);

        match self.step {
            MenuStep::LanguageSelection => self.render_language_selection(f, chunks[1]),
            MenuStep::ModeSelection => self.render_mode_selection(f, chunks[1]),
        }

        self.render_help(f, chunks[2]);
    }

    fn render_title(&self, f: &mut Frame, area: Rect) {
        let title = Paragraph::new("TypeGlobe - Quiz & Typing Game")
            .style(STYLE_TITLE)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, area);
    }

    fn render_language_selection(&self, f: &mut Frame, area: Rect) {
        let languages = vec!["日本語 (Japanese)", "English"];
        let items: Vec<ListItem> = languages
            .iter()
            .enumerate()
            .map(|(i, &lang)| {
                let style = if i == self.selected_language {
                    STYLE_SELECTED
                } else {
                    STYLE_NORMAL
                };
                ListItem::new(Line::from(Span::styled(lang, style)))
            })
            .collect();

        let language_list = List::new(items)
            .block(Block::default().title("言語を選択してください / Select Language").borders(Borders::ALL))
            .highlight_style(STYLE_SELECTED);

        let mut state = ListState::default();
        state.select(Some(self.selected_language));
        f.render_stateful_widget(language_list, area, &mut state);
    }

    fn render_mode_selection(&self, f: &mut Frame, area: Rect) {
        let modes = vec![
            "クイズモード / Quiz Mode",
            "タイピングモード / Typing Mode",
            "クイズ+タイピングモード / Quiz+Typing Mode",
            "タイムアタック25 / Time Attack 25",
            "RPGモード / RPG Mode",
            "ステルスモード / Stealth Mode",
        ];
        let items: Vec<ListItem> = modes
            .iter()
            .enumerate()
            .map(|(i, &mode)| {
                let style = if i == self.selected_mode {
                    STYLE_SELECTED
                } else {
                    STYLE_NORMAL
                };
                ListItem::new(Line::from(Span::styled(mode, style)))
            })
            .collect();

        let mode_list = List::new(items)
            .block(Block::default().title("ゲームモードを選択してください / Select Game Mode").borders(Borders::ALL))
            .highlight_style(STYLE_SELECTED);

        let mut state = ListState::default();
        state.select(Some(self.selected_mode));
        f.render_stateful_widget(mode_list, area, &mut state);
    }

    fn render_help(&self, f: &mut Frame, area: Rect) {
        let help_text = match self.step {
            MenuStep::LanguageSelection => "↑↓/j/k: 選択 / Select | Enter: 決定 / Confirm | q: 終了 / Quit",
            MenuStep::ModeSelection => "↑↓/j/k: 選択 / Select | Enter: 決定 / Confirm | Esc: 戻る / Back | q: 終了 / Quit",
        };

        let help = Paragraph::new(help_text)
            .style(STYLE_HELP)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help, area);
    }
}

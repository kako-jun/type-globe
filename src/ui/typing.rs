use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Gauge},
    Frame, Terminal,
};
use std::io;
use crate::game::{TypingGame, TypingResult, CharacterStatus};

pub struct TypingUI {
    typing_game: TypingGame,
    finished: bool,
    final_result: Option<TypingResult>,
}

impl TypingUI {
    pub fn new(target_text: String) -> Self {
        Self {
            typing_game: TypingGame::new(target_text),
            finished: false,
            final_result: None,
        }
    }

    pub fn run(&mut self) -> Result<TypingResult, Box<dyn std::error::Error>> {
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

    fn run_app(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<TypingResult, Box<dyn std::error::Error>> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                if self.handle_input(key) {
                    break;
                }
            }
        }

        Ok(self.final_result.clone().unwrap_or_else(|| TypingResult {
            wpm: 0.0,
            accuracy: 0.0,
            total_time: std::time::Duration::new(0, 0),
            total_characters: 0,
            correct_characters: 0,
            errors: 0,
        }))
    }

    fn handle_input(&mut self, key: KeyEvent) -> bool {
        if self.finished {
            match key.code {
                KeyCode::Char('q') | KeyCode::Enter | KeyCode::Esc => return true,
                _ => {}
            }
            return false;
        }

        match key.code {
            KeyCode::Char('q') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    return true;
                }
            }
            KeyCode::Esc => return true,
            
            KeyCode::Char(ch) => {
                self.typing_game.type_character(ch);
                if self.typing_game.is_finished() {
                    if let Some(result) = self.typing_game.get_result() {
                        self.final_result = Some(result);
                        self.finished = true;
                    }
                }
            }
            
            KeyCode::Backspace => {
                self.typing_game.backspace();
            }
            
            _ => {}
        }
        
        false
    }

    fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(4),
                Constraint::Min(8),
                Constraint::Length(4),
                Constraint::Length(3),
            ])
            .split(f.area());

        self.render_title(f, chunks[0]);
        self.render_progress(f, chunks[1]);
        self.render_text(f, chunks[2]);
        self.render_stats(f, chunks[3]);
        self.render_help(f, chunks[4]);
    }

    fn render_title(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let title = if self.finished {
            "TypeGlobe - タイピング完了！"
        } else {
            "TypeGlobe - タイピングモード"
        };
        
        let title_widget = Paragraph::new(title)
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title_widget, area);
    }

    fn render_progress(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let progress = self.typing_game.get_progress();
        
        let gauge = Gauge::default()
            .block(Block::default().title("進捗").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Blue))
            .ratio(progress as f64);
        
        f.render_widget(gauge, area);
    }

    fn render_text(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let target_text = self.typing_game.get_target_text();
        let typed_text = self.typing_game.get_typed_text();
        let character_status = self.typing_game.get_character_status();
        
        let mut spans = Vec::new();
        
        for (i, (ch, status)) in target_text.chars().zip(character_status.iter()).enumerate() {
            let style = match status {
                CharacterStatus::Correct => Style::default().fg(Color::Green),
                CharacterStatus::Incorrect => Style::default().fg(Color::Red).add_modifier(Modifier::UNDERLINED),
                CharacterStatus::Current => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                CharacterStatus::Untyped => Style::default().fg(Color::Gray),
            };
            
            spans.push(Span::styled(ch.to_string(), style));
        }
        
        let text_paragraph = Paragraph::new(Line::from(spans))
            .style(Style::default())
            .alignment(Alignment::Left)
            .block(Block::default().title("テキスト").borders(Borders::ALL))
            .wrap(ratatui::widgets::Wrap { trim: false });
        
        f.render_widget(text_paragraph, area);
    }

    fn render_stats(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let stats_text = if self.finished {
            if let Some(result) = &self.final_result {
                format!(
                    "WPM: {:.1} | 正確性: {:.1}% | 時間: {:.1}s | エラー: {}",
                    result.wpm,
                    result.accuracy,
                    result.total_time.as_secs_f32(),
                    result.errors
                )
            } else {
                "結果を取得中...".to_string()
            }
        } else {
            let current_wpm = self.typing_game.calculate_wpm();
            let current_accuracy = self.typing_game.calculate_accuracy();
            let position = self.typing_game.get_current_position();
            let total = self.typing_game.get_target_text().len();
            
            format!(
                "WPM: {:.1} | 正確性: {:.1}% | 進捗: {}/{}",
                current_wpm,
                current_accuracy,
                position,
                total
            )
        };
        
        let stats_paragraph = Paragraph::new(stats_text)
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center)
            .block(Block::default().title("統計").borders(Borders::ALL));
        f.render_widget(stats_paragraph, area);
    }

    fn render_help(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let help_text = if self.finished {
            "Enter/q: 終了"
        } else {
            "文字入力: タイピング | Backspace: 削除 | Esc/Ctrl+q: 終了"
        };

        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help, area);
    }
}
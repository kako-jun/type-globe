use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Gauge},
    Frame, Terminal,
};
use std::io;
use crate::game::{QuizGame, QuizResult};
use crate::types::{Question, Language};

pub struct QuizUI {
    quiz_game: QuizGame,
    selected_choice: usize,
    current_result: Option<QuizResult>,
    show_result: bool,
}

impl QuizUI {
    pub fn new(questions: Vec<Question>, language: Language) -> Self {
        let mut quiz_game = QuizGame::new(questions, language);
        quiz_game.start();
        
        Self {
            quiz_game,
            selected_choice: 0,
            current_result: None,
            show_result: false,
        }
    }

    pub fn run(&mut self) -> Result<u32, Box<dyn std::error::Error>> {
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

    fn run_app(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<u32, Box<dyn std::error::Error>> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                if self.handle_input(key) {
                    break;
                }
            }
        }

        Ok(self.quiz_game.get_final_score())
    }

    fn handle_input(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q') => return true,
            
            KeyCode::Up => {
                if !self.show_result && self.selected_choice > 0 {
                    self.selected_choice -= 1;
                }
            }
            
            KeyCode::Down => {
                if !self.show_result {
                    if let Some(question) = self.quiz_game.get_current_question() {
                        if self.selected_choice < question.choices.len() - 1 {
                            self.selected_choice += 1;
                        }
                    }
                }
            }
            
            KeyCode::Char('1') => {
                if !self.show_result {
                    self.selected_choice = 0;
                }
            }
            KeyCode::Char('2') => {
                if !self.show_result {
                    self.selected_choice = 1;
                }
            }
            KeyCode::Char('3') => {
                if !self.show_result {
                    self.selected_choice = 2;
                }
            }
            KeyCode::Char('4') => {
                if !self.show_result {
                    self.selected_choice = 3;
                }
            }
            
            KeyCode::Enter | KeyCode::Char(' ') => {
                if self.show_result {
                    if self.quiz_game.is_game_finished() {
                        return true;
                    } else {
                        self.show_result = false;
                        self.current_result = None;
                        self.selected_choice = 0;
                    }
                } else {
                    if let Some(result) = self.quiz_game.answer_question(self.selected_choice) {
                        self.current_result = Some(result);
                        self.show_result = true;
                    }
                }
            }
            
            KeyCode::Char('s') => {
                if !self.show_result {
                    if self.quiz_game.skip_question() {
                        if self.quiz_game.is_game_finished() {
                            return true;
                        }
                        self.selected_choice = 0;
                    }
                }
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
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(4),
            ])
            .split(f.area());

        self.render_title(f, chunks[0]);
        self.render_progress(f, chunks[1]);
        
        if self.show_result {
            self.render_result(f, chunks[2]);
        } else {
            self.render_question(f, chunks[2]);
        }
        
        self.render_help(f, chunks[3]);
    }

    fn render_title(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let title = Paragraph::new("TypeGlobe - クイズモード")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, area);
    }

    fn render_progress(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let (current, total) = self.quiz_game.get_progress();
        let progress_ratio = if total > 0 { current as f64 / total as f64 } else { 0.0 };
        
        let gauge = Gauge::default()
            .block(Block::default().title(format!("進捗: {}/{}", current, total)).borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Blue))
            .ratio(progress_ratio);
        
        f.render_widget(gauge, area);
    }

    fn render_question(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        if let Some(question) = self.quiz_game.get_current_question() {
            let question_text = self.quiz_game.get_question_text(question);
            let choices = self.quiz_game.get_choice_texts(question);
            
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(4),
                ])
                .split(area);

            let question_paragraph = Paragraph::new(question_text)
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Left)
                .block(Block::default().title("問題").borders(Borders::ALL))
                .wrap(ratatui::widgets::Wrap { trim: true });
            f.render_widget(question_paragraph, chunks[0]);

            let choice_items: Vec<ListItem> = choices
                .iter()
                .enumerate()
                .map(|(i, choice)| {
                    let text = format!("{}. {}", i + 1, choice);
                    let style = if i == self.selected_choice {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    ListItem::new(Line::from(Span::styled(text, style)))
                })
                .collect();

            let choices_list = List::new(choice_items)
                .block(Block::default().title("選択肢").borders(Borders::ALL))
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

            let mut state = ListState::default();
            state.select(Some(self.selected_choice));
            f.render_stateful_widget(choices_list, chunks[1], &mut state);
        } else {
            let no_question = Paragraph::new("問題がありません")
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(no_question, area);
        }
    }

    fn render_result(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        if let (Some(result), Some(question)) = (&self.current_result, self.quiz_game.get_current_question()) {
            let choices = self.quiz_game.get_choice_texts(question);
            
            let result_text = if result.is_correct {
                "正解！".to_string()
            } else {
                format!("不正解。正解は: {}", choices.get(result.correct_answer_index).unwrap_or(&"不明".to_string()))
            };
            
            let result_color = if result.is_correct { Color::Green } else { Color::Red };
            
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(2),
                ])
                .split(area);

            let result_paragraph = Paragraph::new(result_text)
                .style(Style::default().fg(result_color).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center)
                .block(Block::default().title("結果").borders(Borders::ALL));
            f.render_widget(result_paragraph, chunks[0]);

            let stats_text = format!(
                "現在のスコア: {} | 正答率: {:.1}%",
                self.quiz_game.get_final_score(),
                self.quiz_game.get_accuracy() * 100.0
            );
            
            let stats_paragraph = Paragraph::new(stats_text)
                .style(Style::default().fg(Color::Cyan))
                .alignment(Alignment::Center)
                .block(Block::default().title("統計").borders(Borders::ALL));
            f.render_widget(stats_paragraph, chunks[1]);

            let continue_text = if self.quiz_game.is_game_finished() {
                "Enterキーでゲーム終了"
            } else {
                "Enterキーで次の問題へ"
            };
            
            let continue_paragraph = Paragraph::new(continue_text)
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(continue_paragraph, chunks[2]);
        }
    }

    fn render_help(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let help_text = if self.show_result {
            "Enter: 次へ | q: 終了"
        } else {
            "↑↓: 選択 | 1-4: 直接選択 | Enter/Space: 決定 | s: スキップ | q: 終了"
        };

        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help, area);
    }
}
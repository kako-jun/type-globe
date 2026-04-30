use crate::game::{QuizGame, QuizResult};
use crate::types::{Language, Question};
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
    widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::io;

const STYLE_TITLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
const STYLE_SELECTED: Style = Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);
const STYLE_NORMAL: Style = Style::new().fg(Color::White);
const STYLE_HELP: Style = Style::new().fg(Color::Gray);
const STYLE_STATS: Style = Style::new().fg(Color::Cyan);
const STYLE_CORRECT: Style = Style::new().fg(Color::Green).add_modifier(Modifier::BOLD);
const STYLE_INCORRECT: Style = Style::new().fg(Color::Red).add_modifier(Modifier::BOLD);
const STYLE_CONTINUE: Style = Style::new().fg(Color::Yellow);

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

    fn run_app(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                if self.handle_key(key) {
                    break;
                }
            }
        }

        Ok(self.quiz_game.get_final_score())
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q') => return true,

            KeyCode::Up | KeyCode::Char('k') => {
                if !self.show_result {
                    self.selected_choice = self.selected_choice.saturating_sub(1);
                }
            }

            KeyCode::Down | KeyCode::Char('j') => {
                if !self.show_result {
                    if let Some(question) = self.quiz_game.get_current_question() {
                        if self.selected_choice < question.choices.len() - 1 {
                            self.selected_choice += 1;
                        }
                    }
                }
            }

            KeyCode::Char('1') if !self.show_result => self.selected_choice = 0,
            KeyCode::Char('2') if !self.show_result => self.selected_choice = 1,
            KeyCode::Char('3') if !self.show_result => self.selected_choice = 2,
            KeyCode::Char('4') if !self.show_result => self.selected_choice = 3,

            KeyCode::Enter | KeyCode::Char(' ') => {
                if self.show_result {
                    if self.quiz_game.is_game_finished() {
                        return true;
                    }
                    self.show_result = false;
                    self.current_result = None;
                    self.selected_choice = 0;
                } else if let Some(result) = self.quiz_game.answer_question(self.selected_choice) {
                    self.current_result = Some(result);
                    self.show_result = true;
                }
            }

            KeyCode::Char('s') => {
                if !self.show_result && self.quiz_game.skip_question() {
                    if self.quiz_game.is_game_finished() {
                        return true;
                    }
                    self.selected_choice = 0;
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

    fn render_title(&self, f: &mut Frame, area: Rect) {
        let title = Paragraph::new("TypeGlobe - クイズモード")
            .style(STYLE_TITLE)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, area);
    }

    fn render_progress(&self, f: &mut Frame, area: Rect) {
        let (current, total) = self.quiz_game.get_progress();
        let progress_ratio = if total > 0 {
            current as f64 / total as f64
        } else {
            0.0
        };

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .title(format!("進捗: {current}/{total}"))
                    .borders(Borders::ALL),
            )
            .gauge_style(Style::default().fg(Color::Blue))
            .ratio(progress_ratio);

        f.render_widget(gauge, area);
    }

    fn render_question(&self, f: &mut Frame, area: Rect) {
        if let Some(question) = self.quiz_game.get_current_question() {
            let question_text = self.quiz_game.get_question_text(question);
            let choices = self.quiz_game.get_choice_texts(question);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(4)])
                .split(area);

            let question_paragraph = Paragraph::new(question_text)
                .style(STYLE_NORMAL)
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
                        STYLE_SELECTED
                    } else {
                        STYLE_NORMAL
                    };
                    ListItem::new(Line::from(Span::styled(text, style)))
                })
                .collect();

            let choices_list = List::new(choice_items)
                .block(Block::default().title("選択肢").borders(Borders::ALL))
                .highlight_style(STYLE_SELECTED);

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

    fn render_result(&self, f: &mut Frame, area: Rect) {
        let (Some(result), Some(question)) =
            (&self.current_result, self.quiz_game.get_current_question())
        else {
            return;
        };

        let choices = self.quiz_game.get_choice_texts(question);

        let (result_text, result_style) = if result.is_correct {
            ("正解！".to_string(), STYLE_CORRECT)
        } else {
            let correct_text = choices
                .get(result.correct_answer_index)
                .cloned()
                .unwrap_or_else(|| "不明".to_string());
            (format!("不正解。正解は: {correct_text}"), STYLE_INCORRECT)
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(2),
            ])
            .split(area);

        let result_paragraph = Paragraph::new(result_text)
            .style(result_style)
            .alignment(Alignment::Center)
            .block(Block::default().title("結果").borders(Borders::ALL));
        f.render_widget(result_paragraph, chunks[0]);

        let stats_text = format!(
            "現在のスコア: {} | 正答率: {:.1}%",
            self.quiz_game.get_final_score(),
            self.quiz_game.get_accuracy() * 100.0
        );

        let stats_paragraph = Paragraph::new(stats_text)
            .style(STYLE_STATS)
            .alignment(Alignment::Center)
            .block(Block::default().title("統計").borders(Borders::ALL));
        f.render_widget(stats_paragraph, chunks[1]);

        let continue_text = if self.quiz_game.is_game_finished() {
            "Enterキーでゲーム終了"
        } else {
            "Enterキーで次の問題へ"
        };

        let continue_paragraph = Paragraph::new(continue_text)
            .style(STYLE_CONTINUE)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(continue_paragraph, chunks[2]);
    }

    fn render_help(&self, f: &mut Frame, area: Rect) {
        let help_text = if self.show_result {
            "Enter: 次へ | q: 終了"
        } else {
            "↑↓/j/k: 選択 | 1-4: 直接選択 | Enter/Space: 決定 | s: スキップ | q: 終了"
        };

        let help = Paragraph::new(help_text)
            .style(STYLE_HELP)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help, area);
    }
}

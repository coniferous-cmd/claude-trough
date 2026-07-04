use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use rusqlite::Connection;
use std::io::{self, Stdout};

use crate::db;
use crate::models::Task;

pub fn run(conn: &Connection) -> Result<()> {
    // Register a panic hook that restores the terminal before unwinding
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, conn);

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>, conn: &Connection) -> Result<()> {
    let mut tasks = db::list_tasks(conn)?;
    let mut selected_index: usize = 0;

    loop {
        terminal.draw(|f| ui(f, &tasks, selected_index))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('j') | KeyCode::Down => {
                    if !tasks.is_empty() {
                        selected_index = (selected_index + 1).min(tasks.len() - 1);
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    selected_index = selected_index.saturating_sub(1);
                }
                KeyCode::Char(' ') => {
                    if selected_index < tasks.len() {
                        db::toggle_task(conn, tasks[selected_index].id)?;
                        tasks = db::list_tasks(conn)?;
                        if !tasks.is_empty() && selected_index >= tasks.len() {
                            selected_index = tasks.len() - 1;
                        }
                        if tasks.is_empty() {
                            selected_index = 0;
                        }
                    }
                }
                KeyCode::Enter => {
                    if selected_index < tasks.len() {
                        let task = db::get_task(conn, tasks[selected_index].id)?;
                        let new_detail = crate::editor::edit(&task.detail)?;
                        db::update_detail(conn, task.id, &new_detail)?;
                        tasks = db::list_tasks(conn)?;
                        if !tasks.is_empty() && selected_index >= tasks.len() {
                            selected_index = tasks.len() - 1;
                        }
                        if tasks.is_empty() {
                            selected_index = 0;
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn ui(f: &mut Frame, tasks: &[Task], selected_index: usize) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Title bar
    let title = Paragraph::new("Todo List")
        .style(Style::default().add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Task list area
    if tasks.is_empty() {
        let empty = Paragraph::new("No tasks yet")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Tasks"));
        f.render_widget(empty, chunks[1]);
    } else {
        let items: Vec<ListItem> = tasks
            .iter()
            .map(|task| {
                let status = if task.done { "x" } else { " " };
                let style = if task.done {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::CROSSED_OUT)
                } else {
                    Style::default()
                };
                let display = if task.priority > 0 {
                    format!(
                        "[{}] P{} {}. {}",
                        status, task.priority, task.id, task.title
                    )
                } else {
                    format!("[{}] {}. {}", status, task.id, task.title)
                };
                ListItem::new(display).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Tasks"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");
        let mut state = ListState::default();
        state.select(Some(selected_index));
        f.render_stateful_widget(list, chunks[1], &mut state);
    }

    // Key hints at bottom
    let hints = Paragraph::new("j/k: move | Space: toggle | Enter: edit detail | q: quit")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(hints, chunks[2]);
}

use anyhow::Result;
use clap::{Parser, Subcommand};
use rusqlite::Connection;

use crate::db;

#[derive(Parser)]
#[command(name = "trough")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Add a new task (to top of list)
    Add {
        /// Task title
        title: String,
        /// Priority (0-3, higher = more important)
        #[arg(short, long, default_value_t = 0)]
        priority: i64,
    },
    /// Push a new task (to bottom of list)
    Push {
        /// Task title
        title: String,
        /// Task detail
        #[arg(short, long, default_value = "")]
        detail: String,
        /// Priority (0-3, higher = more important)
        #[arg(short, long, default_value_t = 0)]
        priority: i64,
    },
    /// List all tasks
    List,
    /// Show the first task by list order
    First,
    /// Show and complete the next incomplete task by list order
    Next,
    /// Mark a task as done
    Done {
        /// Task ID
        id: i64,
    },
    /// Mark a task as not done
    Undo {
        /// Task ID
        id: i64,
    },
    /// Delete a task
    Delete {
        /// Task ID
        id: i64,
    },
    /// Delete all tasks
    Clear,
    /// Edit task detail with $EDITOR
    Edit {
        /// Task ID
        id: i64,
    },
}

pub fn run(conn: &Connection) -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Add { title, priority } => {
            let task = db::add_task(conn, &title, priority)?;
            println!("Added task #{}: {}", task.id, task.title);
        }
        Command::Push {
            title,
            detail,
            priority,
        } => {
            db::push_task(conn, &title, &detail, priority)?;
        }
        Command::List => {
            let tasks = db::list_tasks(conn)?;
            for task in &tasks {
                print_task_line(task);
            }
        }
        Command::First => match db::first_task(conn)? {
            Some(task) => {
                print_task_line(&task);
                if !task.detail.is_empty() {
                    println!("{}", task.detail);
                }
            }
            None => {}
        },
        Command::Next => match db::next_task(conn)? {
            Some(task) => {
                print_task_line(&task);
                if !task.detail.is_empty() {
                    println!("{}", task.detail);
                }
            }
            None => {}
        },
        Command::Done { id } => {
            db::toggle_task(conn, id)?;
            println!("Marked task #{} as done", id);
        }
        Command::Undo { id } => {
            db::toggle_task(conn, id)?;
            println!("Marked task #{} as not done", id);
        }
        Command::Delete { id } => {
            db::delete_task(conn, id)?;
            println!("Deleted task #{}", id);
        }
        Command::Clear => {
            let count = db::clear_tasks(conn)?;
            println!("Cleared {} task(s)", count);
        }
        Command::Edit { id } => {
            let task = db::get_task(conn, id)?;
            let new_detail = crate::editor::edit(&task.detail)?;
            db::update_detail(conn, id, &new_detail)?;
            println!("Updated detail for task #{}", id);
        }
    }

    Ok(())
}

fn print_task_line(task: &crate::models::Task) {
    println!("{}", format_task_line(task));
}

fn format_task_line(task: &crate::models::Task) -> String {
    let status = if task.done { "✅" } else { "❌" };
    if task.priority > 0 {
        format!("{} P{} {}", status, task.priority, task.title)
    } else {
        format!("{} {}", status, task.title)
    }
}

#[cfg(test)]
mod tests {
    use crate::models::Task;

    use super::format_task_line;

    #[test]
    fn test_format_task_line_with_icon() {
        let task = Task::new("write docs");

        let line = format_task_line(&task);

        assert_eq!(line, "❌ write docs");
    }

    #[test]
    fn test_format_task_line_with_done_icon_and_priority() {
        let mut task = Task::new("write docs");
        task.done = true;
        task.priority = 2;

        let line = format_task_line(&task);

        assert_eq!(line, "✅ P2 write docs");
    }
}

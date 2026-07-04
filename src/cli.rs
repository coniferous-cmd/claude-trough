use anyhow::Result;
use clap::{Parser, Subcommand};
use rusqlite::Connection;

use crate::db;

#[derive(Parser)]
#[command(name = "todo")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Add a new task
    Add {
        /// Task title
        title: String,
        /// Priority (0-3, higher = more important)
        #[arg(short, long, default_value_t = 0)]
        priority: i64,
    },
    /// List all tasks
    List,
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
        Command::List => {
            let tasks = db::list_tasks(conn)?;
            if tasks.is_empty() {
                println!("No tasks yet");
            } else {
                for task in &tasks {
                    let status = if task.done { "x" } else { " " };
                    if task.priority > 0 {
                        println!(
                            "[{}] P{} {}. {}",
                            status, task.priority, task.id, task.title
                        );
                    } else {
                        println!("[{}] {}. {}", status, task.id, task.title);
                    }
                }
            }
        }
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
        Command::Edit { id } => {
            let task = db::get_task(conn, id)?;
            let new_detail = crate::editor::edit(&task.detail)?;
            db::update_detail(conn, id, &new_detail)?;
            println!("Updated detail for task #{}", id);
        }
    }

    Ok(())
}

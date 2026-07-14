use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use rusqlite::Connection;
use std::io::{self, Write};

use crate::db;
use crate::models::ShowFilter;

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
    /// List tasks (defaults to incomplete only)
    List {
        /// Which tasks to show: incomplete (default), completed, or all
        #[arg(short, long, value_enum, default_value_t = ShowFilter::Incomplete)]
        show: ShowFilter,
    },
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
    let stdout = io::stdout();
    let mut out = stdout.lock();
    dispatch_to(conn, cli.command, &mut out)
}

fn dispatch_to<W: Write>(conn: &Connection, command: Command, out: &mut W) -> Result<()> {
    match command {
        Command::Add { title, priority } => {
            let task = db::add_task(conn, &title, priority)?;
            writeln!(out, "Added task #{}: {}", task.id, task.title)?;
        }
        Command::Push {
            title,
            detail,
            priority,
        } => {
            let cwd = std::env::current_dir().context("failed to determine current directory")?;
            let canonical =
                std::fs::canonicalize(&cwd).context("failed to canonicalize current directory")?;
            db::push_task(conn, &title, &detail, priority, Some(&canonical))?;
        }
        Command::List { show } => {
            let tasks = db::list_tasks(conn, show)?;
            for task in &tasks {
                writeln!(out, "{}", format_task_line(task))?;
            }
        }
        Command::First => {
            if let Some(task) = db::first_task(conn)? {
                writeln!(out, "{}", format_task_line(&task))?;
                if !task.detail.is_empty() {
                    writeln!(out, "{}", task.detail)?;
                }
            }
        }
        Command::Next => {
            let cwd = std::env::current_dir().context("failed to determine current directory")?;
            let canonical =
                std::fs::canonicalize(&cwd).context("failed to canonicalize current directory")?;
            if let Some(task) = db::next_task(conn, &canonical)? {
                writeln!(out, "{}", format_task_line(&task))?;
                if !task.detail.is_empty() {
                    writeln!(out, "{}", task.detail)?;
                }
            }
        }
        Command::Done { id } => {
            db::toggle_task(conn, id)?;
            writeln!(out, "Marked task #{} as done", id)?;
        }
        Command::Undo { id } => {
            db::toggle_task(conn, id)?;
            writeln!(out, "Marked task #{} as not done", id)?;
        }
        Command::Delete { id } => {
            db::delete_task(conn, id)?;
            writeln!(out, "Deleted task #{}", id)?;
        }
        Command::Clear => {
            let count = db::clear_tasks(conn)?;
            writeln!(out, "Cleared {} task(s)", count)?;
        }
        Command::Edit { id } => {
            let task = db::get_task(conn, id)?;
            let new_detail = crate::editor::edit(&task.detail)?;
            db::update_detail(conn, id, &new_detail)?;
            writeln!(out, "Updated detail for task #{}", id)?;
        }
    }

    Ok(())
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

    use super::{Command, format_task_line};
    use crate::db;
    use clap::Parser;
    use rusqlite::Connection;
    use std::io::Write;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE project (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE task (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id INTEGER REFERENCES project(id),
                title TEXT NOT NULL,
                done INTEGER NOT NULL DEFAULT 0,
                detail TEXT NOT NULL DEFAULT '',
                priority INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                deleted_at INTEGER
            );",
        )
        .unwrap();
        conn
    }

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

    #[test]
    fn test_list_default_hides_completed() {
        let conn = test_conn();
        let done = db::add_task(&conn, "done task", 0).unwrap();
        db::toggle_task(&conn, done.id).unwrap();
        let open = db::add_task(&conn, "open task", 0).unwrap();

        let mut buf: Vec<u8> = Vec::new();
        super::dispatch_to(
            &conn,
            Command::List {
                show: crate::models::ShowFilter::Incomplete,
            },
            &mut buf,
        )
        .unwrap();

        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("open task"), "open task should appear: {out}");
        assert!(
            !out.contains("done task"),
            "done task should be hidden: {out}"
        );
        assert!(open.id > 0);
    }

    #[test]
    fn test_list_completed_hides_incomplete() {
        let conn = test_conn();
        let done = db::add_task(&conn, "done task", 0).unwrap();
        db::toggle_task(&conn, done.id).unwrap();
        db::add_task(&conn, "open task", 0).unwrap();

        let mut buf: Vec<u8> = Vec::new();
        super::dispatch_to(
            &conn,
            Command::List {
                show: crate::models::ShowFilter::Completed,
            },
            &mut buf,
        )
        .unwrap();

        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("done task"), "done task should appear: {out}");
        assert!(
            !out.contains("open task"),
            "open task should be hidden: {out}"
        );
    }

    #[test]
    fn test_list_all_shows_both() {
        let conn = test_conn();
        let done = db::add_task(&conn, "done task", 0).unwrap();
        db::toggle_task(&conn, done.id).unwrap();
        db::add_task(&conn, "open task", 0).unwrap();

        let mut buf: Vec<u8> = Vec::new();
        super::dispatch_to(
            &conn,
            Command::List {
                show: crate::models::ShowFilter::All,
            },
            &mut buf,
        )
        .unwrap();

        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("done task"), "done task should appear: {out}");
        assert!(out.contains("open task"), "open task should appear: {out}");
    }

    #[test]
    fn test_list_empty_view_produces_no_output() {
        let conn = test_conn();
        // only done tasks; default filter hides them
        let done = db::add_task(&conn, "done task", 0).unwrap();
        db::toggle_task(&conn, done.id).unwrap();

        let mut buf: Vec<u8> = Vec::new();
        super::dispatch_to(
            &conn,
            Command::List {
                show: crate::models::ShowFilter::Incomplete,
            },
            &mut buf,
        )
        .unwrap();

        assert!(
            buf.is_empty(),
            "expected no output, got: {:?}",
            String::from_utf8_lossy(&buf)
        );
        // suppress unused warning
        let _ = done.id;
    }

    #[test]
    fn test_next_uses_canonical_current_project_path() {
        let conn = test_conn();
        let cwd = std::fs::canonicalize(std::env::current_dir().unwrap()).unwrap();
        let current = db::push_task(&conn, "current project", "", 0, Some(&cwd)).unwrap();
        let other = db::push_task(
            &conn,
            "other project",
            "",
            3,
            Some(std::path::Path::new("/tmp/cli-next-other-project")),
        )
        .unwrap();
        let mut buf = Vec::new();

        super::dispatch_to(&conn, Command::Next, &mut buf).unwrap();

        let out = String::from_utf8(buf).unwrap();
        assert!(
            out.contains("current project"),
            "current task missing: {out}"
        );
        assert!(!out.contains("other project"), "other task leaked: {out}");
        assert!(db::get_task(&conn, current.id).unwrap().done);
        assert!(!db::get_task(&conn, other.id).unwrap().done);
    }

    #[test]
    fn test_clap_list_default_is_incomplete() {
        let cli = super::Cli::try_parse_from(["trough", "list"]).unwrap();
        match cli.command {
            Command::List { show } => assert_eq!(show, crate::models::ShowFilter::Incomplete),
            _ => panic!("expected List command"),
        }
    }

    #[test]
    fn test_clap_list_long_flag() {
        let cli = super::Cli::try_parse_from(["trough", "list", "--show", "completed"]).unwrap();
        match cli.command {
            Command::List { show } => assert_eq!(show, crate::models::ShowFilter::Completed),
            _ => panic!("expected List command"),
        }
    }

    #[test]
    fn test_clap_list_short_flag() {
        let cli = super::Cli::try_parse_from(["trough", "list", "-s", "all"]).unwrap();
        match cli.command {
            Command::List { show } => assert_eq!(show, crate::models::ShowFilter::All),
            _ => panic!("expected List command"),
        }
    }

    #[test]
    fn test_clap_list_invalid_value_rejected() {
        let result = super::Cli::try_parse_from(["trough", "list", "--show", "bogus"]);
        assert!(result.is_err());
    }

    // silence unused Write import if all tests compile
    #[allow(dead_code)]
    fn _assert_write<T: Write>(_: T) {}
}

use std::path::PathBuf;

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::models::Task;

fn db_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("cannot determine home directory")?;
    let dir = home.join(".todo");
    std::fs::create_dir_all(&dir).context("failed to create ~/.todo directory")?;
    Ok(dir.join("todo.db"))
}

pub fn init() -> Result<Connection> {
    let path = db_path()?;
    let conn = Connection::open(&path).context("failed to open database")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS task (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            done INTEGER NOT NULL DEFAULT 0,
            detail TEXT NOT NULL DEFAULT '',
            priority INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )
    .context("failed to create task table")?;
    migrate_schema(&conn)?;
    Ok(conn)
}

pub fn migrate_schema(conn: &Connection) -> Result<()> {
    let has_priority: bool = conn
        .prepare("PRAGMA table_info(task)")
        .context("failed to prepare pragma")?
        .query_map([], |row| row.get::<_, String>(1))
        .context("failed to query table info")?
        .filter_map(|r| r.ok())
        .any(|name| name == "priority");

    if !has_priority {
        conn.execute(
            "ALTER TABLE task ADD COLUMN priority INTEGER NOT NULL DEFAULT 0",
            [],
        )
        .context("failed to add priority column")?;
    }
    Ok(())
}

fn row_to_task(row: &rusqlite::Row) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get("id")?,
        title: row.get("title")?,
        done: row.get::<_, i32>("done")? != 0,
        detail: row.get("detail")?,
        priority: row.get("priority")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

pub fn add_task(conn: &Connection, title: &str, priority: i64) -> Result<Task> {
    let mut task = Task::new(title);
    task.priority = priority;
    conn.execute(
        "INSERT INTO task (title, detail, priority, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![task.title, task.detail, task.priority, task.created_at, task.updated_at],
    )
    .context("failed to insert task")?;
    let id = conn.last_insert_rowid();
    get_task(conn, id)
}

pub fn push_task(conn: &Connection, title: &str, priority: i64) -> Result<Task> {
    let mut task = Task::new(title);
    task.priority = priority;
    task.created_at = 0; // push to bottom via old timestamp
    task.updated_at = 0;
    conn.execute(
        "INSERT INTO task (title, detail, priority, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![task.title, task.detail, task.priority, task.created_at, task.updated_at],
    )
    .context("failed to insert task")?;
    let id = conn.last_insert_rowid();
    get_task(conn, id)
}

pub fn list_tasks(conn: &Connection) -> Result<Vec<Task>> {
    let mut stmt = conn
        .prepare("SELECT id, title, done, detail, priority, created_at, updated_at FROM task ORDER BY priority DESC, created_at DESC")
        .context("failed to prepare list query")?;
    let rows = stmt
        .query_map([], row_to_task)
        .context("failed to query tasks")?;
    let mut tasks = Vec::new();
    for row in rows {
        tasks.push(row.context("failed to read task row")?);
    }
    Ok(tasks)
}

pub fn toggle_task(conn: &Connection, id: i64) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .context("system clock is before unix epoch")?
        .as_secs() as i64;
    let updated = conn
        .execute(
            "UPDATE task SET done = CASE WHEN done = 0 THEN 1 ELSE 0 END, updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, id],
        )
        .context("failed to toggle task")?;
    if updated == 0 {
        anyhow::bail!("task with id {} not found", id);
    }
    Ok(())
}

pub fn delete_task(conn: &Connection, id: i64) -> Result<()> {
    let deleted = conn
        .execute("DELETE FROM task WHERE id = ?1", rusqlite::params![id])
        .context("failed to delete task")?;
    if deleted == 0 {
        anyhow::bail!("task with id {} not found", id);
    }
    Ok(())
}

pub fn update_detail(conn: &Connection, id: i64, detail: &str) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .context("system clock is before unix epoch")?
        .as_secs() as i64;
    let updated = conn
        .execute(
            "UPDATE task SET detail = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![detail, now, id],
        )
        .context("failed to update detail")?;
    if updated == 0 {
        anyhow::bail!("task with id {} not found", id);
    }
    Ok(())
}

pub fn get_task(conn: &Connection, id: i64) -> Result<Task> {
    conn.query_row(
        "SELECT id, title, done, detail, priority, created_at, updated_at FROM task WHERE id = ?1",
        rusqlite::params![id],
        row_to_task,
    )
    .with_context(|| format!("task with id {} not found", id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE task (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                done INTEGER NOT NULL DEFAULT 0,
                detail TEXT NOT NULL DEFAULT '',
                priority INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_add_and_get_task() {
        let conn = test_conn();
        let task = add_task(&conn, "buy milk", 0).unwrap();
        assert_eq!(task.title, "buy milk");
        assert!(!task.done);
        assert_eq!(task.detail, "");
        assert!(task.id > 0);

        let fetched = get_task(&conn, task.id).unwrap();
        assert_eq!(fetched.title, "buy milk");
    }

    #[test]
    fn test_add_multiple_tasks() {
        let conn = test_conn();
        add_task(&conn, "first", 0).unwrap();
        add_task(&conn, "second", 0).unwrap();
        add_task(&conn, "third", 0).unwrap();
        let tasks = list_tasks(&conn).unwrap();
        assert_eq!(tasks.len(), 3);
    }

    #[test]
    fn test_list_tasks_order() {
        let conn = test_conn();
        add_task(&conn, "oldest", 0).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
        add_task(&conn, "newest", 0).unwrap();
        let tasks = list_tasks(&conn).unwrap();
        // newest first (created_at DESC)
        assert_eq!(tasks[0].title, "newest");
        assert_eq!(tasks[1].title, "oldest");
    }

    #[test]
    fn test_toggle_task() {
        let conn = test_conn();
        let task = add_task(&conn, "toggle me", 0).unwrap();
        assert!(!task.done);

        toggle_task(&conn, task.id).unwrap();
        let toggled = get_task(&conn, task.id).unwrap();
        assert!(toggled.done);

        toggle_task(&conn, task.id).unwrap();
        let untoggled = get_task(&conn, task.id).unwrap();
        assert!(!untoggled.done);
    }

    #[test]
    fn test_toggle_nonexistent() {
        let conn = test_conn();
        let result = toggle_task(&conn, 999);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_delete_task() {
        let conn = test_conn();
        let task = add_task(&conn, "delete me", 0).unwrap();
        assert_eq!(list_tasks(&conn).unwrap().len(), 1);

        delete_task(&conn, task.id).unwrap();
        assert_eq!(list_tasks(&conn).unwrap().len(), 0);
    }

    #[test]
    fn test_delete_nonexistent() {
        let conn = test_conn();
        let result = delete_task(&conn, 999);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_update_detail() {
        let conn = test_conn();
        let task = add_task(&conn, "write docs", 0).unwrap();
        assert_eq!(task.detail, "");

        update_detail(&conn, task.id, "use markdown").unwrap();
        let updated = get_task(&conn, task.id).unwrap();
        assert_eq!(updated.detail, "use markdown");
    }

    #[test]
    fn test_update_detail_nonexistent() {
        let conn = test_conn();
        let result = update_detail(&conn, 999, "nope");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_get_nonexistent() {
        let conn = test_conn();
        let result = get_task(&conn, 999);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_list() {
        let conn = test_conn();
        let tasks = list_tasks(&conn).unwrap();
        assert!(tasks.is_empty());
    }
}

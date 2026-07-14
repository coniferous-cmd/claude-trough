use std::path::PathBuf;

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension};

use crate::models::{ShowFilter, Task};

fn db_path() -> Result<PathBuf> {
    let config = dirs::config_dir().context("cannot determine config directory")?;
    let dir = config.join("trough");
    std::fs::create_dir_all(&dir).context("failed to create config directory for trough")?;
    Ok(dir.join("trough.db"))
}

fn old_db_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("cannot determine home directory")?;
    Ok(home.join(".todo").join("todo.db"))
}

pub fn init() -> Result<Connection> {
    let path = db_path()?;

    // Migrate data from old ~/.todo/todo.db (todo-cli) if it exists and new path does not
    let old_path = old_db_path()?;
    if old_path.exists() && !path.exists() {
        std::fs::copy(&old_path, &path)
            .context("failed to migrate database from old ~/.todo/todo.db")?;
        println!(
            "Migrated database from {} to {}",
            old_path.display(),
            path.display()
        );
    }

    let conn = Connection::open(&path).context("failed to open database")?;
    conn.execute("PRAGMA foreign_keys = ON", [])
        .context("failed to enable foreign keys")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS task (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            done INTEGER NOT NULL DEFAULT 0,
            detail TEXT NOT NULL DEFAULT '',
            priority INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            deleted_at INTEGER
        )",
        [],
    )
    .context("failed to create task table")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS project (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL UNIQUE,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )
    .context("failed to create project table")?;
    migrate_schema(&conn)?;
    Ok(conn)
}

pub fn migrate_schema(conn: &Connection) -> Result<()> {
    let columns: Vec<String> = conn
        .prepare("PRAGMA table_info(task)")
        .context("failed to prepare pragma")?
        .query_map([], |row| row.get::<_, String>(1))
        .context("failed to query table info")?
        .filter_map(|r| r.ok())
        .collect();

    if !columns.iter().any(|name| name == "priority") {
        conn.execute(
            "ALTER TABLE task ADD COLUMN priority INTEGER NOT NULL DEFAULT 0",
            [],
        )
        .context("failed to add priority column")?;
    }
    if !columns.iter().any(|name| name == "deleted_at") {
        conn.execute("ALTER TABLE task ADD COLUMN deleted_at INTEGER", [])
            .context("failed to add deleted_at column")?;
    }
    if !columns.iter().any(|name| name == "project_id") {
        conn.execute(
            "ALTER TABLE task ADD COLUMN project_id INTEGER REFERENCES project(id)",
            [],
        )
        .context("failed to add project_id column")?;
    }
    conn.execute(
        "CREATE INDEX IF NOT EXISTS task_project_id_idx ON task(project_id)",
        [],
    )
    .context("failed to create project_id index")?;
    Ok(())
}

fn unix_now() -> Result<i64> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .context("system clock is before unix epoch")?
        .as_secs() as i64)
}

fn row_to_task(row: &rusqlite::Row) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get("id")?,
        project_id: row.get("project_id")?,
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

pub fn push_task(
    conn: &Connection,
    title: &str,
    detail: &str,
    priority: i64,
    project_path: Option<&std::path::Path>,
) -> Result<Task> {
    let mut task = Task::new(title);
    task.detail = detail.to_string();
    task.priority = priority;
    task.created_at = 0; // push to bottom via old timestamp
    task.updated_at = 0;

    let tx = conn.unchecked_transaction()?;

    let project_id: Option<i64> = match project_path {
        Some(path) => {
            let path_str = path
                .to_str()
                .context("project path contains invalid UTF-8")?;
            let now = unix_now()?;
            tx.execute(
                "INSERT INTO project (path, created_at, updated_at) VALUES (?1, ?2, ?3) \
                 ON CONFLICT(path) DO UPDATE SET updated_at = ?3",
                rusqlite::params![path_str, now, now],
            )
            .context("failed to upsert project")?;
            let id: i64 = tx
                .query_row(
                    "SELECT id FROM project WHERE path = ?1",
                    rusqlite::params![path_str],
                    |row| row.get(0),
                )
                .context("failed to retrieve project id")?;
            Some(id)
        }
        None => None,
    };

    tx.execute(
        "INSERT INTO task (project_id, title, detail, priority, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            project_id,
            task.title,
            task.detail,
            task.priority,
            task.created_at,
            task.updated_at
        ],
    )
    .context("failed to insert task")?;

    tx.commit()?;

    let id = conn.last_insert_rowid();
    get_task(conn, id)
}

pub fn list_tasks(conn: &Connection, filter: ShowFilter) -> Result<Vec<Task>> {
    let sql = match filter {
        ShowFilter::Incomplete => {
            "SELECT id, project_id, title, done, detail, priority, created_at, updated_at FROM task \
             WHERE deleted_at IS NULL AND done = 0 \
             ORDER BY priority DESC, created_at DESC"
        }
        ShowFilter::Completed => {
            "SELECT id, project_id, title, done, detail, priority, created_at, updated_at FROM task \
             WHERE deleted_at IS NULL AND done = 1 \
             ORDER BY priority DESC, created_at DESC"
        }
        ShowFilter::All => {
            "SELECT id, project_id, title, done, detail, priority, created_at, updated_at FROM task \
             WHERE deleted_at IS NULL \
             ORDER BY priority DESC, created_at DESC"
        }
    };
    let mut stmt = conn.prepare(sql).context("failed to prepare list query")?;
    let rows = stmt
        .query_map([], row_to_task)
        .context("failed to query tasks")?;
    let mut tasks = Vec::new();
    for row in rows {
        tasks.push(row.context("failed to read task row")?);
    }
    Ok(tasks)
}

pub fn first_task(conn: &Connection) -> Result<Option<Task>> {
    conn.query_row(
        "SELECT id, project_id, title, done, detail, priority, created_at, updated_at FROM task WHERE deleted_at IS NULL ORDER BY priority DESC, created_at DESC LIMIT 1",
        [],
        row_to_task,
    )
    .optional()
    .context("failed to query first task")
}

pub fn next_task(conn: &Connection, project_path: &std::path::Path) -> Result<Option<Task>> {
    let project_path = project_path
        .to_str()
        .context("project path contains invalid UTF-8")?;
    let Some(task) = conn
        .query_row(
            "SELECT task.id, task.project_id, task.title, task.done, task.detail, \
                    task.priority, task.created_at, task.updated_at \
             FROM task \
             JOIN project ON project.id = task.project_id \
             WHERE project.path = ?1 AND task.deleted_at IS NULL AND task.done = 0 \
             ORDER BY task.priority DESC, task.created_at DESC \
             LIMIT 1",
            rusqlite::params![project_path],
            row_to_task,
        )
        .optional()
        .context("failed to query next task")?
    else {
        return Ok(None);
    };

    let now = unix_now()?;
    conn.execute(
        "UPDATE task SET done = 1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
        rusqlite::params![now, task.id],
    )
    .context("failed to mark next task as done")?;

    get_task(conn, task.id).map(Some)
}

pub fn toggle_task(conn: &Connection, id: i64) -> Result<()> {
    let now = unix_now()?;
    let updated = conn
        .execute(
            "UPDATE task SET done = CASE WHEN done = 0 THEN 1 ELSE 0 END, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            rusqlite::params![now, id],
        )
        .context("failed to toggle task")?;
    if updated == 0 {
        anyhow::bail!("task with id {} not found", id);
    }
    Ok(())
}

pub fn delete_task(conn: &Connection, id: i64) -> Result<()> {
    let now = unix_now()?;
    let deleted = conn
        .execute(
            "UPDATE task SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            rusqlite::params![now, id],
        )
        .context("failed to delete task")?;
    if deleted == 0 {
        anyhow::bail!("task with id {} not found", id);
    }
    Ok(())
}

pub fn clear_tasks(conn: &Connection) -> Result<usize> {
    let now = unix_now()?;
    conn.execute(
        "UPDATE task SET deleted_at = ?1, updated_at = ?1 WHERE deleted_at IS NULL",
        rusqlite::params![now],
    )
    .context("failed to clear tasks")
}

pub fn update_detail(conn: &Connection, id: i64, detail: &str) -> Result<()> {
    let now = unix_now()?;
    let updated = conn
        .execute(
            "UPDATE task SET detail = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL",
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
        "SELECT id, project_id, title, done, detail, priority, created_at, updated_at FROM task WHERE id = ?1 AND deleted_at IS NULL",
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

    fn legacy_test_conn() -> Connection {
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

    fn task_row_count(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM task", [], |row| row.get(0))
            .unwrap()
    }

    fn has_column(conn: &Connection, column: &str) -> bool {
        conn.prepare("PRAGMA table_info(task)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .any(|name| name == column)
    }

    #[test]
    fn test_migrate_schema_adds_deleted_at() {
        let conn = legacy_test_conn();
        assert!(!has_column(&conn, "deleted_at"));

        migrate_schema(&conn).unwrap();

        assert!(has_column(&conn, "deleted_at"));
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
        let tasks = list_tasks(&conn, ShowFilter::All).unwrap();
        assert_eq!(tasks.len(), 3);
    }

    #[test]
    fn test_push_task_with_detail() {
        let conn = test_conn();
        let task = push_task(&conn, "write docs", "include usage examples", 0, None).unwrap();

        assert_eq!(task.title, "write docs");
        assert_eq!(task.detail, "include usage examples");
    }

    #[test]
    fn test_first_task_uses_list_order() {
        let conn = test_conn();
        add_task(&conn, "low priority", 0).unwrap();
        add_task(&conn, "high priority", 2).unwrap();

        let task = first_task(&conn).unwrap().unwrap();
        assert_eq!(task.title, "high priority");
    }

    #[test]
    fn test_first_task_empty() {
        let conn = test_conn();
        let task = first_task(&conn).unwrap();
        assert!(task.is_none());
    }

    #[test]
    fn test_next_task_marks_first_incomplete_done() {
        let conn = test_conn();
        let path = std::path::Path::new("/tmp/next-current-project");
        push_task(&conn, "low priority", "", 0, Some(path)).unwrap();
        let high = push_task(&conn, "high priority", "", 2, Some(path)).unwrap();

        let task = next_task(&conn, path).unwrap().unwrap();

        assert_eq!(task.id, high.id);
        assert!(task.done);
        assert!(get_task(&conn, high.id).unwrap().done);
    }

    #[test]
    fn test_next_task_skips_completed_tasks() {
        let conn = test_conn();
        let path = std::path::Path::new("/tmp/next-skip-completed");
        let completed = push_task(&conn, "completed high priority", "", 3, Some(path)).unwrap();
        toggle_task(&conn, completed.id).unwrap();
        let incomplete = push_task(&conn, "incomplete low priority", "", 0, Some(path)).unwrap();

        let task = next_task(&conn, path).unwrap().unwrap();

        assert_eq!(task.id, incomplete.id);
        assert!(task.done);
    }

    #[test]
    fn test_next_task_empty_when_no_incomplete_tasks() {
        let conn = test_conn();
        let path = std::path::Path::new("/tmp/next-empty-project");
        let task = push_task(&conn, "done", "", 0, Some(path)).unwrap();
        toggle_task(&conn, task.id).unwrap();

        let next = next_task(&conn, path).unwrap();

        assert!(next.is_none());
    }

    #[test]
    fn test_next_task_is_strictly_scoped_to_project() {
        let conn = test_conn();
        let current_path = std::path::Path::new("/tmp/next-project-a");
        let other_path = std::path::Path::new("/tmp/next-project-b");
        let current = push_task(&conn, "current project", "", 0, Some(current_path)).unwrap();
        let other = push_task(&conn, "other project", "", 3, Some(other_path)).unwrap();
        let unscoped = add_task(&conn, "unscoped", 3).unwrap();

        let task = next_task(&conn, current_path).unwrap().unwrap();

        assert_eq!(task.id, current.id);
        assert!(task.done);
        assert!(!get_task(&conn, other.id).unwrap().done);
        assert!(!get_task(&conn, unscoped.id).unwrap().done);
    }

    #[test]
    fn test_next_task_unknown_project_does_not_create_project_or_fallback() {
        let conn = test_conn();
        let other_path = std::path::Path::new("/tmp/next-existing-project");
        let other = push_task(&conn, "other project", "", 0, Some(other_path)).unwrap();
        let unscoped = add_task(&conn, "unscoped", 0).unwrap();
        let project_count_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM project", [], |row| row.get(0))
            .unwrap();

        let task = next_task(&conn, std::path::Path::new("/tmp/next-unknown-project")).unwrap();

        assert!(task.is_none());
        assert!(!get_task(&conn, other.id).unwrap().done);
        assert!(!get_task(&conn, unscoped.id).unwrap().done);
        let project_count_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM project", [], |row| row.get(0))
            .unwrap();
        assert_eq!(project_count_after, project_count_before);
    }

    #[test]
    fn test_next_task_skips_deleted_task_in_current_project() {
        let conn = test_conn();
        let path = std::path::Path::new("/tmp/next-skip-deleted");
        let deleted = push_task(&conn, "deleted high", "", 3, Some(path)).unwrap();
        delete_task(&conn, deleted.id).unwrap();
        let active = push_task(&conn, "active low", "", 0, Some(path)).unwrap();

        let task = next_task(&conn, path).unwrap().unwrap();

        assert_eq!(task.id, active.id);
        assert!(task.done);
    }

    #[test]
    fn test_list_tasks_order() {
        let conn = test_conn();
        add_task(&conn, "oldest", 0).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
        add_task(&conn, "newest", 0).unwrap();
        let tasks = list_tasks(&conn, ShowFilter::All).unwrap();
        // newest first (created_at DESC)
        assert_eq!(tasks[0].title, "newest");
        assert_eq!(tasks[1].title, "oldest");
    }

    #[test]
    fn test_list_tasks_incomplete_excludes_done() {
        let conn = test_conn();
        let done = add_task(&conn, "done low", 0).unwrap();
        toggle_task(&conn, done.id).unwrap();
        let open = add_task(&conn, "open low", 0).unwrap();

        let tasks = list_tasks(&conn, ShowFilter::Incomplete).unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, open.id);
        assert!(!tasks[0].done);
    }

    #[test]
    fn test_list_tasks_completed_excludes_incomplete() {
        let conn = test_conn();
        let done = add_task(&conn, "done high", 2).unwrap();
        toggle_task(&conn, done.id).unwrap();
        let _open = add_task(&conn, "open high", 2).unwrap();

        let tasks = list_tasks(&conn, ShowFilter::Completed).unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, done.id);
        assert!(tasks[0].done);
    }

    #[test]
    fn test_list_tasks_all_includes_both() {
        let conn = test_conn();
        let done = add_task(&conn, "done", 0).unwrap();
        toggle_task(&conn, done.id).unwrap();
        let open = add_task(&conn, "open", 0).unwrap();

        let tasks = list_tasks(&conn, ShowFilter::All).unwrap();

        assert_eq!(tasks.len(), 2);
        let ids: Vec<i64> = tasks.iter().map(|t| t.id).collect();
        assert!(ids.contains(&done.id));
        assert!(ids.contains(&open.id));
    }

    #[test]
    fn test_list_tasks_incomplete_preserves_order() {
        let conn = test_conn();
        let high = add_task(&conn, "high", 2).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
        let newer_low = add_task(&conn, "newer low", 0).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
        let older_low = add_task(&conn, "older low", 0).unwrap();
        // mark the older one as done so it should NOT appear in incomplete
        toggle_task(&conn, older_low.id).unwrap();

        let tasks = list_tasks(&conn, ShowFilter::Incomplete).unwrap();

        // priority DESC then created_at DESC: high first, then newer low
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].id, high.id);
        assert_eq!(tasks[1].id, newer_low.id);
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
        assert_eq!(list_tasks(&conn, ShowFilter::All).unwrap().len(), 1);

        delete_task(&conn, task.id).unwrap();
        assert_eq!(list_tasks(&conn, ShowFilter::All).unwrap().len(), 0);
        assert_eq!(task_row_count(&conn), 1);
        assert!(get_task(&conn, task.id).is_err());
        assert!(
            conn.query_row(
                "SELECT deleted_at FROM task WHERE id = ?1",
                rusqlite::params![task.id],
                |row| row.get::<_, Option<i64>>(0),
            )
            .unwrap()
            .is_some()
        );
    }

    #[test]
    fn test_delete_nonexistent() {
        let conn = test_conn();
        let result = delete_task(&conn, 999);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_clear_tasks() {
        let conn = test_conn();
        add_task(&conn, "first", 0).unwrap();
        add_task(&conn, "second", 0).unwrap();

        let cleared = clear_tasks(&conn).unwrap();

        assert_eq!(cleared, 2);
        assert!(list_tasks(&conn, ShowFilter::All).unwrap().is_empty());
        assert!(first_task(&conn).unwrap().is_none());
        assert_eq!(task_row_count(&conn), 2);
    }

    #[test]
    fn test_clear_tasks_ignores_already_deleted_tasks() {
        let conn = test_conn();
        let deleted = add_task(&conn, "deleted", 0).unwrap();
        add_task(&conn, "active", 0).unwrap();
        delete_task(&conn, deleted.id).unwrap();

        let cleared = clear_tasks(&conn).unwrap();

        assert_eq!(cleared, 1);
        assert!(list_tasks(&conn, ShowFilter::All).unwrap().is_empty());
        assert_eq!(task_row_count(&conn), 2);
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
    fn test_migrate_schema_adds_project_table_and_column() {
        let conn = legacy_test_conn();
        assert!(!has_column(&conn, "project_id"));

        migrate_schema(&conn).unwrap();

        // Verify project_id column added
        assert!(has_column(&conn, "project_id"));

        // Verify index exists
        let has_index: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='index' AND name='task_project_id_idx'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(has_index, "task_project_id_idx should exist");
    }

    #[test]
    fn test_migrate_schema_on_current_schema_adds_project_id() {
        // Simulate current production schema (has deleted_at, no project_id)
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE task (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
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
        assert!(!has_column(&conn, "project_id"));
        assert!(has_column(&conn, "deleted_at"));

        migrate_schema(&conn).unwrap();

        assert!(has_column(&conn, "project_id"));

        // Add project table so normal API works (normally created by init())
        conn.execute_batch(
            "CREATE TABLE project (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );",
        )
        .unwrap();

        // After migration, normal API works
        let task = add_task(&conn, "post-migration", 0).unwrap();
        assert!(task.project_id.is_none());
        assert_eq!(task.title, "post-migration");
    }

    #[test]
    fn test_migrate_schema_is_idempotent() {
        let conn = legacy_test_conn();
        // Insert directly since add_task/get_task need project_id column
        conn.execute(
            "INSERT INTO task (title, done, detail, priority, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params!["survivor", 0, "", 3, 100, 100],
        )
        .unwrap();

        migrate_schema(&conn).unwrap();
        migrate_schema(&conn).unwrap();

        let task = get_task(&conn, 1).unwrap();
        assert_eq!(task.title, "survivor");
        assert_eq!(task.priority, 3);
        assert!(task.project_id.is_none());
    }

    #[test]
    fn test_migrate_schema_preserves_old_data() {
        let conn = legacy_test_conn();
        // Insert directly, then verify after migration
        conn.execute(
            "INSERT INTO task (title, done, detail, priority, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params!["legacy", 0, "", 5, 200, 200],
        )
        .unwrap();

        migrate_schema(&conn).unwrap();

        let fetched = get_task(&conn, 1).unwrap();
        assert_eq!(fetched.title, "legacy");
        assert_eq!(fetched.priority, 5);
        assert!(fetched.project_id.is_none());
    }

    fn test_conn_with_project() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
            CREATE TABLE project (
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
            );
            CREATE INDEX task_project_id_idx ON task(project_id);",
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_foreign_key_enforced_on_project_id() {
        let conn = test_conn_with_project();
        let result = conn.execute(
            "INSERT INTO task (project_id, title, created_at, updated_at) VALUES (999, 'orphan', 0, 0)",
            [],
        );
        assert!(
            result.is_err(),
            "foreign key should reject invalid project_id"
        );
    }

    #[test]
    fn test_push_creates_project_on_new_path() {
        let conn = test_conn_with_project();
        let path = std::path::Path::new("/tmp/test-project-a");

        let task = push_task(&conn, "task from A", "", 0, Some(path)).unwrap();

        assert_eq!(task.project_id, Some(1));
        assert_eq!(task.title, "task from A");

        let project_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM project", [], |row| row.get(0))
            .unwrap();
        assert_eq!(project_count, 1);
    }

    #[test]
    fn test_push_reuses_project_on_same_path() {
        let conn = test_conn_with_project();
        let path = std::path::Path::new("/tmp/test-project-b");

        push_task(&conn, "first from B", "", 0, Some(path)).unwrap();
        push_task(&conn, "second from B", "", 0, Some(path)).unwrap();

        let project_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM project", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            project_count, 1,
            "should reuse project, not create duplicate"
        );

        let tasks = list_tasks(&conn, ShowFilter::All).unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].project_id, Some(1));
        assert_eq!(tasks[1].project_id, Some(1));
    }

    #[test]
    fn test_push_creates_separate_projects_for_different_paths() {
        let conn = test_conn_with_project();
        let path_a = std::path::Path::new("/tmp/project-a");
        let path_b = std::path::Path::new("/tmp/project-b");

        push_task(&conn, "from A", "", 0, Some(path_a)).unwrap();
        push_task(&conn, "from B", "", 0, Some(path_b)).unwrap();

        let project_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM project", [], |row| row.get(0))
            .unwrap();
        assert_eq!(project_count, 2);
    }

    #[test]
    fn test_push_without_path_has_no_project() {
        let conn = test_conn();

        let task = push_task(&conn, "no project task", "details", 0, None).unwrap();

        assert!(task.project_id.is_none());
        assert_eq!(task.detail, "details");
    }

    #[test]
    fn test_push_preserves_detail_and_priority() {
        let conn = test_conn_with_project();
        let path = std::path::Path::new("/tmp/test-proj");

        let task = push_task(&conn, "important", "critical detail", 3, Some(path)).unwrap();

        assert_eq!(task.detail, "critical detail");
        assert_eq!(task.priority, 3);
    }

    #[test]
    fn test_push_task_at_bottom() {
        let conn = test_conn_with_project();
        let path = std::path::Path::new("/tmp/bottom-test");

        add_task(&conn, "normal task", 0).unwrap();
        push_task(&conn, "pushed task", "", 0, Some(path)).unwrap();

        let tasks = list_tasks(&conn, ShowFilter::All).unwrap();
        assert_eq!(tasks.len(), 2);
        // pushed task should be at the end (oldest created_at)
        assert_eq!(tasks[0].title, "normal task");
        assert_eq!(tasks[1].title, "pushed task");
    }

    #[test]
    fn test_add_task_creates_no_project() {
        let conn = test_conn_with_project();

        let task = add_task(&conn, "add task", 0).unwrap();

        assert!(task.project_id.is_none());
    }

    #[test]
    fn test_list_tasks_all_includes_tasks_with_and_without_project() {
        let conn = test_conn_with_project();
        let path = std::path::Path::new("/tmp/mixed");

        add_task(&conn, "unscoped", 0).unwrap();
        push_task(&conn, "scoped", "", 0, Some(path)).unwrap();

        let tasks = list_tasks(&conn, ShowFilter::All).unwrap();
        assert_eq!(tasks.len(), 2);
    }
}

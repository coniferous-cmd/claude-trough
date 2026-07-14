# Database

Path: `~/.config/trough/trough.db`

```sql
CREATE TABLE project (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 path TEXT NOT NULL UNIQUE,
 created_at INTEGER NOT NULL,
 updated_at INTEGER NOT NULL
);

CREATE TABLE task(
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

CREATE INDEX task_project_id_idx ON task(project_id);
```

The relationship is `project 1:N task`: tasks created by `push` belong to the
project identified by the current directory's canonical absolute path. A project
may contain any number of tasks.

Foreign keys are enabled with `PRAGMA foreign_keys = ON` on every connection
(`db::init()`). Migration in `db::migrate_schema()` detects the absence of
`task.project_id` at runtime, adds the column with `ALTER TABLE`, and creates
`task_project_id_idx`. The `project` table is created in `init()` with `CREATE
TABLE IF NOT EXISTS`. All migration steps are idempotent.

Migration leaves existing tasks unscoped with `project_id = NULL`. `push`
creates or reuses the current directory's project and associates the new task;
`add` continues to create an unscoped task. Project deletion must not cascade
into physical task deletion.

Global commands (`list`, `first`, TUI) do not filter by `project_id`. `next`
matches the canonical current directory to `project.path` and only completes an
incomplete task from that project. It does not create a missing project or fall
back to another project or an unscoped task.

`deleted_at` is `NULL` for active tasks. Task deletion is logical: `delete`
sets `deleted_at` for one task, and `clear` sets `deleted_at` for all active
tasks. Rows are not physically removed.

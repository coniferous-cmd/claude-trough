# Interfaces

This document records the public command interface and the database interface.

## CLI

Supported commands:

```text
trough add <TITLE> [-p|--priority <PRIORITY>]
trough push <TITLE> [-d|--detail <DETAIL>] [-p|--priority <PRIORITY>]
trough list [-s|--show <incomplete|completed|all>]
trough first
trough next
trough done <ID>
trough undo <ID>
trough delete <ID>
trough clear
trough edit <ID>
```

Examples:

```sh
trough add "buy milk"
trough add "write docs" --priority 2
trough push "plan release" --detail "include checklist"
trough list
trough list --show all
trough list -s completed
trough first
trough next
trough done 1
trough clear
trough edit 1
```

Notes:

- Priority defaults to `0`.
- Push detail defaults to an empty string.
- `push` does not print output on success.
- `list` defaults to showing incomplete tasks only.
- `list --show incomplete` shows incomplete tasks only and is the default.
- `list --show completed` shows completed tasks only.
- `list --show all` shows all non-deleted tasks.
- `list` prints no output when there are no tasks in the selected view.
- CLI task output uses `✅` for completed tasks and `❌` for incomplete tasks.
- `first` returns the first task by list ordering and does not delete it.
- `first` prints no output when there are no active tasks.
- `next` returns the first incomplete task for the canonical current-directory
  project by list ordering and marks it done.
- `next` prints no output when the current project is unknown or has no
  incomplete tasks. It does not fall back to another project or an unscoped
  task.
- `delete` removes one task from normal views by logical deletion.
- `clear` removes all tasks from normal views by logical deletion.
- Priority is documented as `0-3`, but the current code does not enforce the range.
- `done` and `undo` currently both toggle completion state.

## npm package

The npm package exposes the same `trough` command through a Node.js wrapper.
The wrapper selects a prebuilt Rust binary for the current platform from:

```text
dist/<platform>/trough
dist/<platform>/trough.exe
```

Supported package platforms:

- `darwin-x64`
- `darwin-arm64`
- `linux-x64`
- `linux-arm64`
- `win32-x64`
- `win32-arm64`

The npm package must include:

- `bin/trough.js`
- `dist/`
- `docs/`
- `Cargo.toml`
- `Cargo.lock`

The publish workflow builds each supported platform, verifies all expected
release binaries exist, runs `npm pack --dry-run`, then publishes with npm
provenance. `package.json` and `Cargo.toml` versions must match before an
automatic version tag can be created.

## SQL

### Project relationship

The project table groups tasks by the working directory where `push` runs. A
project has zero or more tasks, and each push-created task belongs to exactly
one project.

```sql
CREATE TABLE project (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
```

`project.path` stores the canonical absolute path of the working directory.
`task.project_id` is nullable to preserve legacy tasks and tasks created by
`add`. The foreign key uses the default restrictive delete behavior rather than
`ON DELETE CASCADE`, so deleting a project cannot physically delete its tasks.

```sql
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

CREATE INDEX task_project_id_idx ON task(project_id);
```

The migration is performed by `init()` and `migrate_schema()` in `db.rs`:

1. `init()` calls `PRAGMA foreign_keys = ON` on every connection open, then
   creates both tables with `CREATE TABLE IF NOT EXISTS` (idempotent).
2. `migrate_schema()` detects missing columns on the `task` table and adds
   `project_id` with `ALTER TABLE`, then creates `task_project_id_idx` with
   `CREATE INDEX IF NOT EXISTS` (also idempotent).
3. Existing task rows retain `project_id = NULL` — no data is backfilled.
4. The project table is created in `init()` so any future migration can rely on
   it existing.

When `trough push` runs, it resolves the current directory to a canonical
absolute path, creates the project row if the path has not been seen, or reuses
the existing row otherwise. The new task receives that project's ID. `add`
continues to create an unscoped task. Project-management commands are
intentionally outside the current scope — the schema is designed to support
`WHERE project_id = ?` queries without further migration when those features
are added.

`deleted_at` is `NULL` for active tasks. Delete-style commands set
`deleted_at` instead of physically deleting rows, preserving task history in the
database.

The current `list`, `first`, and TUI commands execute global queries that do not
filter by `project_id`. `next` resolves the canonical current directory and
selects through the matching project path:

```sql
SELECT task.id, task.project_id, task.title, task.done, task.detail,
       task.priority, task.created_at, task.updated_at
FROM task
JOIN project ON project.id = task.project_id
WHERE project.path = ?1
  AND task.deleted_at IS NULL
  AND task.done = 0
ORDER BY task.priority DESC, task.created_at DESC
LIMIT 1;
```

If this query returns a task, `next` marks it done and returns the updated row.
It does not insert a project on lookup and does not consider tasks from other
projects or tasks where `project_id IS NULL`. Existing list commands remain
global and keep their current ordering.

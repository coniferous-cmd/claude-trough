# Architecture

`trough` is a Rust single-binary application with two user interfaces over the same SQLite database.

## Runtime Flow

1. `main.rs` starts the process.
2. `db::init()` opens or creates the SQLite database.
3. If command-line arguments exist, `cli::run(&conn)` handles the request.
4. If no command-line arguments exist, `ui::run(&conn)` opens the TUI.

## Modules

- `main.rs`: startup and mode dispatch.
- `cli.rs`: Clap command parsing and CLI output.
- `ui.rs`: ratatui/crossterm list UI and keyboard handling.
- `db.rs`: database path, schema, migrations, and SQL operations.
- `editor.rs`: `$EDITOR` / `$VISUAL` integration.
- `models.rs`: `Task` model and constructors.

## Data Flow

Both CLI and TUI call database functions directly. The database module returns `Task` values or errors. UI code does not write SQL directly.

```text
CLI commands ┐
             ├─ db.rs ─ SQLite database
TUI actions  ┘

TUI edit / CLI edit ─ editor.rs ─ external editor
```

## Data Model

Projects group tasks. The relationship is one-to-many:

```text
project 1 ─────< task
                 task.project_id → project.id
```

- A task created by `push` belongs to exactly one project.
- Existing tasks and tasks created by `add` may remain unscoped.
- A project may contain zero or more tasks.
- A project's identity is the canonical absolute path of the current working
  directory at the time `push` runs.
- Project deletion must not cascade into physical task deletion.
- Global task ordering remains unchanged.

The initial project-aware migration adds a nullable `task.project_id`, so
existing task rows require no inferred project. Repeated pushes from the same
directory reuse the same project row.

### Push data flow

When `trough push` runs, the data flows through these layers:

1. `cli.rs` reads `std::env::current_dir()` and canonicalizes it with
   `std::fs::canonicalize`.
2. The canonical path is passed explicitly to `db::push_task()` as a `&Path`.
   The database layer never reads the process working directory itself — this
   keeps the function testable from any directory.
3. Inside `db::push_task()`, a transaction wraps two operations:
   - `INSERT ... ON CONFLICT(path) DO UPDATE` upserts a row in the `project`
     table. The `path UNIQUE` constraint prevents duplicates. Non-UTF-8 paths
     produce a descriptive error.
   - `INSERT INTO task (project_id, ...)` inserts the new task with the
     project's ID.
4. The transaction commits atomically — a partial write (project upsert
   succeeded but task insert failed) is impossible.
5. `db::get_task()` returns the complete `Task` value to the caller.

```text
CLI: push <title>
  │  std::env::current_dir() + canonicalize
  ▼
db::push_task(conn, title, detail, priority, Some(&canonical_path))
  │
  ├─ BEGIN TRANSACTION
  │   ├─ project upsert (INSERT ... ON CONFLICT)
  │   └─ task insert (project_id = project.id)
  ├─ COMMIT
  │
  ▼
Task { id, project_id, title, ... }
```

### Next data flow

`trough next` resolves the current directory using the same canonicalization as
`push`, then passes that path to `db::next_task()`. The database query joins the
task to the existing project path and selects the first incomplete,
non-deleted task using the normal priority and creation-time ordering. It marks
only that task complete and returns it for display.

`next` never creates a project. If the canonical path has no project or its
project has no incomplete tasks, the command returns without output. Tasks from
other projects and tasks with `project_id = NULL` are never used as fallbacks.

The database is stored at the user's config location, intended as:

```text
~/.config/trough/trough.db
```

SQLite is bundled through `rusqlite` so the app can remain a single binary from the user's perspective.

## Boundaries

- SQL belongs in `db.rs`.
- Terminal rendering belongs in `ui.rs`.
- Command parsing belongs in `cli.rs`.
- External editor process handling belongs in `editor.rs`.

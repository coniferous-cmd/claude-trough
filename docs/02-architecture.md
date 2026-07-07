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

## Persistence

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

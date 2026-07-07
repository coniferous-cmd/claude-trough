# Decisions

This file summarizes the current architectural decisions behind `trough`.

## Single Binary

`trough` should remain one executable. This keeps installation and usage simple.

Decision:

- One Rust binary.
- No background daemon.
- No separate service process.

## SQLite Storage

SQLite is used instead of JSON or plain text files.

Decision:

- Use SQLite for structured local persistence.
- Use `rusqlite` with the bundled SQLite feature.
- Keep database access centralized in `db.rs`.

## External Editor

Long-form task detail editing is delegated to the user's existing editor.

Decision:

- Use `$EDITOR`, then `$VISUAL`, then `vi`.
- Do not embed a custom text editor in the TUI.

## Dual Interface

The app supports both command mode and list mode.

Decision:

- Commands are for quick shell actions.
- The TUI is for review and keyboard navigation.
- Both interfaces operate on the same SQLite database.

# Interfaces

This document records the public command interface and the database interface.

## CLI

Supported commands:

```text
trough add <TITLE> [-p|--priority <PRIORITY>]
trough push <TITLE> [-d|--detail <DETAIL>] [-p|--priority <PRIORITY>]
trough list
trough next
trough first
trough done <ID>
trough undo <ID>
trough delete <ID>
trough edit <ID>
```

Examples:

```sh
trough add "buy milk"
trough add "write docs" --priority 2
trough push "plan release" --detail "include checklist"
trough list
trough next
trough done 1
trough edit 1
```

Notes:

- Priority defaults to `0`.
- Push detail defaults to an empty string.
- `next` returns the first task by list ordering and does not delete it.
- `first` is an alias for `next`.
- Priority is documented as `0-3`, but the current code does not enforce the range.
- `done` and `undo` currently both toggle completion state.

## SQL

The main table is `task`.

```sql
CREATE TABLE task (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    done INTEGER NOT NULL DEFAULT 0,
    detail TEXT NOT NULL DEFAULT '',
    priority INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
```

List ordering:

```sql
SELECT id, title, done, detail, priority, created_at, updated_at
FROM task
ORDER BY priority DESC, created_at DESC;
```

The application should treat this ordering as user-visible behavior.

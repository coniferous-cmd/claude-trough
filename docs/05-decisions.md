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

## Projects Own Tasks

Projects provide the first level of task organization, scoping `push`-created
tasks to the working directory.

### Identity: canonical absolute path

Decision: a project's unique identifier is the canonical absolute path of the
directory where `trough push` runs, stored as `project.path TEXT NOT NULL UNIQUE`.

Rationale:

- A filesystem path is always available from `std::env::current_dir()` with
  no user configuration or input.
- `std::fs::canonicalize` resolves symlinks, `.` and `..`, producing a stable
  path that survives directory renaming outside the repo. Without canonicalization,
  `push` from `/work/app` and `/work/app/./.` would produce different projects.
- The basename alone (_e.g._ `"app"`) is insufficient — two different
  directories named `app` on the same system would incorrectly merge their
  tasks.
- No user-facing project name is needed; the directory path is the name. This
  avoids a naming UI and a naming-decision step in the `push` flow.
- A path-based key makes reuse automatic: pushing from the same directory
  after the first use hits the `ON CONFLICT` branch and reuses the existing
  project row.

### Nullable project_id

Decision: `task.project_id INTEGER REFERENCES project(id)` is nullable.

Rationale:

- Pre-existing tasks created before the migration have no project — inferring
  one would require guesswork or batch directory heuristics that do not belong
  in a local-first tool.
- `trough add` is explicitly a "capture quick task" command that bypasses
  project association. Making `add` require a project would break the quick-
  capture workflow.
- A nullable foreign key lets the schema express "this task has no project"
  directly, without sentinel values or a pseudo-project like the rejected
  `Inbox` idea.
- Global queries (`list`, `first`, TUI) tolerate `project_id IS NULL` because
  they do not filter by project. Project-scoped `next` intentionally excludes
  unscoped tasks.

### No cascading delete

Decision: the foreign key from `task.project_id` to `project.id` uses the
default restrictive action (no `ON DELETE CASCADE`).

Rationale:

- A project is an organizational label, not a container that owns the lifetime
  of its tasks. Removing a project should not destroy task history.
- Tasks are soft-deleted via `deleted_at`. If a project is removed, its tasks
  remain readable in the database and could be reassigned or re-exposed by a
  future project-management command.
- SQLite foreign keys are off by default (the `PRAGMA` must be set per-
  connection). Even without the PRAGMA, the `REFERENCES` clause is valid
  syntax, so older connections that skip the PRAGMA do not crash.

### Scope limits

Decision: project-awareness covers `push` association and strict current-project
selection for `next`. Project-filtered listing, project switching, and project
CRUD are not implemented and are not planned for the current feature set.

Rationale:

- The immediate user need is automatic organization of `push`-created tasks
  by working directory, not a full project browser.
- Adding listing filters or switch commands before the schema has real-world
  use would be speculative. The schema is designed so that future
  `WHERE project_id = ?` queries require no migration.
- `list`, `first`, and TUI remain global. `next` is project-scoped because it
  mutates a task and must not complete work belonging to another directory.

### Strict next scope

Decision: `trough next` selects only tasks whose project path matches the
canonical current directory. An unknown or exhausted project produces no
output; there is no fallback to another project or to an unscoped task.

Rationale:

- Completing a task is a mutation, so an implicit global fallback could change
  unrelated project state.
- Legacy tasks and tasks created by `add` have no project identity and cannot
  safely be attributed to the current directory.
- Looking up the project without upserting it keeps an empty `next` call
  read-only and avoids accumulating projects that have never received a task.

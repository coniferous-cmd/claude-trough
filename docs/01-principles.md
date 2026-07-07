# Principles

`trough` is a terminal-first todo manager. It should stay fast, local, and keyboard friendly.

## Product Direction

- Keep the app focused on personal task capture and review.
- Support both quick command-line actions and an interactive list view.
- Store data locally; do not require network services.
- Prefer predictable terminal behavior over rich but fragile UI features.
- Keep the binary easy to install, run, and remove.

## Design Principles

- Keyboard first: common actions should not require a mouse.
- Single binary: avoid extra daemons, services, or companion apps.
- Local persistence: SQLite is the source of truth.
- Editor integration: use the user's existing editor for long-form details.
- Small surface area: add features only when they fit the todo workflow.

## Implementation Principles

- Keep modules small and single-purpose.
- Let `db.rs` own persistence details.
- Keep CLI, TUI, docs, and tests aligned when behavior changes.
- Prefer explicit behavior over hidden automation.
- Use existing Rust ecosystem libraries for terminal, CLI, and SQLite concerns.

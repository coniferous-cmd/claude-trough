# CLAUDE.md

This file is a routing entry for Claude. Keep it short; detailed project rules live under `.claude/rules/`.

## Read First

Start with:

- `.claude/rules/architecture-rules.md`

Then read the focused rule file for the task:

- `.claude/rules/module-rules.md` for module boundaries and ownership.
- `.claude/rules/data-rules.md` for SQLite schema, migrations, ordering, and model behavior.
- `.claude/rules/interface-rules.md` for CLI, TUI, and editor behavior.
- `.claude/rules/development-rules.md` for error handling, tests, and style.
- `.claude/rules/command-rules.md` for command-specific local workflow rules, if present.

## Project Summary

`trough` is a Rust single-binary, terminal-first todo app.

- CLI mode handles quick shell commands.
- TUI mode opens when running `trough` without arguments.
- SQLite is the local source of truth.
- `$EDITOR` / `$VISUAL` is used for task detail editing.

## Working Rule

Do not duplicate detailed rules here. Update the relevant file in `.claude/rules/` and keep this file as the stable routing index.

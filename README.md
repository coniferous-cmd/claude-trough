# trough

`trough` is a fast, local-first task queue for the terminal. It combines a
scriptable command-line interface with a keyboard-driven terminal UI, and keeps
your tasks in a local SQLite database.

## Features

- CLI commands for adding, listing, completing, editing, and deleting tasks
- Interactive terminal UI when run without a subcommand
- Priority-based ordering
- Automatic project association — tasks created with `push` are grouped by
  working directory
- Task details edited with `$EDITOR` or `$VISUAL`
- Local SQLite storage with no external service required
- Automatic migration from the legacy `~/.todo/todo.db` database

## Install from source

Building `trough` requires a recent Rust toolchain.

```sh
git clone https://github.com/coniferous-cmd/claude-trough.git trough
cd trough
cargo install --path .
```

You can also build and run it directly from the repository:

```sh
cargo build --release
./target/release/trough
```

## Usage

Run `trough` without arguments to open the terminal UI:

```sh
trough
```

The UI supports the following keys:

| Key | Action |
| --- | --- |
| `j` / `Down` | Select the next task |
| `k` / `Up` | Select the previous task |
| `Space` | Toggle the selected task |
| `Enter` | Edit the selected task's detail |
| `q` | Quit |

Use subcommands for scripts and quick changes:

```sh
# Add to the top of the queue
trough add "write release notes"
trough add "fix production issue" --priority 3

# Push to the bottom of the queue, automatically associated with the current
# working directory as a project
trough push "review backlog" --detail "Start with stale issues"

# Inspect the queue
trough list
trough list --show completed
trough list --show all
trough first

# Complete the next incomplete task for the current project and print it
trough next

# Update tasks by ID
trough done 1
trough undo 1
trough edit 1
trough delete 1

# Remove all tasks from normal views
trough clear
```

Run `trough help` or `trough help <COMMAND>` for the complete command reference.

Tasks are ordered by priority, then newest first. Priorities default to `0`;
the CLI describes the intended range as `0` through `3`, although that range is
not currently enforced. The `done` and `undo` commands both toggle the current
completion state, so use them only when you know the task's current state.

## Data and editor configuration

The database is stored in your operating system's configuration directory under
`trough/trough.db` (for example, `~/.config/trough/trough.db` on Linux).
Deleted tasks are soft-deleted in SQLite and do not appear in normal views.

When `trough push` creates a task, it creates or reuses a project identified by
the current directory's canonical absolute path and associates the task with it.
Existing tasks and tasks created by `trough add` remain unscoped.

For example, running the following commands from `/work/app`:

```sh
cd /work/app
trough push "update dependencies"
trough push "write tests" --detail "cover the API module"
```

...creates a project with path `/work/app` and two tasks associated with it. A
subsequent `trough push` from `/work/docs` would create a separate project
with path `/work/docs` and its own task list. The `next` command only completes
a task associated with the canonical current directory; it prints nothing when
that project has no incomplete tasks and never falls back to another project or
an unscoped task. The `list`, `first`, and TUI views remain global.

This relationship is documented in
[`docs/02-architecture.md`](docs/02-architecture.md) and
[`docs/06-interfaces.md`](docs/06-interfaces.md).

`trough edit` and the UI's `Enter` action use `$EDITOR`, then `$VISUAL`, falling
back to `vi` when neither variable is set. For example:

```sh
export EDITOR=nvim
```

## Development

```sh
cargo test
cargo fmt --check
cargo clippy --all-targets --all-features
```

The repository also contains an npm wrapper used to package prebuilt binaries.
To inspect its package contents locally, run:

```sh
npm run pack:dry-run
```

Design notes and the public interfaces are documented in [`docs/`](docs/).

## License

See [LICENSE](LICENSE).

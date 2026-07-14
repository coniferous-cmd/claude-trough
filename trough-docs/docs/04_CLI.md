# CLI

```
trough add "task"
trough add "task" --priority 1
trough push "task"
trough push "task" --detail "more context"
trough push "task" --detail "more context" --priority 1
trough list
trough list --show all
trough list -s completed
trough first
trough next
trough done 1
trough undo 1
trough delete 1
trough clear
trough edit 1
```

`delete` and `clear` hide tasks with logical deletion. They do not physically
remove task rows from the database.

`push` prints no output on success. `list` defaults to incomplete tasks only.
Use `list --show incomplete`, `list --show completed`, or `list --show all` to
select the completion view; `-s` is the short form of `--show`. `list` prints no
output when there are no tasks in the selected view. CLI task output uses `✅`
for completed tasks and `❌` for incomplete tasks. `first` shows the first task
without changing it. `first` prints no output when there are no active tasks.
`next` shows and completes the first incomplete task associated with the
canonical current-directory project. It prints no output when that project is
unknown or has no incomplete tasks, and it does not fall back to another
project or an unscoped task.

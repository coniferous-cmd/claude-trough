# CLI

```
trough add "task"
trough add "task" --priority 1
trough push "task"
trough push "task" --detail "more context"
trough push "task" --detail "more context" --priority 1
trough list
trough next
trough first
trough done 1
trough undo 1
trough delete 1
trough clear
trough edit 1
```

`delete` and `clear` hide tasks with logical deletion. They do not physically
remove task rows from the database.

`push` prints no output on success. `list` prints no output when there are no
active tasks. CLI task output uses `✅` for completed tasks and `❌` for
incomplete tasks.

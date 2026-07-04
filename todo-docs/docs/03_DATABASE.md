# Database

Path: `~/.todo/todo.db`

```sql
CREATE TABLE task(
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 title TEXT NOT NULL,
 done INTEGER NOT NULL DEFAULT 0,
 detail TEXT NOT NULL DEFAULT '',
 priority INTEGER NOT NULL DEFAULT 0,
 created_at INTEGER NOT NULL,
 updated_at INTEGER NOT NULL
);
```

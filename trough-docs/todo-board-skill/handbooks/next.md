---
name: next
description: Retrieve and execute the next queued task.
---

# Next

Use the bundled `../tools/todo next` command resolved relative to this handbook directory.

## Workflow

1. Run the bundled `../tools/todo next` command and capture its output.
2. Read the returned title, description, and task spec.
3. If no task is returned, report that the queue is empty.
4. Read the repository instructions and inspect the relevant code or docs.
5. Follow the plan constraints, tests, and acceptance criteria.
6. Execute only the returned task, then verify the result.

## Rules

- The bundled executable must exist and be executable.
- Do not modify or replace the bundled executable.
- If retrieval fails, report the failure and stop.
- Respect repository instructions, shell wrappers, environment requirements, and approval boundaries.

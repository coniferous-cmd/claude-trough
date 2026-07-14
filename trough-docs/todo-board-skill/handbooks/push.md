---
name: todo-push
description: Push finalized plans.
---

# Push

Use the bundled `../tools/todo push` command resolved relative to this handbook directory.

## Workflow

1. Derive a short action title.
2. Write the completed implementation plan description.
3. Resolve the bundled tool from this skill directory.
4. Run `<skill-dir>/tools/todo push "<title>" -d "<plan>"`.
5. Keep the title and plan as separate safe arguments.
6. Report success and the resulting id.

## Rules

- Use the bundled tool, not `PATH`.
- Preserve the executable.
- Only push finalized plans.
- Do not create `todo/*.md` files.
- Do not modify or replace the bundled executable.

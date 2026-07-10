# Brainwares Workspace Integration Rules

This repository uses **Brainwares** (`bw`) for Obsidian-style markdown memory storage and code-reference hash tracking.

You MUST follow these rules during your session:
1. **Status Audit**: Before writing or changing code, run `bw status` to check if there are any outdated memory files or broken links.
2. **Context Resolution**: Read relevant memories using `bw read <note_name>` or search memories with `bw query <term>` to gain full context about coding guidelines, system details, or database schemas.
3. **Reference Maintenance**: After modifying any code files in the codebase, run `bw status`. If any references are outdated, inspect the associated markdown note under `.brainwares/memories/`, update its contents to reflect the new codebase state, and run `bw update <note_name>` to synchronize the hashes.
4. **Clean State**: Keep the vault clean and verified before completing your task.

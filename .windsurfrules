# Brainwares Workspace Integration Rules
 
This repository uses **Brainwares** (`bw`) for Obsidian-style markdown memory storage and code-reference hash tracking.
 
You MUST follow these rules during your session:
1. **Status Audit**: Before writing or changing code, run `bw status` to check if there are any outdated memory files or broken links.
2. **Context Resolution**: Read relevant memories using `bw read <note_name>` or search memories with `bw query <term>` to gain full context. Note that `bw query` supports tokenized multi-word search queries (words are matched across fields, and results are ranked by relevance).
3. **Reference Maintenance & Edits**: After modifying any code files in the codebase, run `bw status`. If any references are outdated, read the memory page using `bw read <note_name>` and update its contents by running `bw write <note_name>` (e.g. `echo "new content" | bw write <note_name>`), and run `bw update <note_name>` to synchronize the hashes. Do NOT write or edit memory markdown files directly on the filesystem. If you created any new source or configuration files, document them in a memory note (creating a new one if necessary using `bw add <note_name>`) and run `bw link <note_name> <file_path>` to link and track them.
4. **Dependency & Memory Relations**: When adding or updating memory notes, scan the code file's imports and dependencies. You MUST declare relations to other memory notes (dependencies, tests, etc.) ONLY by using the CLI command: `bw relate <memory> <target>` (or `bw relate <memory> <target> --remove` to delete a relation). This stores relations in the frontmatter at the top of the files. Do NOT write inline double-bracket wiki-links (`[[dependency-note-name]]`) in the note body.
5. **Clean State**: Keep the vault clean and verified before completing your task.

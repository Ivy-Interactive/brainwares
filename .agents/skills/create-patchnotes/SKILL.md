---
name: bw-create-patchnotes
description: >-
  Generates release patch notes for the brainwares CLI repository from git commits between two tags or refs, and updates the corresponding GitHub release notes.
---

# Brainwares CLI Patch Notes Generator

This skill guides the agent in generating release patch notes and updating the corresponding GitHub release description.

## Workflow

### 1. Plan and Determine Range
- Retrieve all git tags using:
  ```bash
  git tag --sort=-v:refname
  ```
- Ask the user to confirm the tag range (e.g. from the previous release tag like `v0.1.0` to the latest release tag `v0.1.1`).

### 2. Extract Commits
- Run git log to list all commits within the range, formatted as a bulleted list:
  ```bash
  git log <FromRef>..<ToRef> --pretty=format:"* %s (%h)"
  ```

### 3. Draft Release Notes
- Create a markdown document `.releases/release-notes-v<version>.md` (e.g., `.releases/release-notes-v0.1.1.md`) containing:
  - An overview of the release.
  - Bulleted changes categorized under headings:
    - `Features & Enhancements`
    - `Bug Fixes & Improvements`
    - `Internal & Chore`
  - References/links to PRs or issues if mentioned in the commits.

### 4. Update GitHub Release Notes
- Check the current release description:
  ```bash
  gh release view <tag> --json body -q .body
  ```
- Update the GitHub release description with the drafted markdown file using the `gh` CLI:
  ```bash
  gh release edit <tag> --notes-file .releases/release-notes-v<version>.md
  ```

### 5. Commit and Push
- Commit and push the drafted notes to the repository:
  ```bash
  git add .releases/release-notes-v<version>.md
  git commit -m "docs: add release notes for v<version>"
  git push origin main
  ```

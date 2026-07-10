---
title: Getting Started with Brainwares
references:
- path: Cargo.toml
  hash: 614446d2664e867486542e80de4ee2448aa9119ea4372a81ee96d7471dcf337a
tags:
- tutorial
- setup
last_updated: 2026-07-10T21:58:11.472567900+00:00
---

# Getting Started with Brainwares

Brainwares merges the concepts of **Obsidian** (connected local Markdown notes) and **Promptware** (self-improving, context-aware prompt modules).

## 1. Hashing Code References

We have linked this note to your `Cargo.toml` file! If you make any modifications to `Cargo.toml`, your brainwares memory will detect that it is out-of-sync.

Try this workflow:
1. Run `bw status` (it should say `Outdated memories: 0`).
2. Add a space or comment to `Cargo.toml`.
3. Run `bw status` again. It will flag this memory page as `[OUTDATED CODE]`.
4. Run `bw update getting-started` to re-hash the file and mark it clean again!

## 2. Linking Notes (Wiki-Links)

You can link memory notes using Obsidian double-bracket syntax: [[index]].
To check references and backlinks for this note:
```bash
bw read getting-started
```

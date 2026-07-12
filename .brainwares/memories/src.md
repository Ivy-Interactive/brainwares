---
title: Src
references:
- path: src/models.rs
  hash: 3e4520b4d8c1a579f27c3946709f540dfffe58a3a24b49835e59614d8ae5fb49
- path: src/commands.rs
  hash: b59b753ede75c82d6bc1c9db6af1ebcbc6892754e3e79bd06fd87e17f53f2ce8
- path: src/main.rs
  hash: 2371f62b8f865501a0dd18c03c4efb782e105f28249a8774adb1cdb0caa38d65
- path: src/parser.rs
  hash: ed47de0de466de22635ff16cd236dffadde6e4c568b513a8623a0d22d0968ae1
- path: src/hash.rs
  hash: 661c16a520b92112a4b6f260a20caba130677443ac9df0e09c605903d01e3c01
- path: src/vault.rs
  hash: 47677d21b5a345ee941aebaa0f58b7dab48ba281b26615b81dd778ab2a9cad84
- path: src/cli.rs
  hash: a890d0718e3d2f982de03ea8acc11fd8b4d6003cd8b86aaf7dc76ec07902f47d
- path: src/engine.rs
  hash: ca88fa4d18bf76d5031aab210b8ceb5de58c90dffd879338a0aa990164daef5e
tags:
- folder
- index
last_updated: 2026-07-12T20:21:33.043062+00:00
---

# Src

Scaffolded memory page for the `src` directory.

## Core Files Reference Map

*   `models.rs`: Data structures and serialization schemas for config and memory frontmatter.
*   `commands.rs`: Core handlers for CLI subcommands including init, status, and indexing.
*   `main.rs`: Entry point parsing CLI arguments and routing subcommand handlers.
*   `parser.rs`: Obsidian-style Markdown frontmatter parser and wiki-link extractor.
*   `hash.rs`: File hashing helper using SHA-256 to detect out-of-date states.
*   `vault.rs`: Vault configuration loading, path helpers, and memory backlink resolver.
*   `cli.rs`: Command Line Interface structure definitions mapped via clap.
*   `engine.rs`: Diagnostic status check, validation rules, and context loader routines.

---
title: Src
references:
- path: src/models.rs
  hash: 3e4520b4d8c1a579f27c3946709f540dfffe58a3a24b49835e59614d8ae5fb49
- path: src/commands.rs
  hash: 6ba46296ee40592027b7da764e080740fddf167ad8518f42e4887238c76312c2
- path: src/main.rs
  hash: de333780ed142470e031a9c9d9002a06c97d39876c204ea67b378986eec30c26
- path: src/parser.rs
  hash: ed47de0de466de22635ff16cd236dffadde6e4c568b513a8623a0d22d0968ae1
- path: src/hash.rs
  hash: 661c16a520b92112a4b6f260a20caba130677443ac9df0e09c605903d01e3c01
- path: src/vault.rs
  hash: da257976f03f3d78a2a7aaa9bf11634f7982278f37f37a8d71317327f2f0a638
- path: src/cli.rs
  hash: a8cd5a1872cb8c31655cd05e20cac71b7385df994d899886057fa19f3583e948
- path: src/engine.rs
  hash: ca88fa4d18bf76d5031aab210b8ceb5de58c90dffd879338a0aa990164daef5e
tags:
- folder
- index
last_updated: 2026-07-12T15:27:25.078654+00:00
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

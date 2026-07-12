---
title: Src
references:
- path: src/models.rs
  hash: 3e4520b4d8c1a579f27c3946709f540dfffe58a3a24b49835e59614d8ae5fb49
- path: src/commands.rs
  hash: aae5a9ae2f66c717243f302796e3543f02b978803270dd654845bcae2714f861
- path: src/main.rs
  hash: 4cb1db4a125a2fde13a57b65783353e42c9aac7a334067609c21d309db9671ce
- path: src/parser.rs
  hash: ed47de0de466de22635ff16cd236dffadde6e4c568b513a8623a0d22d0968ae1
- path: src/hash.rs
  hash: 661c16a520b92112a4b6f260a20caba130677443ac9df0e09c605903d01e3c01
- path: src/vault.rs
  hash: da257976f03f3d78a2a7aaa9bf11634f7982278f37f37a8d71317327f2f0a638
- path: src/cli.rs
  hash: b6a2d3c69ac3a4900d6e2d926f609db2a9c2267e9bb643bb8decadfa5453601e
- path: src/engine.rs
  hash: ca88fa4d18bf76d5031aab210b8ceb5de58c90dffd879338a0aa990164daef5e
tags:
- folder
- index
last_updated: 2026-07-12T14:56:22.039418+00:00
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

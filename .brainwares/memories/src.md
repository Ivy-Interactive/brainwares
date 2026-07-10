---
title: Src
references:
- path: src/models.rs
  hash: 3c412b9a942f3be1bfd2aa002da64021263b70c271ceaa1591b382042740e085
- path: src/commands.rs
  hash: d79fea3c6086c94ee84813e69f78544bed1df6ad4b643b5d674635e38a4fe3d7
- path: src/main.rs
  hash: 58567204bae682fbae97db5ab935d65e13a1474a60667d52a801d2760a0ca8e3
- path: src/parser.rs
  hash: a8c8c2c8c1b5e17510059a6ca7b11ec8e0e7580d71984192d7214ceacd22017a
- path: src/hash.rs
  hash: c39e65bdef00bd4c87f95505e0ebb295c3b0672b210f30dcc7998ce1b7cae474
- path: src/vault.rs
  hash: 2c98db5fa9c4f5decf3e9e72f86683629ffcab23c2882b74ff651aac6fa6a242
- path: src/cli.rs
  hash: 9269ad4c1d115909eb66afd3198c3b67c0445dcfea6b13a7ad730ba5904c7d35
- path: src/engine.rs
  hash: c21f3ed85f4f69ba0d9064921e28aa22a300701ad94857cae49e61e46581ccd9
tags:
- folder
- index
last_updated: 2026-07-10T18:42:41.237685100+00:00
---

# Src

Scaffolded memory page for the `src` directory.

## Core Files Reference Map

*   `models.rs`: Data structures and serialization schemas for config and memory frontmatter.
*   `commands.rs`: Core handlers for CLI subcommands including init, status, indexing, and UI visualization.
*   `main.rs`: Entry point parsing CLI arguments and routing subcommand handlers.
*   `parser.rs`: Obsidian-style Markdown frontmatter parser and wiki-link extractor.
*   `hash.rs`: File hashing helper using SHA-256 to detect out-of-date states.
*   `vault.rs`: Vault configuration loading, path helpers, and memory backlink resolver.
*   `cli.rs`: Command Line Interface structure definitions mapped via clap.
*   `engine.rs`: Diagnostic status check, validation rules, and context loader routines.

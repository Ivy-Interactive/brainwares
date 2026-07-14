---
title: Src
references:
- path: src/models.rs
  hash: 79d808c465fb5f14e2b293f7e198ccb66a3a9ce22939fc4922cf37acdfad0ad0
- path: src/commands.rs
  hash: e43ec8bf2fa826216a2565b9df6ebf95498a401636c88487ca643b56596deb0c
- path: src/main.rs
  hash: a71db57b585c8de734d69ab10ca7d9e332b828fa55fe93e9740fa0eb5af3f1e6
- path: src/parser.rs
  hash: ed47de0de466de22635ff16cd236dffadde6e4c568b513a8623a0d22d0968ae1
- path: src/hash.rs
  hash: 661c16a520b92112a4b6f260a20caba130677443ac9df0e09c605903d01e3c01
- path: src/vault.rs
  hash: 8f1de7d148b6d8b2a59241e40269c11efe6372f25d8ba91a639b9b794462fed0
- path: src/cli.rs
  hash: 9245eb8c02052f79f13b16f4c34eb05f88611fd4d13ed07514607c52cf8e7d3c
- path: src/engine.rs
  hash: f5ed302035757cf17f6ceabbbffc2858442725ddf6928d2af4c827b3518b2dd2
tags:
- folder
- index
last_updated: 2026-07-14T11:31:33.099990+00:00
type: null
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

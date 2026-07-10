use crate::models::{Backlink, Config, MemoryPage};
use crate::parser::parse_memory_file;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn find_vault_path() -> PathBuf {
    let mut current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    loop {
        let candidate = current.join(".brainwares");
        if candidate.is_dir() {
            return candidate;
        }
        if !current.pop() {
            break;
        }
    }
    // Default to local directory .brainwares
    PathBuf::from(".brainwares")
}

pub fn get_workspace_root(vault_path: &Path) -> PathBuf {
    vault_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn init_vault(vault_path: &Path) -> Result<Config, String> {
    if !vault_path.exists() {
        fs::create_dir_all(vault_path)
            .map_err(|e| format!("Failed to create vault directory: {}", e))?;
    }

    let memories_dir = vault_path.join("memories");
    if !memories_dir.exists() {
        fs::create_dir_all(&memories_dir)
            .map_err(|e| format!("Failed to create memories directory: {}", e))?;
        
        // Let's create a default index.md
        let index_path = memories_dir.join("index.md");
        if !index_path.exists() {
            let default_index = "\
---
title: Welcome to Brainwares
tags: [welcome, index]
---

# Welcome to Brainwares

This is the entry point of your Obsidian-style memory vault for AI agents.
Use wiki-links like [[Another Memory]] to link files.
";
            fs::write(index_path, default_index)
                .map_err(|e| format!("Failed to write default index.md: {}", e))?;
        }
    }

    let programs_dir = vault_path.join("programs");
    if !programs_dir.exists() {
        fs::create_dir_all(&programs_dir)
            .map_err(|e| format!("Failed to create programs directory: {}", e))?;
    }

    let logs_dir = vault_path.join("logs");
    if !logs_dir.exists() {
        fs::create_dir_all(&logs_dir)
            .map_err(|e| format!("Failed to create logs directory: {}", e))?;
    }

    let config_path = vault_path.join("config.json");
    let config = if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config: {}", e))?
    } else {
        let default_config = Config::default();
        let content = serde_json::to_string_pretty(&default_config)
            .map_err(|e| format!("Failed to serialize default config: {}", e))?;
        fs::write(&config_path, content)
            .map_err(|e| format!("Failed to write default config: {}", e))?;
        default_config
    };

    Ok(config)
}

pub fn load_memories(vault_path: &Path) -> Result<Vec<MemoryPage>, String> {
    let memories_dir = vault_path.join("memories");
    if !memories_dir.is_dir() {
        return Err(format!("Memories directory not found at {:?}", memories_dir));
    }

    let mut memories = Vec::new();
    for entry in fs::read_dir(memories_dir).map_err(|e| format!("Failed to read memories dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Directory entry error: {}", e))?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
            let content = fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read memory file {:?}: {}", path, e))?;
            let page = parse_memory_file(&content, &path)?;
            memories.push(page);
        }
    }
    Ok(memories)
}

pub fn normalize_memory_name(name: &str) -> String {
    name.to_lowercase()
        .replace(" ", "-")
        .replace("_", "-")
        .replace(".md", "")
}

pub fn get_backlinks(memories: &[MemoryPage]) -> HashMap<String, Vec<Backlink>> {
    let mut backlinks_map: HashMap<String, Vec<Backlink>> = HashMap::new();

    for source_page in memories {
        let source_name = source_page.name.clone();
        let source_path = source_page.file_path.to_string_lossy().to_string();

        // Scan page body line by line to collect context
        for (line_num, line) in source_page.body.lines().enumerate() {
            let extracted = crate::parser::extract_wiki_links(line);
            for (target_name, _raw_match) in extracted {
                // Normalize target name for case-insensitive, space/hyphen robust matching
                let target_normalized = normalize_memory_name(&target_name);
                
                let context = format!("Line {}: {}", line_num + 1, line.trim());
                let backlink = Backlink {
                    source_name: source_name.clone(),
                    source_path: source_path.clone(),
                    context,
                };
                
                backlinks_map
                    .entry(target_normalized)
                    .or_default()
                    .push(backlink);
            }
        }
    }

    backlinks_map
}


use crate::models::{Backlink, Config, MemoryPage};
use crate::parser::parse_memory_file;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn get_global_config_path() -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;
    Some(PathBuf::from(home).join(".config").join("brainwares").join("config.json"))
}

pub fn get_global_memories_dir() -> Option<PathBuf> {
    let tendril_home = std::env::var("TENDRIL_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(home).join(".tendril")
        });
    let tendril_memories = tendril_home.join("Promptwares").join("memories");
    if tendril_memories.is_dir() {
        return Some(tendril_memories);
    }
    
    let global_config = get_global_config_path()?;
    let parent = global_config.parent()?;
    Some(parent.join("memories"))
}

pub fn load_global_config() -> Config {
    if let Some(path) = get_global_config_path() {
        if path.is_file() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str::<Config>(&content) {
                    return config;
                }
            }
        }
    }
    Config::default()
}

pub fn load_local_config(vault_path: &Path) -> Option<Config> {
    let config_path = vault_path.join("config.json");
    if config_path.is_file() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(config) = serde_json::from_str::<Config>(&content) {
                return Some(config);
            }
        }
    }
    None
}

pub fn load_merged_config(vault_path: &Path) -> Config {
    let mut config = load_global_config();
    if let Some(local) = load_local_config(vault_path) {
        config.default_vault_dir = local.default_vault_dir;
        
        // Merge ignore patterns and deduplicate
        let mut merged_ignores = config.ignore_patterns;
        for pattern in local.ignore_patterns {
            if !merged_ignores.contains(&pattern) {
                merged_ignores.push(pattern);
            }
        }
        config.ignore_patterns = merged_ignores;
    }

    // Dynamically load patterns from workspace's .gitignore file(s)
    let workspace_root = get_workspace_root(vault_path);
    let gitignore_patterns = load_gitignore_patterns(&workspace_root);
    for pattern in gitignore_patterns {
        if !config.ignore_patterns.contains(&pattern) {
            config.ignore_patterns.push(pattern);
        }
    }

    config
}

pub fn find_vault_path() -> PathBuf {
    if let Ok(vault) = std::env::var("BW_VAULT") {
        let trimmed = vault.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    let global_config = load_global_config();
    let vault_dir_name = &global_config.default_vault_dir;

    let mut current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    loop {
        let candidate = current.join(vault_dir_name);
        if candidate.is_dir() {
            return candidate;
        }
        if !current.pop() {
            break;
        }
    }

    let tendril_home = std::env::var("TENDRIL_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(home).join(".tendril")
        });
    let central_candidate = tendril_home.join(vault_dir_name);
    if central_candidate.is_dir() {
        return central_candidate;
    }

    // Default to local directory
    PathBuf::from(vault_dir_name)
}

pub fn get_project_name() -> Option<String> {
    if let Ok(project) = std::env::var("BW_PROJECT") {
        let trimmed = project.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    let current_dir = std::env::current_dir().ok()?;
    
    // 1. Try to find name in config.yaml
    let tendril_home = std::env::var("TENDRIL_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(home).join(".tendril")
        });
    let config_path = tendril_home.join("config.yaml");
    if config_path.exists() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            let mut current_project_name = None;
            let target_path = fs::canonicalize(&current_dir).unwrap_or_else(|_| current_dir.clone());
            
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("- name:") || trimmed.starts_with("name:") {
                    if let Some(idx) = trimmed.find(':') {
                        current_project_name = Some(trimmed[idx + 1..].trim().to_string());
                    }
                } else if trimmed.starts_with("path:") {
                    if let Some(idx) = trimmed.find(':') {
                        let repo_path_raw = trimmed[idx + 1..].trim();
                        let repo_path = PathBuf::from(repo_path_raw);
                        if let Ok(full_repo_path) = fs::canonicalize(&repo_path) {
                            if target_path.starts_with(&full_repo_path) {
                                if let Some(name) = current_project_name {
                                    return Some(name);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Fall back to git repo directory name
    let mut dir = current_dir;
    loop {
        if dir.join(".git").exists() {
            return dir.file_name().map(|n| n.to_string_lossy().to_string());
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

pub fn get_workspace_root(vault_path: &Path) -> PathBuf {
    if let Ok(current_dir) = std::env::current_dir() {
        let vault_parent = vault_path.parent().unwrap_or(vault_path);
        if current_dir.starts_with(vault_parent) {
            let mut dir = current_dir;
            loop {
                if dir.join(".git").exists() {
                    return dir;
                }
                if !dir.pop() {
                    break;
                }
            }
        }
    }

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

    let workspace_root = get_workspace_root(vault_path);

    let memories_dir = vault_path.join("memories");
    if !memories_dir.exists() {
        fs::create_dir_all(&memories_dir)
            .map_err(|e| format!("Failed to create memories directory: {}", e))?;
    }

    // 1. Create memories/index.md
    let index_path = memories_dir.join("index.md");
    if !index_path.exists() {
        let default_index = "\
---
title: Welcome to Brainwares
tags: [welcome, index]
---

# Welcome to Brainwares

This is the main entry point of your brainwares memory vault.

- Read the [[getting-started]] guide to learn how to use this tool.
- Explore Promptware templates under `.brainwares/programs/`.
- Use the CLI `bw status` to verify your workspace status!
";
        fs::write(index_path, default_index)
            .map_err(|e| format!("Failed to write index.md: {}", e))?;
    }

    // 2. Create memories/getting-started.md
    let getting_started_path = memories_dir.join("getting-started.md");
    if !getting_started_path.exists() {
        // Try to find any existing file in the workspace to hash for the demo reference
        let (ref_file, ref_hash) = if let Some(file_path) = find_any_file_in_workspace(&workspace_root) {
            let full_path = workspace_root.join(&file_path);
            let hash = crate::hash::calculate_file_hash(&full_path)
                .unwrap_or_else(|_| "d41d8cd98f00b204e9800998ecf8427e".to_string());
            (file_path, hash)
        } else {
            // Write a fallback README.md if the workspace is completely empty
            let readme_path = workspace_root.join("README.md");
            let _ = fs::write(&readme_path, "# README\n");
            let hash = crate::hash::calculate_file_hash(&readme_path)
                .unwrap_or_else(|_| "d41d8cd98f00b204e9800998ecf8427e".to_string());
            ("README.md".to_string(), hash)
        };

        let default_getting_started = format!("\
---
title: Getting Started with Brainwares
references:
  - path: \"{}\"
    hash: \"{}\"
tags: [tutorial, setup]
---

# Getting Started with Brainwares

Brainwares merges the concepts of **Obsidian** (connected local Markdown notes) and **Promptware** (self-improving, context-aware prompt modules).

## 1. Hashing Code References

We have linked this note to your `{}` file! If you make any modifications to `{}`, your brainwares memory will detect that it is out-of-sync.

Try this workflow:
1. Run `bw status` (it should say `Outdated memories: 0`).
2. Add a space or comment to `{}`.
3. Run `bw status` again. It will flag this memory page as `[OUTDATED CODE]`.
4. Run `bw update getting-started` to re-hash the file and mark it clean again!

## 2. Linking Notes (Wiki-Links)

You can link memory notes using Obsidian double-bracket syntax: [[index]].
To check references and backlinks for this note:
```bash
bw read getting-started
```
", ref_file, ref_hash, ref_file, ref_file, ref_file);

        fs::write(getting_started_path, default_getting_started)
            .map_err(|e| format!("Failed to write getting-started.md: {}", e))?;
    }

    let programs_dir = vault_path.join("programs");
    if !programs_dir.exists() {
        fs::create_dir_all(&programs_dir)
            .map_err(|e| format!("Failed to create programs directory: {}", e))?;
    }

    // 3. Create programs/refactor.md
    let refactor_program_path = programs_dir.join("refactor.md");
    if !refactor_program_path.exists() {
        let refactor_content = "\
# Program: Code Refactoring

You are an expert software engineer tasked with refactoring code files to follow clean coding standards, optimize performance, and improve maintainability.

## Instructions
1. Inspect the code files referenced in the compiled prompt.
2. Review the context provided in the memories section.
3. Perform the requested refactor on the code files.
4. Reflect on the changes. If any code structure changed, write or update relevant memory pages using the `bw link` or `bw update` command.
";
        fs::write(refactor_program_path, refactor_content)
            .map_err(|e| format!("Failed to write refactor.md program: {}", e))?;
    }

    // 4. Create programs/document.md
    let document_program_path = programs_dir.join("document.md");
    if !document_program_path.exists() {
        let document_content = "\
# Program: Codebase Documentation

You are an automated documenter. Your task is to update or generate memory files documenting the files in this codebase.

## Instructions
1. Read the code files in the workspace.
2. Draft a clear overview of the system architecture.
3. Write or update memories explaining how key modules interact.
4. Link the memory pages to their respective code files using `bw link <memory> <code_file>`.
";
        fs::write(document_program_path, document_content)
            .map_err(|e| format!("Failed to write document.md program: {}", e))?;
    }

    let logs_dir = vault_path.join("logs");
    if !logs_dir.exists() {
        fs::create_dir_all(&logs_dir)
            .map_err(|e| format!("Failed to create logs directory: {}", e))?;
    }

    // 4.5. Create .gitignore inside the vault
    let gitignore_path = vault_path.join(".gitignore");
    if !gitignore_path.exists() {
        let default_gitignore = "\
# Ignore temporary runtime logs and visualizer files
logs/
ui/
";
        fs::write(gitignore_path, default_gitignore)
            .map_err(|e| format!("Failed to write .gitignore inside vault: {}", e))?;
    }

    // Ensure global config exists
    if let Some(global_path) = get_global_config_path() {
        if !global_path.exists() {
            if let Some(parent) = global_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let default_config = Config::default();
            if let Ok(content) = serde_json::to_string_pretty(&default_config) {
                let _ = fs::write(&global_path, content);
            }
        }
    }

    let config_path = vault_path.join("config.json");
    let config = if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read local config: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse local config: {}", e))?
    } else {
        // Local config will default to standard settings
        let default_config = Config::default();
        let content = serde_json::to_string_pretty(&default_config)
            .map_err(|e| format!("Failed to serialize default local config: {}", e))?;
        fs::write(&config_path, content)
            .map_err(|e| format!("Failed to write default local config: {}", e))?;
        default_config
    };

    Ok(config)
}


fn find_markdown_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir).map_err(|e| format!("Failed to read directory {:?}: {}", dir, e))? {
            let entry = entry.map_err(|e| format!("Directory entry error: {}", e))?;
            let path = entry.path();
            if path.is_dir() {
                find_markdown_files(&path, files)?;
            } else if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                files.push(path);
            }
        }
    }
    Ok(())
}

pub fn load_memories(vault_path: &Path) -> Result<Vec<MemoryPage>, String> {
    let mut memories = Vec::new();
    let mut loaded_names = std::collections::HashSet::new();

    // 1. Load Local Memories
    let local_memories_dir = vault_path.join("memories");
    if local_memories_dir.is_dir() {
        let mut files = Vec::new();
        if let Some(proj_name) = get_project_name() {
            // Load common memories directly in memories/ (non-recursive)
            if let Ok(entries) = fs::read_dir(&local_memories_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                        files.push(path);
                    }
                }
            }
            // Load project-specific memories in memories/<proj_name>/ (recursive)
            let project_memories_dir = local_memories_dir.join(&proj_name);
            if project_memories_dir.is_dir() {
                find_markdown_files(&project_memories_dir, &mut files)?;
            }
        } else {
            // Fallback: load all files recursively
            find_markdown_files(&local_memories_dir, &mut files)?;
        }

        for path in files {
            let content = fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read local memory file {:?}: {}", path, e))?;
            let mut page = parse_memory_file(&content, &path)?;
            
            if let Ok(rel_path) = path.strip_prefix(&local_memories_dir) {
                let mut final_rel = rel_path;
                if let Some(proj_name) = get_project_name() {
                    if let Ok(project_rel) = rel_path.strip_prefix(&proj_name) {
                        final_rel = project_rel;
                    }
                }
                let rel_str = final_rel.to_string_lossy().replace('\\', "/");
                let name = if rel_str.ends_with(".md") {
                    &rel_str[..rel_str.len() - 3]
                } else {
                    &rel_str
                };
                page.name = name.to_string();
            }

            loaded_names.insert(page.name.to_lowercase());
            memories.push(page);
        }
    }

    // 2. Load Global Memories
    if let Some(global_memories_dir) = get_global_memories_dir() {
        let target_dir = if let Some(proj_name) = get_project_name() {
            global_memories_dir.join(&proj_name)
        } else {
            global_memories_dir.clone()
        };

        if target_dir.is_dir() {
            let mut files = Vec::new();
            find_markdown_files(&target_dir, &mut files)?;
            for path in files {
                let content = fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read global memory file {:?}: {}", path, e))?;
                let mut page = parse_memory_file(&content, &path)?;
                
                if let Ok(rel_path) = path.strip_prefix(&global_memories_dir) {
                    let mut final_rel = rel_path;
                    if let Some(proj_name) = get_project_name() {
                        if let Ok(project_rel) = rel_path.strip_prefix(&proj_name) {
                            final_rel = project_rel;
                        }
                    }
                    let rel_str = final_rel.to_string_lossy().replace('\\', "/");
                    let name = if rel_str.ends_with(".md") {
                        &rel_str[..rel_str.len() - 3]
                    } else {
                        &rel_str
                    };
                    page.name = name.to_string();
                }

                let name_lower = page.name.to_lowercase();
                if !loaded_names.contains(&name_lower) {
                    loaded_names.insert(name_lower);
                    memories.push(page);
                }
            }
        }
    }

    Ok(memories)
}

pub fn normalize_memory_name(name: &str) -> String {
    let mut normalized = name.to_lowercase()
        .replace(" ", "-")
        .replace("_", "-")
        .replace(".md", "");
    
    // Strip brackets, parentheses, braces, and other symbols that break wiki-links or file paths
    normalized.retain(|c| {
        c != '[' && c != ']' && c != '(' && c != ')' && c != '{' && c != '}' && c != '*' && c != '?' && c != ':' && c != '"' && c != '\''
    });
    
    normalized
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

pub fn find_any_file_in_workspace(workspace_root: &Path) -> Option<String> {
    // Try root Cargo.toml
    if workspace_root.join("Cargo.toml").is_file() {
        return Some("Cargo.toml".to_string());
    }
    // Try root README.md
    if workspace_root.join("README.md").is_file() {
        return Some("README.md".to_string());
    }
    // Try any other file in workspace root first
    if let Ok(entries) = fs::read_dir(workspace_root) {
        for entry in entries {
            if let Ok(e) = entry {
                let path = e.path();
                if path.is_file() {
                    if let Some(name) = path.file_name() {
                        let name_str = name.to_string_lossy().to_string();
                        if !name_str.starts_with('.') && name_str != "package-lock.json" && name_str != "pnpm-lock.yaml" {
                            return Some(name_str);
                        }
                    }
                }
            }
        }
    }
    // Fallback to recursive scan
    let walker = walkdir::WalkDir::new(workspace_root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.') && name != "node_modules" && name != "target" && name != "bin" && name != "obj"
        });
        
    for entry in walker {
        if let Ok(e) = entry {
            let path = e.path();
            if path.is_file() {
                if let Ok(rel) = path.strip_prefix(workspace_root) {
                    return Some(rel.to_string_lossy().to_string());
                }
            }
        }
    }
    None
}

pub fn load_gitignore_patterns(workspace_root: &Path) -> Vec<String> {
    let mut patterns = Vec::new();
    let gitignore_path = workspace_root.join(".gitignore");
    if gitignore_path.is_file() {
        if let Ok(content) = fs::read_to_string(&gitignore_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
                    continue;
                }
                patterns.push(trimmed.to_string());
            }
        }
    }
    patterns
}


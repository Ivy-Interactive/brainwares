use crate::engine::{check_vault_status, ReferenceStatus};
use crate::hash::calculate_file_hash;
use crate::models::{CodeReference, Frontmatter, MemoryPage, MemoryType};
use crate::parser::{parse_memory_file, serialize_memory_file};
use crate::vault::{get_backlinks, get_workspace_root, init_vault, load_memories};
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};

// Helper to resolve memory name or path to the exact path of the memory file
fn resolve_memory_path(vault_path: &Path, input: &str) -> Result<PathBuf, String> {
    // 1. Check if input is already a path pointing to a file
    let path = PathBuf::from(input);
    if path.is_file() {
        return Ok(path);
    }

    let mut file_name = input.to_string();
    if !file_name.ends_with(".md") {
        file_name.push_str(".md");
    }

    // 2. Otherwise look up in local memories dir
    let local_memories_dir = vault_path.join("memories");
    let resolved = local_memories_dir.join(&file_name);
    if resolved.is_file() {
        return Ok(resolved);
    }

    // Try auto-resolving under a subfolder matching the resolved project name
    if let Some(proj_name) = crate::vault::get_project_name() {
        let project_resolved = local_memories_dir.join(&proj_name).join(&file_name);
        if project_resolved.is_file() {
            return Ok(project_resolved);
        }
    }

    // Try lowercased lookup in local memories dir
    if let Ok(entries) = fs::read_dir(&local_memories_dir) {
        let input_lower = file_name.to_lowercase();
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() && p.file_name().and_then(|n| n.to_str()).map(|s| s.to_lowercase()) == Some(input_lower.clone()) {
                return Ok(p);
            }
        }
    }

    // 3. Try global memories dir
    if let Some(global_memories_dir) = crate::vault::get_global_memories_dir() {
        let resolved_global = global_memories_dir.join(&file_name);
        if resolved_global.is_file() {
            return Ok(resolved_global);
        }

        // Lowercased lookup in global memories dir
        if let Ok(entries) = fs::read_dir(&global_memories_dir) {
            let input_lower = file_name.to_lowercase();
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && p.file_name().and_then(|n| n.to_str()).map(|s| s.to_lowercase()) == Some(input_lower.clone()) {
                    return Ok(p);
                }
            }
        }
    }

    Err(format!(
        "Memory file '{}' not found in local memories directory {:?} or global vault memories.",
        input, local_memories_dir
    ))
}

pub fn handle_init(vault_path: &Path) -> Result<(), String> {
    println!("Initializing brainwares vault at {:?}", vault_path);
    let _config = init_vault(vault_path)?;
    println!("SUCCESS: Vault initialized successfully.");
    println!("Directory structure created:");
    println!("  - memories/ (Obsidian-style notes)");
    println!("  - programs/ (Promptware instruction programs)");
    println!("  - logs/     (Execution history)");
    println!("  - config.json");
    Ok(())
}

pub fn handle_status(vault_path: &Path) -> Result<(), String> {
    let memories = load_memories(vault_path)?;
    let status = check_vault_status(vault_path, &memories);

    println!("================= VAULT STATUS =================");
    println!("Vault path: {:?}", vault_path);
    let file_count = status.memories.iter().filter(|m| m.memory_type == MemoryType::File).count();
    let user_count = status.memories.iter().filter(|m| m.memory_type == MemoryType::User).count();
    println!("Total memories: {} (File-based: {}, User-based: {})", status.total_memories, file_count, user_count);
    println!("------------------------------------------------");

    for m in &status.memories {
        let mut issues = Vec::new();

        for ref_status in &m.references {
            match &ref_status.status {
                ReferenceStatus::Ok => {}
                ReferenceStatus::Outdated { stored, current } => {
                    issues.push(format!(
                        "  [OUTDATED CODE] {} (stored: {}, current: {})",
                        ref_status.path,
                        &stored[..std::cmp::min(8, stored.len())],
                        &current[..std::cmp::min(8, current.len())]
                    ));
                }
                ReferenceStatus::Missing => {
                    issues.push(format!("  [MISSING CODE] {}", ref_status.path));
                }
            }
        }

        for broken in &m.broken_links {
            issues.push(format!("  [BROKEN LINK] [[{}]]", broken));
        }

        if m.is_orphan {
            issues.push("  [ORPHAN] Not linked by any other memory page".to_string());
        }

        if m.has_placeholders {
            issues.push("  [INCOMPLETE] Contains pending file descriptions ([Enter description...])".to_string());
        }

        if !issues.is_empty() {
            println!("Memory: {}", m.memory_name);
            for issue in issues {
                println!("{}", issue);
            }
            println!();
        }
    }

    println!("------------------------------------------------");
    println!("Outdated memories:     {}", status.outdated_memories_count);
    println!("Broken wiki-links:     {}", status.broken_links_count);
    println!("Orphan memories:       {}", status.orphan_count);
    println!("Incomplete templates:  {}", status.incomplete_memories_count);
    println!("================================================");

    Ok(())
}

pub fn handle_add(
    vault_path: &Path,
    name: String,
    tags: Option<String>,
    title: Option<String>,
    global: bool,
    memory_type: Option<String>,
) -> Result<(), String> {
    let memories_dir = if global {
        let dir = crate::vault::get_global_memories_dir()
            .ok_or_else(|| "Could not locate global memories directory".to_string())?;
        if !dir.exists() {
            fs::create_dir_all(&dir)
                .map_err(|e| format!("Failed to create global memories directory: {}", e))?;
        }
        dir
    } else {
        let mut dir = vault_path.join("memories");
        if !dir.exists() {
            return Err("Vault not initialized. Run 'bw init' first.".to_string());
        }

        if !name.contains('/') && !name.contains('\\') {
            if let Some(proj_name) = crate::vault::get_project_name() {
                dir = dir.join(proj_name);
            }
        }
        dir
    };

    let mut safe_name = name.trim().replace(" ", "-");
    if !safe_name.ends_with(".md") {
        safe_name.push_str(".md");
    }

    let file_path = memories_dir.join(&safe_name);
    if file_path.exists() {
        return Err(format!("Memory note at {:?} already exists.", file_path));
    }

    let parsed_tags = tags
        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_else(Vec::new);

    let display_title = title.unwrap_or_else(|| {
        name.trim()
            .replace("-", " ")
            .replace("_", " ")
            .to_string()
    });

    let parsed_type = match memory_type.as_deref() {
        Some("file") => Some(MemoryType::File),
        Some("user") => Some(MemoryType::User),
        Some(other) => return Err(format!("Invalid memory type '{}'. Must be 'file' or 'user'.", other)),
        None => None,
    };

    let fm = Frontmatter {
        title: Some(display_title.clone()),
        references: Some(Vec::new()),
        tags: Some(parsed_tags),
        last_updated: Some(Utc::now().to_rfc3339()),
        memory_type: parsed_type,
    };

    let page = MemoryPage {
        file_path: file_path.clone(),
        name: file_path.file_stem().unwrap().to_string_lossy().to_string(),
        frontmatter: fm,
        body: format!("# {}\n\nWrite your memory here...\n", display_title),
    };

    let serialized = serialize_memory_file(&page)?;
    if let Some(parent) = file_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create memory subdirectories: {}", e))?;
        }
    }
    fs::write(&file_path, serialized)
        .map_err(|e| format!("Failed to write memory note: {}", e))?;

    println!("SUCCESS: Created memory page at {:?}", file_path);
    Ok(())
}

pub fn handle_remove(
    vault_path: &Path,
    name: String,
    global: bool,
) -> Result<(), String> {
    let memories_dir = if global {
        crate::vault::get_global_memories_dir()
            .ok_or_else(|| "Could not locate global memories directory".to_string())?
    } else {
        let mut dir = vault_path.join("memories");
        if !dir.exists() {
            return Err("Vault not initialized. Run 'bw init' first.".to_string());
        }

        if !name.contains('/') && !name.contains('\\') {
            if let Some(proj_name) = crate::vault::get_project_name() {
                dir = dir.join(proj_name);
            }
        }
        dir
    };

    let mut safe_name = name.trim().replace(" ", "-");
    if !safe_name.ends_with(".md") {
        safe_name.push_str(".md");
    }

    let file_path = memories_dir.join(&safe_name);
    if !file_path.exists() {
        // Try case-insensitive lookup
        if let Ok(entries) = fs::read_dir(&memories_dir) {
            let input_lower = safe_name.to_lowercase();
            let mut found_path = None;
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && p.file_name().and_then(|n| n.to_str()).map(|s| s.to_lowercase()) == Some(input_lower.clone()) {
                    found_path = Some(p);
                    break;
                }
            }
            if let Some(p) = found_path {
                fs::remove_file(&p)
                    .map_err(|e| format!("Failed to remove memory note: {}", e))?;
                println!("SUCCESS: Removed memory page at {:?}", p);
                return Ok(());
            }
        }
        return Err(format!("Memory note '{}' not found in vault.", name));
    }

    fs::remove_file(&file_path)
        .map_err(|e| format!("Failed to remove memory note: {}", e))?;

    println!("SUCCESS: Removed memory page at {:?}", file_path);
    Ok(())
}

pub fn handle_link(vault_path: &Path, memory: String, code_file: String) -> Result<(), String> {
    let workspace_root = get_workspace_root(vault_path);
    let code_path = workspace_root.join(&code_file);
    if !code_path.exists() {
        return Err(format!("Code file not found in workspace: {:?}", code_file));
    }

    let hash = calculate_file_hash(&code_path)?;
    let memory_file = resolve_memory_path(vault_path, &memory)?;

    let content = fs::read_to_string(&memory_file)
        .map_err(|e| format!("Failed to read memory file: {}", e))?;
    
    let mut page = parse_memory_file(&content, &memory_file)?;

    let mut refs = page.frontmatter.references.unwrap_or_default();
    
    // Check if reference already exists, if so update hash
    if let Some(pos) = refs.iter().position(|r| r.path == code_file) {
        refs[pos].hash = hash.clone();
        println!("Updating link to code file '{}' with hash '{}'", code_file, hash);
    } else {
        refs.push(CodeReference {
            path: code_file.clone(),
            hash: hash.clone(),
        });
        println!("Adding new link to code file '{}' with hash '{}'", code_file, hash);
    }

    page.frontmatter.references = Some(refs);
    page.frontmatter.last_updated = Some(Utc::now().to_rfc3339());

    let serialized = serialize_memory_file(&page)?;
    fs::write(&memory_file, serialized)
        .map_err(|e| format!("Failed to update memory file: {}", e))?;

    println!("SUCCESS: Reference linked in memory '{}'", page.name);
    Ok(())
}

pub fn handle_update(
    vault_path: &Path,
    memory: String,
    code_file: Option<String>,
) -> Result<(), String> {
    let workspace_root = get_workspace_root(vault_path);
    let memory_file = resolve_memory_path(vault_path, &memory)?;

    let content = fs::read_to_string(&memory_file)
        .map_err(|e| format!("Failed to read memory file: {}", e))?;
    
    let mut page = parse_memory_file(&content, &memory_file)?;

    let mut refs = match page.frontmatter.references {
        Some(r) => r,
        None => return Err("Memory has no references to update.".to_string()),
    };

    let mut modified = false;

    if let Some(target_file) = code_file {
        // Update specific reference
        let idx = refs.iter().position(|r| r.path == target_file)
            .ok_or_else(|| format!("Reference to '{}' not found in memory frontmatter.", target_file))?;

        let code_path = workspace_root.join(&target_file);
        if !code_path.exists() {
            refs.remove(idx);
            println!("Pruned reference to deleted file '{}'", target_file);
            modified = true;
        } else {
            let new_hash = calculate_file_hash(&code_path)?;
            if refs[idx].hash != new_hash {
                refs[idx].hash = new_hash;
                modified = true;
            }
            println!("Updated hash for '{}'", target_file);
        }
    } else {
        // Update all references
        let mut updated_refs = Vec::new();
        for r in refs {
            let code_path = workspace_root.join(&r.path);
            if code_path.exists() {
                let mut updated_r = r.clone();
                if let Ok(new_hash) = calculate_file_hash(&code_path) {
                    if updated_r.hash != new_hash {
                        println!("Updated hash for '{}' from '{}' to '{}'", updated_r.path, updated_r.hash, new_hash);
                        updated_r.hash = new_hash;
                        modified = true;
                    }
                }
                updated_refs.push(updated_r);
            } else {
                println!("Pruned reference to deleted file '{}'", r.path);
                modified = true;
            }
        }
        refs = updated_refs;
    }

    if modified {
        page.frontmatter.references = if refs.is_empty() { None } else { Some(refs) };
        page.frontmatter.last_updated = Some(Utc::now().to_rfc3339());

        let serialized = serialize_memory_file(&page)?;
        fs::write(&memory_file, serialized)
            .map_err(|e| format!("Failed to write updated memory file: {}", e))?;

        println!("SUCCESS: Updated references for '{}'", page.name);
    } else {
        println!("No changes detected for '{}'", page.name);
    }
    Ok(())
}

pub fn handle_shake(vault_path: &Path) -> Result<(), String> {
    let memories = load_memories(vault_path)?;
    let status = check_vault_status(vault_path, &memories);

    println!("================= SHAKING VAULT =================");
    let mut broken_links_found = false;
    let mut orphans_found = false;

    for m in &status.memories {
        if !m.broken_links.is_empty() {
            broken_links_found = true;
            println!("Memory '{}' has broken wiki-links to:", m.memory_name);
            for broken in &m.broken_links {
                println!("  - [[{}]]", broken);
            }
        }

        if m.is_orphan {
            orphans_found = true;
            println!("Orphan Memory note found: '{}' ({:?})", m.memory_name, m.file_path);
        }
    }

    if !broken_links_found {
        println!("No broken wiki-links found.");
    }
    if !orphans_found {
        println!("No orphan memory notes found.");
    }

    // Prune references to deleted files across all memories
    let workspace_root = get_workspace_root(vault_path);
    let mut pruned_refs_count = 0;
    for page in &memories {
        if let Some(refs) = &page.frontmatter.references {
            let mut updated_refs = Vec::new();
            let mut modified = false;
            for r in refs {
                let code_path = workspace_root.join(&r.path);
                if code_path.exists() {
                    updated_refs.push(r.clone());
                } else {
                    println!("Memory '{}': Pruned reference to deleted file '{}'", page.name, r.path);
                    pruned_refs_count += 1;
                    modified = true;
                }
            }
            if modified {
                let mut updated_page = page.clone();
                updated_page.frontmatter.references = if updated_refs.is_empty() { None } else { Some(updated_refs) };
                updated_page.frontmatter.last_updated = Some(Utc::now().to_rfc3339());
                
                let serialized = serialize_memory_file(&updated_page)?;
                fs::write(&page.file_path, serialized)
                    .map_err(|e| format!("Failed to write updated memory file '{:?}': {}", page.file_path, e))?;
            }
        }
    }

    if pruned_refs_count > 0 {
        println!("Pruned {} references to deleted files.", pruned_refs_count);
    } else {
        println!("No deleted file references found to prune.");
    }

    // Clean up empty logs
    let logs_dir = vault_path.join("logs");
    let mut pruned_logs = 0;
    if logs_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(logs_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    if let Ok(metadata) = p.metadata() {
                        if metadata.len() == 0 {
                            if fs::remove_file(&p).is_ok() {
                                pruned_logs += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    if pruned_logs > 0 {
        println!("Pruned {} empty log files from logs/", pruned_logs);
    } else {
        println!("No empty log files found to prune.");
    }

    println!("=================================================");
    Ok(())
}

pub fn handle_query(vault_path: &Path, term: String) -> Result<(), String> {
    let memories = load_memories(vault_path)?;
    let backlinks = get_backlinks(&memories);
    
    // Split the query into lowercase tokens
    let tokens: Vec<String> = term.split_whitespace().map(|s| s.to_lowercase()).collect();
    
    println!("Query results for '{}':", term);
    println!("------------------------------------------------");

    if tokens.is_empty() {
        println!("No query term provided.");
        return Ok(());
    }

    let mut scored_results = Vec::new();

    for page in &memories {
        let mut matches_all_tokens = true;
        let mut score = 0;

        for token in &tokens {
            let title_match = page.frontmatter.title.as_ref().map(|t| t.to_lowercase().contains(token)).unwrap_or(false);
            let name_match = page.name.to_lowercase().contains(token);
            let tags_match = page.frontmatter.tags.as_ref().map(|tags| tags.iter().any(|tag| tag.to_lowercase().contains(token))).unwrap_or(false);
            let body_match = page.body.to_lowercase().contains(token);

            if !(name_match || title_match || tags_match || body_match) {
                matches_all_tokens = false;
                break;
            }

            // Calculate relevance score
            if name_match {
                score += 20;
            }
            if title_match {
                score += 15;
            }
            if tags_match {
                score += 10;
            }
            if body_match {
                // Find count of occurrences in the body
                let count = page.body.to_lowercase().matches(token).count();
                score += std::cmp::min(10, count * 2);
            }
        }

        if matches_all_tokens {
            scored_results.push((page, score));
        }
    }

    // Sort by score in descending order
    scored_results.sort_by(|a, b| b.1.cmp(&a.1));

    let match_count = scored_results.len();

    for (page, score) in &scored_results {
        println!("Memory: {} ({:?}) [Score: {}]", page.name, page.file_path.file_name().unwrap(), score);
        if let Some(t) = &page.frontmatter.title {
            println!("  Title: {}", t);
        }
        if let Some(tags) = &page.frontmatter.tags {
            println!("  Tags: {:?}", tags);
        }
        
        // Print brief content snippets containing at least one query token
        let mut printed_snippets = 0;
        let mut snippet_lines = Vec::new();
        for line in page.body.lines() {
            let line_lower = line.to_lowercase();
            let mut line_matched = false;
            for token in &tokens {
                if line_lower.contains(token) {
                    line_matched = true;
                    break;
                }
            }
            if line_matched {
                snippet_lines.push(line.trim());
                printed_snippets += 1;
                if printed_snippets >= 5 { // Limit snippets per page
                    break;
                }
            }
        }
        if !snippet_lines.is_empty() {
            println!("  Matching snippets:");
            for line in snippet_lines {
                println!("    ... {} ...", line);
            }
        }

        // Print backlinks
        let page_backlinks = backlinks.get(&page.name.to_lowercase());
        if let Some(bls) = page_backlinks {
            println!("  Backlinks (linked from):");
            for bl in bls {
                println!("    - [[{}]] in {}", bl.source_name, bl.context);
            }
        }

        println!();
    }

    if match_count == 0 {
        println!("No matching memory notes found.");
    } else {
        println!("Found {} matching memory notes.", match_count);
    }
    Ok(())
}

pub fn handle_read(vault_path: &Path, name: String) -> Result<(), String> {
    let memories = load_memories(vault_path)?;
    let memory_file = resolve_memory_path(vault_path, &name)?;

    let content = fs::read_to_string(&memory_file)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let page = parse_memory_file(&content, &memory_file)?;
    let workspace_root = get_workspace_root(vault_path);

    println!("=================================================");
    println!("Memory Name: {}", page.name);
    let inferred_type = page.frontmatter.memory_type
        .unwrap_or_else(|| {
            if page.frontmatter.references.as_ref().map(|r| !r.is_empty()).unwrap_or(false) {
                MemoryType::File
            } else {
                MemoryType::User
            }
        });
    let type_str = match inferred_type {
        MemoryType::File => "file",
        MemoryType::User => "user",
    };
    println!("Type:        {}", type_str);
    if let Some(title) = &page.frontmatter.title {
        println!("Title:       {}", title);
    }
    if let Some(tags) = &page.frontmatter.tags {
        println!("Tags:        {:?}", tags);
    }
    if let Some(ref_time) = &page.frontmatter.last_updated {
        println!("Updated:     {}", ref_time);
    }
    println!("=================================================");

    let mut is_outdated = false;
    if inferred_type == MemoryType::File {
        if let Some(refs) = &page.frontmatter.references {
            for r in refs {
                let code_path = workspace_root.join(&r.path);
                if !code_path.exists() {
                    is_outdated = true;
                    break;
                }
                match calculate_file_hash(&code_path) {
                    Ok(current_hash) => {
                        if current_hash != r.hash {
                            is_outdated = true;
                            break;
                        }
                    }
                    Err(_) => {
                        is_outdated = true;
                        break;
                    }
                }
            }
        }
    }
    if is_outdated {
        println!("WARNING: This memory page is potentially outdated due to code changes.");
        println!("=================================================");
    }

    // Verify references and print status
    if let Some(refs) = &page.frontmatter.references {
        if !refs.is_empty() {
            println!("Code References:");
            for r in refs {
                let code_path = workspace_root.join(&r.path);
                let status_str = if !code_path.exists() {
                    "MISSING".to_string()
                } else {
                    match calculate_file_hash(&code_path) {
                        Ok(current_hash) => {
                            if current_hash == r.hash {
                                "OK".to_string()
                            } else {
                                format!("OUTDATED (current: {})", &current_hash[..8])
                            }
                        }
                        Err(_) => "ERROR".to_string(),
                    }
                };
                println!("  - {} -> status: {}", r.path, status_str);
            }
            println!("=================================================");
        }
    }

    // Print backlinks
    let backlinks = get_backlinks(&memories);
    if let Some(bls) = backlinks.get(&page.name.to_lowercase()) {
        println!("Backlinks (what links here):");
        for bl in bls {
            println!("  - [[{}]] (in context: {})", bl.source_name, bl.context);
        }
        println!("=================================================");
    }

    // Print outgoing wiki-links
    let outgoing = crate::parser::extract_wiki_links(&page.body);
    if !outgoing.is_empty() {
        println!("Outgoing Links (wiki-links in this page):");
        let mut unique_outgoing = std::collections::HashSet::new();
        for (target, _) in outgoing {
            let target_normalized = crate::vault::normalize_memory_name(&target);
            if unique_outgoing.insert(target_normalized.clone()) {
                println!("  - [[{}]]", target);
            }
        }
        println!("=================================================");
    }

    println!("\n{}", page.body);
    Ok(())
}

pub fn handle_compile(vault_path: &Path, program: String, args: Vec<String>) -> Result<(), String> {
    let programs_dir = vault_path.join("programs");
    let mut prog_file = program.clone();
    if !prog_file.ends_with(".md") {
        prog_file.push_str(".md");
    }

    let program_path = programs_dir.join(&prog_file);
    if !program_path.is_file() {
        return Err(format!("Program instruction file not found at {:?}", program_path));
    }

    let program_instructions = fs::read_to_string(&program_path)
        .map_err(|e| format!("Failed to read program file: {}", e))?;

    // Load memories to check references and include recent state
    let memories = load_memories(vault_path).unwrap_or_default();
    
    // Construct the prompt payload
    let mut compiled_prompt = String::new();
    compiled_prompt.push_str("You are an agentic application that evolves over time.\n\n");
    
    compiled_prompt.push_str("## Program Folder\n");
    compiled_prompt.push_str(&format!("Vault path is located at: {:?}\n\n", vault_path));

    if !args.is_empty() {
        compiled_prompt.push_str("## Arguments\n");
        compiled_prompt.push_str(&format!("Arguments provided: {:?}\n\n", args));
    }

    let workspace_root = get_workspace_root(vault_path);
    compiled_prompt.push_str("## Memories\n");
    compiled_prompt.push_str("Below are the current active memories from your database:\n");
    for page in &memories {
        compiled_prompt.push_str(&format!("=== Memory: {} ===\n", page.name));
        if let Some(title) = &page.frontmatter.title {
            compiled_prompt.push_str(&format!("Title: {}\n", title));
        }

        // Check if page is outdated:
        let inferred_type = page.frontmatter.memory_type.clone()
            .unwrap_or_else(|| {
                if page.frontmatter.references.as_ref().map(|r| !r.is_empty()).unwrap_or(false) {
                    MemoryType::File
                } else {
                    MemoryType::User
                }
            });
        let mut is_outdated = false;
        if inferred_type == MemoryType::File {
            if let Some(refs) = &page.frontmatter.references {
                for r in refs {
                    let code_path = workspace_root.join(&r.path);
                    if !code_path.exists() {
                        is_outdated = true;
                        break;
                    }
                    match calculate_file_hash(&code_path) {
                        Ok(current_hash) => {
                            if current_hash != r.hash {
                                is_outdated = true;
                                break;
                            }
                        }
                        Err(_) => {
                            is_outdated = true;
                            break;
                        }
                    }
                }
            }
        }
        if is_outdated {
            compiled_prompt.push_str("WARNING: This memory page is potentially outdated due to code changes.\n");
        }

        compiled_prompt.push_str(&format!("Content:\n{}\n\n", page.body));
    }

    compiled_prompt.push_str("## Program Instructions\n");
    compiled_prompt.push_str("Follow these step-by-step instructions to execute the task:\n");
    compiled_prompt.push_str(&program_instructions);
    
    compiled_prompt.push_str("\n\n## Reflection Loop\n");
    compiled_prompt.push_str("At the end of your run, if you changed code files, write memories or update referenced file hashes in the vault to keep memories verified.\n");

    println!("{}", compiled_prompt);
    Ok(())
}

const GLOBAL_RULES_CONTENT: &str = r#"# Antigravity Global Rules

## Brainwares Workspace Integration

Whenever a `.brainwares` vault directory is detected in the active workspace, the agent MUST automatically hook up with it:

1. **Context Discovery**:
   - Before executing tasks, check for the presence of a `.brainwares/` folder.
   - If present, run `bw status` to scan for any outdated references or broken wiki-links.
   - Automatically load and read relevant memories using `bw read <note_name>` or `bw query <term>` to gain full context about coding guidelines, rules, or system details.

2. **Reference Maintenance & Reflection**:
   - After completing edits on any codebase files, run `bw status` to see if your changes caused any memory notes to become outdated.
   - If references are outdated, inspect the associated markdown memory file, update its content to reflect the new codebase state, and run `bw update <note_name>` to synchronize the hashes.
   - If you created any new source or configuration files, proactively document them in a memory note (creating a new one if necessary) and run `bw link <note_name> <file_path>` to register their initial hashes.
   - Keep the vault clean and green before completing your turn.
"#;

const AGENT_RULES_CONTENT: &str = r#"# Brainwares Workspace Integration Rules
 
This repository uses **Brainwares** (`bw`) for Obsidian-style markdown memory storage and code-reference hash tracking.
 
You MUST follow these rules during your session:
1. **Status Audit**: Before writing or changing code, run `bw status` to check if there are any outdated memory files or broken links.
2. **Context Resolution**: Read relevant memories using `bw read <note_name>` or search memories with `bw query <term>` to gain full context. Note that `bw query` supports tokenized multi-word search queries (words are matched across fields, and results are ranked by relevance).
3. **Reference Maintenance**: After modifying any code files in the codebase, run `bw status`. If any references are outdated, inspect the associated markdown note under `.brainwares/memories/`, update its contents to reflect the new codebase state, and run `bw update <note_name>` to synchronize the hashes. If you created any new source or configuration files, document them in a memory note (creating a new one if necessary) and run `bw link <note_name> <file_path>` to link and track them.
4. **Dependency Mapping**: When adding or updating memory notes, you MUST scan the code file's imports and dependencies. Add double-bracket Obsidian-style wiki-links (e.g. `[[dependency-note-name]]`) in the memory page content to establish relationships between modules and tests. Do NOT assume dependencies follow the folder structure; map them based on code imports and usage to produce a complete dependency/brain map.
5. **Clean State**: Keep the vault clean and verified before completing your task.
"#;

pub fn handle_rules() -> Result<(), String> {
    println!("{}", AGENT_RULES_CONTENT);
    Ok(())
}

pub fn handle_integrate() -> Result<(), String> {
    // 1. Configure Global Antigravity Integration
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| "Could not find home directory environment variable (HOME or USERPROFILE)".to_string())?;
    
    let gemini_config_dir = PathBuf::from(home).join(".gemini").join("config");
    if !gemini_config_dir.exists() {
        if let Err(e) = fs::create_dir_all(&gemini_config_dir) {
            println!("WARNING: Failed to create Gemini config directory: {}", e);
        }
    }

    let old_global_rules_str = r#"# Antigravity Global Rules

## Brainwares Workspace Integration

Whenever a `.brainwares` vault directory is detected in the active workspace, the agent MUST automatically hook up with it:

1. **Context Discovery**:
   - Before executing tasks, check for the presence of a `.brainwares/` folder.
   - If present, run `bw status` to scan for any outdated references or broken wiki-links.
   - Automatically load and read relevant memories using `bw read <note_name>` or `bw query <term>` to gain full context about coding guidelines, rules, or system details.

2. **Reference Maintenance & Reflection**:
   - After completing edits on any codebase files, run `bw status` to see if your changes caused any memory notes to become outdated.
   - If references are outdated, inspect the associated markdown memory file, update its content to reflect the new codebase state, and run `bw update <note_name>` to synchronize the hashes.
   - Keep the vault clean and green before completing your turn."#;

    if gemini_config_dir.exists() {
        let agents_md_path = gemini_config_dir.join("AGENTS.md");
        let mut current_content = String::new();
        if agents_md_path.is_file() {
            if let Ok(c) = fs::read_to_string(&agents_md_path) {
                current_content = c;
            }
        }

        if !current_content.contains("Brainwares Workspace Integration") {
            let separator = if current_content.is_empty() || current_content.ends_with('\n') { "" } else { "\n\n" };
            let new_content = format!("{}{}{}", current_content, separator, GLOBAL_RULES_CONTENT);
            if fs::write(&agents_md_path, new_content).is_ok() {
                println!("SUCCESS: Configured global Antigravity rules at {:?}", agents_md_path);
            }
        } else if current_content.contains(old_global_rules_str) {
            let updated = current_content.replace(old_global_rules_str, GLOBAL_RULES_CONTENT);
            if fs::write(&agents_md_path, updated).is_ok() {
                println!("SUCCESS: Updated global Antigravity rules at {:?}", agents_md_path);
            }
        } else if !current_content.contains("proactively document them") {
            let old_short = "If references are outdated, inspect the associated markdown memory file, update its content to reflect the new codebase state, and run `bw update <note_name>` to synchronize the hashes.\n   - Keep the vault clean and green before completing your turn.";
            let new_short = "If references are outdated, inspect the associated markdown memory file, update its content to reflect the new codebase state, and run `bw update <note_name>` to synchronize the hashes.\n   - If you created any new source or configuration files, proactively document them in a memory note (creating a new one if necessary) and run `bw link <note_name> <file_path>` to register their initial hashes.\n   - Keep the vault clean and green before completing your turn.";
            if current_content.contains(old_short) {
                let updated = current_content.replace(old_short, new_short);
                if fs::write(&agents_md_path, updated).is_ok() {
                    println!("SUCCESS: Updated global Antigravity rules at {:?}", agents_md_path);
                }
            } else {
                println!("INFO: Global Antigravity rules already configured (custom format).");
            }
        } else {
            println!("INFO: Global Antigravity rules already configured.");
        }
    }

    // 2. Configure Local Workspace Rules (CLAUDE.md, .cursorrules, .windsurfrules)
    let local_vault = PathBuf::from(".brainwares");
    if local_vault.is_dir() {
        println!("Configuring agent integration rules for local workspace...");
        
        let files_to_create = vec![
            ("CLAUDE.md", "Claude Code"),
            (".cursorrules", "Cursor"),
            (".windsurfrules", "Windsurf"),
            ("AGENTS.md", "OpenCode"),
        ];

        for (filename, agent_name) in files_to_create {
            let path = PathBuf::from(filename);
            let mut current = String::new();
            if path.is_file() {
                if let Ok(content) = fs::read_to_string(&path) {
                    current = content;
                }
            }

            if let Some(pos) = current.find("# Brainwares Workspace Integration Rules") {
                if !current.contains("it does NOT do semantic search") {
                    let prefix = &current[..pos];
                    let new_content = format!("{}{}", prefix, AGENT_RULES_CONTENT);
                    if fs::write(&path, new_content).is_ok() {
                        println!("SUCCESS: Updated {} integration rules in {}", agent_name, filename);
                    } else {
                        println!("WARNING: Failed to write to {}", filename);
                    }
                } else {
                    println!("INFO: {} rules already configured in {}.", agent_name, filename);
                }
            } else {
                let separator = if current.is_empty() || current.ends_with('\n') { "" } else { "\n\n" };
                let new_content = format!("{}{}{}", current, separator, AGENT_RULES_CONTENT);
                if fs::write(&path, new_content).is_ok() {
                    println!("SUCCESS: Configured {} integration rules in {}", agent_name, filename);
                } else {
                    println!("WARNING: Failed to write to {}", filename);
                }
            }
        }
    } else {
        println!("INFO: Local workspace .brainwares vault not found. Local rules integration skipped.");
        println!("      -> Run 'bw init' first to set up a vault, then run 'bw integrate' in the project root.");
    }

    Ok(())
}

pub fn handle_doctor() -> Result<(), String> {
    println!("Checking Brainwares system configuration...");
    println!("------------------------------------------------");

    // 1. Check PATH executable
    let mut bw_ok = std::process::Command::new("bw")
        .arg("--help")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok();

    if !bw_ok {
        bw_ok = std::process::Command::new("brainwares")
            .arg("--help")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok();
    }

    if bw_ok {
        println!("[✓] Brainwares CLI binary is executable and in your PATH.");
    } else {
        println!("[✗] Brainwares CLI binary was not found in PATH.");
        println!("    -> To fix this, run: cargo install --path .");
    }

    // 2. Check Global Agent Integration
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();
    
    let agents_md_path = PathBuf::from(&home).join(".gemini").join("config").join("AGENTS.md");
    let mut integration_ok = false;
    if agents_md_path.is_file() {
        if let Ok(content) = fs::read_to_string(&agents_md_path) {
            if content.contains("Brainwares Workspace Integration") {
                integration_ok = true;
            }
        }
    }

    if integration_ok {
        println!("[✓] Antigravity Global Agent rules are configured at {:?}", agents_md_path);
    } else {
        println!("[✗] Antigravity Global Agent rules are NOT configured.");
        println!("    -> To fix this, run: bw integrate");
    }

    // 3. Check Workspace initialization
    let local_vault = PathBuf::from(".brainwares");
    if local_vault.is_dir() {
        println!("[✓] Local workspace has a .brainwares vault initialized.");
        
        // 4. Check Workspace Agent Rules Files
        let workspace_rules = vec![
            ("CLAUDE.md", "Claude Code"),
            (".cursorrules", "Cursor"),
            (".windsurfrules", "Windsurf"),
        ];

        for (filename, agent_name) in workspace_rules {
            let path = PathBuf::from(filename);
            let mut configured = false;
            if path.is_file() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if content.contains("Brainwares Workspace Integration Rules") {
                        configured = true;
                    }
                }
            }

            if configured {
                println!("[✓] {} integration rules are configured in local {}", agent_name, filename);
            } else {
                println!("[✗] {} integration rules are NOT configured in local {}", agent_name, filename);
                println!("    -> To fix this, run: bw integrate");
            }
        }
    } else {
        println!("[ ] Local workspace does not have a .brainwares vault initialized.");
        println!("    -> Run 'bw init' to bootstrap a vault in this project.");
    }

    // 5. Check configuration files
    if let Some(global_config_path) = crate::vault::get_global_config_path() {
        if global_config_path.is_file() {
            println!("[✓] User-wide global config found at {:?}", global_config_path);
            let global_config = crate::vault::load_global_config();
            println!("    Default vault folder name: '{}'", global_config.default_vault_dir);
            println!("    Global ignore patterns: {:?}", global_config.ignore_patterns);
        } else {
            println!("[ ] Global config not found (will be initialized upon running 'bw init').");
        }
    }

    if local_vault.is_dir() {
        if let Some(local_config) = crate::vault::load_local_config(&local_vault) {
            println!("[✓] Repository-wide local config found.");
            println!("    Local ignore patterns: {:?}", local_config.ignore_patterns);
            
            let merged = crate::vault::load_merged_config(&local_vault);
            println!("    Effective merged ignore patterns: {:?}", merged.ignore_patterns);
        } else {
            println!("[✗] Local config file config.json not found in vault.");
        }
    }

    println!("------------------------------------------------");
    Ok(())
}


pub fn handle_index(vault_path: &Path) -> Result<(), String> {
    let workspace_root = get_workspace_root(vault_path);
    let merged_config = crate::vault::load_merged_config(vault_path);
    let memories_dir = vault_path.join("memories");
    
    println!("Indexing codebase under workspace: {:?}", workspace_root);
    
    // Group files by parent directory path
    let mut dir_files: std::collections::HashMap<PathBuf, Vec<crate::models::CodeReference>> = std::collections::HashMap::new();
    let mut dir_set: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    
    // Setup gitignore-aware WalkBuilder
    let mut builder = ignore::WalkBuilder::new(&workspace_root);
    builder
        .standard_filters(true)
        .hidden(true);
        
    // Add custom ignore overrides from config
    let mut override_builder = ignore::overrides::OverrideBuilder::new(&workspace_root);
    for pattern in &merged_config.ignore_patterns {
        let clean = pattern.trim_start_matches("**/").trim_end_matches('/');
        if !clean.is_empty() {
            let _ = override_builder.add(&format!("!{}", clean));
        }
    }
    if let Ok(overrides) = override_builder.build() {
        builder.overrides(overrides);
    }
    
    let walker = builder.build();
    
    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        
        // We only index files
        if !path.is_file() {
            continue;
        }
        
        // Skip common binary files
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            let skip_exts = ["png", "jpg", "jpeg", "gif", "ico", "svg", "lock", "db", "bin", "exe", "wasm", "node_modules"];
            if skip_exts.contains(&ext_str.as_str()) {
                continue;
            }
        }
        
        let parent = match path.parent() {
            Some(p) => p.to_path_buf(),
            None => continue,
        };
        
        // Skip root directory files as the main entry note is index.md
        if parent == workspace_root {
            continue;
        }
        
        // Skip hidden paths
        if parent.components().any(|c| c.as_os_str().to_string_lossy().starts_with('.')) {
            continue;
        }
        
        let rel_path = match path.strip_prefix(&workspace_root) {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(_) => continue,
        };
        
        let hash = match crate::hash::calculate_file_hash(path) {
            Ok(h) => h,
            Err(_) => continue,
        };
        
        dir_files.entry(parent.clone()).or_default().push(crate::models::CodeReference {
            path: rel_path,
            hash,
        });
        
        // Walk up parent ancestors to build the directory tree hierarchy
        let mut current = parent;
        while current != workspace_root {
            dir_set.insert(current.clone());
            if let Some(p) = current.parent() {
                current = p.to_path_buf();
            } else {
                break;
            }
        }
    }
    
    let mut scaffolded_count = 0;
    let mut top_level_dirs = Vec::new();
    
    for dir_path in &dir_set {
        let references = dir_files.get(dir_path).cloned().unwrap_or_default();
        
        // Find direct child subdirectories in our indexed set
        let mut subdirs = Vec::new();
        for other_path in &dir_set {
            if other_path.parent() == Some(dir_path) {
                subdirs.push(other_path.clone());
            }
        }
        subdirs.sort();
        
        // Keep track of top-level directories to link in index.md
        if dir_path.parent() == Some(&workspace_root) {
            top_level_dirs.push(dir_path.clone());
        }
        
        let rel_path = match dir_path.strip_prefix(&workspace_root) {
            Ok(p) => p,
            Err(_) => continue,
        };
        
        let rel_path_str = rel_path.to_string_lossy().to_string();
        if rel_path_str.is_empty() {
            continue;
        }
        
        // Normalize name: replace path slashes with hyphens
        let normalized_note_name = rel_path_str
            .replace('\\', "-")
            .replace('/', "-");
        let memory_name = crate::vault::normalize_memory_name(&normalized_note_name);
        let note_path = memories_dir.join(format!("{}.md", memory_name));
        
        // If the note already exists, don't overwrite it
        if note_path.exists() {
            continue;
        }
        
        // Humanize title (e.g. "ivy-framework/src" -> "Ivy Framework Src")
        let title = humanize_title(&normalized_note_name);
        
        // Build frontmatter
        let frontmatter = crate::models::Frontmatter {
            title: Some(title.clone()),
            tags: Some(vec!["folder".to_string(), "index".to_string()]),
            references: if references.is_empty() { None } else { Some(references.clone()) },
            last_updated: Some(chrono::Utc::now().to_rfc3339()),
            memory_type: Some(MemoryType::File),
        };
        
        // Build markdown body
        let mut body = format!("# {}\n\nScaffolded memory page for the `{}` directory.\n\n", title, rel_path_str);
        
        if !subdirs.is_empty() {
            body.push_str("## Subdirectories\n\n");
            for subdir in &subdirs {
                if let Ok(sub_rel) = subdir.strip_prefix(&workspace_root) {
                    let sub_rel_str = sub_rel.to_string_lossy().to_string();
                    let normalized_sub = sub_rel_str.replace('\\', "-").replace('/', "-");
                    let sub_memory_name = crate::vault::normalize_memory_name(&normalized_sub);
                    let sub_title = humanize_title(&normalized_sub);
                    body.push_str(&format!("- [[{}]] ({})\n", sub_memory_name, sub_title));
                }
            }
            body.push_str("\n");
        }
        
        if !references.is_empty() {
            body.push_str("## Core Files Reference Map\n\n");
            for ref_item in &references {
                let file_name = Path::new(&ref_item.path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| ref_item.path.clone());
                body.push_str(&format!("*   `{}`: [Enter description for file's role in this folder]\n", file_name));
            }
        }
        
        let memory_page = crate::models::MemoryPage {
            name: memory_name.clone(),
            frontmatter,
            body,
            file_path: note_path.clone(),
        };
        
        // Write the note to file
        let serialized = serialize_memory_file(&memory_page)?;
        fs::write(&memory_page.file_path, serialized)
            .map_err(|e| format!("Failed to write memory note: {}", e))?;
            
        println!("SUCCESS: Created memory page [[{}]] referencing {} files.", memory_name, references.len());
        scaffolded_count += 1;
    }
    
    // Sort top level dirs for deterministic indexing in index.md
    top_level_dirs.sort();
    
    // Update index.md with Codebase Directories if they are not already listed
    let index_path = memories_dir.join("index.md");
    if index_path.exists() && !top_level_dirs.is_empty() {
        if let Ok(mut content) = fs::read_to_string(&index_path) {
            if !content.contains("## Codebase Directories") {
                let mut dir_block = "\n\n## Codebase Directories\n\n".to_string();
                for dir_path in &top_level_dirs {
                    if let Ok(rel) = dir_path.strip_prefix(&workspace_root) {
                        let rel_str = rel.to_string_lossy().to_string();
                        let normalized = rel_str.replace('\\', "-").replace('/', "-");
                        let memory_name = crate::vault::normalize_memory_name(&normalized);
                        dir_block.push_str(&format!("- [[{}]]\n", memory_name));
                    }
                }
                content.push_str(&dir_block);
                let _ = fs::write(&index_path, content);
            }
        }
    }
    
    println!("------------------------------------------------");
    println!("Indexing completed. Created {} new memory notes.", scaffolded_count);
    Ok(())
}

fn humanize_title(name: &str) -> String {
    let clean_name = name.replace("[[", "[").replace("]]", "]");
    clean_name.split(&['_', '-', '/'])
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn handle_write(
    vault_path: &Path,
    memory: String,
    content: String,
) -> Result<(), String> {
    let memory_file = match resolve_memory_path(vault_path, &memory) {
        Ok(path) => path,
        Err(_) => {
            let memories_dir = vault_path.join("memories");
            if !memories_dir.exists() {
                return Err("Vault not initialized. Run 'bw init' first.".to_string());
            }
            let mut safe_name = memory.trim().replace(" ", "-");
            if !safe_name.ends_with(".md") {
                safe_name.push_str(".md");
            }
            memories_dir.join(&safe_name)
        }
    };

    let mut page = if memory_file.exists() {
        let file_content = fs::read_to_string(&memory_file)
            .map_err(|e| format!("Failed to read memory file: {}", e))?;
        parse_memory_file(&file_content, &memory_file)?
    } else {
        let display_title = memory.trim()
            .replace("-", " ")
            .replace("_", " ")
            .to_string();
        MemoryPage {
            file_path: memory_file.clone(),
            name: memory_file.file_stem().unwrap().to_string_lossy().to_string(),
            frontmatter: Frontmatter {
                title: Some(display_title),
                references: Some(Vec::new()),
                tags: Some(Vec::new()),
                last_updated: None,
                memory_type: None,
            },
            body: String::new(),
        }
    };

    page.body = content;
    page.frontmatter.last_updated = Some(Utc::now().to_rfc3339());
    page.file_path = memory_file.clone();

    let serialized = serialize_memory_file(&page)?;
    fs::write(&memory_file, serialized)
        .map_err(|e| format!("Failed to write memory file: {}", e))?;

    println!("SUCCESS: Wrote content to memory page at {:?}", memory_file);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_memory_file;
    use std::fs;

    #[test]
    fn test_handle_update_pruning() {
        let temp_dir = std::env::temp_dir().join(format!("bw_test_update_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));
        fs::create_dir_all(&temp_dir).unwrap();

        // 1. Initialize vault in temp_dir
        let vault_path = temp_dir.join(".brainwares");
        handle_init(&vault_path).unwrap();

        // 2. Create a dummy code file and link it to a memory note
        let code_file_name = "test_code.rs";
        let code_file_path = temp_dir.join(code_file_name);
        fs::write(&code_file_path, "fn main() {}").unwrap();

        let memory_name = "my-test-memory";
        handle_add(&vault_path, memory_name.to_string(), None, None, false, None).unwrap();
        handle_link(&vault_path, memory_name.to_string(), code_file_name.to_string()).unwrap();

        // Check reference exists
        let memory_file = resolve_memory_path(&vault_path, memory_name).unwrap();
        let content = fs::read_to_string(&memory_file).unwrap();
        let page = parse_memory_file(&content, &memory_file).unwrap();
        assert_eq!(page.frontmatter.references.as_ref().unwrap().len(), 1);
        assert_eq!(page.frontmatter.references.as_ref().unwrap()[0].path, code_file_name);

        // 3. Delete the code file
        fs::remove_file(&code_file_path).unwrap();

        // 4. Run update and check that reference is pruned
        handle_update(&vault_path, memory_name.to_string(), None).unwrap();

        let content2 = fs::read_to_string(&memory_file).unwrap();
        let page2 = parse_memory_file(&content2, &memory_file).unwrap();
        assert!(page2.frontmatter.references.is_none() || page2.frontmatter.references.as_ref().unwrap().is_empty());

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_handle_shake_pruning() {
        let temp_dir = std::env::temp_dir().join(format!("bw_test_shake_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));
        fs::create_dir_all(&temp_dir).unwrap();

        // 1. Initialize vault in temp_dir
        let vault_path = temp_dir.join(".brainwares");
        handle_init(&vault_path).unwrap();

        // 2. Create dummy code files
        let code1 = "code1.rs";
        let code2 = "code2.rs";
        fs::write(temp_dir.join(code1), "const X: i32 = 1;").unwrap();
        fs::write(temp_dir.join(code2), "const Y: i32 = 2;").unwrap();

        // 3. Add memory notes and link
        let mem1 = "mem-one";
        let mem2 = "mem-two";
        handle_add(&vault_path, mem1.to_string(), None, None, false, None).unwrap();
        handle_add(&vault_path, mem2.to_string(), None, None, false, None).unwrap();

        handle_link(&vault_path, mem1.to_string(), code1.to_string()).unwrap();
        handle_link(&vault_path, mem2.to_string(), code2.to_string()).unwrap();

        // Delete code1.rs, keep code2.rs
        fs::remove_file(temp_dir.join(code1)).unwrap();

        // Run shake
        handle_shake(&vault_path).unwrap();

        // Verify mem-one has code1 pruned, while mem-two still has code2
        let file1 = resolve_memory_path(&vault_path, mem1).unwrap();
        let content1 = fs::read_to_string(&file1).unwrap();
        let page1 = parse_memory_file(&content1, &file1).unwrap();
        assert!(page1.frontmatter.references.is_none() || page1.frontmatter.references.as_ref().unwrap().is_empty());

        let file2 = resolve_memory_path(&vault_path, mem2).unwrap();
        let content2 = fs::read_to_string(&file2).unwrap();
        let page2 = parse_memory_file(&content2, &file2).unwrap();
        assert_eq!(page2.frontmatter.references.as_ref().unwrap().len(), 1);
        assert_eq!(page2.frontmatter.references.as_ref().unwrap()[0].path, code2);

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_handle_write() {
        let temp_dir = std::env::temp_dir().join(format!("bw_test_write_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));
        fs::create_dir_all(&temp_dir).unwrap();

        // 1. Initialize vault
        let vault_path = temp_dir.join(".brainwares");
        handle_init(&vault_path).unwrap();

        // 2. Write to a new note
        let note_name = "test-write-scenario";
        let content_one = "Hello world, this is a test write.";
        handle_write(&vault_path, note_name.to_string(), content_one.to_string()).unwrap();

        // Verify file is created and contains correct content
        let note_path = resolve_memory_path(&vault_path, note_name).unwrap();
        assert!(note_path.exists());
        let file_content = fs::read_to_string(&note_path).unwrap();
        let parsed = parse_memory_file(&file_content, &note_path).unwrap();
        assert_eq!(parsed.body.trim(), content_one);
        assert_eq!(parsed.frontmatter.title.as_deref(), Some("test write scenario"));
        assert!(parsed.frontmatter.last_updated.is_some());

        // 3. Overwrite it with new content
        let content_two = "Updated content here!";
        handle_write(&vault_path, note_name.to_string(), content_two.to_string()).unwrap();

        let file_content_updated = fs::read_to_string(&note_path).unwrap();
        let parsed_updated = parse_memory_file(&file_content_updated, &note_path).unwrap();
        assert_eq!(parsed_updated.body.trim(), content_two);
        // Title should be preserved
        assert_eq!(parsed_updated.frontmatter.title.as_deref(), Some("test write scenario"));

        // 4. Overwrite preserving manual edits (e.g. tags)
        // Let's manually edit tags in the page and serialize it
        let mut edited_page = parsed_updated.clone();
        edited_page.frontmatter.tags = Some(vec!["manual-tag".to_string()]);
        let serialized_edited = serialize_memory_file(&edited_page).unwrap();
        fs::write(&note_path, serialized_edited).unwrap();

        // Call handle_write again
        let content_three = "Yet another update!";
        handle_write(&vault_path, note_name.to_string(), content_three.to_string()).unwrap();

        let file_content_final = fs::read_to_string(&note_path).unwrap();
        let parsed_final = parse_memory_file(&file_content_final, &note_path).unwrap();
        assert_eq!(parsed_final.body.trim(), content_three);
        // Manual tag must be preserved!
        assert_eq!(parsed_final.frontmatter.tags.as_ref().unwrap().len(), 1);
        assert_eq!(parsed_final.frontmatter.tags.as_ref().unwrap()[0], "manual-tag");

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_handle_remove() {
        let temp_dir = std::env::temp_dir().join(format!("bw_test_remove_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));
        fs::create_dir_all(&temp_dir).unwrap();

        let vault_path = temp_dir.join(".brainwares");
        handle_init(&vault_path).unwrap();

        let note_name = "test-delete-me";
        handle_add(&vault_path, note_name.to_string(), None, None, false, None).unwrap();
        let note_path = resolve_memory_path(&vault_path, note_name).unwrap();
        assert!(note_path.exists());

        // Remove the note
        handle_remove(&vault_path, note_name.to_string(), false).unwrap();
        assert!(!note_path.exists());

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_memory_type_status() {
        let temp_dir = std::env::temp_dir().join(format!("bw_test_types_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));
        fs::create_dir_all(&temp_dir).unwrap();

        let vault_path = temp_dir.join(".brainwares");
        handle_init(&vault_path).unwrap();

        // 1. Add file memory (should check references)
        let file_mem = "file-memory";
        let code_file = "code.rs";
        let code_path = temp_dir.join(code_file);
        fs::write(&code_path, "fn test() {}").unwrap();

        handle_add(&vault_path, file_mem.to_string(), None, None, false, Some("file".to_string())).unwrap();
        handle_link(&vault_path, file_mem.to_string(), code_file.to_string()).unwrap();

        // Delete it to make it missing/outdated
        fs::remove_file(&code_path).unwrap();

        // 2. Add user memory (should skip reference checks)
        let user_mem = "user-memory";
        handle_add(&vault_path, user_mem.to_string(), None, None, false, Some("user".to_string())).unwrap();
        
        // Manually add references to user-memory to show that it is ignored
        let user_file = resolve_memory_path(&vault_path, user_mem).unwrap();
        let user_content = fs::read_to_string(&user_file).unwrap();
        let mut user_page = parse_memory_file(&user_content, &user_file).unwrap();
        user_page.frontmatter.references = Some(vec![CodeReference {
            path: "nonexistent.rs".to_string(),
            hash: "invalidhash".to_string(),
        }]);
        let serialized_user = serialize_memory_file(&user_page).unwrap();
        fs::write(&user_file, serialized_user).unwrap();

        let memories = load_memories(&vault_path).unwrap();
        let status = check_vault_status(&vault_path, &memories);

        // Outdated memories should only be 1 (file-memory), as user-memory references are ignored
        assert_eq!(status.outdated_memories_count, 1);
        
        let file_res = status.memories.iter().find(|m| m.memory_name == file_mem).unwrap();
        assert_eq!(file_res.memory_type, MemoryType::File);
        assert!(!file_res.references.is_empty());

        let user_res = status.memories.iter().find(|m| m.memory_name == user_mem).unwrap();
        assert_eq!(user_res.memory_type, MemoryType::User);
        // Even though user-memory has references, they should not have been checked in status
        assert!(user_res.references.is_empty());

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }
}



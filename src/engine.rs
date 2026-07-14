use crate::hash::calculate_file_hash;
use crate::models::{MemoryPage, MemoryType};
use crate::vault::{get_backlinks, get_workspace_root};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum ReferenceStatus {
    Ok,
    Outdated { stored: String, current: String },
    Missing,
}

#[derive(Debug, Clone)]
pub struct FileCheckResult {
    pub path: String,
    pub status: ReferenceStatus,
}

#[derive(Debug, Clone)]
pub struct MemoryCheckResult {
    pub memory_name: String,
    pub file_path: PathBuf,
    pub memory_type: MemoryType,
    pub references: Vec<FileCheckResult>,
    pub broken_links: Vec<String>, // list of targets that don't exist
    pub is_orphan: bool,
    pub has_placeholders: bool,
}

pub struct VaultStatus {
    pub memories: Vec<MemoryCheckResult>,
    pub total_memories: usize,
    pub outdated_memories_count: usize,
    pub broken_links_count: usize,
    pub orphan_count: usize,
    pub incomplete_memories_count: usize,
}

pub fn check_vault_status(vault_path: &Path, memories: &[MemoryPage]) -> VaultStatus {
    let workspace_root = get_workspace_root(vault_path);
    let backlinks = get_backlinks(memories);

    // Create a set of normalized memory names for quick lookup
    let existing_names: HashSet<String> = memories
        .iter()
        .map(|m| crate::vault::normalize_memory_name(&m.name))
        .collect();

    let mut results = Vec::new();
    let mut outdated_memories_count = 0;
    let mut broken_links_count = 0;
    let mut orphan_count = 0;

    for page in memories {
        let mut references_status = Vec::new();
        let mut has_outdated_or_missing = false;

        let memory_type = page.frontmatter.memory_type
            .unwrap_or_else(|| {
                if page.frontmatter.references.as_ref().map(|r| !r.is_empty()).unwrap_or(false) {
                    MemoryType::File
                } else {
                    MemoryType::User
                }
            });

        // 1. Check code references (only for file-based memories)
        if memory_type == MemoryType::File {
            if let Some(refs) = &page.frontmatter.references {
                for r in refs {
                    let code_file_path = workspace_root.join(&r.path);
                    let status = if !code_file_path.exists() {
                        has_outdated_or_missing = true;
                        ReferenceStatus::Missing
                    } else {
                        match calculate_file_hash(&code_file_path) {
                            Ok(current_hash) => {
                                if current_hash == r.hash {
                                    ReferenceStatus::Ok
                                } else {
                                    has_outdated_or_missing = true;
                                    ReferenceStatus::Outdated {
                                        stored: r.hash.clone(),
                                        current: current_hash,
                                    }
                                }
                            }
                            Err(_) => {
                                has_outdated_or_missing = true;
                                ReferenceStatus::Missing
                            }
                        }
                    };

                    references_status.push(FileCheckResult {
                        path: r.path.clone(),
                        status,
                    });
                }
            }
        }

        // 2. Check wiki-links and frontmatter relations
        let mut broken_links = Vec::new();
        let mut all_links = Vec::new();

        if let Some(relations) = &page.frontmatter.relations {
            for target in relations {
                all_links.push(target.clone());
            }
        }

        let wiki_links = crate::parser::extract_wiki_links(&page.body);
        for (target, _raw) in wiki_links {
            all_links.push(target);
        }

        for target in all_links {
            let normalized = crate::vault::normalize_memory_name(&target);
            // Check if the target is a memory page or a file in the workspace
            if !existing_names.contains(&normalized) {
                // As a fallback, check if it's a direct file path relative to workspace root
                let direct_file = workspace_root.join(&target);
                if !direct_file.exists() {
                    if !broken_links.contains(&target) {
                        broken_links.push(target);
                        broken_links_count += 1;
                    }
                }
            }
        }

        // 3. Check if orphan
        // A memory page is an orphan if:
        // - it is not index.md (case-insensitive "index")
        // - it has no backlinks from other memories
        let normalized_name = crate::vault::normalize_memory_name(&page.name);
        let is_index = normalized_name == "index";
        let has_backlinks = backlinks.contains_key(&normalized_name);
        let is_orphan = !is_index && !has_backlinks;

        if is_orphan {
            orphan_count += 1;
        }

        if has_outdated_or_missing {
            outdated_memories_count += 1;
        }

        // Check for uncompleted template placeholders
        let has_placeholders = page.body.contains("[Enter description") || page.body.contains("[Enter ");
        
        results.push(MemoryCheckResult {
            memory_name: page.name.clone(),
            file_path: page.file_path.clone(),
            memory_type,
            references: references_status,
            broken_links,
            is_orphan,
            has_placeholders,
        });
    }

    let incomplete_count = results.iter().filter(|r| r.has_placeholders).count();

    VaultStatus {
        memories: results,
        total_memories: memories.len(),
        outdated_memories_count,
        broken_links_count,
        orphan_count,
        incomplete_memories_count: incomplete_count,
    }
}

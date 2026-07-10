use crate::models::{Frontmatter, MemoryPage};
use regex::Regex;
use std::path::Path;

pub fn parse_memory_file(content: &str, path: &Path) -> Result<MemoryPage, String> {
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("Invalid path name: {:?}", path))?
        .to_string();

    let trimmed = content.trim_start();
    if trimmed.starts_with("---") {
        // Find second occurrence of ---
        if let Some(end_fm_idx) = trimmed[3..].find("---") {
            let fm_start_idx = 3;
            let fm_end_idx = end_fm_idx + 3;
            let yaml_content = &trimmed[fm_start_idx..fm_end_idx];
            let body = trimmed[fm_end_idx + 3..].trim_start().to_string();

            let frontmatter: Frontmatter = serde_yaml::from_str(yaml_content)
                .map_err(|e| format!("Failed to parse YAML frontmatter in {:?}: {}", path, e))?;

            return Ok(MemoryPage {
                file_path: path.to_path_buf(),
                name,
                frontmatter,
                body,
            });
        }
    }

    // No frontmatter
    Ok(MemoryPage {
        file_path: path.to_path_buf(),
        name,
        frontmatter: Frontmatter::default(),
        body: content.to_string(),
    })
}

pub fn serialize_memory_file(page: &MemoryPage) -> Result<String, String> {
    let yaml_str = serde_yaml::to_string(&page.frontmatter)
        .map_err(|e| format!("Failed to serialize YAML frontmatter: {}", e))?;
    
    // serde_yaml output starts with "---" and ends with "\n" or contains them, but let's ensure it has proper triple-dashes
    let fm_block = if yaml_str.trim().is_empty() || yaml_str.trim() == "{}" {
        "".to_string()
    } else {
        format!("---\n{}---\n\n", yaml_str.trim_start())
    };

    Ok(format!("{}{}", fm_block, page.body))
}

pub fn extract_wiki_links(body: &str) -> Vec<(String, String)> {
    // Regex for [[PageName]] or [[PageName#Header]] or [[PageName|Label]]
    // Group 1 captures the page name (excluding # or | components)
    // Group 0 captures the full raw link e.g. [[PageName#Header|Label]]
    let re = Regex::new(r"\[\[([^\]#|]+)(?:#[^\]|]+)?(?:\|[^\]]+)?\]\]").unwrap();
    let mut links = Vec::new();
    for cap in re.captures_iter(body) {
        if let Some(target) = cap.get(1) {
            let target_name = target.as_str().trim().to_string();
            let raw_match = cap.get(0).unwrap().as_str().to_string();
            links.push((target_name, raw_match));
        }
    }
    links
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_with_frontmatter() {
        let content = "\
---
title: Auth Page
tags: [auth, test]
references:
  - path: src/auth.rs
    hash: 1234abcd
---
# Main Header
Body text with [[Another Note#Section|Label]].
";
        let path = PathBuf::from("memories/auth-page.md");
        let page = parse_memory_file(content, &path).unwrap();
        assert_eq!(page.name, "auth-page");
        assert_eq!(page.frontmatter.title.as_deref(), Some("Auth Page"));
        assert_eq!(page.frontmatter.tags.unwrap(), vec!["auth", "test"]);
        assert_eq!(page.frontmatter.references.unwrap()[0].path, "src/auth.rs");
        assert!(page.body.contains("# Main Header"));

        let links = extract_wiki_links(&page.body);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].0, "Another Note");
        assert_eq!(links[0].1, "[[Another Note#Section|Label]]");
    }
}

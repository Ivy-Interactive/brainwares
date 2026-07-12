use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub default_vault_dir: String,
    pub ignore_patterns: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_vault_dir: ".brainwares".to_string(),
            ignore_patterns: vec![
                ".git".to_string(),
                ".brainwares".to_string(),
            ],
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct CodeReference {
    pub path: String,
    pub hash: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Frontmatter {
    pub title: Option<String>,
    pub references: Option<Vec<CodeReference>>,
    pub tags: Option<Vec<String>>,
    pub last_updated: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MemoryPage {
    pub file_path: PathBuf,
    pub name: String,
    pub frontmatter: Frontmatter,
    pub body: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Backlink {
    pub source_name: String,
    pub source_path: String,
    pub context: String, // Surrounding text/line where link occurred
}

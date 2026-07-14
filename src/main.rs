mod cli;
mod commands;
mod engine;
mod hash;
mod models;
mod parser;
mod vault;

use clap::Parser;
use cli::{Cli, Commands};
use std::path::PathBuf;

fn main() {
    let cli = Cli::parse();

    if let Some(project) = &cli.project {
        unsafe {
            std::env::set_var("BW_PROJECT", project);
        }
    }

    // Resolve vault path: either explicitly specified, or discovered
    let vault_path = match cli.vault {
        Some(v) => PathBuf::from(v),
        None => vault::find_vault_path(),
    };

    let result = match cli.command {
        Commands::Init => commands::handle_init(&vault_path),
        Commands::Status => {
            if !vault_path.is_dir() {
                Err(format!(
                    "Vault directory {:?} does not exist. Initialize it first using 'bw init'.",
                    vault_path
                ))
            } else {
                commands::handle_status(&vault_path)
            }
        }
        Commands::Add { name, tags, title, global, memory_type } => {
            if global {
                commands::handle_add(&vault_path, name, tags, title, true, memory_type)
            } else if !vault_path.is_dir() {
                Err(format!(
                    "Vault directory {:?} does not exist. Initialize it first using 'bw init' or use --global to add a user-wide memory.",
                    vault_path
                ))
            } else {
                commands::handle_add(&vault_path, name, tags, title, false, memory_type)
            }
        }
        Commands::Link { memory, code_file } => {
            if !vault_path.is_dir() {
                Err(format!(
                    "Vault directory {:?} does not exist. Initialize it first using 'bw init'.",
                    vault_path
                ))
            } else {
                commands::handle_link(&vault_path, memory, code_file)
            }
        }
        Commands::Relate { memory, target, remove } => {
            if !vault_path.is_dir() {
                Err(format!(
                    "Vault directory {:?} does not exist. Initialize it first using 'bw init'.",
                    vault_path
                ))
            } else {
                commands::handle_relate(&vault_path, memory, target, remove)
            }
        }
        Commands::Update { memory, code_file } => {
            if !vault_path.is_dir() {
                Err(format!(
                    "Vault directory {:?} does not exist. Initialize it first using 'bw init'.",
                    vault_path
                ))
            } else {
                commands::handle_update(&vault_path, memory, code_file)
            }
        }
        Commands::Shake => {
            if !vault_path.is_dir() {
                Err(format!(
                    "Vault directory {:?} does not exist. Initialize it first using 'bw init'.",
                    vault_path
                ))
            } else {
                commands::handle_shake(&vault_path)
            }
        }
        Commands::Query { term } => {
            if !vault_path.is_dir() {
                Err(format!(
                    "Vault directory {:?} does not exist. Initialize it first using 'bw init'.",
                    vault_path
                ))
            } else {
                commands::handle_query(&vault_path, term)
            }
        }
        Commands::Read { name } => {
            if !vault_path.is_dir() {
                Err(format!(
                    "Vault directory {:?} does not exist. Initialize it first using 'bw init'.",
                    vault_path
                ))
            } else {
                commands::handle_read(&vault_path, name)
            }
        }
        Commands::Compile { program, args } => {
            if !vault_path.is_dir() {
                Err(format!(
                    "Vault directory {:?} does not exist. Initialize it first using 'bw init'.",
                    vault_path
                ))
            } else {
                commands::handle_compile(&vault_path, program, args)
            }
        }
        Commands::Integrate => commands::handle_integrate(),
        Commands::Doctor => commands::handle_doctor(),
        Commands::Write { memory, content } => {
            if !vault_path.is_dir() {
                Err(format!(
                    "Vault directory {:?} does not exist. Initialize it first using 'bw init'.",
                    vault_path
                ))
            } else {
                let final_content_res = match content {
                    Some(c) => Ok(c),
                    None => {
                        use std::io::{self, Read};
                        let mut buffer = String::new();
                        io::stdin()
                            .read_to_string(&mut buffer)
                            .map(|_| buffer)
                            .map_err(|e| format!("Failed to read from stdin: {}", e))
                    }
                };
                final_content_res.and_then(|final_content| {
                    commands::handle_write(&vault_path, memory, final_content)
                })
            }
        }
        Commands::Index => {
            if !vault_path.is_dir() {
                Err(format!(
                    "Vault directory {:?} does not exist. Initialize it first using 'bw init'.",
                    vault_path
                ))
            } else {
                commands::handle_index(&vault_path)
            }
        }
        Commands::Remove { name, global } => {
            commands::handle_remove(&vault_path, name, global)
        }
        Commands::Rules => {
            commands::handle_rules()
        }
    };

    if let Err(e) = result {
        eprintln!("ERROR: {}", e);
        std::process::exit(1);
    }
}

// Testing comment


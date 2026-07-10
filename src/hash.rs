use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub fn calculate_file_hash<P: AsRef<Path>>(path: P) -> Result<String, String> {
    let mut file = File::open(&path).map_err(|e| format!("Could not open file {:?}: {}", path.as_ref(), e))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 4096];

    loop {
        let count = file.read(&mut buffer).map_err(|e| format!("Error reading file {:?}: {}", path.as_ref(), e))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

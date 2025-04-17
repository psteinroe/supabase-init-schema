use std::{
    fs,
    io::prelude::*,
    path::{Path, PathBuf},
};

use crate::locations::StatementLocation;

pub fn write_nodes(nodes: &[StatementLocation], out_dir: &Path) -> Vec<PathBuf> {
    nodes
        .iter()
        .map(|n| {
            let path = n.path(out_dir, nodes);

            let content = n.sql();

            // Create parent directories if they don't exist
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("Failed to create parent directories");
            }

            // Check if file exists and if content is already in it
            let file_exists = path.exists();
            let content_exists = if file_exists {
                match fs::read_to_string(&path) {
                    Ok(existing_content) => existing_content.contains(&content),
                    Err(_) => false,
                }
            } else {
                false
            };

            // Only append if content doesn't already exist
            if !content_exists {
                let mut file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .expect("Failed to open file");
                writeln!(file, "{}", content).expect("Failed to write to file");
            }

            path
        })
        .collect()
}

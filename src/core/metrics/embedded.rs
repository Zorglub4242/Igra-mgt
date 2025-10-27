/// Embedded default plugin configuration files
///
/// This module embeds the default plugin TOML files into the binary
/// and provides functionality to extract them to the filesystem on first run.

use rust_embed::RustEmbed;
use std::path::Path;
use std::fs;
use anyhow::{Result, Context};

#[derive(RustEmbed)]
#[folder = "plugins/"]
#[include = "*.toml"]
pub struct EmbeddedPlugins;

/// Get list of all embedded plugin filenames
pub fn list_embedded_plugins() -> Vec<String> {
    EmbeddedPlugins::iter()
        .map(|f| f.to_string())
        .collect()
}

/// Get content of an embedded plugin by filename
pub fn get_embedded_plugin(filename: &str) -> Option<Vec<u8>> {
    EmbeddedPlugins::get(filename)
        .map(|f| f.data.to_vec())
}

/// Extract all embedded plugins to a directory
///
/// This will create the directory if it doesn't exist and write all
/// embedded plugin files to it. Existing files will be skipped to avoid
/// overwriting user customizations.
pub fn extract_plugins_to_dir<P: AsRef<Path>>(target_dir: P) -> Result<usize> {
    let target_dir = target_dir.as_ref();

    // Create directory if it doesn't exist
    fs::create_dir_all(target_dir)
        .context(format!("Failed to create plugin directory: {}", target_dir.display()))?;

    let mut extracted_count = 0;

    for filename in EmbeddedPlugins::iter() {
        let target_path = target_dir.join(filename.as_ref());

        // Skip if file already exists (preserve user customizations)
        if target_path.exists() {
            eprintln!("[INFO] Skipping existing plugin: {}", filename);
            continue;
        }

        if let Some(content) = EmbeddedPlugins::get(&filename) {
            fs::write(&target_path, content.data.as_ref())
                .context(format!("Failed to write plugin file: {}", target_path.display()))?;

            eprintln!("[INFO] Extracted plugin: {} -> {}", filename, target_path.display());
            extracted_count += 1;
        }
    }

    Ok(extracted_count)
}

/// Check if a directory has any plugin files
pub fn has_plugins<P: AsRef<Path>>(dir: P) -> bool {
    let dir = dir.as_ref();
    if !dir.exists() || !dir.is_dir() {
        return false;
    }

    // Check if directory contains any .toml files
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Some(ext) = entry.path().extension() {
                if ext == "toml" {
                    return true;
                }
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_embedded_plugins() {
        let plugins = list_embedded_plugins();
        assert!(!plugins.is_empty(), "Should have embedded plugins");

        // Check for some expected plugins
        assert!(plugins.iter().any(|p| p.contains("reth") || p.contains("geth")));
    }

    #[test]
    fn test_get_embedded_plugin() {
        let plugins = list_embedded_plugins();
        if let Some(first_plugin) = plugins.first() {
            let content = get_embedded_plugin(first_plugin);
            assert!(content.is_some(), "Should be able to read embedded plugin");

            let content = content.unwrap();
            assert!(!content.is_empty(), "Plugin content should not be empty");
        }
    }
}

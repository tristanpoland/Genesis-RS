//! Blueprint generation for determining manifest files to merge.

use super::Kit;
use genesis_types::{GenesisError, Result};
use std::path::PathBuf;
use walkdir::WalkDir;

/// Blueprint containing manifest files to merge.
#[derive(Debug, Clone)]
pub struct Blueprint {
    /// Base manifest files (always included)
    pub base: Vec<PathBuf>,
    /// Feature-specific manifest files
    pub features: Vec<PathBuf>,
    /// Subkit manifest files
    pub subkits: Vec<PathBuf>,
}

impl Blueprint {
    /// Generate blueprint for given features.
    pub fn generate(kit: &dyn Kit, features: &[String]) -> Result<Self> {
        let kit_path = kit.path();
        let mut base = Vec::new();
        let mut feature_files = Vec::new();
        let mut subkit_files = Vec::new();

        let base_yml = kit_path.join("base.yml");
        if base_yml.exists() {
            base.push(base_yml);
        }

        let manifests_dir = kit_path.join("manifests");
        if manifests_dir.exists() {
            for entry in WalkDir::new(&manifests_dir)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |e| e == "yml") {
                    let stem = path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");

                    if stem == "base" || stem == "kit" {
                        base.push(path.to_path_buf());
                    }
                }
            }
        }

        for feature in features {
            let feature_file = kit_path.join("manifests").join(format!("{}.yml", feature));
            if feature_file.exists() {
                feature_files.push(feature_file);
            }

            let feature_dir = kit_path.join("manifests").join(feature);
            if feature_dir.is_dir() {
                for entry in WalkDir::new(&feature_dir)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    let path = entry.path();
                    if path.is_file() && path.extension().map_or(false, |e| e == "yml") {
                        feature_files.push(path.to_path_buf());
                    }
                }
            }
        }

        let subkits_dir = kit_path.join("subkits");
        if subkits_dir.exists() {
            for entry in WalkDir::new(&subkits_dir)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |e| e == "yml") {
                    subkit_files.push(path.to_path_buf());
                }
            }
        }

        Ok(Self {
            base,
            features: feature_files,
            subkits: subkit_files,
        })
    }

    /// Get all manifest files in merge order.
    pub fn all_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        files.extend(self.base.iter().cloned());
        files.extend(self.features.iter().cloned());
        files.extend(self.subkits.iter().cloned());
        files
    }

    /// Get count of manifest files.
    pub fn file_count(&self) -> usize {
        self.base.len() + self.features.len() + self.subkits.len()
    }
}

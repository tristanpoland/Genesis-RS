//! Manifest caching system for performance optimization.

use super::types::{CachedManifest, ManifestMetadata, YamlContent};
use genesis_types::{GenesisError, Result, EnvName};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use chrono::{Duration, Utc};
use tracing::{debug, info, warn};

/// Manifest cache manager.
pub struct ManifestCache {
    cache_dir: PathBuf,
    max_age: Duration,
    max_entries: usize,
}

impl ManifestCache {
    /// Create new manifest cache with default settings.
    pub fn new(cache_dir: impl AsRef<Path>) -> Self {
        Self {
            cache_dir: cache_dir.as_ref().to_path_buf(),
            max_age: Duration::hours(24),
            max_entries: 100,
        }
    }

    /// Set maximum cache age.
    pub fn with_max_age(mut self, max_age: Duration) -> Self {
        self.max_age = max_age;
        self
    }

    /// Set maximum number of cache entries.
    pub fn with_max_entries(mut self, max_entries: usize) -> Self {
        self.max_entries = max_entries;
        self
    }

    /// Get cache file path for an environment.
    fn cache_path(&self, env_name: &EnvName) -> PathBuf {
        self.cache_dir.join(format!("{}.cache.json", env_name.as_str()))
    }

    /// Get cache entry.
    pub fn get(&self, env_name: &EnvName) -> Result<Option<CachedManifest>> {
        let path = self.cache_path(env_name);

        if !path.exists() {
            debug!("No cache entry for {}", env_name);
            return Ok(None);
        }

        match CachedManifest::load_from_file(&path) {
            Ok(cached) => {
                if cached.is_expired(self.max_age) {
                    info!("Cache expired for {}", env_name);
                    self.remove(env_name)?;
                    return Ok(None);
                }

                if !cached.validate()? {
                    warn!("Cache integrity check failed for {}", env_name);
                    self.remove(env_name)?;
                    return Ok(None);
                }

                debug!("Cache hit for {}", env_name);
                Ok(Some(cached))
            }
            Err(e) => {
                warn!("Failed to load cache for {}: {}", env_name, e);
                Ok(None)
            }
        }
    }

    /// Store manifest in cache.
    pub fn put(&self, env_name: &EnvName, content: YamlContent, metadata: ManifestMetadata) -> Result<()> {
        std::fs::create_dir_all(&self.cache_dir)
            .map_err(|e| GenesisError::Manifest(format!("Failed to create cache dir: {}", e)))?;

        let cached = CachedManifest::new(content, metadata);
        let path = self.cache_path(env_name);

        cached.save_to_file(&path)?;
        info!("Cached manifest for {}", env_name);

        self.cleanup()?;

        Ok(())
    }

    /// Remove cache entry.
    pub fn remove(&self, env_name: &EnvName) -> Result<()> {
        let path = self.cache_path(env_name);

        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| GenesisError::Manifest(format!("Failed to remove cache: {}", e)))?;
            debug!("Removed cache for {}", env_name);
        }

        Ok(())
    }

    /// Clear all cache entries.
    pub fn clear(&self) -> Result<()> {
        if !self.cache_dir.exists() {
            return Ok(());
        }

        let entries = std::fs::read_dir(&self.cache_dir)
            .map_err(|e| GenesisError::Manifest(format!("Failed to read cache dir: {}", e)))?;

        let mut removed = 0;
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "json") {
                    if let Err(e) = std::fs::remove_file(&path) {
                        warn!("Failed to remove cache file {:?}: {}", path, e);
                    } else {
                        removed += 1;
                    }
                }
            }
        }

        info!("Cleared {} cache entries", removed);
        Ok(())
    }

    /// Cleanup old cache entries.
    fn cleanup(&self) -> Result<()> {
        if !self.cache_dir.exists() {
            return Ok(());
        }

        let entries = std::fs::read_dir(&self.cache_dir)
            .map_err(|e| GenesisError::Manifest(format!("Failed to read cache dir: {}", e)))?;

        let mut cache_files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();

        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "json") {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            cache_files.push((path, modified));
                        }
                    }
                }
            }
        }

        if cache_files.len() <= self.max_entries {
            return Ok(());
        }

        cache_files.sort_by(|a, b| a.1.cmp(&b.1));

        let to_remove = cache_files.len() - self.max_entries;
        for (path, _) in cache_files.iter().take(to_remove) {
            if let Err(e) = std::fs::remove_file(path) {
                warn!("Failed to remove old cache file {:?}: {}", path, e);
            } else {
                debug!("Removed old cache entry: {:?}", path);
            }
        }

        info!("Cleaned up {} old cache entries", to_remove);
        Ok(())
    }

    /// Get cache statistics.
    pub fn stats(&self) -> Result<CacheStats> {
        let mut stats = CacheStats {
            total_entries: 0,
            total_size_bytes: 0,
            expired_entries: 0,
            entries_by_env: HashMap::new(),
        };

        if !self.cache_dir.exists() {
            return Ok(stats);
        }

        let entries = std::fs::read_dir(&self.cache_dir)
            .map_err(|e| GenesisError::Manifest(format!("Failed to read cache dir: {}", e)))?;

        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "json") {
                    stats.total_entries += 1;

                    if let Ok(metadata) = entry.metadata() {
                        stats.total_size_bytes += metadata.len();
                    }

                    if let Ok(cached) = CachedManifest::load_from_file(&path) {
                        if cached.is_expired(self.max_age) {
                            stats.expired_entries += 1;
                        }

                        let env_name = cached.metadata.env_name.as_str().to_string();
                        stats.entries_by_env.insert(env_name, cached.cached_at);
                    }
                }
            }
        }

        Ok(stats)
    }

    /// Verify cache integrity for all entries.
    pub fn verify(&self) -> Result<CacheVerification> {
        let mut verification = CacheVerification {
            total_checked: 0,
            valid_entries: 0,
            invalid_entries: 0,
            invalid_paths: Vec::new(),
        };

        if !self.cache_dir.exists() {
            return Ok(verification);
        }

        let entries = std::fs::read_dir(&self.cache_dir)
            .map_err(|e| GenesisError::Manifest(format!("Failed to read cache dir: {}", e)))?;

        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "json") {
                    verification.total_checked += 1;

                    match CachedManifest::load_from_file(&path) {
                        Ok(cached) => {
                            if cached.validate()? {
                                verification.valid_entries += 1;
                            } else {
                                verification.invalid_entries += 1;
                                verification.invalid_paths.push(path);
                            }
                        }
                        Err(_) => {
                            verification.invalid_entries += 1;
                            verification.invalid_paths.push(path);
                        }
                    }
                }
            }
        }

        info!(
            "Cache verification: {}/{} valid entries",
            verification.valid_entries,
            verification.total_checked
        );

        Ok(verification)
    }

    /// Repair cache by removing invalid entries.
    pub fn repair(&self) -> Result<usize> {
        let verification = self.verify()?;
        let mut repaired = 0;

        for path in verification.invalid_paths {
            if let Err(e) = std::fs::remove_file(&path) {
                warn!("Failed to remove invalid cache file {:?}: {}", path, e);
            } else {
                repaired += 1;
                info!("Removed invalid cache entry: {:?}", path);
            }
        }

        Ok(repaired)
    }
}

/// Cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Total number of cache entries
    pub total_entries: usize,

    /// Total size in bytes
    pub total_size_bytes: u64,

    /// Number of expired entries
    pub expired_entries: usize,

    /// Cached environments with timestamps
    pub entries_by_env: HashMap<String, chrono::DateTime<Utc>>,
}

impl CacheStats {
    /// Get human-readable size.
    pub fn size_human(&self) -> String {
        let kb = self.total_size_bytes as f64 / 1024.0;
        if kb < 1024.0 {
            format!("{:.2} KB", kb)
        } else {
            let mb = kb / 1024.0;
            format!("{:.2} MB", mb)
        }
    }

    /// Get valid entry count.
    pub fn valid_entries(&self) -> usize {
        self.total_entries - self.expired_entries
    }
}

/// Cache verification results.
#[derive(Debug, Clone)]
pub struct CacheVerification {
    /// Total entries checked
    pub total_checked: usize,

    /// Number of valid entries
    pub valid_entries: usize,

    /// Number of invalid entries
    pub invalid_entries: usize,

    /// Paths of invalid entries
    pub invalid_paths: Vec<PathBuf>,
}

impl CacheVerification {
    /// Check if all entries are valid.
    pub fn is_clean(&self) -> bool {
        self.invalid_entries == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_put_get() {
        let temp_dir = TempDir::new().unwrap();
        let cache = ManifestCache::new(temp_dir.path());

        let env_name = EnvName::new("test-env").unwrap();
        let content = "test: value".to_string();
        let metadata = ManifestMetadata::new(
            env_name.clone(),
            "test-kit",
            "1.0.0",
            vec![],
        );

        cache.put(&env_name, content.clone(), metadata.clone()).unwrap();

        let cached = cache.get(&env_name).unwrap().unwrap();
        assert_eq!(cached.content, content);
        assert_eq!(cached.metadata.env_name, env_name);
    }

    #[test]
    fn test_cache_expiration() {
        let temp_dir = TempDir::new().unwrap();
        let cache = ManifestCache::new(temp_dir.path())
            .with_max_age(Duration::seconds(-1));

        let env_name = EnvName::new("test-env").unwrap();
        let content = "test: value".to_string();
        let metadata = ManifestMetadata::new(
            env_name.clone(),
            "test-kit",
            "1.0.0",
            vec![],
        );

        cache.put(&env_name, content, metadata).unwrap();

        let result = cache.get(&env_name).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_clear() {
        let temp_dir = TempDir::new().unwrap();
        let cache = ManifestCache::new(temp_dir.path());

        let env1 = EnvName::new("env1").unwrap();
        let env2 = EnvName::new("env2").unwrap();

        let metadata1 = ManifestMetadata::new(env1.clone(), "kit", "1.0.0", vec![]);
        let metadata2 = ManifestMetadata::new(env2.clone(), "kit", "1.0.0", vec![]);

        cache.put(&env1, "content1".to_string(), metadata1).unwrap();
        cache.put(&env2, "content2".to_string(), metadata2).unwrap();

        let stats = cache.stats().unwrap();
        assert_eq!(stats.total_entries, 2);

        cache.clear().unwrap();

        let stats = cache.stats().unwrap();
        assert_eq!(stats.total_entries, 0);
    }
}

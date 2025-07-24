//! Cache abstraction and implementation using zero-cost abstractions
//!
//! This module provides a trait-based cache system that allows for different
//! implementations while maintaining zero-cost abstractions through compile-time dispatch.

use crate::types::{CacheKey, DirectoryPath, FileName, ResourceType};
use ahash::AHashMap;
use bytes::Bytes;
use parking_lot::RwLock;
use std::sync::Arc;

/// Cache trait for zero-cost abstraction over different cache implementations
pub trait Cache: Clone + Send + Sync + 'static {
    /// Get a random file from the cache for the given resource type
    fn get_random_file(&self, resource: ResourceType) -> Option<FileName>;

    /// Get a random quote from the cache
    fn get_random_quote(&self) -> Option<Bytes>;

    /// Get a random joke from the cache
    fn get_random_joke(&self) -> Option<Bytes>;

    /// Get cached image data by key
    fn get_image(&self, key: &CacheKey) -> Option<Bytes>;

    /// Store image data in cache
    fn store_image(&self, key: CacheKey, data: Bytes);

    /// Update file lists for a resource type
    fn update_files(&self, resource: ResourceType, files: Vec<FileName>);

    /// Update quotes
    fn update_quotes(&self, quotes: Vec<Bytes>);

    /// Update jokes
    fn update_jokes(&self, jokes: Vec<Bytes>);

    /// Get file count for a resource type
    fn file_count(&self, resource: ResourceType) -> usize;

    /// Get quote count
    fn quote_count(&self) -> usize;

    /// Get joke count
    fn joke_count(&self) -> usize;
}

/// High-performance cache implementation using RwLocks and fast hashmaps
#[derive(Clone)]
pub struct FileCache {
    gary_files: Arc<RwLock<Vec<FileName>>>,
    goober_files: Arc<RwLock<Vec<FileName>>>,
    quotes: Arc<RwLock<Vec<Bytes>>>,
    jokes: Arc<RwLock<Vec<Bytes>>>,
    image_cache: Arc<RwLock<AHashMap<String, Bytes>>>,
}

impl FileCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self {
            gary_files: Arc::new(RwLock::new(Vec::new())),
            goober_files: Arc::new(RwLock::new(Vec::new())),
            quotes: Arc::new(RwLock::new(Vec::new())),
            jokes: Arc::new(RwLock::new(Vec::new())),
            image_cache: Arc::new(RwLock::new(AHashMap::new())),
        }
    }

    /// Get the appropriate file list for a resource type
    fn get_files(&self, resource: ResourceType) -> &Arc<RwLock<Vec<FileName>>> {
        match resource {
            ResourceType::Gary => &self.gary_files,
            ResourceType::Goober => &self.goober_files,
        }
    }

    /// Get a random index for the given length using fastrand
    #[inline]
    fn random_index(len: usize) -> usize {
        fastrand::usize(..len)
    }
}

impl Default for FileCache {
    fn default() -> Self {
        Self::new()
    }
}

impl Cache for FileCache {
    #[inline]
    fn get_random_file(&self, resource: ResourceType) -> Option<FileName> {
        let files = self.get_files(resource).read();
        if files.is_empty() {
            None
        } else {
            let index = Self::random_index(files.len());
            Some(files[index].clone())
        }
    }

    #[inline]
    fn get_random_quote(&self) -> Option<Bytes> {
        let quotes = self.quotes.read();
        if quotes.is_empty() {
            None
        } else {
            let index = Self::random_index(quotes.len());
            Some(quotes[index].clone())
        }
    }

    #[inline]
    fn get_random_joke(&self) -> Option<Bytes> {
        let jokes = self.jokes.read();
        if jokes.is_empty() {
            None
        } else {
            let index = Self::random_index(jokes.len());
            Some(jokes[index].clone())
        }
    }

    #[inline]
    fn get_image(&self, key: &CacheKey) -> Option<Bytes> {
        self.image_cache.read().get(key.as_ref() as &str).cloned()
    }

    #[inline]
    fn store_image(&self, key: CacheKey, data: Bytes) {
        self.image_cache
            .write()
            .insert((key.as_ref() as &str).to_string(), data);
    }

    fn update_files(&self, resource: ResourceType, files: Vec<FileName>) {
        *self.get_files(resource).write() = files;
    }

    fn update_quotes(&self, quotes: Vec<Bytes>) {
        *self.quotes.write() = quotes;
    }

    fn update_jokes(&self, jokes: Vec<Bytes>) {
        *self.jokes.write() = jokes;
    }

    fn file_count(&self, resource: ResourceType) -> usize {
        self.get_files(resource).read().len()
    }

    fn quote_count(&self) -> usize {
        self.quotes.read().len()
    }

    fn joke_count(&self) -> usize {
        self.jokes.read().len()
    }
}

/// Cache operations trait for loading data into cache
pub trait CacheLoader<C: Cache> {
    /// Load file list from directory
    fn load_file_list(
        &self,
        dir: &DirectoryPath,
    ) -> impl std::future::Future<Output = Vec<FileName>> + Send;

    /// Load quotes/jokes from JSON file
    fn load_text_content(
        &self,
        file_path: &str,
    ) -> impl std::future::Future<Output = Vec<Bytes>> + Send;

    /// Preload small images into cache
    fn preload_images(
        &self,
        dir: &DirectoryPath,
        cache: &C,
    ) -> impl std::future::Future<Output = ()> + Send;

    /// Initialize cache with all data
    fn initialize_cache(
        &self,
        cache: &C,
        config: &crate::config::Config,
    ) -> impl std::future::Future<Output = ()> + Send;
}

/// Default cache loader implementation
pub struct DefaultCacheLoader;

impl DefaultCacheLoader {
    pub fn new() -> Self {
        Self
    }

    /// Fast JSON escaping for simple strings
    fn escape_json_string(s: &str) -> Bytes {
        if s.chars()
            .all(|c| c.is_ascii() && c != '"' && c != '\\' && c != '\n' && c != '\r' && c != '\t')
        {
            Bytes::from(s.to_string())
        } else {
            Bytes::from(
                serde_json::to_string(s)
                    .unwrap()
                    .trim_matches('"')
                    .to_string(),
            )
        }
    }
}

impl Default for DefaultCacheLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Cache> CacheLoader<C> for DefaultCacheLoader {
    fn load_file_list(
        &self,
        dir: &DirectoryPath,
    ) -> impl std::future::Future<Output = Vec<FileName>> + Send {
        let dir = dir.clone();
        async move {
            let mut files = Vec::new();
            if let Ok(mut entries) = tokio::fs::read_dir(dir.as_str()).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    if let Ok(file_type) = entry.file_type().await {
                        if file_type.is_file() {
                            let filename = entry.file_name().to_string_lossy().to_string();
                            files.push(FileName::new_unchecked(filename));
                        }
                    }
                }
            }
            files
        }
    }

    fn load_text_content(
        &self,
        file_path: &str,
    ) -> impl std::future::Future<Output = Vec<Bytes>> + Send {
        let file_path = file_path.to_string();
        async move {
            match tokio::fs::read_to_string(&file_path).await {
                Ok(content) => {
                    if let Ok(lines) = serde_json::from_str::<Vec<String>>(&content) {
                        lines
                            .into_iter()
                            .map(|line| Self::escape_json_string(&line))
                            .collect()
                    } else {
                        Vec::new()
                    }
                }
                Err(_) => Vec::new(),
            }
        }
    }

    fn preload_images(
        &self,
        dir: &DirectoryPath,
        cache: &C,
    ) -> impl std::future::Future<Output = ()> + Send {
        let dir = dir.clone();
        let cache = cache.clone();
        async move {
            if let Ok(mut entries) = tokio::fs::read_dir(dir.as_str()).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    if let Ok(file_type) = entry.file_type().await {
                        if file_type.is_file() {
                            let file_path = entry.path();
                            if let Ok(metadata) = tokio::fs::metadata(&file_path).await {
                                // Only cache files smaller than 1MB
                                if metadata.len() < 1024 * 1024 {
                                    if let Ok(content) = tokio::fs::read(&file_path).await {
                                        let filename =
                                            entry.file_name().to_string_lossy().to_string();
                                        let key = CacheKey::new(filename);
                                        cache.store_image(key, Bytes::from(content));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn initialize_cache(
        &self,
        cache: &C,
        config: &crate::config::Config,
    ) -> impl std::future::Future<Output = ()> + Send {
        let cache = cache.clone();
        let config = config.clone();
        let loader = DefaultCacheLoader::new();
        async move {
            println!("Loading file lists and content...");

            let gary_files_fut = CacheLoader::<C>::load_file_list(&loader, &config.gary_dir);
            let goober_files_fut = CacheLoader::<C>::load_file_list(&loader, &config.goober_dir);
            let quotes_fut = CacheLoader::<C>::load_text_content(&loader, &config.quotes_file);
            let jokes_fut = CacheLoader::<C>::load_text_content(&loader, &config.jokes_file);

            let (gary_files, goober_files, quotes, jokes) =
                tokio::join!(gary_files_fut, goober_files_fut, quotes_fut, jokes_fut);

            cache.update_files(ResourceType::Gary, gary_files);
            cache.update_files(ResourceType::Goober, goober_files);
            cache.update_quotes(quotes);
            cache.update_jokes(jokes);

            println!("Preloading small images...");
            let preload_gary = CacheLoader::<C>::preload_images(&loader, &config.gary_dir, &cache);
            let preload_goober = CacheLoader::<C>::preload_images(&loader, &config.goober_dir, &cache);
            tokio::join!(preload_gary, preload_goober);

            println!(
                "Cache initialized: {} gary files, {} goober files, {} quotes, {} jokes",
                cache.file_count(ResourceType::Gary),
                cache.file_count(ResourceType::Goober),
                cache.quote_count(),
                cache.joke_count()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_operations() {
        let cache: FileCache = FileCache::new();

        // Test file operations
        let files = vec![
            FileName::new_unchecked("test1.jpg".to_string()),
            FileName::new_unchecked("test2.jpg".to_string()),
        ];
        cache.update_files(ResourceType::Gary, files);

        assert_eq!(cache.file_count(ResourceType::Gary), 2);
        assert!(cache.get_random_file(ResourceType::Gary).is_some());

        // Test quote operations
        let quotes = vec![Bytes::from("Quote 1"), Bytes::from("Quote 2")];
        cache.update_quotes(quotes);

        assert_eq!(cache.quote_count(), 2);
        assert!(cache.get_random_quote().is_some());
    }

    #[test]
    fn test_image_cache() {
        let cache = FileCache::new();
        let key = CacheKey::new("test.jpg");
        let data = Bytes::from("image data");

        cache.store_image(key.clone(), data.clone());
        assert_eq!(cache.get_image(&key), Some(data));
    }
}

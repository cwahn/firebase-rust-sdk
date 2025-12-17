//! Firestore Settings and Source types
//!
//! # C++ Reference
//! - `firestore/src/include/firebase/firestore/settings.h:49`
//! - `firestore/src/include/firebase/firestore/source.h:30`

/// Settings for configuring Firestore behavior
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/settings.h:49` - Settings class
///
/// Configure various Firestore settings including persistence, cache size, and network options.
#[derive(Debug, Clone)]
pub struct Settings {
    /// Host of the Firestore backend to connect to
    ///
    /// Default: "firestore.googleapis.com"
    pub host: String,

    /// Whether to use SSL for communication
    ///
    /// Default: true
    pub ssl_enabled: bool,

    /// Whether to enable local persistent storage
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/settings.h:120` - is_persistence_enabled()
    ///
    /// When enabled, Firestore caches documents locally and serves them when offline.
    ///
    /// Default: true
    pub persistence_enabled: bool,

    /// Cache size threshold for on-disk data in bytes
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/settings.h:123` - cache_size_bytes()
    ///
    /// If the cache grows beyond this size, Firestore will start removing data
    /// that hasn't been recently used. Set to -1 for unlimited cache.
    ///
    /// Default: 100 MB (104857600 bytes)
    pub cache_size_bytes: i64,

    /// Directory path for local cache storage
    ///
    /// If None, uses platform default:
    /// - macOS/Linux: `~/.firebase_cache/{project_id}/`
    /// - Windows: `%APPDATA%/firebase_cache/{project_id}/`
    /// - WASM: IndexedDB (not filesystem)
    pub cache_directory: Option<std::path::PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            host: "firestore.googleapis.com".to_string(),
            ssl_enabled: true,
            persistence_enabled: true,
            cache_size_bytes: 100 * 1024 * 1024, // 100 MB
            cache_directory: None,
        }
    }
}

impl Settings {
    /// Constant to use with cache_size_bytes to disable garbage collection
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/settings.h:57` - kCacheSizeUnlimited
    pub const CACHE_SIZE_UNLIMITED: i64 = -1;

    /// Creates default settings
    pub fn new() -> Self {
        Self::default()
    }
}

/// Source options for Firestore queries
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/source.h:30` - Source enum
///
/// Configures where Firestore should fetch data from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Default behavior - try server first, fall back to cache if offline
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/source.h:40` - kDefault
    Default,

    /// Only fetch from server, fail if offline
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/source.h:44` - kServer
    Server,

    /// Only fetch from local cache, fail if not cached
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/source.h:48` - kCache
    Cache,
}

impl Default for Source {
    fn default() -> Self {
        Source::Default
    }
}

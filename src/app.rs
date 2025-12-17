//! Firebase App
//!
//! # C++ Reference
//! - `app/src/app.cc` - App implementation
//! - `app/src/include/firebase/app.h` - App class

use crate::auth::AuthInner;
use crate::error::FirebaseError;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use tokio::sync::RwLock;

/// Global map of App names to App instances
///
/// C++ equivalent: Similar to Auth/Firestore singleton patterns
static APP_INSTANCES: Lazy<RwLock<HashMap<String, App>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Firebase App instance
///
/// # C++ Reference
/// - `app/src/include/firebase/app.h`
///
/// The App is the central configuration object for Firebase services.
/// It holds credentials and project configuration that Auth and Firestore use.
///
/// Each app name has at most one App instance (singleton pattern).
#[derive(Clone)]
pub struct App {
    inner: Arc<AppInner>,
}

struct AppInner {
    name: String,
    options: AppOptions,
    /// Internal reference to Auth for this app (if initialized)
    auth_ref: RwLock<Option<Weak<AuthInner>>>,
}

/// Firebase App configuration options
///
/// # C++ Reference
/// - `app/src/include/firebase/app.h` - AppOptions
#[derive(Clone, Debug)]
pub struct AppOptions {
    /// Firebase API key
    pub api_key: String,
    /// Google Cloud project ID
    pub project_id: String,
    /// App name (optional, defaults to "[DEFAULT]")
    pub app_name: Option<String>,
}

impl App {
    /// Create a new Firebase App with the given options
    ///
    /// # C++ Reference
    /// - `app/src/app.cc` - App::Create()
    ///
    /// If an app with the same name already exists, returns the existing instance.
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::{App, AppOptions};
    ///
    /// let options = AppOptions {
    ///     api_key: "YOUR_API_KEY".to_string(),
    ///     project_id: "your-project-id".to_string(),
    ///     app_name: None,
    /// };
    /// let app = App::create(options).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(options: AppOptions) -> Result<Self, FirebaseError> {
        // Validate options (error case first)
        if options.api_key.is_empty() {
            return Err(FirebaseError::ApiKeyNotConfigured);
        }
        if options.project_id.is_empty() {
            return Err(FirebaseError::Internal(
                "Project ID cannot be empty".to_string(),
            ));
        }

        let name = match options.app_name.clone() {
            None => "[DEFAULT]".to_string(),
            Some(n) => n,
        };

        let mut instances = APP_INSTANCES.write().await;

        // Check if instance already exists
        if let Some(app) = instances.get(&name) {
            return Ok(app.clone());
        }

        // Create new App instance
        let app = App {
            inner: Arc::new(AppInner {
                name: name.clone(),
                options,
                auth_ref: RwLock::new(None),
            }),
        };

        instances.insert(name, app.clone());

        Ok(app)
    }

    /// Get the default Firebase App instance
    ///
    /// # C++ Reference
    /// - `app/src/app.cc` - App::GetInstance()
    ///
    /// Returns the app with name "[DEFAULT]" if it exists.
    pub async fn get_instance() -> Result<Self, FirebaseError> {
        let instances = APP_INSTANCES.read().await;
        instances.get("[DEFAULT]").cloned().ok_or_else(|| {
            FirebaseError::Internal(
                "Default Firebase App not initialized. Call App::create() first.".to_string(),
            )
        })
    }

    /// Get a named Firebase App instance
    ///
    /// # C++ Reference
    /// - `app/src/app.cc` - App::GetInstance(name)
    pub async fn get_instance_with_name(name: &str) -> Result<Self, FirebaseError> {
        let instances = APP_INSTANCES.read().await;
        instances.get(name).cloned().ok_or_else(|| {
            FirebaseError::Internal(format!(
                "Firebase App '{}' not found. Call App::create() first.",
                name
            ))
        })
    }

    /// Get the app name
    pub fn name(&self) -> &str {
        &self.inner.name
    }

    /// Get the app options
    pub fn options(&self) -> &AppOptions {
        &self.inner.options
    }

    /// Internal: Register Auth instance with this App
    pub(crate) async fn register_auth(&self, auth_inner: Weak<AuthInner>) {
        *self.inner.auth_ref.write().await = Some(auth_inner);
    }

    /// Internal: Unregister Auth instance from this App
    #[allow(dead_code)]
    pub(crate) async fn unregister_auth(&self) {
        *self.inner.auth_ref.write().await = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_app() {
        let options = AppOptions {
            api_key: "test-api-key".to_string(),
            project_id: "test-project".to_string(),
            app_name: Some("test-app".to_string()),
        };

        let app = App::create(options).await.expect("Failed to create app");
        assert_eq!(app.name(), "test-app");
    }

    #[tokio::test]
    async fn test_create_app_singleton() {
        let options = AppOptions {
            api_key: "test-api-key-2".to_string(),
            project_id: "test-project-2".to_string(),
            app_name: Some("test-app-2".to_string()),
        };

        let app1 = App::create(options.clone())
            .await
            .expect("Failed to create app");
        let app2 = App::create(options).await.expect("Failed to create app");

        assert_eq!(app1.name(), app2.name());
    }

    #[tokio::test]
    async fn test_empty_api_key_error() {
        let options = AppOptions {
            api_key: "".to_string(),
            project_id: "test-project".to_string(),
            app_name: None,
        };

        let result = App::create(options).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_default_app_name() {
        let options = AppOptions {
            api_key: "test-api-key-3".to_string(),
            project_id: "test-project-3".to_string(),
            app_name: None,
        };

        let app = App::create(options).await.expect("Failed to create app");
        assert_eq!(app.name(), "[DEFAULT]");
    }
}

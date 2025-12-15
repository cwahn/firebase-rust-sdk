//! Firebase Authentication
//!
//! # C++ Reference
//! - `auth/src/auth.cc:65` - GetAuth implementation with global map
//! - `auth/src/include/firebase/auth.h:128` - Auth class

use crate::auth::types::{User, AuthResult};
use crate::error::{FirebaseError, AuthError};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global map of API keys to Auth instances
/// 
/// C++ equivalent: `std::map<App*, Auth*> g_auths` at auth/src/auth.cc:62
static AUTH_INSTANCES: Lazy<RwLock<HashMap<String, Auth>>> = 
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Firebase Authentication instance
///
/// # C++ Reference
/// - `auth/src/include/firebase/auth.h:128`
///
/// Each API key has at most one Auth instance (singleton pattern).
/// Use `Auth::get_auth(api_key)` to obtain or create an instance.
#[derive(Clone)]
pub struct Auth {
    inner: Arc<AuthInner>,
}

struct AuthInner {
    api_key: String,
    current_user: RwLock<Option<Arc<User>>>,
    http_client: reqwest::Client,
}

impl Auth {
    /// Get or create Auth instance for the given API key
    ///
    /// # C++ Reference
    /// - `auth/src/auth.cc:65` - Auth::GetAuth(app)
    ///
    /// Returns existing Auth if one exists for this API key, otherwise creates new.
    /// Thread-safe singleton pattern following C++ implementation.
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::Auth;
    ///
    /// let auth = Auth::get_auth("YOUR_API_KEY").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_auth(api_key: impl Into<String>) -> Result<Self, FirebaseError> {
        let api_key = api_key.into();
        
        // Validate API key (error case first)
        if api_key.is_empty() {
            return Err(FirebaseError::ApiKeyNotConfigured);
        }
        
        let mut instances = AUTH_INSTANCES.write().await;
        
        // Check if instance already exists
        let existing = instances.get(&api_key);
        if let Some(auth) = existing {
            return Ok(auth.clone());
        }
        
        // Create new Auth instance
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| FirebaseError::Internal(format!("Failed to create HTTP client: {}", e)))?;
        
        let auth = Auth {
            inner: Arc::new(AuthInner {
                api_key: api_key.clone(),
                current_user: RwLock::new(None),
                http_client,
            }),
        };
        
        instances.insert(api_key, auth.clone());
        
        Ok(auth)
    }

    /// Get the current signed-in user
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth.h:148` - current_user()
    ///
    /// Returns None if no user is currently signed in.
    pub async fn current_user(&self) -> Option<Arc<User>> {
        self.inner.current_user.read().await.clone()
    }

    /// Sign out the current user
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth.h:357` - SignOut()
    ///
    /// Always succeeds and clears the current user.
    pub async fn sign_out(&self) -> Result<(), FirebaseError> {
        let mut user = self.inner.current_user.write().await;
        *user = None;
        Ok(())
    }

    /// Get the API key for this Auth instance
    pub fn api_key(&self) -> &str {
        &self.inner.api_key
    }

    /// Internal: Get HTTP client
    pub(crate) fn http_client(&self) -> &reqwest::Client {
        &self.inner.http_client
    }

    /// Internal: Set current user
    pub(crate) async fn set_current_user(&self, user: Option<Arc<User>>) {
        let mut current = self.inner.current_user.write().await;
        *current = user;
    }

    /// Sign in with email and password
    ///
    /// # C++ Reference
    /// - `auth/src/desktop/auth_desktop.cc:405` - SignInWithEmailAndPassword
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::Auth;
    ///
    /// let auth = Auth::get_auth("YOUR_API_KEY").await?;
    /// let result = auth.sign_in_with_email_and_password("user@example.com", "password").await?;
    /// println!("Signed in: {}", result.user.uid);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn sign_in_with_email_and_password(
        &self,
        email: impl AsRef<str>,
        password: impl AsRef<str>,
    ) -> Result<AuthResult, FirebaseError> {
        let email = email.as_ref();
        let password = password.as_ref();

        // Validate email (error case first)
        if email.is_empty() {
            return Err(AuthError::InvalidEmail.into());
        }

        // Validate password (error case first)
        if password.is_empty() {
            return Err(AuthError::InvalidPassword.into());
        }

        // Call Firebase Auth REST API
        let url = format!(
            "https://identitytoolkit.googleapis.com/v1/accounts:signInWithPassword?key={}",
            self.inner.api_key
        );

        let response = self.inner.http_client
            .post(&url)
            .json(&serde_json::json!({
                "email": email,
                "password": password,
                "returnSecureToken": true
            }))
            .send()
            .await?;

        // Handle error responses first
        if !response.status().is_success() {
            let error_body: serde_json::Value = response.json().await?;
            let error_message = error_body["error"]["message"]
                .as_str()
                .unwrap_or("UNKNOWN_ERROR");
            return Err(AuthError::from_error_code(error_message).into());
        }

        // Parse successful response
        let user_data: SignInResponse = response.json().await?;
        let user = Arc::new(user_data.into_user());

        // Update current user
        self.set_current_user(Some(Arc::clone(&user))).await;

        Ok(AuthResult {
            user,
            additional_user_info: Some(crate::auth::types::AdditionalUserInfo {
                provider_id: "password".to_string(),
                is_new_user: false,
                profile: None,
            }),
        })
    }

    /// Create new user with email and password
    ///
    /// # C++ Reference
    /// - `auth/src/desktop/auth_desktop.cc:422` - CreateUserWithEmailAndPassword
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::Auth;
    ///
    /// let auth = Auth::get_auth("YOUR_API_KEY").await?;
    /// let result = auth.create_user_with_email_and_password("newuser@example.com", "password123").await?;
    /// println!("Created user: {}", result.user.uid);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_user_with_email_and_password(
        &self,
        email: impl AsRef<str>,
        password: impl AsRef<str>,
    ) -> Result<AuthResult, FirebaseError> {
        let email = email.as_ref();
        let password = password.as_ref();

        // Validate email (error case first)
        if email.is_empty() {
            return Err(AuthError::InvalidEmail.into());
        }

        // Validate password (error case first)
        if password.is_empty() {
            return Err(AuthError::InvalidPassword.into());
        }

        // Call Firebase Auth REST API
        let url = format!(
            "https://identitytoolkit.googleapis.com/v1/accounts:signUp?key={}",
            self.inner.api_key
        );

        let response = self.inner.http_client
            .post(&url)
            .json(&serde_json::json!({
                "email": email,
                "password": password,
                "returnSecureToken": true
            }))
            .send()
            .await?;

        // Handle error responses first
        if !response.status().is_success() {
            let error_body: serde_json::Value = response.json().await?;
            let error_message = error_body["error"]["message"]
                .as_str()
                .unwrap_or("UNKNOWN_ERROR");
            return Err(AuthError::from_error_code(error_message).into());
        }

        // Parse successful response
        let user_data: SignInResponse = response.json().await?;
        let user = Arc::new(user_data.into_user());

        // Update current user
        self.set_current_user(Some(Arc::clone(&user))).await;

        Ok(AuthResult {
            user,
            additional_user_info: Some(crate::auth::types::AdditionalUserInfo {
                provider_id: "password".to_string(),
                is_new_user: true,
                profile: None,
            }),
        })
    }
}

/// Firebase Auth REST API response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignInResponse {
    local_id: String,
    email: Option<String>,
    display_name: Option<String>,
    id_token: String,
    refresh_token: String,
    expires_in: Option<String>,
    registered: Option<bool>,
}

impl SignInResponse {
    fn into_user(self) -> User {
        User {
            uid: self.local_id,
            email: self.email,
            display_name: self.display_name,
            photo_url: None,
            phone_number: None,
            email_verified: false,
            is_anonymous: false,
            metadata: crate::auth::types::UserMetadata {
                creation_timestamp: chrono::Utc::now().timestamp_millis(),
                last_sign_in_timestamp: chrono::Utc::now().timestamp_millis(),
            },
            provider_data: vec![],
            id_token: Some(self.id_token),
            refresh_token: Some(self.refresh_token),
        }
    }
}

impl std::fmt::Debug for Auth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Auth")
            .field("api_key", &"<redacted>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_auth_creates_instance() {
        let auth = Auth::get_auth("test_api_key_1").await.unwrap();
        assert_eq!(auth.api_key(), "test_api_key_1");
    }

    #[tokio::test]
    async fn test_get_auth_returns_same_instance() {
        let auth1 = Auth::get_auth("test_api_key_2").await.unwrap();
        let auth2 = Auth::get_auth("test_api_key_2").await.unwrap();
        
        // Should return same instance (same Arc pointer)
        assert!(Arc::ptr_eq(&auth1.inner, &auth2.inner));
    }

    #[tokio::test]
    async fn test_get_auth_empty_key_error() {
        let result = Auth::get_auth("").await;
        assert!(matches!(result, Err(FirebaseError::ApiKeyNotConfigured)));
    }

    #[tokio::test]
    async fn test_current_user_initially_none() {
        let auth = Auth::get_auth("test_api_key_3").await.unwrap();
        assert!(auth.current_user().await.is_none());
    }

    #[tokio::test]
    async fn test_sign_out_clears_user() {
        let auth = Auth::get_auth("test_api_key_4").await.unwrap();
        
        // Set a user manually for testing
        use crate::auth::types::UserMetadata;
        let user = Arc::new(User {
            uid: "test_uid".to_string(),
            email: Some("test@example.com".to_string()),
            display_name: None,
            photo_url: None,
            phone_number: None,
            email_verified: false,
            is_anonymous: false,
            metadata: UserMetadata {
                creation_timestamp: 0,
                last_sign_in_timestamp: 0,
            },
            provider_data: vec![],
            id_token: None,
            refresh_token: None,
        });
        
        auth.set_current_user(Some(user)).await;
        assert!(auth.current_user().await.is_some());
        
        auth.sign_out().await.unwrap();
        assert!(auth.current_user().await.is_none());
    }

    #[tokio::test]
    async fn test_different_api_keys_different_instances() {
        let auth1 = Auth::get_auth("key_a").await.unwrap();
        let auth2 = Auth::get_auth("key_b").await.unwrap();
        
        // Should be different instances
        assert!(!Arc::ptr_eq(&auth1.inner, &auth2.inner));
        assert_eq!(auth1.api_key(), "key_a");
        assert_eq!(auth2.api_key(), "key_b");
    }

    #[tokio::test]
    async fn test_sign_in_validates_email() {
        let auth = Auth::get_auth("test_key").await.unwrap();
        let result = auth.sign_in_with_email_and_password("", "password").await;
        assert!(matches!(result, Err(FirebaseError::Auth(AuthError::InvalidEmail))));
    }

    #[tokio::test]
    async fn test_sign_in_validates_password() {
        let auth = Auth::get_auth("test_key").await.unwrap();
        let result = auth.sign_in_with_email_and_password("test@example.com", "").await;
        assert!(matches!(result, Err(FirebaseError::Auth(AuthError::InvalidPassword))));
    }

    #[tokio::test]
    async fn test_create_user_validates_email() {
        let auth = Auth::get_auth("test_key").await.unwrap();
        let result = auth.create_user_with_email_and_password("", "password123").await;
        assert!(matches!(result, Err(FirebaseError::Auth(AuthError::InvalidEmail))));
    }

    #[tokio::test]
    async fn test_create_user_validates_password() {
        let auth = Auth::get_auth("test_key").await.unwrap();
        let result = auth.create_user_with_email_and_password("new@example.com", "").await;
        assert!(matches!(result, Err(FirebaseError::Auth(AuthError::InvalidPassword))));
    }
}

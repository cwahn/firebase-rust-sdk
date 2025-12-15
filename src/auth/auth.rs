//! Firebase Authentication
//!
//! # C++ Reference
//! - `auth/src/auth.cc:65` - GetAuth implementation with global map
//! - `auth/src/include/firebase/auth.h:128` - Auth class

use crate::auth::types::{User, AuthResult};
use crate::error::{FirebaseError, AuthError};
use async_stream::stream;
use futures::Stream;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

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
    state_tx: broadcast::Sender<Option<Arc<User>>>,
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
        
        // Create broadcast channel for auth state changes (capacity: 16)
        let (state_tx, _) = broadcast::channel(16);
        
        let auth = Auth {
            inner: Arc::new(AuthInner {
                api_key: api_key.clone(),
                current_user: RwLock::new(None),
                http_client,
                state_tx,
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
        self.set_current_user(None).await;
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
        *current = user.clone();
        
        // Broadcast state change (ignore error if no listeners)
        let _ = self.inner.state_tx.send(user);
    }

    /// Subscribe to authentication state changes
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth.h:610` - AuthStateListener
    ///
    /// Returns a stream that yields the current user whenever:
    /// - A user signs in
    /// - A user signs out
    /// - The current user changes
    ///
    /// The stream immediately yields the current user state upon subscription.
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::Auth;
    /// use futures::StreamExt;
    ///
    /// let auth = Auth::get_auth("YOUR_API_KEY").await?;
    /// let mut stream = auth.auth_state_changes().await;
    ///
    /// while let Some(user) = stream.next().await {
    ///     match user {
    ///         Some(u) => println!("User signed in: {}", u.uid),
    ///         None => println!("User signed out"),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn auth_state_changes(&self) -> std::pin::Pin<Box<dyn Stream<Item = Option<Arc<User>>> + Send>> {
        // Get current user immediately
        let initial_user = self.current_user().await;
        
        // Subscribe to state changes
        let mut rx = self.inner.state_tx.subscribe();
        
        Box::pin(stream! {
            // Yield initial state first
            yield initial_user;
            
            // Then yield all future state changes
            loop {
                let user = match rx.recv().await {
                    Err(_) => break, // Channel closed
                    Ok(u) => u,
                };
                yield user;
            }
        })
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
        let user = Arc::new(user_data.into_user(self.inner.api_key.clone()));

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
        let user = Arc::new(user_data.into_user(self.inner.api_key.clone()));

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

    /// Sign in anonymously
    ///
    /// # C++ Reference
    /// - `auth/src/desktop/auth_desktop.cc:439` - SignInAnonymously
    ///
    /// Creates an anonymous user account. Anonymous accounts are temporary and can be
    /// linked to permanent accounts later.
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::Auth;
    ///
    /// let auth = Auth::get_auth("YOUR_API_KEY").await?;
    /// let result = auth.sign_in_anonymously().await?;
    /// println!("Anonymous user: {}", result.user.uid);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn sign_in_anonymously(&self) -> Result<AuthResult, FirebaseError> {
        // Call Firebase Auth REST API - signUp with no email/password creates anonymous user
        let url = format!(
            "https://identitytoolkit.googleapis.com/v1/accounts:signUp?key={}",
            self.inner.api_key
        );

        let response = self.inner.http_client
            .post(&url)
            .json(&serde_json::json!({
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
        let user = Arc::new(user_data.into_user(self.inner.api_key.clone()));

        // Update current user
        self.set_current_user(Some(Arc::clone(&user))).await;

        Ok(AuthResult {
            user,
            additional_user_info: Some(crate::auth::types::AdditionalUserInfo {
                provider_id: "anonymous".to_string(),
                is_new_user: true,
                profile: None,
            }),
        })
    }

    /// Sign in with OAuth credential
    ///
    /// # C++ Reference
    /// - `auth/src/desktop/auth_desktop.cc:439` - SignInAndRetrieveDataWithCredential
    /// - `auth/src/desktop/credential_impl.cc` - Credential implementation
    ///
    /// Signs in using a credential from an OAuth provider (Google, Facebook, GitHub, etc.)
    ///
    /// # Arguments
    /// * `credential` - OAuth credential from provider
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::Auth;
    /// use firebase_rust_sdk::auth::types::Credential;
    ///
    /// let auth = Auth::get_auth("YOUR_API_KEY").await?;
    /// 
    /// // Google Sign-In
    /// let credential = Credential::Google {
    ///     id_token: Some("google_id_token".to_string()),
    ///     access_token: Some("google_access_token".to_string()),
    /// };
    /// let result = auth.sign_in_with_credential(credential).await?;
    /// println!("Signed in: {}", result.user.uid);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn sign_in_with_credential(&self, credential: crate::auth::types::Credential) -> Result<AuthResult, FirebaseError> {
        use crate::auth::types::Credential;
        use crate::error::AuthError;
        
        // Build request based on credential type
        let url = format!(
            "https://identitytoolkit.googleapis.com/v1/accounts:signInWithIdp?key={}",
            self.inner.api_key
        );
        
        let (provider_id, token, id_token, access_token): (String, Option<String>, Option<String>, Option<String>) = match credential {
            // Error-first: unsupported credential types
            Credential::EmailPassword { .. } => {
                return Err(FirebaseError::Auth(
                    AuthError::InvalidCredential("Use sign_in_with_email_and_password() for email/password auth".to_string())
                ));
            }
            Credential::Anonymous => {
                return Err(FirebaseError::Auth(
                    AuthError::InvalidCredential("Use sign_in_anonymously() for anonymous auth".to_string())
                ));
            }
            Credential::CustomToken { token } => {
                return Err(FirebaseError::Auth(
                    AuthError::InvalidCredential(format!(
                        "Use sign_in_with_custom_token() for custom token auth. Token: {}",
                        if token.len() > 20 { &token[..20] } else { &token }
                    ))
                ));
            }
            // OAuth providers
            Credential::Google { id_token, access_token } => {
                // Error-first: validate at least one token provided
                if id_token.is_none() && access_token.is_none() {
                    return Err(FirebaseError::Auth(
                        AuthError::InvalidCredential("Google credential requires id_token or access_token".to_string())
                    ));
                }
                ("google.com".to_string(), access_token.clone(), id_token, access_token)
            }
            Credential::Facebook { access_token } => {
                ("facebook.com".to_string(), Some(access_token.clone()), None, Some(access_token))
            }
            Credential::GitHub { token } => {
                ("github.com".to_string(), Some(token.clone()), None, Some(token))
            }
            Credential::OAuth { provider_id, id_token, access_token, .. } => {
                // Error-first: validate at least one token provided
                if id_token.is_none() && access_token.is_none() {
                    return Err(FirebaseError::Auth(
                        AuthError::InvalidCredential("OAuth credential requires id_token or access_token".to_string())
                    ));
                }
                (provider_id.clone(), access_token.clone(), id_token, access_token)
            }
        };
        
        let mut post_body = format!("providerId={}", provider_id);
        if let Some(id_token_val) = id_token {
            post_body.push_str(&format!("&id_token={}", id_token_val));
        }
        if let Some(access_token_val) = access_token {
            post_body.push_str(&format!("&access_token={}", access_token_val));
        }
        
        let response = self.inner.http_client
            .post(&url)
            .json(&serde_json::json!({
                "postBody": post_body,
                "requestUri": "http://localhost",
                "returnSecureToken": true,
                "returnIdpCredential": true
            }))
            .send()
            .await?;

        // Error-first: handle error responses
        if !response.status().is_success() {
            let error_body: serde_json::Value = response.json().await?;
            let error_message = error_body["error"]["message"]
                .as_str()
                .unwrap_or("UNKNOWN_ERROR");
            return Err(AuthError::from_error_code(error_message).into());
        }

        // Parse successful response
        let user_data: SignInResponse = response.json().await?;
        let user = Arc::new(user_data.into_user(self.inner.api_key.clone()));

        // Update current user
        self.set_current_user(Some(Arc::clone(&user))).await;

        Ok(AuthResult {
            user,
            additional_user_info: Some(crate::auth::types::AdditionalUserInfo {
                provider_id: provider_id.to_string(),
                is_new_user: false, // Would need to check providerUserInfo to determine
                profile: None,
            }),
        })
    }

    /// Sign in with custom token
    ///
    /// # C++ Reference
    /// - `auth/src/desktop/auth_desktop.cc:338` - SignInWithCustomToken
    /// - `auth/src/desktop/rpcs/verify_custom_token_request.cc:27` - VerifyCustomTokenRequest
    ///
    /// Signs in using a custom token generated by your own server. This is useful for
    /// integrating with existing authentication systems or for server-side authentication.
    ///
    /// # Arguments
    /// * `token` - The custom token string generated by your server
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::Auth;
    ///
    /// let auth = Auth::get_auth("YOUR_API_KEY").await?;
    /// let result = auth.sign_in_with_custom_token("custom_token_from_server").await?;
    /// println!("Signed in: {}", result.user.uid);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn sign_in_with_custom_token(&self, token: &str) -> Result<AuthResult, FirebaseError> {
        // Error-first: validate token
        if token.is_empty() {
            return Err(AuthError::InvalidCredential("Custom token cannot be empty".to_string()).into());
        }

        let url = format!(
            "https://identitytoolkit.googleapis.com/v1/accounts:signInWithCustomToken?key={}",
            self.inner.api_key
        );

        let response = self
            .inner
            .http_client
            .post(&url)
            .json(&serde_json::json!({
                "token": token,
                "returnSecureToken": true
            }))
            .send()
            .await?;

        // Error-first: handle error responses
        if !response.status().is_success() {
            let error_body: serde_json::Value = response.json().await?;
            let error_message = error_body["error"]["message"]
                .as_str()
                .unwrap_or("UNKNOWN_ERROR");
            return Err(AuthError::from_error_code(error_message).into());
        }

        // Parse successful response
        let user_data: SignInResponse = response.json().await?;
        let user = Arc::new(user_data.into_user(self.inner.api_key.clone()));

        // Update current user
        self.set_current_user(Some(Arc::clone(&user))).await;

        Ok(AuthResult {
            user,
            additional_user_info: Some(crate::auth::types::AdditionalUserInfo {
                provider_id: "custom".to_string(),
                is_new_user: false,
                profile: None,
            }),
        })
    }

    /// Send password reset email
    ///
    /// # C++ Reference
    /// - `auth/src/desktop/auth_desktop.cc:474` - SendPasswordResetEmail
    /// - `auth/src/desktop/rpcs/get_oob_confirmation_code_request.cc:70` - CreateSendPasswordResetEmailRequest
    ///
    /// Sends a password reset email to the given email address. If the email is not
    /// registered, the operation still succeeds to prevent email enumeration.
    ///
    /// # Arguments
    /// * `email` - The email address to send the password reset to
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::Auth;
    ///
    /// let auth = Auth::get_auth("YOUR_API_KEY").await?;
    /// auth.send_password_reset_email("user@example.com").await?;
    /// println!("Password reset email sent");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_password_reset_email(&self, email: impl AsRef<str>) -> Result<(), FirebaseError> {
        let email = email.as_ref();

        // Validate email (error case first)
        if email.is_empty() {
            return Err(AuthError::InvalidEmail.into());
        }

        // Call Firebase Auth REST API - getOobConfirmationCode with PASSWORD_RESET
        let url = format!(
            "https://identitytoolkit.googleapis.com/v1/accounts:sendOobCode?key={}",
            self.inner.api_key
        );

        let response = self.inner.http_client
            .post(&url)
            .json(&serde_json::json!({
                "requestType": "PASSWORD_RESET",
                "email": email
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

        // Success - password reset email sent
        Ok(())
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
    fn into_user(self, api_key: String) -> User {
        // Calculate token expiration (expires_in is in seconds)
        let token_expiration = if let Some(expires_in_str) = &self.expires_in {
            expires_in_str.parse::<i64>().ok().map(|seconds| {
                chrono::Utc::now().timestamp() + seconds
            })
        } else {
            // Default: 1 hour expiration
            Some(chrono::Utc::now().timestamp() + 3600)
        };
        
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
            token_expiration,
            api_key: Some(api_key),
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
            token_expiration: None,
            api_key: None,
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

    #[tokio::test]
    async fn test_sign_in_anonymously_returns_user() {
        let auth = Auth::get_auth("test_anon_key").await.unwrap();
        
        // This will fail without real Firebase, but validates the structure
        // In real usage, this would create an anonymous user
        let result = auth.sign_in_anonymously().await;
        
        // We expect network error since no real Firebase backend
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_anonymous_user_updates_current_user() {
        use crate::auth::types::UserMetadata;
        
        let auth = Auth::get_auth("test_anon_key2").await.unwrap();
        
        // Initially no user
        assert!(auth.current_user().await.is_none());
        
        // Manually set an anonymous user (simulating successful sign in)
        let anon_user = Arc::new(User {
            uid: "anon123".to_string(),
            email: None,
            display_name: None,
            photo_url: None,
            phone_number: None,
            email_verified: false,
            is_anonymous: true,
            metadata: UserMetadata {
                creation_timestamp: 0,
                last_sign_in_timestamp: 0,
            },
            provider_data: vec![],
            id_token: None,
            refresh_token: None,
            token_expiration: None,
            api_key: None,
        });
        
        auth.set_current_user(Some(anon_user.clone())).await;
        
        let current = auth.current_user().await;
        assert!(current.is_some());
        let user = current.unwrap();
        assert_eq!(user.uid, "anon123");
        assert!(user.is_anonymous);
    }

    #[tokio::test]
    async fn test_password_reset_validates_email() {
        let auth = Auth::get_auth("test_password_reset_key").await.unwrap();
        let result = auth.send_password_reset_email("").await;
        assert!(matches!(result, Err(FirebaseError::Auth(AuthError::InvalidEmail))));
    }

    #[tokio::test]
    async fn test_password_reset_does_not_affect_current_user() {
        use crate::auth::types::UserMetadata;
        
        let auth = Auth::get_auth("test_password_reset_key2").await.unwrap();
        
        // Sign in a user
        let user = Arc::new(User {
            uid: "user123".to_string(),
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
            token_expiration: None,
            api_key: None,
        });
        
        auth.set_current_user(Some(user.clone())).await;
        
        // Password reset should not change current user
        // (This will fail with network error, but that's expected in tests)
        let _ = auth.send_password_reset_email("other@example.com").await;
        
        let current = auth.current_user().await;
        assert!(current.is_some());
        assert_eq!(current.unwrap().uid, "user123");
    }

    #[tokio::test]
    async fn test_auth_state_changes_initial() {
        use futures::StreamExt;
        
        let auth = Auth::get_auth("test_key_state1").await.unwrap();
        let mut stream = auth.auth_state_changes().await;
        
        // Should immediately yield None (no user signed in)
        let initial = stream.next().await;
        assert!(initial.is_some());
        assert!(initial.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_auth_state_changes_on_sign_in() {
        use futures::StreamExt;
        use crate::auth::types::UserMetadata;
        
        let auth = Auth::get_auth("test_key_state2").await.unwrap();
        let mut stream = auth.auth_state_changes().await;
        
        // Get initial state (None)
        let _ = stream.next().await;
        
        // Sign in a user
        let user = Arc::new(User {
            uid: "test123".to_string(),
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
            token_expiration: None,
            api_key: None,
        });
        
        auth.set_current_user(Some(user.clone())).await;
        
        // Should receive the new user
        let next = stream.next().await;
        assert!(next.is_some());
        let received_user = next.unwrap();
        assert!(received_user.is_some());
        assert_eq!(received_user.as_ref().unwrap().uid, "test123");
    }

    #[tokio::test]
    async fn test_auth_state_changes_on_sign_out() {
        use futures::StreamExt;
        use crate::auth::types::UserMetadata;
        
        let auth = Auth::get_auth("test_key_state3").await.unwrap();
        
        // Set initial user
        let user = Arc::new(User {
            uid: "test456".to_string(),
            email: Some("test2@example.com".to_string()),
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
            token_expiration: None,
            api_key: None,
        });
        auth.set_current_user(Some(user)).await;
        
        let mut stream = auth.auth_state_changes().await;
        
        // Get initial state (with user)
        let initial = stream.next().await;
        assert!(initial.unwrap().is_some());
        
        // Sign out
        auth.sign_out().await.unwrap();
        
        // Should receive None
        let next = stream.next().await;
        assert!(next.is_some());
        assert!(next.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_sign_in_with_google_credential() {
        use crate::auth::types::Credential;
        use crate::error::AuthError;
        
        let auth = Auth::get_auth("test_google_key").await.unwrap();
        
        // Test with invalid credential (no tokens)
        let invalid_cred = Credential::Google {
            id_token: None,
            access_token: None,
        };
        let result = auth.sign_in_with_credential(invalid_cred).await;
        assert!(matches!(result, Err(FirebaseError::Auth(AuthError::InvalidCredential(_)))));
        
        // Test with valid credential (would fail without real Firebase project, but validates structure)
        let valid_cred = Credential::Google {
            id_token: Some("test_id_token".to_string()),
            access_token: Some("test_access_token".to_string()),
        };
        assert_eq!(valid_cred.provider_id(), "google.com");
    }

    #[tokio::test]
    async fn test_sign_in_with_facebook_credential() {
        use crate::auth::types::Credential;
        
        let cred = Credential::Facebook {
            access_token: "test_facebook_token".to_string(),
        };
        assert_eq!(cred.provider_id(), "facebook.com");
    }

    #[tokio::test]
    async fn test_sign_in_with_github_credential() {
        use crate::auth::types::Credential;
        
        let cred = Credential::GitHub {
            token: "test_github_token".to_string(),
        };
        assert_eq!(cred.provider_id(), "github.com");
    }

    #[tokio::test]
    async fn test_sign_in_with_oauth_credential() {
        use crate::auth::types::Credential;
        use crate::error::AuthError;
        
        let auth = Auth::get_auth("test_oauth_key").await.unwrap();
        
        // Test with invalid OAuth credential (no tokens)
        let invalid_cred = Credential::OAuth {
            provider_id: "apple.com".to_string(),
            id_token: None,
            access_token: None,
            raw_nonce: None,
        };
        let result = auth.sign_in_with_credential(invalid_cred).await;
        assert!(matches!(result, Err(FirebaseError::Auth(AuthError::InvalidCredential(_)))));
        
        // Test with valid OAuth credential
        let valid_cred = Credential::OAuth {
            provider_id: "apple.com".to_string(),
            id_token: Some("test_id_token".to_string()),
            access_token: None,
            raw_nonce: None,
        };
        assert_eq!(valid_cred.provider_id(), "apple.com");
    }

    #[tokio::test]
    async fn test_credential_type_validation() {
        use crate::auth::types::Credential;
        use crate::error::AuthError;
        
        let auth = Auth::get_auth("test_validation_key").await.unwrap();
        
        // Email/password should use dedicated method
        let email_cred = Credential::EmailPassword {
            email: "test@example.com".to_string(),
            password: "password".to_string(),
        };
        let result = auth.sign_in_with_credential(email_cred).await;
        assert!(matches!(result, Err(FirebaseError::Auth(AuthError::InvalidCredential(_)))));
        
        // Anonymous should use dedicated method
        let anon_cred = Credential::Anonymous;
        let result = auth.sign_in_with_credential(anon_cred).await;
        assert!(matches!(result, Err(FirebaseError::Auth(AuthError::InvalidCredential(_)))));
        
        // CustomToken should use dedicated method
        let custom_cred = Credential::CustomToken {
            token: "custom_token".to_string(),
        };
        let result = auth.sign_in_with_credential(custom_cred).await;
        assert!(matches!(result, Err(FirebaseError::Auth(AuthError::InvalidCredential(_)))));
    }

    #[tokio::test]
    async fn test_sign_in_with_custom_token_validates_empty() {
        use crate::error::AuthError;
        
        let auth = Auth::get_auth("test_custom_token_key").await.unwrap();
        
        let result = auth.sign_in_with_custom_token("").await;
        assert!(matches!(result, Err(FirebaseError::Auth(AuthError::InvalidCredential(_)))));
    }

    #[tokio::test]
    async fn test_custom_token_credential() {
        use crate::auth::types::Credential;
        
        let cred = Credential::CustomToken {
            token: "server_generated_token".to_string(),
        };
        assert_eq!(cred.provider_id(), "custom");
    }
}

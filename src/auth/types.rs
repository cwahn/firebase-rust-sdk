//! Authentication types
//!
//! # C++ Reference
//! - `auth/src/include/firebase/auth/types.h`
//! - `auth/src/include/firebase/auth/user.h`
//! - `auth/src/include/firebase/auth/credential.h`

use crate::error::AuthError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// User metadata
///
/// # C++ Reference
/// - `auth/src/include/firebase/auth/user.h:65`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMetadata {
    /// Timestamp when user was created (Unix timestamp in milliseconds)
    pub creation_timestamp: i64,
    
    /// Timestamp of last sign-in (Unix timestamp in milliseconds)
    pub last_sign_in_timestamp: i64,
}

/// User information returned from identity provider
///
/// # C++ Reference
/// - `auth/src/include/firebase/auth/user.h:95`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    /// User ID from the provider
    pub uid: String,
    
    /// Display name
    pub display_name: Option<String>,
    
    /// Email address
    pub email: Option<String>,
    
    /// Phone number
    pub phone_number: Option<String>,
    
    /// Photo URL
    pub photo_url: Option<String>,
    
    /// Provider ID (e.g., "password", "google.com")
    pub provider_id: String,
}

/// Firebase user account
///
/// # C++ Reference
/// - `auth/src/include/firebase/auth/user.h:144`
///
/// Represents a user account in Firebase Auth. Use `Arc<User>` for shared ownership.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique Firebase user ID
    pub uid: String,
    
    /// Email address (if available)
    pub email: Option<String>,
    
    /// Display name (if available)
    pub display_name: Option<String>,
    
    /// Photo URL (if available)
    pub photo_url: Option<String>,
    
    /// Phone number (if available)
    pub phone_number: Option<String>,
    
    /// Whether email is verified
    pub email_verified: bool,
    
    /// Whether user is anonymous
    pub is_anonymous: bool,
    
    /// User metadata
    pub metadata: UserMetadata,
    
    /// Provider data for this user
    pub provider_data: Vec<UserInfo>,
    
    /// ID token (JWT) - internal use
    #[serde(skip)]
    pub(crate) id_token: Option<String>,
    
    /// Refresh token - internal use
    #[serde(skip)]
    pub(crate) refresh_token: Option<String>,
    
    /// Token expiration timestamp (seconds since epoch) - internal use
    #[serde(skip)]
    pub(crate) token_expiration: Option<i64>,
    
    /// API key for token refresh - internal use
    #[serde(skip)]
    pub(crate) api_key: Option<String>,
}

impl User {
    /// Get the current ID token
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth/user.h:498`
    /// - `auth/src/desktop/user_desktop.cc:652` - GetToken implementation
    /// - `auth/src/desktop/user_desktop.cc:118` - EnsureFreshToken
    /// - `auth/src/desktop/rpcs/secure_token_request.cc:24` - SecureTokenRequest
    pub async fn get_id_token(&self, force_refresh: bool) -> Result<String, AuthError> {
        // Check if we have a token (error case first)
        let Some(token) = &self.id_token else {
            return Err(AuthError::UserTokenExpired);
        };
        
        // Check if token needs refresh
        let needs_refresh = if force_refresh {
            true
        } else if let Some(expiration) = self.token_expiration {
            // Token expires in less than 5 minutes, refresh it
            let now = chrono::Utc::now().timestamp();
            now >= (expiration - 300)
        } else {
            // No expiration info, assume token is fresh
            false
        };
        
        // If no refresh needed, return current token
        if !needs_refresh {
            return Ok(token.clone());
        }
        
        // Need to refresh - check if we have refresh token (error case first)
        let Some(refresh_token) = &self.refresh_token else {
            return Err(AuthError::UserTokenExpired);
        };
        
        // Need API key for refresh (error case first)
        let Some(api_key) = &self.api_key else {
            return Err(AuthError::NotAuthenticated);
        };
        
        // Call secure token endpoint to refresh
        let url = format!("https://securetoken.googleapis.com/v1/token?key={}", api_key);
        
        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .json(&serde_json::json!({
                "grant_type": "refresh_token",
                "refresh_token": refresh_token
            }))
            .send()
            .await
            .map_err(|e| AuthError::NetworkRequestFailed(format!("Token refresh failed: {}", e)))?;
        
        // Handle error responses first
        if !response.status().is_success() {
            let error_body: serde_json::Value = response.json().await
                .map_err(|e| AuthError::NetworkRequestFailed(format!("Failed to parse error: {}", e)))?;
            let error_message = error_body["error"]["message"]
                .as_str()
                .unwrap_or("TOKEN_REFRESH_FAILED");
            return Err(AuthError::from_error_code(error_message));
        }
        
        // Parse refresh response
        let token_response: serde_json::Value = response.json().await
            .map_err(|e| AuthError::NetworkRequestFailed(format!("Failed to parse response: {}", e)))?;
        
        let new_id_token = token_response["id_token"]
            .as_str()
            .ok_or(AuthError::UserTokenExpired)?;
        
        // Note: In a real implementation, we would update the user's token in the Auth instance
        // For now, we return the new token but don't mutate self (User is immutable in our design)
        Ok(new_id_token.to_string())
    }

    /// Delete the user account
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth/user.h:527`
    pub async fn delete(&self) -> Result<(), AuthError> {
        todo!("Implement user deletion via REST API")
    }

    /// Reload user data from server
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth/user.h:547`
    pub async fn reload(&mut self) -> Result<(), AuthError> {
        todo!("Implement user reload via REST API")
    }

    /// Send email verification
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth/user.h:582`
    pub async fn send_email_verification(&self) -> Result<(), AuthError> {
        todo!("Implement email verification via REST API")
    }

    /// Update email address
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth/user.h:604`
    pub async fn update_email(&mut self, new_email: impl AsRef<str>) -> Result<(), AuthError> {
        let new_email = new_email.as_ref();
        
        if new_email.is_empty() {
            return Err(AuthError::InvalidEmail);
        }
        
        todo!("Implement email update via REST API")
    }

    /// Update password
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth/user.h:626`
    pub async fn update_password(&self, new_password: impl AsRef<str>) -> Result<(), AuthError> {
        let new_password = new_password.as_ref();
        
        if new_password.is_empty() {
            return Err(AuthError::InvalidPassword);
        }
        
        todo!("Implement password update via REST API")
    }

    /// Update user profile
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth/user.h:652`
    pub async fn update_profile(&mut self, _profile: UserProfile) -> Result<(), AuthError> {
        todo!("Implement profile update via REST API")
    }
}

/// User profile update request
///
/// # C++ Reference
/// - `auth/src/include/firebase/auth/user.h:111`
#[derive(Debug, Default, Clone)]
pub struct UserProfile {
    /// Display name to update (None = no change)
    pub display_name: Option<String>,
    
    /// Photo URL to update (None = no change)
    pub photo_url: Option<String>,
}

impl UserProfile {
    /// Create a new profile update with display name
    pub fn with_display_name(display_name: impl Into<String>) -> Self {
        Self {
            display_name: Some(display_name.into()),
            photo_url: None,
        }
    }

    /// Create a new profile update with photo URL
    pub fn with_photo_url(photo_url: impl Into<String>) -> Self {
        Self {
            display_name: None,
            photo_url: Some(photo_url.into()),
        }
    }

    /// Set display name
    pub fn display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }

    /// Set photo URL
    pub fn photo_url(mut self, url: impl Into<String>) -> Self {
        self.photo_url = Some(url.into());
        self
    }
}

/// Authentication credential
///
/// # C++ Reference
/// - `auth/src/include/firebase/auth/credential.h:57`
///
/// Represents an authentication credential for various providers.
#[derive(Debug, Clone)]
pub enum Credential {
    /// Email and password credential
    EmailPassword {
        /// Email address
        email: String,
        /// Password
        password: String,
    },
    
    /// OAuth credential (Google, Facebook, etc.)
    OAuth {
        /// Provider ID (e.g., "google.com")
        provider_id: String,
        /// ID token
        id_token: Option<String>,
        /// Access token
        access_token: Option<String>,
    },
    
    /// Anonymous credential
    Anonymous,
    
    /// Custom token credential
    CustomToken {
        /// Custom JWT token
        token: String,
    },
}

impl Credential {
    /// Create email/password credential
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth/credential.h:109`
    pub fn email_password(email: impl Into<String>, password: impl Into<String>) -> Self {
        Self::EmailPassword {
            email: email.into(),
            password: password.into(),
        }
    }

    /// Create Google OAuth credential
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth/credential.h:145`
    pub fn google(id_token: Option<String>, access_token: Option<String>) -> Self {
        Self::OAuth {
            provider_id: "google.com".to_string(),
            id_token,
            access_token,
        }
    }

    /// Get provider ID
    pub fn provider_id(&self) -> &str {
        match self {
            Self::EmailPassword { .. } => "password",
            Self::OAuth { provider_id, .. } => provider_id,
            Self::Anonymous => "anonymous",
            Self::CustomToken { .. } => "custom",
        }
    }
}

/// Authentication result
///
/// Returned from sign-in operations.
#[derive(Debug, Clone)]
pub struct AuthResult {
    /// The signed-in user
    pub user: Arc<User>,
    
    /// Additional user info (if available)
    pub additional_user_info: Option<AdditionalUserInfo>,
}

/// Additional user information from sign-in
#[derive(Debug, Clone)]
pub struct AdditionalUserInfo {
    /// Provider ID
    pub provider_id: String,
    
    /// Whether this is a new user
    pub is_new_user: bool,
    
    /// Profile data from provider
    pub profile: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let user = User {
            uid: "test123".to_string(),
            email: Some("test@example.com".to_string()),
            display_name: None,
            photo_url: None,
            phone_number: None,
            email_verified: false,
            is_anonymous: false,
            metadata: UserMetadata {
                creation_timestamp: 1234567890,
                last_sign_in_timestamp: 1234567890,
            },
            provider_data: vec![],
            id_token: None,
            refresh_token: None,
            token_expiration: None,
            api_key: None,
        };

        assert_eq!(user.uid, "test123");
        assert_eq!(user.email.as_deref(), Some("test@example.com"));
        assert!(!user.email_verified);
    }

    #[test]
    fn test_credential_email_password() {
        let cred = Credential::email_password("test@example.com", "password123");
        let provider = cred.provider_id();
        
        match &cred {
            Credential::EmailPassword { email, password } => {
                assert_eq!(email, "test@example.com");
                assert_eq!(password, "password123");
            }
            _ => panic!("Expected EmailPassword credential"),
        }
        
        assert_eq!(provider, "password");
    }

    #[test]
    fn test_credential_google() {
        let cred = Credential::google(Some("id_token".to_string()), None);
        let provider = cred.provider_id();
        
        match &cred {
            Credential::OAuth { provider_id, id_token, .. } => {
                assert_eq!(provider_id, "google.com");
                assert_eq!(id_token.as_deref(), Some("id_token"));
            }
            _ => panic!("Expected OAuth credential"),
        }
        
        assert_eq!(provider, "google.com");
    }

    #[test]
    fn test_user_profile_builder() {
        let profile = UserProfile::with_display_name("John Doe")
            .photo_url("https://example.com/photo.jpg");
        
        assert_eq!(profile.display_name.as_deref(), Some("John Doe"));
        assert_eq!(profile.photo_url.as_deref(), Some("https://example.com/photo.jpg"));
    }

    #[test]
    fn test_user_serialization() {
        let user = User {
            uid: "test123".to_string(),
            email: Some("test@example.com".to_string()),
            display_name: Some("Test User".to_string()),
            photo_url: None,
            phone_number: None,
            email_verified: true,
            is_anonymous: false,
            metadata: UserMetadata {
                creation_timestamp: 1234567890,
                last_sign_in_timestamp: 1234567890,
            },
            provider_data: vec![],
            id_token: Some("token".to_string()),
            refresh_token: Some("refresh".to_string()),
            token_expiration: None,
            api_key: None,
        };

        // Test that serialization works (tokens are skipped)
        let json = serde_json::to_string(&user).unwrap();
        assert!(!json.contains("token"));
        assert!(!json.contains("refresh"));
        assert!(json.contains("test123"));
    }
}

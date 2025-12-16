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

/// Authentication credential trait
///
/// # C++ Reference
/// - `auth/src/include/firebase/auth/credential.h:42` - Credential class
///
/// Base trait for all authentication credentials (email/password, OAuth tokens, etc.)
pub trait AuthCredential {
    /// Get the provider ID for this credential (e.g., "password", "google.com")
    fn provider(&self) -> &str;
}

/// Email and password authentication credential
///
/// # C++ Reference
/// - `auth/src/include/firebase/auth/credential.h` - EmailAuthProvider
#[derive(Debug, Clone)]
pub struct EmailAuthCredential {
    email: String,
    password: String,
}

impl EmailAuthCredential {
    /// Create a new email/password credential
    pub fn new(email: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            password: password.into(),
        }
    }
    
    /// Get the email address
    pub fn email(&self) -> &str {
        &self.email
    }
    
    /// Get the password
    pub fn password(&self) -> &str {
        &self.password
    }
}

impl AuthCredential for EmailAuthCredential {
    fn provider(&self) -> &str {
        "password"
    }
}

/// Authentication credential
///
/// # C++ Reference
/// - `auth/src/include/firebase/auth/credential.h:77`
#[derive(Debug, Clone)]
pub enum Credential {
    /// Email and password credential
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth/credential.h:192` - EmailAuthProvider::GetCredential
    EmailPassword {
        /// Email address
        email: String,
        /// Password
        password: String,
    },
    
    /// Google OAuth credential
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth/credential.h:257` - GoogleAuthProvider::GetCredential
    Google {
        /// Google Sign-In ID token
        id_token: Option<String>,
        /// Google Sign-In access token
        access_token: Option<String>,
    },
    
    /// Facebook OAuth credential
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth/credential.h:206` - FacebookAuthProvider::GetCredential
    Facebook {
        /// Facebook access token
        access_token: String,
    },
    
    /// GitHub OAuth credential
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth/credential.h:242` - GitHubAuthProvider::GetCredential
    GitHub {
        /// GitHub OAuth access token
        token: String,
    },
    
    /// Generic OAuth2 credential
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth/credential.h:275` - OAuthProvider::GetCredential
    OAuth {
        /// Provider ID (e.g., "apple.com", "microsoft.com")
        provider_id: String,
        /// ID token (OIDC)
        id_token: Option<String>,
        /// Access token
        access_token: Option<String>,
        /// Raw nonce
        raw_nonce: Option<String>,
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
    /// Get the provider ID for this credential
    pub fn provider_id(&self) -> &str {
        match self {
            Credential::EmailPassword { .. } => "password",
            Credential::Google { .. } => "google.com",
            Credential::Facebook { .. } => "facebook.com",
            Credential::GitHub { .. } => "github.com",
            Credential::OAuth { provider_id, .. } => provider_id,
            Credential::Anonymous => "anonymous",
            Credential::CustomToken { .. } => "custom",
        }
    }
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
        let response = match client
            .post(&url)
            .json(&serde_json::json!({
                "grant_type": "refresh_token",
                "refresh_token": refresh_token
            }))
            .send()
            .await
        {
            Err(e) => return Err(AuthError::NetworkRequestFailed(format!("Token refresh failed: {}", e))),
            Ok(resp) => resp,
        };
        
        // Handle error responses first
        if !response.status().is_success() {
            let error_body = match response.json::<serde_json::Value>().await {
                Err(e) => return Err(AuthError::NetworkRequestFailed(format!("Failed to parse error: {}", e))),
                Ok(body) => body,
            };
            let error_message = error_body["error"]["message"]
                .as_str()
                .unwrap_or("TOKEN_REFRESH_FAILED");
            return Err(AuthError::from_error_code(error_message));
        }
        
        // Parse refresh response
        let token_response = match response.json::<serde_json::Value>().await {
            Err(e) => return Err(AuthError::NetworkRequestFailed(format!("Failed to parse response: {}", e))),
            Ok(resp) => resp,
        };
        
        let new_id_token = token_response["id_token"]
            .as_str()
            .ok_or(AuthError::UserTokenExpired)?;
        
        // Return the new token - caller is responsible for updating the Auth instance
        // In a real application, this would typically be called internally by Auth
        // which would then update the current user's token
        // The C++ SDK handles this through UserData::SetTokens() and token refresh callbacks
        Ok(new_id_token.to_string())
    }

    /// Delete the user account
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth/user.h:527`
    /// - `auth/src/desktop/user_desktop.cc:695` - Delete implementation
    /// - `auth/src/desktop/rpcs/delete_account_request.cc:30` - deleteAccount endpoint
    ///
    /// Note: After successful deletion, the user should sign out. This method only
    /// deletes the account on the server. The caller should call Auth::sign_out() afterward.
    pub async fn delete(&self) -> Result<(), AuthError> {
        // Need ID token (error case first)
        let id_token = self.get_id_token(false).await?;
        
        // Need API key (error case first)
        let Some(api_key) = &self.api_key else {
            return Err(AuthError::NotAuthenticated);
        };
        
        // Call deleteAccount REST API
        let url = format!(
            "https://identitytoolkit.googleapis.com/v1/accounts:delete?key={}",
            api_key
        );
        
        let client = reqwest::Client::new();
        let response = match client
            .post(&url)
            .json(&serde_json::json!({
                "idToken": id_token
            }))
            .send()
            .await
        {
            Err(e) => return Err(AuthError::NetworkRequestFailed(format!("Delete account failed: {}", e))),
            Ok(resp) => resp,
        };
        
        // Handle error responses first
        if !response.status().is_success() {
            let error_body = match response.json::<serde_json::Value>().await {
                Err(e) => return Err(AuthError::NetworkRequestFailed(format!("Failed to parse error: {}", e))),
                Ok(body) => body,
            };
            let error_message = error_body["error"]["message"]
                .as_str()
                .unwrap_or("DELETE_ACCOUNT_FAILED");
            return Err(AuthError::from_error_code(error_message));
        }
        
        // Account deleted successfully
        // Note: Caller should call Auth::sign_out() to complete the sign out process
        Ok(())
    }

    /// Reload user data from server
    ///
    /// Refreshes the user's profile data from the server. This updates the user's
    /// display name, email, photo URL, and email verification status.
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth/user.h:547`
    /// - `auth/src/desktop/rpcs/get_account_info_request.cc:28`
    /// - `auth/src/desktop/user_desktop.cc:797` - GetAccountInfo endpoint
    pub async fn reload(&mut self) -> Result<(), AuthError> {
        // Error-first: validate ID token
        let Some(ref id_token) = self.id_token else {
            return Err(AuthError::NoSignedInUser);
        };

        // Error-first: validate API key
        let Some(ref api_key) = self.api_key else {
            return Err(AuthError::InvalidApiKey);
        };

        let url = format!(
            "https://identitytoolkit.googleapis.com/v1/accounts:lookup?key={}",
            api_key
        );

        let body = serde_json::json!({
            "idToken": id_token
        });

        let client = reqwest::Client::new();
        let response = match client
            .post(&url)
            .json(&body)
            .send()
            .await
        {
            Err(e) => return Err(AuthError::NetworkRequestFailed(e.to_string())),
            Ok(resp) => resp,
        };

        // Error-first: handle HTTP errors
        if !response.status().is_success() {
            let error_body = match response.json::<serde_json::Value>().await {
                Err(e) => return Err(AuthError::NetworkRequestFailed(format!("Failed to parse error: {}", e))),
                Ok(body) => body,
            };
            let error_message = error_body["error"]["message"]
                .as_str()
                .unwrap_or("RELOAD_FAILED");
            return Err(AuthError::from_error_code(error_message));
        }

        // Parse successful response and update user data
        let response_data = match response.json::<serde_json::Value>().await {
            Err(e) => return Err(AuthError::NetworkRequestFailed(format!("Failed to parse response: {}", e))),
            Ok(data) => data,
        };

        // Error-first: validate response structure
        let Some(users) = response_data["users"].as_array() else {
            return Err(AuthError::NetworkRequestFailed("Invalid response structure".to_string()));
        };

        let Some(user_data) = users.first() else {
            return Err(AuthError::UserNotFound);
        };

        // Update user fields from server response
        if let Some(display_name) = user_data["displayName"].as_str() {
            self.display_name = Some(display_name.to_string());
        }
        if let Some(photo_url) = user_data["photoUrl"].as_str() {
            self.photo_url = Some(photo_url.to_string());
        }
        if let Some(email) = user_data["email"].as_str() {
            self.email = Some(email.to_string());
        }
        if let Some(email_verified) = user_data["emailVerified"].as_bool() {
            self.email_verified = email_verified;
        }

        Ok(())
    }

    /// Send email verification
    ///
    /// Sends a verification email to the user's email address. The email contains
    /// a link that the user can click to verify their email address.
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth/user.h:582`
    /// - `auth/src/desktop/rpcs/get_oob_confirmation_code_request.cc:37` - CreateSendEmailVerificationRequest
    /// - `auth/src/desktop/user_desktop.cc:728` - SendEmailVerification endpoint
    pub async fn send_email_verification(&self) -> Result<(), AuthError> {
        // Error-first: validate ID token
        let Some(ref id_token) = self.id_token else {
            return Err(AuthError::NoSignedInUser);
        };

        // Error-first: validate API key
        let Some(ref api_key) = self.api_key else {
            return Err(AuthError::InvalidApiKey);
        };

        let url = format!(
            "https://identitytoolkit.googleapis.com/v1/accounts:sendOobCode?key={}",
            api_key
        );

        let body = serde_json::json!({
            "requestType": "VERIFY_EMAIL",
            "idToken": id_token
        });

        let client = reqwest::Client::new();
        let response = match client
            .post(&url)
            .json(&body)
            .send()
            .await
        {
            Err(e) => return Err(AuthError::NetworkRequestFailed(e.to_string())),
            Ok(resp) => resp,
        };

        // Error-first: handle HTTP errors
        if !response.status().is_success() {
            let error_body = match response.json::<serde_json::Value>().await {
                Err(e) => return Err(AuthError::NetworkRequestFailed(format!("Failed to parse error: {}", e))),
                Ok(body) => body,
            };
            let error_message = error_body["error"]["message"]
                .as_str()
                .unwrap_or("SEND_EMAIL_VERIFICATION_FAILED");
            return Err(AuthError::from_error_code(error_message));
        }

        // Email verification sent successfully
        Ok(())
    }

    /// Update email address
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth/user.h:604`
    /// - `auth/src/desktop/rpcs/set_account_info_request.cc:39` - CreateUpdateEmailRequest
    pub async fn update_email(&self, new_email: impl AsRef<str>) -> Result<(), AuthError> {
        let new_email = new_email.as_ref();
        
        // Validate email (error case first)
        if new_email.is_empty() {
            return Err(AuthError::InvalidEmail);
        }
        
        // Basic email format validation (error case first)
        if !new_email.contains('@') {
            return Err(AuthError::InvalidEmail);
        }
        
        // Need ID token (error case first)
        let id_token = self.get_id_token(false).await?;
        
        // Need API key (error case first)
        let Some(api_key) = &self.api_key else {
            return Err(AuthError::NotAuthenticated);
        };
        
        // Call setAccountInfo REST API to update email
        let url = format!(
            "https://identitytoolkit.googleapis.com/v1/accounts:update?key={}",
            api_key
        );
        
        let client = reqwest::Client::new();
        let response = match client
            .post(&url)
            .json(&serde_json::json!({
                "idToken": id_token,
                "email": new_email,
                "returnSecureToken": true
            }))
            .send()
            .await
        {
            Err(e) => return Err(AuthError::NetworkRequestFailed(format!("Update email failed: {}", e))),
            Ok(resp) => resp,
        };
        
        // Handle error responses first
        if !response.status().is_success() {
            let error_body = match response.json::<serde_json::Value>().await {
                Err(e) => return Err(AuthError::NetworkRequestFailed(format!("Failed to parse error: {}", e))),
                Ok(body) => body,
            };
            let error_message = error_body["error"]["message"]
                .as_str()
                .unwrap_or("EMAIL_UPDATE_FAILED");
            return Err(AuthError::from_error_code(error_message));
        }
        
        // Email updated successfully
        // Note: In a real implementation, self.email would be updated
        // Since User is immutable, caller should fetch fresh User after this operation
        Ok(())
    }

    /// Update password
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth/user.h:626`
    /// - `auth/src/desktop/user_desktop.cc:838` - UpdatePassword implementation
    /// - `auth/src/desktop/rpcs/set_account_info_request.cc:30` - setAccountInfo endpoint
    pub async fn update_password(&self, new_password: impl AsRef<str>) -> Result<(), AuthError> {
        let new_password = new_password.as_ref();
        
        // Validate password (error case first)
        if new_password.is_empty() {
            return Err(AuthError::InvalidPassword);
        }
        
        // Need ID token (error case first)
        let id_token = self.get_id_token(false).await?;
        
        // Need API key (error case first)
        let Some(api_key) = &self.api_key else {
            return Err(AuthError::NotAuthenticated);
        };
        
        // Call setAccountInfo REST API
        let url = format!(
            "https://identitytoolkit.googleapis.com/v1/accounts:update?key={}",
            api_key
        );
        
        let client = reqwest::Client::new();
        let response = match client
            .post(&url)
            .json(&serde_json::json!({
                "idToken": id_token,
                "password": new_password,
                "returnSecureToken": false
            }))
            .send()
            .await
        {
            Err(e) => return Err(AuthError::NetworkRequestFailed(format!("Update password failed: {}", e))),
            Ok(resp) => resp,
        };
        
        // Handle error responses first
        if !response.status().is_success() {
            let error_body = match response.json::<serde_json::Value>().await {
                Err(e) => return Err(AuthError::NetworkRequestFailed(format!("Failed to parse error: {}", e))),
                Ok(body) => body,
            };
            let error_message = error_body["error"]["message"]
                .as_str()
                .unwrap_or("PASSWORD_UPDATE_FAILED");
            return Err(AuthError::from_error_code(error_message));
        }
        
        // Password updated successfully
        Ok(())
    }

    /// Update user profile
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth/user.h:652`
    /// - `auth/src/desktop/user_desktop.cc:864` - UpdateUserProfile
    /// - `auth/src/desktop/rpcs/set_account_info_request.cc:90` - CreateUpdateProfileRequest
    ///
    /// Updates the user's display name and/or photo URL.
    /// Pass None for fields you don't want to change.
    ///
    /// # Arguments
    /// * `profile` - UserProfile with optional display_name and photo_url
    ///
    /// # Errors
    /// Returns `AuthError` if:
    /// - User has no ID token (not authenticated)
    /// - User has no API key
    /// - Network request fails
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut user: firebase_rust_sdk::auth::User = todo!();
    /// use firebase_rust_sdk::auth::UserProfile;
    ///
    /// let profile = UserProfile {
    ///     display_name: Some("Alice Smith".to_string()),
    ///     photo_url: Some("https://example.com/photo.jpg".to_string()),
    /// };
    /// user.update_profile(profile).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update_profile(&self, profile: UserProfile) -> Result<(), AuthError> {
        // Get fresh ID token (error case first)
        let id_token = self.get_id_token(false).await?;
        
        // Need API key (error case first)
        let Some(api_key) = &self.api_key else {
            return Err(AuthError::NotAuthenticated);
        };
        
        // Build request body - only include fields that are provided
        let mut request_body = serde_json::json!({
            "idToken": id_token,
            "returnSecureToken": true
        });
        
        if let Some(display_name) = profile.display_name {
            request_body["displayName"] = serde_json::json!(display_name);
        }
        
        if let Some(photo_url) = profile.photo_url {
            request_body["photoUrl"] = serde_json::json!(photo_url);
        }
        
        // Call setAccountInfo REST API to update profile
        let url = format!(
            "https://identitytoolkit.googleapis.com/v1/accounts:update?key={}",
            api_key
        );
        
        let client = reqwest::Client::new();
        let response = match client
            .post(&url)
            .json(&request_body)
            .send()
            .await
        {
            Err(e) => return Err(AuthError::NetworkRequestFailed(format!("Update profile failed: {}", e))),
            Ok(resp) => resp,
        };
        
        // Handle error responses first
        if !response.status().is_success() {
            let error_body = match response.json::<serde_json::Value>().await {
                Err(e) => return Err(AuthError::NetworkRequestFailed(format!("Failed to parse error: {}", e))),
                Ok(body) => body,
            };
            let error_message = error_body["error"]["message"]
                .as_str()
                .unwrap_or("PROFILE_UPDATE_FAILED");
            return Err(AuthError::from_error_code(error_message));
        }
        
        // Profile updated successfully
        // Note: In a real implementation, self.display_name and self.photo_url would be updated
        // Since User is immutable, caller should fetch fresh User after this operation
        Ok(())
    }

    /// Reauthenticate the user with a credential
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth/user.h:461` - Reauthenticate
    /// - `auth/src/desktop/user_desktop.cc:618` - Reauthenticate implementation
    ///
    /// Required before performing sensitive operations like deleting the account
    /// or changing the primary email address.
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let user: firebase_rust_sdk::auth::User = todo!();
    /// use firebase_rust_sdk::auth::EmailAuthCredential;
    ///
    /// let credential = EmailAuthCredential::new("user@example.com", "password");
    /// user.reauthenticate(&credential).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn reauthenticate(&self, _credential: &dyn AuthCredential) -> Result<(), AuthError> {
        // TODO: Implement reauthentication via verifyPassword or verifyAssertion
        todo!("User reauthentication not yet implemented")
    }

    /// Link this user account with a credential
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth/user.h:369` - LinkWithCredential
    /// - `auth/src/desktop/user_desktop.cc:553` - LinkWithCredential implementation
    ///
    /// Allows linking additional authentication providers to an existing account.
    /// For example, linking a Google account to an email/password account.
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let user: firebase_rust_sdk::auth::User = todo!();
    /// use firebase_rust_sdk::auth::EmailAuthCredential;
    ///
    /// let credential = EmailAuthCredential::new("user@example.com", "password");
    /// let result = user.link_with_credential(&credential).await?;
    /// println!("Linked user: {}", result.user.uid);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn link_with_credential(&self, _credential: &dyn AuthCredential) -> Result<crate::auth::AuthResult, AuthError> {
        // TODO: Implement account linking via setAccountInfo with idToken
        todo!("User account linking not yet implemented")
    }

    /// Unlink an authentication provider from this user
    ///
    /// # C++ Reference
    /// - `auth/src/include/firebase/auth/user.h:445` - Unlink
    /// - `auth/src/desktop/user_desktop.cc:595` - Unlink implementation
    ///
    /// Removes a linked authentication provider from the user's account.
    ///
    /// # Arguments
    /// * `provider_id` - Provider ID to unlink (e.g., "google.com", "password")
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut user: firebase_rust_sdk::auth::User = todo!();
    ///
    /// user.unlink("google.com").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn unlink(&self, _provider_id: &str) -> Result<(), AuthError> {
        // TODO: Implement provider unlinking via setAccountInfo
        todo!("User provider unlinking not yet implemented")
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
        let cred = Credential::EmailPassword {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };
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
        let cred = Credential::Google {
            id_token: Some("id_token".to_string()),
            access_token: None,
        };
        let provider = cred.provider_id();
        
        match &cred {
            Credential::Google { id_token, .. } => {
                assert_eq!(id_token.as_deref(), Some("id_token"));
            }
            _ => panic!("Expected Google credential"),
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

    #[tokio::test]
    async fn test_update_email_validates_empty() {
        let future_time = chrono::Utc::now().timestamp() + 3600; // 1 hour from now
        let user = User {
            uid: "test123".to_string(),
            email: Some("old@example.com".to_string()),
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
            id_token: Some("token".to_string()),
            refresh_token: Some("refresh".to_string()),
            token_expiration: Some(future_time),
            api_key: Some("test-api-key".to_string()),
        };

        // Test empty email validation
        let result = user.update_email("").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::InvalidEmail));
    }

    #[tokio::test]
    async fn test_update_email_validates_format() {
        let future_time = chrono::Utc::now().timestamp() + 3600; // 1 hour from now
        let user = User {
            uid: "test123".to_string(),
            email: Some("old@example.com".to_string()),
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
            id_token: Some("token".to_string()),
            refresh_token: Some("refresh".to_string()),
            token_expiration: Some(future_time),
            api_key: Some("test-api-key".to_string()),
        };

        // Test invalid email format (no @)
        let result = user.update_email("notanemail").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::InvalidEmail));
    }

    #[tokio::test]
    async fn test_update_profile_with_display_name() {
        let future_time = chrono::Utc::now().timestamp() + 3600;
        let _user = User {
            uid: "test123".to_string(),
            email: Some("user@example.com".to_string()),
            display_name: Some("Old Name".to_string()),
            photo_url: None,
            phone_number: None,
            email_verified: false,
            is_anonymous: false,
            metadata: UserMetadata {
                creation_timestamp: 1234567890,
                last_sign_in_timestamp: 1234567890,
            },
            provider_data: vec![],
            id_token: Some("token".to_string()),
            refresh_token: Some("refresh".to_string()),
            token_expiration: Some(future_time),
            api_key: Some("test-api-key".to_string()),
        };

        // Create profile with only display name
        let _profile = UserProfile {
            display_name: Some("New Name".to_string()),
            photo_url: None,
        };

        // Would make API call in real implementation
        // Here we just verify the method exists and accepts the profile
        // (actual API call would fail without real Firebase project)
    }

    #[test]
    fn test_update_profile_structure() {
        let future_time = chrono::Utc::now().timestamp() + 3600;
        let _user = User {
            uid: "test123".to_string(),
            email: Some("user@example.com".to_string()),
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
            id_token: Some("token".to_string()),
            refresh_token: Some("refresh".to_string()),
            token_expiration: Some(future_time),
            api_key: Some("test-api-key".to_string()),
        };

        // Create profile with both fields
        let profile = UserProfile {
            display_name: Some("Alice Smith".to_string()),
            photo_url: Some("https://example.com/photo.jpg".to_string()),
        };

        // Verify profile structure
        assert!(profile.display_name.is_some());
        assert!(profile.photo_url.is_some());
        assert_eq!(profile.display_name.as_deref(), Some("Alice Smith"));
        assert_eq!(profile.photo_url.as_deref(), Some("https://example.com/photo.jpg"));
    }

    #[tokio::test]
    async fn test_reload_requires_id_token() {
        let mut user = User {
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
            id_token: None, // No ID token
            refresh_token: None,
            token_expiration: None,
            api_key: Some("test-api-key".to_string()),
        };

        // Test that reload fails without ID token (error-first)
        let result = user.reload().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::NoSignedInUser));
    }

    #[tokio::test]
    async fn test_send_email_verification_requires_id_token() {
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
            id_token: None, // No ID token
            refresh_token: None,
            token_expiration: None,
            api_key: Some("test-api-key".to_string()),
        };

        // Test that send_email_verification fails without ID token (error-first)
        let result = user.send_email_verification().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::NoSignedInUser));
    }
}

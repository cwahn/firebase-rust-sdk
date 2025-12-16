//! Firebase error types
//!
//! Provides a unified error type hierarchy for all Firebase operations.
//!
//! # C++ Reference
//! - `app/src/include/firebase/app.h:122` (Error enum)
//! - `auth/src/include/firebase/auth/types.h:28` (AuthError enum)
//!
//! # Design
//! Uses thiserror for ergonomic error definitions. All errors implement
//! std::error::Error and can be converted to FirebaseError via From trait.


use thiserror::Error;

/// Top-level Firebase error type
///
/// Wraps specific error types (Auth, Firestore, etc.) into a unified type.
/// Supports conversion from all module-specific errors via `From` trait.
///
/// # Example
/// ```
/// use firebase_rust_sdk::{FirebaseError, AuthError};
///
/// let auth_err: FirebaseError = AuthError::InvalidEmail.into();
/// ```
#[derive(Debug, Error)]
pub enum FirebaseError {
    /// Authentication-related errors
    #[error("Auth error: {0}")]
    Auth(#[from] AuthError),

    /// Firestore-related errors
    #[error("Firestore error: {0}")]
    Firestore(#[from] FirestoreError),

    /// Network/HTTP errors
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// JSON serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Generic internal errors
    #[error("Internal error: {0}")]
    Internal(String),

    /// API key not configured
    #[error("API key not configured")]
    ApiKeyNotConfigured,

    /// Invalid API key format
    #[error("Invalid API key: {0}")]
    InvalidApiKey(String),

    /// Operation cancelled
    #[error("Operation cancelled")]
    Cancelled,

    /// Unknown error
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Authentication errors
///
/// # C++ Reference
/// - `auth/src/include/firebase/auth/types.h:28-66`
///
/// Maps Firebase Auth error codes to Rust enum variants.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AuthError {
    /// Email address is invalid
    #[error("Invalid email address")]
    InvalidEmail,

    /// Password is invalid
    #[error("Invalid password")]
    InvalidPassword,

    /// Email already in use by another account
    #[error("Email already in use")]
    EmailAlreadyInUse,

    /// User not found
    #[error("User not found")]
    UserNotFound,

    /// Wrong password
    #[error("Wrong password")]
    WrongPassword,

    /// User account has been disabled
    #[error("User account disabled")]
    UserDisabled,

    /// Too many failed login attempts
    #[error("Too many requests, try again later")]
    TooManyRequests,

    /// Operation not allowed (e.g., provider disabled)
    #[error("Operation not allowed")]
    OperationNotAllowed,

    /// Invalid credential
    #[error("Invalid credential: {0}")]
    InvalidCredential(String),

    /// User token has expired
    #[error("User token expired")]
    UserTokenExpired,

    /// Invalid user token
    #[error("Invalid user token")]
    InvalidUserToken,

    /// Network error
    #[error("Network error: {0}")]
    NetworkRequestFailed(String),

    /// Not authenticated
    #[error("Not authenticated")]
    NotAuthenticated,

    /// No signed-in user
    #[error("No user is currently signed in")]
    NoSignedInUser,

    /// Requires recent login
    #[error("This operation requires recent authentication")]
    RequiresRecentLogin,

    /// Invalid API key
    #[error("Invalid API key")]
    InvalidApiKey,

    /// Account exists with different credential
    #[error("Account exists with different credential")]
    AccountExistsWithDifferentCredential,

    /// Invalid action code
    #[error("Invalid action code")]
    InvalidActionCode,

    /// Action code expired
    #[error("Action code expired")]
    ExpiredActionCode,

    /// Unknown error with code
    #[error("Unknown auth error: code {0}")]
    Unknown(i32),
}

/// Firestore errors
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/firestore_errors.h:32-68`
///
/// Maps Firestore error codes to Rust enum variants.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum FirestoreError {
    /// Document not found
    #[error("Document not found")]
    NotFound,

    /// Permission denied
    #[error("Permission denied")]
    PermissionDenied,

    /// Resource already exists
    #[error("Resource already exists")]
    AlreadyExists,

    /// Resource exhausted (e.g., quota exceeded)
    #[error("Resource exhausted")]
    ResourceExhausted,

    /// Invalid argument
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// Invalid data
    #[error("Invalid data: {0}")]
    InvalidData(String),

    /// Deadline exceeded
    #[error("Deadline exceeded")]
    DeadlineExceeded,

    /// Operation was aborted
    #[error("Operation aborted")]
    Aborted,

    /// Out of range error
    #[error("Out of range: {0}")]
    OutOfRange(String),

    /// Unimplemented feature
    #[error("Feature not implemented")]
    Unimplemented,

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Service unavailable
    #[error("Service unavailable")]
    Unavailable,

    /// Data loss or corruption
    #[error("Data loss")]
    DataLoss,

    /// Unauthenticated
    #[error("Unauthenticated")]
    Unauthenticated,

    /// Connection or network error
    #[error("Connection error: {0}")]
    Connection(String),

    /// Unknown error with code
    #[error("Unknown Firestore error: code {0}")]
    Unknown(i32),
}

impl FirebaseError {
    /// Create an internal error from a string
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    /// Create an unknown error from a string
    pub fn unknown(msg: impl Into<String>) -> Self {
        Self::Unknown(msg.into())
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Network(_) 
            | Self::Auth(AuthError::NetworkRequestFailed(_))
            | Self::Auth(AuthError::TooManyRequests)
            | Self::Firestore(FirestoreError::Unavailable)
            | Self::Firestore(FirestoreError::DeadlineExceeded)
            | Self::Firestore(FirestoreError::ResourceExhausted) => true,
            _ => false,
        }
    }

    /// Check if error indicates authentication is required
    pub fn requires_auth(&self) -> bool {
        matches!(
            self,
            Self::Auth(AuthError::NoSignedInUser)
                | Self::Auth(AuthError::RequiresRecentLogin)
                | Self::Auth(AuthError::UserTokenExpired)
                | Self::Auth(AuthError::InvalidUserToken)
                | Self::Firestore(FirestoreError::Unauthenticated)
        )
    }
}

// Helper to convert from C++ error codes (for future FFI or HTTP responses)
impl AuthError {
    /// Create from Firebase Auth REST API error code
    pub fn from_error_code(code: &str) -> Self {
        match code {
            "EMAIL_NOT_FOUND" => Self::UserNotFound,
            "INVALID_PASSWORD" => Self::WrongPassword,
            "USER_DISABLED" => Self::UserDisabled,
            "TOO_MANY_ATTEMPTS_TRY_LATER" => Self::TooManyRequests,
            "EMAIL_EXISTS" => Self::EmailAlreadyInUse,
            "OPERATION_NOT_ALLOWED" => Self::OperationNotAllowed,
            "INVALID_EMAIL" => Self::InvalidEmail,
            "WEAK_PASSWORD" => Self::InvalidPassword,
            "INVALID_ID_TOKEN" => Self::InvalidUserToken,
            "TOKEN_EXPIRED" => Self::UserTokenExpired,
            "INVALID_API_KEY" => Self::InvalidApiKey,
            "CREDENTIAL_TOO_OLD_LOGIN_AGAIN" => Self::RequiresRecentLogin,
            _ => Self::Unknown(0),
        }
    }
}

impl FirestoreError {
    /// Create from gRPC error code
    pub fn from_grpc_code(code: i32) -> Self {
        match code {
            1 => Self::Aborted,
            2 => Self::Unknown(code),
            3 => Self::InvalidArgument(String::new()),
            4 => Self::DeadlineExceeded,
            5 => Self::NotFound,
            6 => Self::AlreadyExists,
            7 => Self::PermissionDenied,
            8 => Self::ResourceExhausted,
            9 => Self::Aborted,
            10 => Self::OutOfRange(String::new()),
            11 => Self::Unimplemented,
            12 => Self::Internal(String::new()),
            13 => Self::Unavailable,
            14 => Self::DataLoss,
            15 => Self::Unauthenticated,
            _ => Self::Unknown(code),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_error_into_firebase_error() {
        let auth_err = AuthError::InvalidEmail;
        let firebase_err: FirebaseError = auth_err.into();
        
        assert!(matches!(firebase_err, FirebaseError::Auth(AuthError::InvalidEmail)));
    }

    #[test]
    fn test_firestore_error_into_firebase_error() {
        let fs_err = FirestoreError::NotFound;
        let firebase_err: FirebaseError = fs_err.into();
        
        assert!(matches!(firebase_err, FirebaseError::Firestore(FirestoreError::NotFound)));
    }

    #[test]
    fn test_is_retryable() {
        assert!(FirebaseError::Auth(AuthError::NetworkRequestFailed("test".to_string())).is_retryable());
        assert!(FirebaseError::Auth(AuthError::TooManyRequests).is_retryable());
        assert!(!FirebaseError::Auth(AuthError::InvalidEmail).is_retryable());
        
        assert!(FirebaseError::Firestore(FirestoreError::Unavailable).is_retryable());
        assert!(!FirebaseError::Firestore(FirestoreError::NotFound).is_retryable());
    }

    #[test]
    fn test_requires_auth() {
        assert!(FirebaseError::Auth(AuthError::NoSignedInUser).requires_auth());
        assert!(FirebaseError::Auth(AuthError::RequiresRecentLogin).requires_auth());
        assert!(FirebaseError::Firestore(FirestoreError::Unauthenticated).requires_auth());
        assert!(!FirebaseError::Auth(AuthError::InvalidEmail).requires_auth());
    }

    #[test]
    fn test_auth_error_from_code() {
        assert_eq!(AuthError::from_error_code("EMAIL_NOT_FOUND"), AuthError::UserNotFound);
        assert_eq!(AuthError::from_error_code("INVALID_EMAIL"), AuthError::InvalidEmail);
        assert_eq!(AuthError::from_error_code("WEAK_PASSWORD"), AuthError::InvalidPassword);
    }

    #[test]
    fn test_firestore_error_from_grpc() {
        assert_eq!(FirestoreError::from_grpc_code(5), FirestoreError::NotFound);
        assert_eq!(FirestoreError::from_grpc_code(7), FirestoreError::PermissionDenied);
        assert_eq!(FirestoreError::from_grpc_code(13), FirestoreError::Unavailable);
    }

    #[test]
    fn test_error_display() {
        let err = FirebaseError::Auth(AuthError::InvalidEmail);
        let display = format!("{}", err);
        assert!(display.contains("Auth error"));
        assert!(display.contains("Invalid email"));
    }

    #[test]
    fn test_auth_error_equality() {
        assert_eq!(AuthError::InvalidEmail, AuthError::InvalidEmail);
        assert_ne!(AuthError::InvalidEmail, AuthError::WrongPassword);
    }
}

//! Firebase Rust SDK
//!
//! Idiomatic Rust port of Firebase C++ SDK (Auth + Firestore modules)
//!
//! # Example (Email/Password Auth)
//! ```no_run
//! use firebase_rust_sdk::Auth;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let auth = Auth::get_auth("YOUR_API_KEY").await?;
//! let result = auth.sign_in_with_email_and_password("user@example.com", "password").await?;
//! println!("Signed in: {}", result.user.uid);
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

// Implementation status tracker
#[cfg(feature = "tracker")]
pub mod implementation_tracker;

// Core modules
pub mod error;
pub mod app;

// Re-export App for convenience
pub use app::{App, AppOptions};

// Auth module
pub mod auth {
    //! Firebase Authentication
    
    pub mod auth;
    pub mod types;
    // pub mod providers;
    
    pub use auth::Auth;
    pub(crate) use auth::AuthInner;
    pub use types::{User, UserInfo, Credential, AuthResult, AdditionalUserInfo, UserProfile};
}

// Firestore module  
pub mod firestore;

// Re-exports for convenience
pub use error::{FirebaseError, AuthError, FirestoreError};

// Auth re-exports
pub use auth::{Auth, types::{User, Credential, AuthResult}};

// Firestore re-exports
pub use firestore::{
    Firestore, 
    DocumentReference, CollectionReference, DocumentSnapshot,
    Value, MapValue, Timestamp, GeoPoint,
    WriteBatch, Transaction
};

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_types_exist() {
        // Basic smoke test
        let _err: FirebaseError = AuthError::InvalidEmail.into();
    }
}

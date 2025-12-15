//! Firebase Rust SDK
//!
//! Idiomatic Rust port of Firebase C++ SDK (Auth + Firestore modules)
//!
//! # Example (Email/Password Auth)
//! ```no_run
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use firebase_rust_sdk::Auth;
//!
//! let auth = Auth::get_auth("YOUR_API_KEY").await?;
//! let user = auth.sign_in_with_email_and_password("user@example.com", "password").await?;
//! println!("Signed in: {}", user.uid);
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

// Implementation status tracker
#[cfg(feature = "tracker")]
pub mod implementation_tracker;

// Core modules (to be implemented)
pub mod error;

// Auth module
pub mod auth {
    //! Firebase Authentication
    
    pub mod auth;
    pub mod types;
    // pub mod providers;
    
    pub use auth::Auth;
}

// Firestore module  
pub mod firestore {
    //! Cloud Firestore
    
    pub mod types;
    // pub mod document;
    // pub mod collection;
    // pub mod query;
}

// Re-exports for convenience
pub use error::{FirebaseError, AuthError, FirestoreError};

// Auth re-exports
pub use auth::{Auth, types::{User, Credential, AuthResult}};
// Auth re-exports (will be added as implemented)
// pub use auth::{Auth, User, Credential, AuthResult};

// Firestore re-exports (will be added as implemented)
// pub use firestore::{Firestore, DocumentReference, FieldValue};

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_types_exist() {
        // Basic smoke test
        let _err: FirebaseError = AuthError::InvalidEmail.into();
    }
}

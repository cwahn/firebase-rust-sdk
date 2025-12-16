//! Integration tests for Firebase Authentication
//!
//! These tests interact with real Firebase services and require:
//! 1. A Firebase project with Authentication enabled
//! 2. Environment variables set in .env file
//! 3. Run with: cargo test --features integration-tests -- --test-threads=1
//!
//! See INTEGRATION_TESTS.md for setup instructions.

#![cfg(feature = "integration-tests")]

use firebase_rust_sdk::{Auth, auth::{User, Credential}};
use std::env;

/// Load environment variables from .env file
fn load_env() {
    dotenvy::dotenv().ok();
}

/// Get test credentials from environment
fn get_test_config() -> (String, String, String, String) {
    load_env();
    
    let api_key = env::var("FIREBASE_API_KEY")
        .expect("FIREBASE_API_KEY must be set in .env file");
    let project_id = env::var("FIREBASE_PROJECT_ID")
        .expect("FIREBASE_PROJECT_ID must be set in .env file");
    let email = env::var("TEST_USER_EMAIL")
        .expect("TEST_USER_EMAIL must be set in .env file");
    let password = env::var("TEST_USER_PASSWORD")
        .expect("TEST_USER_PASSWORD must be set in .env file");
    
    (api_key, project_id, email, password)
}

/// Test: Sign in with email and password
#[tokio::test]
async fn test_sign_in_with_email_password() {
    let (api_key, _, email, password) = get_test_config();
    
    let auth = Auth::get_auth(&api_key).await
        .expect("Failed to get Auth instance");
    
    // Sign in
    let result = auth.sign_in_with_email_and_password(&email, &password).await
        .expect("Failed to sign in");
    
    // Verify user data
    assert!(!result.user.uid.is_empty());
    assert_eq!(result.user.email.as_deref(), Some(email.as_str()));
    assert!(result.user.id_token.is_some());
    
    // Sign out
    auth.sign_out().await.expect("Failed to sign out");
    assert!(auth.current_user().await.is_none());
    
    println!("✅ Email/password sign in works!");
}

/// Test: Anonymous authentication
#[tokio::test]
async fn test_anonymous_auth() {
    let (api_key, _, _, _) = get_test_config();
    
    let auth = Auth::get_auth(&api_key).await
        .expect("Failed to get Auth instance");
    
    // Sign in anonymously
    let result = auth.sign_in_anonymously().await
        .expect("Failed to sign in anonymously");
    
    // Verify it's anonymous
    assert!(result.user.is_anonymous);
    assert!(!result.user.uid.is_empty());
    assert!(result.user.id_token.is_some());
    
    // Clean up: delete the anonymous user
    let uid = result.user.uid.clone();
    result.user.delete().await.expect("Failed to delete user");
    auth.sign_out().await.expect("Failed to sign out");
    
    println!("✅ Anonymous auth works! (cleaned up user {})", uid);
}

/// Test: Create user, sign in, delete
#[tokio::test]
async fn test_create_and_delete_user() {
    let (api_key, _, _, _) = get_test_config();
    
    let auth = Auth::get_auth(&api_key).await
        .expect("Failed to get Auth instance");
    
    // Generate unique email for this test
    let timestamp = chrono::Utc::now().timestamp();
    let test_email = format!("test+{}@example.com", timestamp);
    let test_password = "TempPassword123!";
    
    // Create new user
    let result = auth.create_user_with_email_and_password(&test_email, test_password).await
        .expect("Failed to create user");
    
    assert_eq!(result.user.email.as_deref(), Some(test_email.as_str()));
    assert!(!result.user.uid.is_empty());
    
    let uid = result.user.uid.clone();
    
    // Sign out
    auth.sign_out().await.expect("Failed to sign out");
    
    // Sign in with the new user
    let result2 = auth.sign_in_with_email_and_password(&test_email, test_password).await
        .expect("Failed to sign in with new user");
    
    assert_eq!(result2.user.uid, uid);
    
    // Delete the user
    result2.user.delete().await.expect("Failed to delete user");
    auth.sign_out().await.expect("Failed to sign out");
    
    println!("✅ Create/delete user works! (cleaned up user {})", uid);
}

/// Test: Token refresh
#[tokio::test]
async fn test_token_refresh() {
    let (api_key, _, email, password) = get_test_config();
    
    let auth = Auth::get_auth(&api_key).await
        .expect("Failed to get Auth instance");
    
    // Sign in
    let result = auth.sign_in_with_email_and_password(&email, &password).await
        .expect("Failed to sign in");
    
    let old_token = result.user.id_token.clone().unwrap();
    
    // Refresh token (force refresh)
    let new_token = result.user.get_id_token(true).await
        .expect("Failed to refresh token");
    
    // Tokens should be different
    assert_ne!(old_token, new_token);
    assert!(!new_token.is_empty());
    
    // Sign out
    auth.sign_out().await.expect("Failed to sign out");
    
    println!("✅ Token refresh works!");
}

/// Test: Update user profile
#[tokio::test]
async fn test_update_profile() {
    let (api_key, _, email, password) = get_test_config();
    
    let auth = Auth::get_auth(&api_key).await
        .expect("Failed to get Auth instance");
    
    // Sign in
    let result = auth.sign_in_with_email_and_password(&email, &password).await
        .expect("Failed to sign in");
    
    let mut user = result.user;
    
    // Update display name
    let timestamp = chrono::Utc::now().timestamp();
    let new_name = format!("Test User {}", timestamp);
    
    user.update_display_name(&new_name).await
        .expect("Failed to update display name");
    
    // Reload to verify
    user.reload().await.expect("Failed to reload user");
    assert_eq!(user.display_name.as_deref(), Some(new_name.as_str()));
    
    // Sign out
    auth.sign_out().await.expect("Failed to sign out");
    
    println!("✅ Update profile works!");
}

/// Test: Send password reset email
#[tokio::test]
async fn test_password_reset() {
    let (api_key, _, email, _) = get_test_config();
    
    let auth = Auth::get_auth(&api_key).await
        .expect("Failed to get Auth instance");
    
    // Send password reset email
    auth.send_password_reset_email(&email).await
        .expect("Failed to send password reset email");
    
    println!("✅ Password reset email sent! (check your inbox)");
}

/// Test: User reload
#[tokio::test]
async fn test_user_reload() {
    let (api_key, _, email, password) = get_test_config();
    
    let auth = Auth::get_auth(&api_key).await
        .expect("Failed to get Auth instance");
    
    // Sign in
    let result = auth.sign_in_with_email_and_password(&email, &password).await
        .expect("Failed to sign in");
    
    let mut user = result.user;
    let old_email = user.email.clone();
    
    // Reload user data
    user.reload().await.expect("Failed to reload user");
    
    // Email should be the same
    assert_eq!(user.email, old_email);
    
    // Sign out
    auth.sign_out().await.expect("Failed to sign out");
    
    println!("✅ User reload works!");
}

/// Test: Send email verification
#[tokio::test]
async fn test_send_email_verification() {
    let (api_key, _, email, password) = get_test_config();
    
    let auth = Auth::get_auth(&api_key).await
        .expect("Failed to get Auth instance");
    
    // Sign in
    let result = auth.sign_in_with_email_and_password(&email, &password).await
        .expect("Failed to sign in");
    
    // Send verification email
    result.user.send_email_verification().await
        .expect("Failed to send email verification");
    
    // Sign out
    auth.sign_out().await.expect("Failed to sign out");
    
    println!("✅ Email verification sent! (check your inbox)");
}

/// Test: Update password
#[tokio::test]
async fn test_update_password() {
    let (api_key, _, _, _) = get_test_config();
    
    let auth = Auth::get_auth(&api_key).await
        .expect("Failed to get Auth instance");
    
    // Create temporary user
    let timestamp = chrono::Utc::now().timestamp();
    let test_email = format!("test+pwd{}@example.com", timestamp);
    let old_password = "OldPassword123!";
    let new_password = "NewPassword456!";
    
    // Create user
    let result = auth.create_user_with_email_and_password(&test_email, old_password).await
        .expect("Failed to create user");
    
    let uid = result.user.uid.clone();
    
    // Update password
    result.user.update_password(new_password).await
        .expect("Failed to update password");
    
    // Sign out
    auth.sign_out().await.expect("Failed to sign out");
    
    // Try to sign in with new password
    let result2 = auth.sign_in_with_email_and_password(&test_email, new_password).await
        .expect("Failed to sign in with new password");
    
    assert_eq!(result2.user.uid, uid);
    
    // Clean up
    result2.user.delete().await.expect("Failed to delete user");
    auth.sign_out().await.expect("Failed to sign out");
    
    println!("✅ Update password works! (cleaned up user {})", uid);
}

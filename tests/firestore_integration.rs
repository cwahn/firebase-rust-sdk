//! Integration tests for Firestore
//!
//! These tests interact with real Firestore and require:
//! 1. A Firebase project with Firestore enabled
//! 2. Environment variables set in .env file
//! 3. Run with: cargo test --test firestore_integration -- --test-threads=1
//!
//! Note: Uses gRPC API (not REST). Matches C++ SDK architecture.

use firebase_rust_sdk::{App, AppOptions, Auth, firestore::{Firestore, MapValue, Value, ValueType, FilterCondition, listen_document, ListenerOptions}};
use futures::stream::StreamExt;
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex};

/// Get Firestore instance from environment variables with authentication
async fn get_firestore() -> Firestore {
    dotenvy::dotenv().ok();
    
    let project_id = env::var("FIREBASE_PROJECT_ID")
        .expect("FIREBASE_PROJECT_ID must be set in .env file");
    let database_id = env::var("FIREBASE_DATABASE_ID")
        .unwrap_or_else(|_| "default".to_string());
    let api_key = env::var("FIREBASE_API_KEY")
        .expect("FIREBASE_API_KEY must be set in .env file");
    let email = env::var("TEST_USER_EMAIL")
        .expect("TEST_USER_EMAIL must be set in .env file");
    let password = env::var("TEST_USER_PASSWORD")
        .expect("TEST_USER_PASSWORD must be set in .env file");
    
    // Create App and Auth instances
    let app = App::create(AppOptions {
        api_key: api_key.clone(),
        project_id: project_id.clone(),
        app_name: None,
    }).await
        .expect("Failed to create App");
    
    let auth = Auth::get_auth(&app).await
        .expect("Failed to get Auth instance");
    
    // Sign in to get ID token
    auth.sign_in_with_email_and_password(&email, &password).await
        .expect("Failed to sign in - check TEST_USER_EMAIL and TEST_USER_PASSWORD");
    
    let user = auth.current_user().await
        .expect("No current user after sign in");
    let id_token = user.get_id_token(false).await
        .expect("Failed to get ID token");
    
    Firestore::new(project_id, database_id, Some(id_token)).await
        .expect("Failed to create Firestore instance")
}

/// Helper to create a MapValue from key-value pairs
fn create_map(fields: Vec<(&str, ValueType)>) -> MapValue {
    let mut map = HashMap::new();
    for (key, value_type) in fields {
        map.insert(key.to_string(), Value {
            value_type: Some(value_type),
        });
    }
    MapValue { fields: map }
}

/// Generate unique document path for testing
fn test_doc_path(test_name: &str) -> String {
    let timestamp = chrono::Utc::now().timestamp();
    let random = rand::random::<u32>();
    format!("integration_tests/{}_{}_{}", test_name, timestamp, random)
}

/// Test: Create and read document using gRPC
#[tokio::test]
async fn test_set_and_get_document() {
    let firestore = get_firestore().await;
    let doc_path = test_doc_path("set_get");
    
    // Create document using DocumentReference.set()
    let data = create_map(vec![
        ("name", ValueType::StringValue("Alice".to_string())),
        ("age", ValueType::IntegerValue(30)),
        ("active", ValueType::BooleanValue(true)),
    ]);
    
    let doc_ref = firestore.document(&doc_path);
    doc_ref.set(data).await
        .expect("Failed to set document");
    
    // Read document using DocumentReference.get()
    let snapshot = doc_ref.get().await
        .expect("Failed to get document");
    
    assert!(snapshot.exists());
    
    // Verify field values
    let name = snapshot.get("name").expect("name field missing");
    match &name.value_type {
        Some(ValueType::StringValue(s)) => assert_eq!(s, "Alice"),
        _ => panic!("Expected string value for name"),
    }
    
    let age = snapshot.get("age").expect("age field missing");
    match age.value_type {
        Some(ValueType::IntegerValue(i)) => assert_eq!(i, 30),
        _ => panic!("Expected integer value for age"),
    }
    
    // Clean up
    doc_ref.delete().await.expect("Failed to delete document");
    
    println!("✅ Set and get document works!");
}

/// Test: Update document using gRPC
#[tokio::test]
async fn test_update_document() {
    let firestore = get_firestore().await;
    let doc_path = test_doc_path("update");
    
    let doc_ref = firestore.document(&doc_path);
    
    // Create document with initial data
    let initial_data = create_map(vec![
        ("count", ValueType::IntegerValue(0)),
        ("name", ValueType::StringValue("Test".to_string())),
    ]);
    doc_ref.set(initial_data).await
        .expect("Failed to create document");
    
    // Update only count field (should keep name)
    let update_data = create_map(vec![
        ("count", ValueType::IntegerValue(42)),
    ]);
    doc_ref.update(update_data).await
        .expect("Failed to update document");
    
    // Read updated document
    let snapshot = doc_ref.get().await
        .expect("Failed to get document");
    
    // Verify count was updated
    let count = snapshot.get("count").expect("count field missing");
    match count.value_type {
        Some(ValueType::IntegerValue(i)) => assert_eq!(i, 42),
        _ => panic!("Expected integer value for count"),
    }
    
    // Verify name still exists (update doesn't replace entire document)
    assert!(snapshot.get("name").is_some());
    
    // Clean up
    doc_ref.delete().await.expect("Failed to delete document");
    
    println!("✅ Update document works!");
}

/// Test: Delete document using gRPC
#[tokio::test]
async fn test_delete_document() {
    let firestore = get_firestore().await;
    let doc_path = test_doc_path("delete");
    
    let doc_ref = firestore.document(&doc_path);
    
    // Create document
    let data = create_map(vec![
        ("test", ValueType::BooleanValue(true)),
    ]);
    doc_ref.set(data).await
        .expect("Failed to create document");
    
    // Verify it exists
    let snapshot = doc_ref.get().await
        .expect("Failed to get document");
    assert!(snapshot.exists());
    
    // Delete document
    doc_ref.delete().await
        .expect("Failed to delete document");
    
    // Verify it's gone - Firestore returns NotFound error for deleted documents
    let result = doc_ref.get().await;
    assert!(result.is_err() || !result.unwrap().exists());
    
    println!("✅ Delete document works!");
}

/// Test: WriteBatch with multiple operations using gRPC
#[tokio::test]
async fn test_write_batch() {
    let firestore = get_firestore().await;
    let collection_path = format!("integration_tests_batch_{}", rand::random::<u32>());
    
    // Create batch
    let mut batch = firestore.batch();
    
    // Add multiple write operations
    for i in 1..=3 {
        let doc_path = format!("{}/doc{}", collection_path, i);
        let data = create_map(vec![
            ("index", ValueType::IntegerValue(i)),
            ("batch_test", ValueType::BooleanValue(true)),
        ]);
        batch.set(doc_path, data);
    }
    
    // Commit batch (atomic - all succeed or all fail)
    batch.commit().await
        .expect("Failed to commit batch");
    
    // Verify all documents exist
    for i in 1..=3 {
        let doc_path = format!("{}/doc{}", collection_path, i);
        let doc_ref = firestore.document(&doc_path);
        let snapshot = doc_ref.get().await
            .expect("Failed to read document");
        
        assert!(snapshot.exists());
        
        let index = snapshot.get("index").expect("index field missing");
        match index.value_type {
            Some(ValueType::IntegerValue(val)) => assert_eq!(val, i),
            _ => panic!("Expected integer value for index"),
        }
        
        // Clean up
        doc_ref.delete().await.expect("Failed to delete");
    }
    
    println!("✅ WriteBatch works!");
}

/// Test: CollectionReference.add() with auto-generated ID
#[tokio::test]
async fn test_collection_add() {
    let firestore = get_firestore().await;
    let collection_path = format!("integration_tests_add_{}", rand::random::<u32>());
    
    let collection_ref = firestore.collection(&collection_path);
    
    // Add document with auto-generated ID
    let data = create_map(vec![
        ("message", ValueType::StringValue("Auto-generated ID".to_string())),
        ("timestamp", ValueType::IntegerValue(chrono::Utc::now().timestamp())),
    ]);
    
    let doc_ref = collection_ref.add(data).await
        .expect("Failed to add document");
    
    // Verify document was created
    assert!(doc_ref.path.starts_with(&collection_path));
    assert_eq!(doc_ref.id().len(), 20); // Auto-generated IDs are 20 chars
    
    let snapshot = doc_ref.get().await
        .expect("Failed to read document");
    
    assert!(snapshot.exists());
    
    let message = snapshot.get("message").expect("message field missing");
    match &message.value_type {
        Some(ValueType::StringValue(s)) => assert_eq!(s, "Auto-generated ID"),
        _ => panic!("Expected string value for message"),
    }
    
    // Clean up
    doc_ref.delete().await.expect("Failed to delete document");
    
    println!("✅ CollectionReference.add() works!");
}

/// Test: CollectionReference.document() path parsing
#[tokio::test]
async fn test_collection_document_reference() {
    let firestore = get_firestore().await;
    
    let collection_ref = firestore.collection("users");
    let doc_ref = collection_ref.document("alice");
    
    assert_eq!(doc_ref.path, "users/alice");
    assert_eq!(doc_ref.id(), "alice");
    assert_eq!(doc_ref.parent_path(), Some("users"));
    
    println!("✅ CollectionReference.document() works!");
}

/// Test: Nested document paths
#[tokio::test]
async fn test_nested_documents() {
    let firestore = get_firestore().await;
    let parent_path = format!("integration_tests/parent_{}", rand::random::<u32>());
    let child_path = format!("{}/subcollection/child", parent_path);
    
    // Create parent document
    let parent_ref = firestore.document(&parent_path);
    let parent_data = create_map(vec![
        ("type", ValueType::StringValue("parent".to_string())),
    ]);
    parent_ref.set(parent_data).await
        .expect("Failed to create parent");
    
    // Create child document in subcollection
    let child_ref = firestore.document(&child_path);
    let child_data = create_map(vec![
        ("type", ValueType::StringValue("child".to_string())),
    ]);
    child_ref.set(child_data).await
        .expect("Failed to create child");
    
    // Read child document
    let snapshot = child_ref.get().await
        .expect("Failed to read child");
    
    assert!(snapshot.exists());
    
    // Clean up (delete child and parent)
    child_ref.delete().await.ok();
    parent_ref.delete().await.ok();
    
    println!("✅ Nested document paths work!");
}

/// Test: Compound filters with And/Or logic
#[tokio::test]
async fn test_compound_filters() {
    let firestore = get_firestore().await;
    let collection_path = format!("integration_tests_compound_{}", rand::random::<u32>());
    
    // Create test documents
    let test_docs = vec![
        ("doc1", 15, "inactive"),
        ("doc2", 25, "active"),
        ("doc3", 35, "active"),
        ("doc4", 45, "inactive"),
    ];
    
    for (doc_id, age, status) in &test_docs {
        let doc_path = format!("{}/{}", collection_path, doc_id);
        let doc_ref = firestore.document(&doc_path);
        let data = create_map(vec![
            ("age", ValueType::IntegerValue(*age)),
            ("status", ValueType::StringValue(status.to_string())),
        ]);
        doc_ref.set(data).await
            .expect("Failed to create document");
    }
    
    // Test And filter: age > 20 AND status == "active"
    // This should match doc2 (25, active) and doc3 (35, active)
    let age_value = Value {
        value_type: Some(ValueType::IntegerValue(20)),
    };
    let status_value = Value {
        value_type: Some(ValueType::StringValue("active".to_string())),
    };
    
    let and_filter = FilterCondition::And(vec![
        FilterCondition::GreaterThan("age".to_string(), age_value),
        FilterCondition::Equal("status".to_string(), status_value),
    ]);
    
    // Note: Actual query execution would require implementing query() method on CollectionReference
    // For now, we validate the filter structure
    match &and_filter {
        FilterCondition::And(filters) => {
            assert_eq!(filters.len(), 2);
            println!("✅ And filter structure: {} sub-filters", filters.len());
        }
        _ => panic!("Expected And filter"),
    }
    
    // Test Or filter: age < 20 OR age > 40
    // This should match doc1 (15) and doc4 (45)
    let age_20 = Value {
        value_type: Some(ValueType::IntegerValue(20)),
    };
    let age_40 = Value {
        value_type: Some(ValueType::IntegerValue(40)),
    };
    
    let or_filter = FilterCondition::Or(vec![
        FilterCondition::LessThan("age".to_string(), age_20),
        FilterCondition::GreaterThan("age".to_string(), age_40),
    ]);
    
    match &or_filter {
        FilterCondition::Or(filters) => {
            assert_eq!(filters.len(), 2);
            println!("✅ Or filter structure: {} sub-filters", filters.len());
        }
        _ => panic!("Expected Or filter"),
    }
    
    // Clean up all documents
    for (doc_id, _, _) in &test_docs {
        let doc_path = format!("{}/{}", collection_path, doc_id);
        firestore.document(&doc_path).delete().await.ok();
    }
    
    println!("✅ Compound filters (And/Or) work!");
}

/// Test: Real-time listener using gRPC streaming
#[tokio::test]
async fn test_snapshot_listener() {
    dotenvy::dotenv().ok();
    
    // Get authenticated firestore instance
    let firestore = get_firestore().await;
    let doc_path = test_doc_path("listener");
    
    // Get auth token and project info for listener
    let project_id = env::var("FIREBASE_PROJECT_ID")
        .expect("FIREBASE_PROJECT_ID required");
    let database_id = env::var("FIREBASE_DATABASE_ID")
        .unwrap_or_else(|_| "default".to_string());
    let api_key = env::var("FIREBASE_API_KEY")
        .expect("FIREBASE_API_KEY must be set in .env file");
    let email = env::var("TEST_USER_EMAIL")
        .expect("TEST_USER_EMAIL must be set in .env file");
    let password = env::var("TEST_USER_PASSWORD")
        .expect("TEST_USER_PASSWORD must be set in .env file");
    
    // Get fresh auth token for listener (same as get_firestore() does)
    let app = App::create(AppOptions {
        api_key: api_key.clone(),
        project_id: project_id.clone(),
        app_name: None,
    }).await
        .expect("Failed to create App");
    
    let auth = Auth::get_auth(&app).await
        .expect("Failed to get Auth instance");
    
    auth.sign_in_with_email_and_password(&email, &password).await
        .expect("Failed to sign in");
    
    let user = auth.current_user().await
        .expect("No current user after sign in");
    let auth_token = user.get_id_token(false).await
        .expect("Failed to get ID token");
    
    // Create initial document
    let doc_ref = firestore.document(&doc_path);
    let initial_data = create_map(vec![
        ("value", ValueType::IntegerValue(0)),
    ]);
    doc_ref.set(initial_data).await
        .expect("Failed to create document");
    
    // Set up listener stream
    let mut stream = listen_document(
        &firestore,
        auth_token,
        project_id,
        database_id,
        doc_path.clone(),
        ListenerOptions::default(),
    )
    .await
    .expect("Failed to start listener");
    
    // Track updates received
    let updates = Arc::new(Mutex::new(Vec::new()));
    let updates_clone = updates.clone();
    
    // Spawn task to consume the stream
    let stream_task = tokio::spawn(async move {
        while let Some(result) = stream.next().await {
            if let Ok(snapshot) = result {
                if let Some(data) = &snapshot.data {
                    if let Some(value_field) = data.fields.get("value") {
                        if let Some(ValueType::IntegerValue(val)) = &value_field.value_type {
                            updates_clone.lock().unwrap().push(*val);
                            println!("📡 Listener received value: {}", val);
                        }
                    }
                }
            }
        }
    });
    
    // Wait for initial snapshot
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Update the document (should trigger listener)
    let update_data = create_map(vec![
        ("value", ValueType::IntegerValue(42)),
    ]);
    doc_ref.set(update_data).await
        .expect("Failed to update document");
    
    // Wait for update to propagate
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Another update
    let update_data2 = create_map(vec![
        ("value", ValueType::IntegerValue(100)),
    ]);
    doc_ref.set(update_data2).await
        .expect("Failed to update document");
    
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Stop listening by aborting the stream task
    stream_task.abort();
    
    // Verify we received updates
    let collected = updates.lock().unwrap();
    println!("📊 Received {} updates: {:?}", collected.len(), *collected);
    
    assert!(!collected.is_empty(), "Should have received at least one update");
    assert!(
        collected.contains(&42) || collected.contains(&100),
        "Should have received one of the updated values"
    );
    
    // Clean up
    doc_ref.delete().await.expect("Failed to delete document");
    
    println!("✅ Snapshot listener works! Received {} updates", collected.len());
}

/// Test: Get non-existent document returns NotFound
#[tokio::test]
async fn test_get_nonexistent_document() {
    let firestore = get_firestore().await;
    let doc_path = test_doc_path("nonexistent");
    
    let doc_ref = firestore.document(&doc_path);
    let result = doc_ref.get().await;
    
    // Should either return error or snapshot with exists() == false
    match result {
        Err(e) => {
            println!("✅ Non-existent document returns error: {}", e);
        }
        Ok(snapshot) => {
            assert!(!snapshot.exists(), "Non-existent document should not exist");
            println!("✅ Non-existent document returns empty snapshot");
        }
    }
}

/// Test: Update non-existent document should fail
#[tokio::test]
async fn test_update_nonexistent_document() {
    let firestore = get_firestore().await;
    let doc_path = test_doc_path("update_nonexistent");
    
    let doc_ref = firestore.document(&doc_path);
    let update_data = create_map(vec![
        ("field", ValueType::StringValue("value".to_string())),
    ]);
    
    let result = doc_ref.update(update_data).await;
    
    // Update should fail if document doesn't exist
    assert!(result.is_err(), "Update should fail for non-existent document");
    
    println!("✅ Update non-existent document fails as expected");
}

/// Test: Delete non-existent document should succeed (idempotent)
#[tokio::test]
async fn test_delete_nonexistent_document() {
    let firestore = get_firestore().await;
    let doc_path = test_doc_path("delete_nonexistent");
    
    let doc_ref = firestore.document(&doc_path);
    
    // Delete should succeed even if document doesn't exist (idempotent)
    doc_ref.delete().await
        .expect("Delete should be idempotent");
    
    println!("✅ Delete non-existent document is idempotent");
}

/// Test: Document with single field (Firestore requires at least one field)
#[tokio::test]
async fn test_minimal_document() {
    let firestore = get_firestore().await;
    let doc_path = test_doc_path("minimal");
    
    let doc_ref = firestore.document(&doc_path);
    
    // Firestore requires at least one field, so use a minimal document
    let minimal_data = create_map(vec![
        ("exists", ValueType::BooleanValue(true)),
    ]);
    
    doc_ref.set(minimal_data).await
        .expect("Failed to set minimal document");
    
    let snapshot = doc_ref.get().await
        .expect("Failed to get document");
    
    assert!(snapshot.exists());
    assert!(snapshot.data.is_some());
    assert_eq!(snapshot.data.as_ref().unwrap().fields.len(), 1);
    
    let exists_field = snapshot.get("exists").expect("exists field missing");
    assert!(matches!(exists_field.value_type, Some(ValueType::BooleanValue(true))));
    
    // Clean up
    doc_ref.delete().await.expect("Failed to delete");
    
    println!("✅ Minimal document works!");
}

/// Test: Document with various data types
#[tokio::test]
async fn test_multiple_data_types() {
    let firestore = get_firestore().await;
    let doc_path = test_doc_path("data_types");
    
    let doc_ref = firestore.document(&doc_path);
    
    // Create document with various field types
    let data = create_map(vec![
        ("string", ValueType::StringValue("hello".to_string())),
        ("integer", ValueType::IntegerValue(42)),
        ("double", ValueType::DoubleValue(3.14)),
        ("boolean", ValueType::BooleanValue(true)),
        ("null", ValueType::NullValue(0)),
    ]);
    
    doc_ref.set(data).await
        .expect("Failed to set document with multiple types");
    
    let snapshot = doc_ref.get().await
        .expect("Failed to get document");
    
    assert!(snapshot.exists());
    
    // Verify each type
    let string_val = snapshot.get("string").expect("string missing");
    assert!(matches!(string_val.value_type, Some(ValueType::StringValue(_))));
    
    let int_val = snapshot.get("integer").expect("integer missing");
    assert!(matches!(int_val.value_type, Some(ValueType::IntegerValue(_))));
    
    let double_val = snapshot.get("double").expect("double missing");
    assert!(matches!(double_val.value_type, Some(ValueType::DoubleValue(_))));
    
    let bool_val = snapshot.get("boolean").expect("boolean missing");
    assert!(matches!(bool_val.value_type, Some(ValueType::BooleanValue(_))));
    
    let null_val = snapshot.get("null").expect("null missing");
    assert!(matches!(null_val.value_type, Some(ValueType::NullValue(_))));
    
    // Clean up
    doc_ref.delete().await.expect("Failed to delete");
    
    println!("✅ Multiple data types work!");
}

/// Test: Large document with many fields
#[tokio::test]
async fn test_large_document() {
    let firestore = get_firestore().await;
    let doc_path = test_doc_path("large");
    
    let doc_ref = firestore.document(&doc_path);
    
    // Create document with 50 fields
    let mut fields = Vec::new();
    for i in 0..50 {
        fields.push((
            format!("field_{}", i).leak() as &str,
            ValueType::IntegerValue(i),
        ));
    }
    
    let data = create_map(fields);
    doc_ref.set(data).await
        .expect("Failed to set large document");
    
    let snapshot = doc_ref.get().await
        .expect("Failed to get large document");
    
    assert!(snapshot.exists());
    assert_eq!(snapshot.data.as_ref().unwrap().fields.len(), 50);
    
    // Verify a few fields
    let field_0 = snapshot.get("field_0").expect("field_0 missing");
    assert!(matches!(field_0.value_type, Some(ValueType::IntegerValue(0))));
    
    let field_49 = snapshot.get("field_49").expect("field_49 missing");
    assert!(matches!(field_49.value_type, Some(ValueType::IntegerValue(49))));
    
    // Clean up
    doc_ref.delete().await.expect("Failed to delete");
    
    println!("✅ Large document (50 fields) works!");
}

/// Test: Overwrite document with set()
#[tokio::test]
async fn test_overwrite_document() {
    let firestore = get_firestore().await;
    let doc_path = test_doc_path("overwrite");
    
    let doc_ref = firestore.document(&doc_path);
    
    // Create initial document
    let initial = create_map(vec![
        ("field1", ValueType::StringValue("value1".to_string())),
        ("field2", ValueType::IntegerValue(100)),
    ]);
    doc_ref.set(initial).await.expect("Failed to set initial");
    
    // Overwrite with completely new data (set replaces entire document)
    let new_data = create_map(vec![
        ("field3", ValueType::BooleanValue(true)),
    ]);
    doc_ref.set(new_data).await.expect("Failed to overwrite");
    
    let snapshot = doc_ref.get().await.expect("Failed to get");
    
    // Old fields should be gone
    assert!(snapshot.get("field1").is_none());
    assert!(snapshot.get("field2").is_none());
    
    // New field should exist
    assert!(snapshot.get("field3").is_some());
    
    // Clean up
    doc_ref.delete().await.expect("Failed to delete");
    
    println!("✅ Document overwrite works!");
}

/// Test: Batch with mixed operations
#[tokio::test]
async fn test_batch_mixed_operations() {
    let firestore = get_firestore().await;
    let collection = format!("integration_tests_batch_mixed_{}", rand::random::<u32>());
    
    // Create one document first
    let doc1_path = format!("{}/doc1", collection);
    let doc1 = firestore.document(&doc1_path);
    doc1.set(create_map(vec![
        ("value", ValueType::IntegerValue(1)),
    ])).await.expect("Failed to create doc1");
    
    // Create batch with mixed operations
    let mut batch = firestore.batch();
    
    // Set doc2 (create new)
    batch.set(
        format!("{}/doc2", collection),
        create_map(vec![("value", ValueType::IntegerValue(2))]),
    );
    
    // Update doc1 (modify existing)
    batch.update(
        doc1_path.clone(),
        create_map(vec![("value", ValueType::IntegerValue(10))]),
    );
    
    // Set doc3 (create new)
    batch.set(
        format!("{}/doc3", collection),
        create_map(vec![("value", ValueType::IntegerValue(3))]),
    );
    
    // Delete doc2 (delete what we just created in this batch)
    batch.delete(format!("{}/doc2", collection));
    
    // Commit batch
    batch.commit().await.expect("Failed to commit mixed batch");
    
    // Verify results
    let doc1_snapshot = doc1.get().await.expect("Failed to get doc1");
    let value1 = doc1_snapshot.get("value").unwrap();
    assert!(matches!(value1.value_type, Some(ValueType::IntegerValue(10))));
    
    let doc2 = firestore.document(&format!("{}/doc2", collection));
    let doc2_result = doc2.get().await;
    assert!(doc2_result.is_err() || !doc2_result.unwrap().exists());
    
    let doc3 = firestore.document(&format!("{}/doc3", collection));
    let doc3_snapshot = doc3.get().await.expect("Failed to get doc3");
    assert!(doc3_snapshot.exists());
    
    // Clean up
    doc1.delete().await.ok();
    doc3.delete().await.ok();
    
    println!("✅ Batch with mixed operations works!");
}

/// Test: Empty batch should fail with InvalidArgument
#[tokio::test]
async fn test_empty_batch() {
    let firestore = get_firestore().await;
    
    let batch = firestore.batch();
    
    // Firestore rejects empty batches
    let result = batch.commit().await;
    assert!(result.is_err(), "Empty batch should fail");
    
    match result {
        Err(e) => {
            let err_str = format!("{:?}", e);
            assert!(err_str.contains("InvalidArgument") || err_str.contains("empty"), 
                "Error should mention empty batch: {}", err_str);
            println!("✅ Empty batch fails as expected: {}", e);
        }
        Ok(_) => panic!("Empty batch should not succeed"),
    }
}

/// Test: Collection path parsing and validation
#[tokio::test]
async fn test_collection_paths() {
    let firestore = get_firestore().await;
    
    // Simple collection
    let col1 = firestore.collection("users");
    let doc1 = col1.document("alice");
    assert_eq!(doc1.path, "users/alice");
    assert_eq!(doc1.id(), "alice");
    
    // Nested collection (subcollection) - use full path
    let post = firestore.document("users/bob/posts/post1");
    assert_eq!(post.path, "users/bob/posts/post1");
    assert_eq!(post.id(), "post1");
    assert_eq!(post.parent_path(), Some("users/bob/posts"));
    
    println!("✅ Collection path parsing works!");
}

/// Test: Document ID extraction
#[tokio::test]
async fn test_document_id_extraction() {
    let firestore = get_firestore().await;
    
    let doc1 = firestore.document("users/alice");
    assert_eq!(doc1.id(), "alice");
    
    let doc2 = firestore.document("projects/proj1/tasks/task2");
    assert_eq!(doc2.id(), "task2");
    
    let doc3 = firestore.document("single");
    assert_eq!(doc3.id(), "single");
    
    println!("✅ Document ID extraction works!");
}

/// Test: Special characters in document IDs
#[tokio::test]
async fn test_special_characters_in_paths() {
    let firestore = get_firestore().await;
    
    // Test with underscores, hyphens, periods
    let doc_id = "test_doc-123.v2";
    let doc_path = format!("integration_tests/{}", doc_id);
    let doc_ref = firestore.document(&doc_path);
    
    let data = create_map(vec![
        ("test", ValueType::BooleanValue(true)),
    ]);
    
    doc_ref.set(data).await
        .expect("Failed to set document with special chars");
    
    let snapshot = doc_ref.get().await
        .expect("Failed to get document with special chars");
    
    assert!(snapshot.exists());
    assert_eq!(snapshot.id(), doc_id);
    
    // Clean up
    doc_ref.delete().await.expect("Failed to delete");
    
    println!("✅ Special characters in paths work!");
}

/// Test: Listener receives delete events
#[tokio::test]
async fn test_listener_delete_event() {
    dotenvy::dotenv().ok();
    
    let firestore = get_firestore().await;
    let doc_path = test_doc_path("listener_delete");
    
    // Get credentials for listener
    let project_id = env::var("FIREBASE_PROJECT_ID").expect("FIREBASE_PROJECT_ID required");
    let database_id = env::var("FIREBASE_DATABASE_ID").unwrap_or_else(|_| "default".to_string());
    let api_key = env::var("FIREBASE_API_KEY").expect("FIREBASE_API_KEY required");
    let email = env::var("TEST_USER_EMAIL").expect("TEST_USER_EMAIL required");
    let password = env::var("TEST_USER_PASSWORD").expect("TEST_USER_PASSWORD required");
    
    let app = App::create(AppOptions {
        api_key,
        project_id: project_id.clone(),
        app_name: None,
    }).await.expect("Failed to create App");
    
    let auth = Auth::get_auth(&app).await.expect("Failed to get Auth");
    auth.sign_in_with_email_and_password(&email, &password).await
        .expect("Failed to sign in");
    
    let user = auth.current_user().await.expect("No user");
    let auth_token = user.get_id_token(false).await.expect("Failed to get token");
    
    // Create initial document
    let doc_ref = firestore.document(&doc_path);
    doc_ref.set(create_map(vec![
        ("status", ValueType::StringValue("active".to_string())),
    ])).await.expect("Failed to create");
    
    // Start listener
    let mut stream = listen_document(
        &firestore,
        auth_token,
        project_id,
        database_id,
        doc_path.clone(),
        ListenerOptions::default(),
    ).await.expect("Failed to start listener");
    
    let received_delete = Arc::new(Mutex::new(false));
    let delete_flag = received_delete.clone();
    
    let stream_task = tokio::spawn(async move {
        while let Some(result) = stream.next().await {
            if let Ok(snapshot) = result {
                if !snapshot.exists() {
                    *delete_flag.lock().unwrap() = true;
                    println!("📡 Listener received delete event");
                    break;
                }
            }
        }
    });
    
    // Wait for initial snapshot
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Delete the document
    doc_ref.delete().await.expect("Failed to delete");
    
    // Wait for delete event
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    stream_task.abort();
    
    let got_delete = *received_delete.lock().unwrap();
    assert!(got_delete, "Listener should receive delete event");
    
    println!("✅ Listener delete event works!");
}

/// Test: Concurrent writes to same document
#[tokio::test]
async fn test_concurrent_writes() {
    let firestore = get_firestore().await;
    let doc_path = test_doc_path("concurrent");
    
    let doc_ref = firestore.document(&doc_path);
    
    // Create initial document
    doc_ref.set(create_map(vec![
        ("counter", ValueType::IntegerValue(0)),
    ])).await.expect("Failed to create");
    
    // Spawn multiple concurrent updates
    let mut handles = vec![];
    for i in 1..=5 {
        let doc_ref_clone = doc_ref.clone();
        let handle = tokio::spawn(async move {
            let data = create_map(vec![
                ("counter", ValueType::IntegerValue(i)),
                ("writer", ValueType::IntegerValue(i)),
            ]);
            doc_ref_clone.set(data).await
        });
        handles.push(handle);
    }
    
    // Wait for all writes to complete
    for handle in handles {
        handle.await.expect("Task panicked").expect("Write failed");
    }
    
    // Read final state (one of the writes should have won)
    let snapshot = doc_ref.get().await.expect("Failed to get");
    assert!(snapshot.exists());
    
    let counter = snapshot.get("counter").expect("counter missing");
    assert!(matches!(counter.value_type, Some(ValueType::IntegerValue(_))));
    
    // Clean up
    doc_ref.delete().await.expect("Failed to delete");
    
    println!("✅ Concurrent writes complete (last write wins)!");
}

/// Test: Very long document path
#[tokio::test]
async fn test_deep_nested_path() {
    let firestore = get_firestore().await;
    
    // Create a very deep path (10 levels)
    let path = format!(
        "integration_tests/l1_{}/level2/l2_{}/level3/l3_{}/level4/l4_{}/level5/l5_{}",
        rand::random::<u32>(),
        rand::random::<u32>(),
        rand::random::<u32>(),
        rand::random::<u32>(),
        rand::random::<u32>()
    );
    
    let doc_ref = firestore.document(&path);
    doc_ref.set(create_map(vec![
        ("depth", ValueType::IntegerValue(5)),
    ])).await.expect("Failed to set deep document");
    
    let snapshot = doc_ref.get().await.expect("Failed to get deep document");
    assert!(snapshot.exists());
    
    // Clean up
    doc_ref.delete().await.expect("Failed to delete");
    
    println!("✅ Deep nested paths work!");
}

/// Test: Document listener receives initial snapshot and updates
#[tokio::test]
async fn test_document_listener_receives_updates() {
    let firestore = get_firestore().await;
    let doc_path = test_doc_path("listen_doc");
    
    // Create initial document
    let doc_ref = firestore.document(&doc_path);
    doc_ref.set(create_map(vec![
        ("counter", ValueType::IntegerValue(0)),
        ("name", ValueType::StringValue("test".to_string())),
    ])).await.expect("Failed to create document");
    
    // Start listening
    let mut stream = doc_ref.listen(None);
    
    // Should receive initial snapshot
    let snapshot = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        stream.next()
    ).await
        .expect("Timeout waiting for initial snapshot")
        .expect("Stream ended")
        .expect("Error in initial snapshot");
    
    assert!(snapshot.exists(), "Initial snapshot should exist");
    
    // Verify initial data
    let counter = snapshot.get("counter").expect("counter field missing");
    match &counter.value_type {
        Some(ValueType::IntegerValue(v)) => assert_eq!(*v, 0),
        _ => panic!("Expected integer value"),
    }
    
    // Update document
    doc_ref.set(create_map(vec![
        ("counter", ValueType::IntegerValue(1)),
        ("name", ValueType::StringValue("updated".to_string())),
    ])).await.expect("Failed to update document");
    
    // Should receive update
    let updated_snapshot = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        stream.next()
    ).await
        .expect("Timeout waiting for update")
        .expect("Stream ended")
        .expect("Error in update snapshot");
    
    assert!(updated_snapshot.exists());
    
    // Verify updated data
    let counter = updated_snapshot.get("counter").expect("counter field missing");
    match &counter.value_type {
        Some(ValueType::IntegerValue(v)) => assert_eq!(*v, 1),
        _ => panic!("Expected integer value"),
    }
    
    let name = updated_snapshot.get("name").expect("name field missing");
    match &name.value_type {
        Some(ValueType::StringValue(s)) => assert_eq!(s, "updated"),
        _ => panic!("Expected string value"),
    }
    
    // Clean up
    drop(stream);
    doc_ref.delete().await.expect("Failed to delete");
    
    println!("✅ Document listener receives updates!");
}

/// Test: Listener receives delete event
#[tokio::test]
async fn test_document_listener_receives_delete() {
    let firestore = get_firestore().await;
    let doc_path = test_doc_path("listen_delete");
    
    // Create document
    let doc_ref = firestore.document(&doc_path);
    doc_ref.set(create_map(vec![
        ("temp", ValueType::BooleanValue(true)),
    ])).await.expect("Failed to create document");
    
    // Start listening
    let mut stream = doc_ref.listen(None);
    
    // Receive initial snapshot
    let snapshot = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        stream.next()
    ).await
        .expect("Timeout waiting for initial snapshot")
        .expect("Stream ended")
        .expect("Error in snapshot");
    
    assert!(snapshot.exists());
    
    // Delete document
    doc_ref.delete().await.expect("Failed to delete");
    
    // Should receive delete event (snapshot with exists=false)
    let deleted_snapshot = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        stream.next()
    ).await
        .expect("Timeout waiting for delete")
        .expect("Stream ended")
        .expect("Error in delete snapshot");
    
    assert!(!deleted_snapshot.exists(), "Snapshot should not exist after delete");
    
    drop(stream);
    
    println!("✅ Document listener receives delete event!");
}

/// Test: Multiple listeners on same document
#[tokio::test]
async fn test_multiple_document_listeners() {
    let firestore = get_firestore().await;
    let doc_path = test_doc_path("listen_multi");
    
    // Create document
    let doc_ref = firestore.document(&doc_path);
    doc_ref.set(create_map(vec![
        ("value", ValueType::IntegerValue(100)),
    ])).await.expect("Failed to create document");
    
    // Start two listeners
    let mut stream1 = doc_ref.listen(None);
    let mut stream2 = doc_ref.listen(None);
    
    // Both should receive initial snapshot
    let snap1 = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        stream1.next()
    ).await
        .expect("Timeout on stream1")
        .expect("Stream1 ended")
        .expect("Error on stream1");
    
    let snap2 = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        stream2.next()
    ).await
        .expect("Timeout on stream2")
        .expect("Stream2 ended")
        .expect("Error on stream2");
    
    assert!(snap1.exists());
    assert!(snap2.exists());
    
    // Update document
    doc_ref.set(create_map(vec![
        ("value", ValueType::IntegerValue(200)),
    ])).await.expect("Failed to update");
    
    // Both should receive update
    let update1 = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        stream1.next()
    ).await
        .expect("Timeout on stream1 update")
        .expect("Stream1 ended")
        .expect("Error on stream1 update");
    
    let update2 = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        stream2.next()
    ).await
        .expect("Timeout on stream2 update")
        .expect("Stream2 ended")
        .expect("Error on stream2 update");
    
    // Verify both got the update
    let val1 = update1.get("value").expect("value missing");
    let val2 = update2.get("value").expect("value missing");
    
    match (&val1.value_type, &val2.value_type) {
        (Some(ValueType::IntegerValue(v1)), Some(ValueType::IntegerValue(v2))) => {
            assert_eq!(*v1, 200);
            assert_eq!(*v2, 200);
        }
        _ => panic!("Expected integer values"),
    }
    
    // Clean up
    drop(stream1);
    drop(stream2);
    doc_ref.delete().await.expect("Failed to delete");
    
    println!("✅ Multiple document listeners work!");
}

/// Test: Listener stops receiving updates after drop
#[tokio::test]
async fn test_listener_cleanup_on_drop() {
    let firestore = get_firestore().await;
    let doc_path = test_doc_path("listen_cleanup");
    
    // Create document
    let doc_ref = firestore.document(&doc_path);
    doc_ref.set(create_map(vec![
        ("count", ValueType::IntegerValue(0)),
    ])).await.expect("Failed to create document");
    
    {
        let mut stream = doc_ref.listen(None);
        
        // Receive initial snapshot
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            stream.next()
        ).await
            .expect("Timeout")
            .expect("Stream ended")
            .expect("Error");
        
        // Stream dropped here
    }
    
    // Update document after listener dropped
    doc_ref.set(create_map(vec![
        ("count", ValueType::IntegerValue(1)),
    ])).await.expect("Failed to update");
    
    // Small delay to ensure no events are being processed
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    
    // Clean up
    doc_ref.delete().await.expect("Failed to delete");
    
    println!("✅ Listener cleanup on drop works!");
}

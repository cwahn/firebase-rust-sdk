//! Cloud Firestore
//!
//! # C++ Reference
//! - `firestore/src/include/firebase/firestore.h:91` - Firestore class

use crate::error::FirebaseError;
use crate::firestore::types::{DocumentReference, DocumentSnapshot, SnapshotMetadata};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use rand::Rng;

/// Global map of project IDs to Firestore instances
///
/// C++ equivalent: Similar to Auth singleton pattern
static FIRESTORE_INSTANCES: Lazy<RwLock<HashMap<String, Firestore>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Cloud Firestore instance
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore.h:91`
///
/// Entry point for the Firebase Firestore SDK.
/// Use `Firestore::get_firestore(project_id)` to obtain an instance.
#[derive(Clone)]
pub struct Firestore {
    inner: Arc<FirestoreInner>,
}

struct FirestoreInner {
    project_id: String,
    database_id: String,
    http_client: reqwest::Client,
}

impl Firestore {
    /// Get or create Firestore instance for the given project
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore.h:118` - GetInstance(app)
    ///
    /// Returns existing Firestore if one exists for this project ID, otherwise creates new.
    /// Thread-safe singleton pattern following C++ implementation.
    ///
    /// # Arguments
    /// * `project_id` - The Google Cloud project ID
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::firestore::Firestore;
    ///
    /// let firestore = Firestore::get_firestore("my-project-id").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_firestore(project_id: impl Into<String>) -> Result<Self, FirebaseError> {
        Self::get_firestore_with_database(project_id, "(default)").await
    }

    /// Get or create Firestore instance with specific database ID
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore.h:158` - GetInstance(app, db_name)
    ///
    /// # Arguments
    /// * `project_id` - The Google Cloud project ID
    /// * `database_id` - The database ID (default: "(default)")
    pub async fn get_firestore_with_database(
        project_id: impl Into<String>,
        database_id: impl Into<String>,
    ) -> Result<Self, FirebaseError> {
        let project_id = project_id.into();
        let database_id = database_id.into();

        // Validate project ID (error case first)
        if project_id.is_empty() {
            return Err(FirebaseError::Internal("Project ID cannot be empty".to_string()));
        }

        // Create composite key for instances map
        let key = format!("{}:{}", project_id, database_id);

        let mut instances = FIRESTORE_INSTANCES.write().await;

        // Check if instance already exists
        let existing = instances.get(&key);
        if let Some(firestore) = existing {
            return Ok(firestore.clone());
        }

        // Create new Firestore instance
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| FirebaseError::Internal(format!("Failed to create HTTP client: {}", e)))?;

        let firestore = Firestore {
            inner: Arc::new(FirestoreInner {
                project_id,
                database_id,
                http_client,
            }),
        };

        instances.insert(key, firestore.clone());

        Ok(firestore)
    }

    /// Get the project ID
    pub fn project_id(&self) -> &str {
        &self.inner.project_id
    }

    /// Get the database ID
    pub fn database_id(&self) -> &str {
        &self.inner.database_id
    }

    /// Get a reference to a collection
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore.h:227` - Collection(path)
    ///
    /// # Arguments
    /// * `path` - Slash-separated path to the collection
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::firestore::Firestore;
    ///
    /// let firestore = Firestore::get_firestore("my-project").await?;
    /// let users = firestore.collection("users");
    /// # Ok(())
    /// # }
    /// ```
    pub fn collection(&self, path: impl AsRef<str>) -> CollectionReference {
        CollectionReference::new(self.clone(), path.as_ref().to_string())
    }

    /// Get a reference to a document
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore.h:246` - Document(path)
    ///
    /// # Arguments
    /// * `path` - Slash-separated path to the document
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::firestore::Firestore;
    ///
    /// let firestore = Firestore::get_firestore("my-project").await?;
    /// let doc = firestore.document("users/alice");
    /// # Ok(())
    /// # }
    /// ```
    pub fn document(&self, path: impl AsRef<str>) -> DocumentReference {
        DocumentReference::new(path.as_ref())
    }

    /// Internal: Get HTTP client
    pub(crate) fn http_client(&self) -> &reqwest::Client {
        &self.inner.http_client
    }
    
    /// Get document data
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/document_reference.h:193` - Get()
    pub async fn get_document(&self, path: impl AsRef<str>) -> Result<DocumentSnapshot, FirebaseError> {
        let url = format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/{}/documents/{}",
            self.project_id(),
            self.database_id(),
            path.as_ref()
        );
        
        let response = self.http_client()
            .get(&url)
            .send()
            .await?;
        
        // Handle 404 (document not found)
        if response.status() == 404 {
            return Ok(DocumentSnapshot {
                reference: DocumentReference::new(path.as_ref()),
                data: None,
                metadata: SnapshotMetadata::default(),
            });
        }
        
        // Handle other errors
        if !response.status().is_success() {
            return Err(FirebaseError::Internal(format!("Get failed: {}", response.status())));
        }
        
        let doc_data: serde_json::Value = response.json().await?;
        let fields = doc_data.get("fields").cloned();
        
        Ok(DocumentSnapshot {
            reference: DocumentReference::new(path.as_ref()),
            data: fields,
            metadata: SnapshotMetadata {
                has_pending_writes: false,
                is_from_cache: false,
            },
        })
    }
    
    /// Set (write/overwrite) document data
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/document_reference.h:206` - Set()
    pub async fn set_document(&self, path: impl AsRef<str>, data: serde_json::Value) -> Result<(), FirebaseError> {
        let url = format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/{}/documents/{}",
            self.project_id(),
            self.database_id(),
            path.as_ref()
        );
        
        let doc = serde_json::json!({ "fields": data });
        
        let response = self.http_client()
            .patch(&url)
            .json(&doc)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(FirebaseError::Internal(format!("Set failed: {}", response.status())));
        }
        
        Ok(())
    }
    
    /// Update specific fields in document
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/document_reference.h:219` - Update()
    pub async fn update_document(&self, path: impl AsRef<str>, data: serde_json::Value) -> Result<(), FirebaseError> {
        let mut update_mask = Vec::new();
        if let Some(obj) = data.as_object() {
            update_mask.extend(obj.keys().map(|k| k.as_str()));
        }
        
        let url = format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/{}/documents/{}?updateMask.fieldPaths={}",
            self.project_id(),
            self.database_id(),
            path.as_ref(),
            update_mask.join("&updateMask.fieldPaths=")
        );
        
        let doc = serde_json::json!({ "fields": data });
        
        let response = self.http_client()
            .patch(&url)
            .json(&doc)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(FirebaseError::Internal(format!("Update failed: {}", response.status())));
        }
        
        Ok(())
    }
    
    /// Delete document
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/document_reference.h:243` - Delete()
    pub async fn delete_document(&self, path: impl AsRef<str>) -> Result<(), FirebaseError> {
        let url = format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/{}/documents/{}",
            self.project_id(),
            self.database_id(),
            path.as_ref()
        );
        
        let response = self.http_client()
            .delete(&url)
            .send()
            .await?;
        
        // 404 is acceptable for delete
        if !response.status().is_success() && response.status() != 404 {
            return Err(FirebaseError::Internal(format!("Delete failed: {}", response.status())));
        }
        
        Ok(())
    }

    /// Execute a query (internal method)
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/query.h:318` - Get()
    async fn execute_query(&self, query: Query) -> Result<Vec<DocumentSnapshot>, FirebaseError> {
        use crate::firestore::types::{FilterCondition, OrderDirection};

        let url = format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/{}/documents:runQuery",
            self.project_id(),
            self.database_id()
        );

        // Build structured query
        let mut structured_query = serde_json::json!({
            "from": [{
                "collectionId": query.collection_path.rsplit('/').next().unwrap_or(&query.collection_path)
            }]
        });

        // Add filters
        if !query.filters.is_empty() {
            let filters: Vec<serde_json::Value> = query.filters.iter().map(|filter| {
                let field_path = filter.field_path();
                match filter {
                    FilterCondition::Equal(_, val) => {
                        serde_json::json!({
                            "fieldFilter": {
                                "field": {"fieldPath": field_path},
                                "op": "EQUAL",
                                "value": convert_value_to_firestore(val.clone())
                            }
                        })
                    }
                    FilterCondition::LessThan(_, val) => {
                        serde_json::json!({
                            "fieldFilter": {
                                "field": {"fieldPath": field_path},
                                "op": "LESS_THAN",
                                "value": convert_value_to_firestore(val.clone())
                            }
                        })
                    }
                    FilterCondition::LessThanOrEqual(_, val) => {
                        serde_json::json!({
                            "fieldFilter": {
                                "field": {"fieldPath": field_path},
                                "op": "LESS_THAN_OR_EQUAL",
                                "value": convert_value_to_firestore(val.clone())
                            }
                        })
                    }
                    FilterCondition::GreaterThan(_, val) => {
                        serde_json::json!({
                            "fieldFilter": {
                                "field": {"fieldPath": field_path},
                                "op": "GREATER_THAN",
                                "value": convert_value_to_firestore(val.clone())
                            }
                        })
                    }
                    FilterCondition::GreaterThanOrEqual(_, val) => {
                        serde_json::json!({
                            "fieldFilter": {
                                "field": {"fieldPath": field_path},
                                "op": "GREATER_THAN_OR_EQUAL",
                                "value": convert_value_to_firestore(val.clone())
                            }
                        })
                    }
                    FilterCondition::ArrayContains(_, val) => {
                        serde_json::json!({
                            "fieldFilter": {
                                "field": {"fieldPath": field_path},
                                "op": "ARRAY_CONTAINS",
                                "value": convert_value_to_firestore(val.clone())
                            }
                        })
                    }
                    FilterCondition::ArrayContainsAny(_, vals) => {
                        serde_json::json!({
                            "fieldFilter": {
                                "field": {"fieldPath": field_path},
                                "op": "ARRAY_CONTAINS_ANY",
                                "value": {
                                    "arrayValue": {
                                        "values": vals.iter().map(|v| convert_value_to_firestore(v.clone())).collect::<Vec<_>>()
                                    }
                                }
                            }
                        })
                    }
                    FilterCondition::In(_, vals) => {
                        serde_json::json!({
                            "fieldFilter": {
                                "field": {"fieldPath": field_path},
                                "op": "IN",
                                "value": {
                                    "arrayValue": {
                                        "values": vals.iter().map(|v| convert_value_to_firestore(v.clone())).collect::<Vec<_>>()
                                    }
                                }
                            }
                        })
                    }
                    FilterCondition::NotEqual(_, val) => {
                        serde_json::json!({
                            "fieldFilter": {
                                "field": {"fieldPath": field_path},
                                "op": "NOT_EQUAL",
                                "value": convert_value_to_firestore(val.clone())
                            }
                        })
                    }
                    FilterCondition::NotIn(_, vals) => {
                        serde_json::json!({
                            "fieldFilter": {
                                "field": {"fieldPath": field_path},
                                "op": "NOT_IN",
                                "value": {
                                    "arrayValue": {
                                        "values": vals.iter().map(|v| convert_value_to_firestore(v.clone())).collect::<Vec<_>>()
                                    }
                                }
                            }
                        })
                    }
                }
            }).collect();

            if filters.len() == 1 {
                structured_query["where"] = filters.into_iter().next().unwrap();
            } else {
                structured_query["where"] = serde_json::json!({
                    "compositeFilter": {
                        "op": "AND",
                        "filters": filters
                    }
                });
            }
        }

        // Add order by
        if !query.order_by.is_empty() {
            structured_query["orderBy"] = serde_json::json!(
                query.order_by.iter().map(|(field, direction)| {
                    serde_json::json!({
                        "field": {"fieldPath": field},
                        "direction": match direction {
                            OrderDirection::Ascending => "ASCENDING",
                            OrderDirection::Descending => "DESCENDING",
                        }
                    })
                }).collect::<Vec<_>>()
            );
        }

        // Add limit
        if let Some(limit) = query.limit_value {
            structured_query["limit"] = serde_json::json!(limit);
        }

        // Add start/end cursors (simplified - actual implementation needs proper cursor handling)
        if query.start_at.is_some() || query.end_at.is_some() {
            // Cursor implementation would go here
            // For now, we'll skip complex cursor logic
        }

        let request_body = serde_json::json!({
            "structuredQuery": structured_query
        });

        let response = self.http_client()
            .post(&url)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(FirebaseError::Internal(format!("Query failed: {}", response.status())));
        }

        let results: Vec<serde_json::Value> = response.json().await?;
        
        let documents: Vec<DocumentSnapshot> = results
            .into_iter()
            .filter_map(|result| {
                result.get("document").and_then(|doc| {
                    let name = doc.get("name")?.as_str()?;
                    let path = name.split("/documents/").nth(1)?;
                    let fields = doc.get("fields").cloned();
                    
                    Some(DocumentSnapshot {
                        reference: DocumentReference::new(path),
                        data: fields,
                        metadata: SnapshotMetadata {
                            has_pending_writes: false,
                            is_from_cache: false,
                        },
                    })
                })
            })
            .collect();

        Ok(documents)
    }
}

/// Convert serde_json::Value to Firestore value format
fn convert_value_to_firestore(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Null => serde_json::json!({"nullValue": null}),
        serde_json::Value::Bool(b) => serde_json::json!({"booleanValue": b}),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_json::json!({"integerValue": i.to_string()})
            } else if let Some(f) = n.as_f64() {
                serde_json::json!({"doubleValue": f})
            } else {
                serde_json::json!({"nullValue": null})
            }
        }
        serde_json::Value::String(s) => serde_json::json!({"stringValue": s}),
        serde_json::Value::Array(arr) => {
            serde_json::json!({
                "arrayValue": {
                    "values": arr.into_iter().map(convert_value_to_firestore).collect::<Vec<_>>()
                }
            })
        }
        serde_json::Value::Object(obj) => {
            serde_json::json!({
                "mapValue": {
                    "fields": obj.into_iter().map(|(k, v)| (k, convert_value_to_firestore(v))).collect::<serde_json::Map<String, serde_json::Value>>()
                }
            })
        }
    }
}

/// Firestore query builder
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/query.h:76`
#[derive(Clone)]
pub struct Query {
    firestore: Firestore,
    collection_path: String,
    filters: Vec<crate::firestore::types::FilterCondition>,
    order_by: Vec<(String, crate::firestore::types::OrderDirection)>,
    limit_value: Option<usize>,
    start_at: Option<Vec<serde_json::Value>>,
    end_at: Option<Vec<serde_json::Value>>,
}

impl Query {
    /// Create a new query for a collection
    pub(crate) fn new(firestore: Firestore, collection_path: String) -> Self {
        Self {
            firestore,
            collection_path,
            filters: Vec::new(),
            order_by: Vec::new(),
            limit_value: None,
            start_at: None,
            end_at: None,
        }
    }

    /// Add a filter condition to the query
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/query.h:142` - Where()
    pub fn where_filter(mut self, condition: crate::firestore::types::FilterCondition) -> Self {
        self.filters.push(condition);
        self
    }

    /// Order results by a field
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/query.h:204` - OrderBy()
    pub fn order_by(
        mut self,
        field: impl Into<String>,
        direction: crate::firestore::types::OrderDirection,
    ) -> Self {
        self.order_by.push((field.into(), direction));
        self
    }

    /// Limit the number of results
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/query.h:232` - Limit()
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit_value = Some(limit);
        self
    }

    /// Start query at cursor values
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/query.h:285` - StartAt()
    pub fn start_at(mut self, values: Vec<serde_json::Value>) -> Self {
        self.start_at = Some(values);
        self
    }

    /// End query at cursor values
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/query.h:298` - EndAt()
    pub fn end_at(mut self, values: Vec<serde_json::Value>) -> Self {
        self.end_at = Some(values);
        self
    }

    /// Execute the query
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/query.h:318` - Get()
    pub async fn get(self) -> Result<Vec<DocumentSnapshot>, FirebaseError> {
        let firestore = self.firestore.clone();
        firestore.execute_query(self).await
    }
}

impl std::fmt::Debug for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Query")
            .field("collection_path", &self.collection_path)
            .field("filters", &self.filters.len())
            .field("order_by", &self.order_by)
            .field("limit", &self.limit_value)
            .finish()
    }
}

/// Collection reference
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/collection_reference.h`
pub struct CollectionReference {
    firestore: Firestore,
    path: String,
}

impl CollectionReference {
    fn new(firestore: Firestore, path: String) -> Self {
        Self { firestore, path }
    }

    /// Get the collection path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get the collection ID (last segment of path)
    pub fn id(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }

    /// Get a document reference within this collection
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/collection_reference.h:96` - Document(path)
    pub fn document(&self, document_id: impl AsRef<str>) -> DocumentReference {
        let full_path = format!("{}/{}", self.path, document_id.as_ref());
        DocumentReference::new(full_path)
    }

    /// Add a new document with auto-generated ID
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/collection_reference.h:124` - Add(data)
    /// - `firestore/src/main/collection_reference_main.cc:78` - AddDocument implementation
    ///
    /// Generates a 20-character alphanumeric ID and creates the document.
    /// This follows Firestore's auto-ID generation pattern.
    ///
    /// # Arguments
    /// * `data` - Document data as JSON value
    ///
    /// # Errors
    /// Returns `FirebaseError::FirestoreError` if:
    /// - Data is not a JSON object
    /// - Network request fails
    /// - API returns an error
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::firestore::Firestore;
    /// use serde_json::json;
    ///
    /// let firestore = Firestore::get_firestore("my-project").await?;
    /// let doc_ref = firestore.collection("users")
    ///     .add(json!({"name": "Alice", "age": 30}))
    ///     .await?;
    /// println!("Created document: {}", doc_ref.path());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn add(&self, data: serde_json::Value) -> Result<DocumentReference, FirebaseError> {
        use crate::error::FirestoreError;
        
        // Error-first: validate data is an object
        if !data.is_object() {
            return Err(FirebaseError::Firestore(
                FirestoreError::InvalidData("Data must be a JSON object".to_string()),
            ));
        }

        // Generate auto-ID: 20 alphanumeric characters
        // Firestore uses [a-zA-Z0-9] for auto-generated IDs
        let doc_id = generate_auto_id();
        let doc_ref = self.document(&doc_id);

        // Use set_document to create the document
        self.firestore.set_document(&doc_ref.path, data).await?;

        Ok(doc_ref)
    }

    /// Create a query for this collection
    ///
    /// # C++ Reference
    /// - Query inherits from CollectionReference in C++
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::firestore::{Firestore, types::{FilterCondition, OrderDirection}};
    /// use serde_json::json;
    ///
    /// let firestore = Firestore::get_firestore("my-project").await?;
    /// let docs = firestore.collection("users")
    ///     .query()
    ///     .where_filter(FilterCondition::GreaterThan("age".to_string(), json!(18)))
    ///     .order_by("age", OrderDirection::Ascending)
    ///     .limit(10)
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn query(&self) -> Query {
        Query::new(self.firestore.clone(), self.path.clone())
    }
}

/// Generate a Firestore auto-ID
///
/// Creates a 20-character random alphanumeric string matching Firestore's
/// auto-ID generation pattern: [a-zA-Z0-9]{20}
///
/// # C++ Reference
/// - Firestore auto-ID generation (called by AddDocument)
fn generate_auto_id() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    const ID_LENGTH: usize = 20;
    
    let mut rng = rand::thread_rng();
    (0..ID_LENGTH)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

impl std::fmt::Debug for Firestore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Firestore")
            .field("project_id", &self.inner.project_id)
            .field("database_id", &self.inner.database_id)
            .finish()
    }
}

impl std::fmt::Debug for CollectionReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CollectionReference")
            .field("path", &self.path)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_firestore_creates_instance() {
        let fs = Firestore::get_firestore("test-project-1").await.unwrap();
        assert_eq!(fs.project_id(), "test-project-1");
        assert_eq!(fs.database_id(), "(default)");
    }

    #[tokio::test]
    async fn test_get_firestore_returns_same_instance() {
        let fs1 = Firestore::get_firestore("test-project-2").await.unwrap();
        let fs2 = Firestore::get_firestore("test-project-2").await.unwrap();

        // Should return same instance (same Arc pointer)
        assert!(Arc::ptr_eq(&fs1.inner, &fs2.inner));
    }

    #[tokio::test]
    async fn test_get_firestore_empty_project_error() {
        let result = Firestore::get_firestore("").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_different_projects_different_instances() {
        let fs1 = Firestore::get_firestore("project-a").await.unwrap();
        let fs2 = Firestore::get_firestore("project-b").await.unwrap();

        // Should be different instances
        assert!(!Arc::ptr_eq(&fs1.inner, &fs2.inner));
        assert_eq!(fs1.project_id(), "project-a");
        assert_eq!(fs2.project_id(), "project-b");
    }

    #[tokio::test]
    async fn test_different_databases_different_instances() {
        let fs1 = Firestore::get_firestore_with_database("project-c", "(default)")
            .await
            .unwrap();
        let fs2 = Firestore::get_firestore_with_database("project-c", "custom-db")
            .await
            .unwrap();

        // Should be different instances even with same project
        assert!(!Arc::ptr_eq(&fs1.inner, &fs2.inner));
        assert_eq!(fs1.database_id(), "(default)");
        assert_eq!(fs2.database_id(), "custom-db");
    }

    #[tokio::test]
    async fn test_collection_reference() {
        let fs = Firestore::get_firestore("test-project-3").await.unwrap();
        let users = fs.collection("users");

        assert_eq!(users.path(), "users");
        assert_eq!(users.id(), "users");
    }

    #[tokio::test]
    async fn test_collection_document() {
        let fs = Firestore::get_firestore("test-project-4").await.unwrap();
        let users = fs.collection("users");
        let alice = users.document("alice");

        assert_eq!(alice.path, "users/alice");
        assert_eq!(alice.id(), "alice");
    }

    #[tokio::test]
    async fn test_document_reference() {
        let fs = Firestore::get_firestore("test-project-5").await.unwrap();
        let doc = fs.document("users/bob");

        assert_eq!(doc.path, "users/bob");
        assert_eq!(doc.id(), "bob");
    }

    #[tokio::test]
    async fn test_nested_collection_reference() {
        let fs = Firestore::get_firestore("test-project-6").await.unwrap();
        let posts = fs.collection("users/alice/posts");

        assert_eq!(posts.path(), "users/alice/posts");
        assert_eq!(posts.id(), "posts");
    }

    #[tokio::test]
    async fn test_query_builder() {
        use crate::firestore::types::{FilterCondition, OrderDirection};
        use serde_json::json;

        let fs = Firestore::get_firestore("test-project-7").await.unwrap();
        let query = fs.collection("users")
            .query()
            .where_filter(FilterCondition::Equal("name".to_string(), json!("Alice")))
            .order_by("age", OrderDirection::Ascending)
            .limit(10);

        // Verify query structure (can't execute without real Firebase)
        assert_eq!(query.collection_path, "users");
        assert_eq!(query.filters.len(), 1);
        assert_eq!(query.order_by.len(), 1);
        assert_eq!(query.limit_value, Some(10));
    }

    #[tokio::test]
    async fn test_query_multiple_filters() {
        use crate::firestore::types::FilterCondition;
        use serde_json::json;

        let fs = Firestore::get_firestore("test-project-8").await.unwrap();
        let query = fs.collection("users")
            .query()
            .where_filter(FilterCondition::GreaterThan("age".to_string(), json!(18)))
            .where_filter(FilterCondition::LessThan("age".to_string(), json!(65)))
            .where_filter(FilterCondition::Equal("active".to_string(), json!(true)));

        assert_eq!(query.filters.len(), 3);
    }

    #[tokio::test]
    async fn test_query_with_cursors() {
        use serde_json::json;

        let fs = Firestore::get_firestore("test-project-9").await.unwrap();
        let query = fs.collection("posts")
            .query()
            .start_at(vec![json!("2024-01-01")])
            .end_at(vec![json!("2024-12-31")]);

        assert!(query.start_at.is_some());
        assert!(query.end_at.is_some());
    }

    #[tokio::test]
    async fn test_filter_condition_field_path() {
        use crate::firestore::types::FilterCondition;
        use serde_json::json;

        let filter = FilterCondition::Equal("user.name".to_string(), json!("Bob"));
        assert_eq!(filter.field_path(), "user.name");

        let filter2 = FilterCondition::GreaterThan("score".to_string(), json!(100));
        assert_eq!(filter2.field_path(), "score");
    }

    #[tokio::test]
    async fn test_filter_condition_operators() {
        use crate::firestore::types::FilterCondition;
        use serde_json::json;

        assert_eq!(FilterCondition::Equal("f".to_string(), json!(1)).operator(), "EQUAL");
        assert_eq!(FilterCondition::LessThan("f".to_string(), json!(1)).operator(), "LESS_THAN");
        assert_eq!(FilterCondition::GreaterThanOrEqual("f".to_string(), json!(1)).operator(), "GREATER_THAN_OR_EQUAL");
        assert_eq!(FilterCondition::ArrayContains("f".to_string(), json!(1)).operator(), "ARRAY_CONTAINS");
        assert_eq!(FilterCondition::In("f".to_string(), vec![json!(1)]).operator(), "IN");
    }

    #[tokio::test]
    async fn test_convert_value_to_firestore() {
        use serde_json::json;

        // Test null
        let result = convert_value_to_firestore(json!(null));
        assert_eq!(result, json!({"nullValue": null}));

        // Test boolean
        let result = convert_value_to_firestore(json!(true));
        assert_eq!(result, json!({"booleanValue": true}));

        // Test integer
        let result = convert_value_to_firestore(json!(42));
        assert_eq!(result, json!({"integerValue": "42"}));

        // Test string
        let result = convert_value_to_firestore(json!("hello"));
        assert_eq!(result, json!({"stringValue": "hello"}));

        // Test array
        let result = convert_value_to_firestore(json!([1, 2, 3]));
        assert!(result.get("arrayValue").is_some());
    }

    #[tokio::test]
    async fn test_collection_reference_add_generates_id() {
        use serde_json::json;
        
        let fs = Firestore::get_firestore("test-project-10").await.unwrap();
        let collection = fs.collection("users");
        
        // Create document with auto-generated ID (would fail without real Firebase)
        // Test that the ID generation works
        let doc_id_1 = generate_auto_id();
        let doc_id_2 = generate_auto_id();
        
        // IDs should be 20 characters
        assert_eq!(doc_id_1.len(), 20);
        assert_eq!(doc_id_2.len(), 20);
        
        // IDs should be different (statistically)
        assert_ne!(doc_id_1, doc_id_2);
        
        // IDs should only contain alphanumeric characters
        assert!(doc_id_1.chars().all(|c| c.is_ascii_alphanumeric()));
        assert!(doc_id_2.chars().all(|c| c.is_ascii_alphanumeric()));
    }
    
    #[tokio::test]
    async fn test_collection_reference_add_validates_data() {
        use serde_json::json;
        use crate::error::{FirebaseError, FirestoreError};
        
        let fs = Firestore::get_firestore("test-project-11").await.unwrap();
        let collection = fs.collection("users");
        
        // Test that non-object data is rejected
        let result = collection.add(json!("not an object")).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            FirebaseError::Firestore(FirestoreError::InvalidData(msg)) => {
                assert!(msg.contains("JSON object"));
            }
            _ => panic!("Expected InvalidData error"),
        }
    }
}

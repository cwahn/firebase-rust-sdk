// Firestore Snapshot Listener Implementation using gRPC Streaming
//
// Based on C++ Firebase SDK's WatchStream architecture:
// - Uses bidirectional gRPC streaming via /google.firestore.v1.Firestore/Listen
// - Sends ListenRequest with target (document or query)
// - Receives ListenResponse stream with document changes
// - Returns ListenerRegistration handle for cleanup

#![allow(missing_docs)]
#![allow(clippy::all)]

// Include generated protobuf code
// build.rs creates proto.rs with proper module structure for cross-references
#[allow(clippy::all)]
mod proto {
    include!(concat!(env!("OUT_DIR"), "/proto.rs"));
}

// Convenient alias for firestore types
use firestore_proto::firestore_client::FirestoreClient;
use firestore_proto::listen_response::ResponseType;
use firestore_proto::value::ValueType;
use firestore_proto::{ListenRequest, Target};
use proto::google::firestore::v1 as firestore_proto;

use crate::error::FirebaseError;
use crate::firestore::types::{DocumentReference, DocumentSnapshot, SnapshotMetadata};
use futures::stream::StreamExt;
use serde_json::json;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Channel;
use tonic::Request;

/// Handle for removing a snapshot listener
///
/// Based on C++ ListenerRegistration pattern
pub struct ListenerRegistration {
    cancel_tx: mpsc::Sender<()>,
}

impl ListenerRegistration {
    /// Removes the listener and stops receiving updates
    pub async fn remove(self) {
        let _ = self.cancel_tx.send(()).await;
    }
}

/// Options for configuring snapshot listeners
#[derive(Debug, Clone, Default)]
pub struct ListenerOptions {
    /// Include metadata-only changes (like hasPendingWrites transitions)
    pub include_metadata_changes: bool,
}

/// Internal function to create gRPC channel with authentication
#[cfg(not(target_arch = "wasm32"))]
async fn create_authenticated_channel(_auth_token: &str) -> Result<Channel, FirebaseError> {
    // Configure TLS with webpki root certificates (similar to C++ SDK's LoadGrpcRootCertificate)
    let tls_config = tonic::transport::ClientTlsConfig::new()
        .with_webpki_roots()
        .domain_name("firestore.googleapis.com");

    // Connect to Firestore gRPC endpoint with TLS
    let channel_builder = Channel::from_static("https://firestore.googleapis.com");
    let channel_builder = match channel_builder.tls_config(tls_config) {
        Err(e) => {
            return Err(FirebaseError::internal(format!(
                "Failed to configure TLS: {}",
                e
            )))
        }
        Ok(builder) => builder,
    };

    let channel_builder = channel_builder
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10));

    let channel = match channel_builder.connect().await {
        Err(e) => {
            eprintln!("gRPC connection error details: {:?}", e);
            return Err(FirebaseError::internal(format!(
                "Failed to connect to Firestore gRPC: {}",
                e
            )));
        }
        Ok(ch) => ch,
    };

    Ok(channel)
}

/// Adds a real-time listener to a Firestore document using gRPC streaming
///
/// This implements the C++ WatchStream pattern:
/// 1. Creates bidirectional gRPC stream to /google.firestore.v1.Firestore/Listen
/// 2. Sends ListenRequest with document target
/// 3. Receives ListenResponse stream with changes
/// 4. Calls callback on each change
/// 5. Returns ListenerRegistration for cleanup
///
/// # Arguments
/// * `auth_token` - Bearer token for authentication
/// * `project_id` - Firebase project ID
/// * `database_id` - Firestore database ID (usually "default")
/// * `document_path` - Full document path (e.g., "users/123")
/// * `options` - Listener configuration options
/// * `callback` - Function called with document snapshots or errors
///
/// # Returns
/// `ListenerRegistration` handle to remove the listener
#[cfg(not(target_arch = "wasm32"))]
pub async fn add_document_listener<F>(
    auth_token: String,
    project_id: String,
    database_id: String,
    document_path: String,
    options: ListenerOptions,
    mut callback: F,
) -> Result<ListenerRegistration, FirebaseError>
where
    F: FnMut(Result<DocumentSnapshot, FirebaseError>) + Send + 'static,
{
    // Create gRPC channel
    let channel = create_authenticated_channel(&auth_token).await?;

    // Create client with authentication interceptor
    // Mirrors C++ GrpcConnection::CreateContext which adds authorization and x-goog-request-params
    let auth_token_clone = auth_token.clone();
    let project_id_clone = project_id.clone();
    let database_id_clone = database_id.clone();
    let mut client = FirestoreClient::with_interceptor(channel, move |mut req: Request<()>| {
        // Add Bearer token for authentication
        let token = format!("Bearer {}", auth_token_clone);
        let Ok(val) = token.parse() else {
            return Err(tonic::Status::unauthenticated("Invalid token"));
        };
        req.metadata_mut().insert("authorization", val);

        // Add required routing header (mirrors C++ kXGoogRequestParams)
        // Format: "projects/{project_id}/databases/{database_id}"
        let resource_prefix = format!(
            "projects/{}/databases/{}",
            project_id_clone, database_id_clone
        );
        let Ok(val) = resource_prefix.parse() else {
            return Err(tonic::Status::invalid_argument("Invalid resource prefix"));
        };
        req.metadata_mut().insert("x-goog-request-params", val);

        Ok(req)
    });

    // Build database path
    let database = format!("projects/{}/databases/{}", project_id, database_id);

    // Build document target
    let documents = vec![format!("{}/documents/{}", database, document_path)];

    // Create target for watching this document
    // Mirrors C++ WatchStream::WatchQuery implementation
    let target = Target {
        target_id: 1,
        once: false,
        expected_count: None,
        target_type: Some(firestore_proto::target::TargetType::Documents(
            firestore_proto::target::DocumentsTarget { documents },
        )),
        resume_type: None,
    };

    // Create listen request
    // Mirrors C++ WatchStreamSerializer::EncodeWatchRequest
    let request = ListenRequest {
        database: database.clone(),
        labels: HashMap::new(),
        target_change: Some(firestore_proto::listen_request::TargetChange::AddTarget(
            target,
        )),
    };

    // Start bidirectional streaming
    // Mirrors C++ GrpcConnection::CreateStream + Stream::Start
    // Note: We use a channel to send requests and keep the stream open
    let (request_sender, request_receiver) = mpsc::channel(10);

    // Send the initial listen request
    if let Err(e) = request_sender.send(request).await {
        return Err(FirebaseError::internal(format!(
            "Failed to send listen request: {}",
            e
        )));
    }

    let response = client.listen(ReceiverStream::new(request_receiver)).await;
    let response_stream = match response {
        Err(e) => {
            return Err(FirebaseError::internal(format!(
                "Failed to start listener: {}",
                e
            )))
        }
        Ok(stream) => stream.into_inner(),
    };

    // Create cancellation channel
    let (cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);

    // Spawn task to process stream
    // Mirrors C++ Stream::OnStreamRead processing
    tokio::spawn(async move {
        let mut stream = response_stream;
        let _request_sender = request_sender; // Keep sender alive to maintain bidirectional stream

        loop {
            tokio::select! {
                _ = cancel_rx.recv() => {
                    // Listener cancelled - mirrors C++ ListenerRegistration::Remove()
                    break;
                }
                message = stream.next() => {


                    let Some(msg) = message else {
                        // Stream ended normally
                        break;
                    };

                    match msg {
                        Err(e) => {
                            // Stream error - mirrors C++ Stream::OnStreamFinish
                            callback(Err(FirebaseError::internal(format!("Stream error: {}", e))));
                            break;
                        }
                        Ok(response) => {
                            // Process the listen response
                            // Mirrors C++ WatchStream::NotifyStreamResponse
                            let snapshot = match process_listen_response(response, &options, &project_id, &database_id) {
                                Err(e) => {
                                    callback(Err(e));
                                    break;
                                }
                                Ok(result) => result,
                            };

                            let Some(snapshot) = snapshot else {
                                // Metadata-only change or filtered out
                                continue;
                            };

                            callback(Ok(snapshot));
                            // Continue listening for more updates
                        }
                    }
                }
            }
        }
    });

    Ok(ListenerRegistration { cancel_tx })
}

/// Process a ListenResponse and convert to DocumentSnapshot
///
/// Mirrors C++ WatchStream::NotifyStreamResponse and WatchStreamSerializer::DecodeWatchChange
fn process_listen_response(
    response: firestore_proto::ListenResponse,
    _options: &ListenerOptions,
    _project_id: &str,
    _database_id: &str,
) -> Result<Option<DocumentSnapshot>, FirebaseError> {
    match response.response_type {
        Some(ResponseType::DocumentChange(change)) => {
            // Document was added or modified
            let Some(doc) = change.document else {
                return Ok(None);
            };

            // Convert protobuf document to DocumentSnapshot
            // Mirrors C++ conversion in document_reference_main.cc
            let path = doc.name.clone();
            let data = convert_proto_fields_to_json(&doc.fields)?;

            // Extract just the document path (remove the database prefix)
            // Path format: "projects/{project}/databases/{database}/documents/{doc_path}"
            let doc_path = path
                .split("/documents/")
                .nth(1)
                .map(|p| p.to_string())
                .unwrap_or(path);

            // Create DocumentSnapshot with simplified DocumentReference
            // The DocumentReference only needs the path
            Ok(Some(DocumentSnapshot {
                reference: DocumentReference::new(doc_path),
                data: Some(data),
                // TODO: Compute actual metadata from ListenResponse
                // has_pending_writes: should check if document has local uncommitted writes
                // is_from_cache: should be true if this is initial data from cache, false for server updates
                // See C++ WatchStream::OnDocumentChange and DocumentSnapshot::FromDocument
                metadata: SnapshotMetadata {
                    has_pending_writes: false, // PLACEHOLDER: should check pending writes
                    is_from_cache: false,      // PLACEHOLDER: should check if from cache or server
                },
            }))
        }
        Some(ResponseType::DocumentDelete(delete)) => {
            // Document was deleted - return snapshot with no data
            // Extract document path from the delete event
            let doc_path = delete.document
                .split("/documents/")
                .nth(1)
                .map(|p| p.to_string())
                .unwrap_or_else(|| delete.document.clone());
            
            Ok(Some(DocumentSnapshot {
                reference: DocumentReference::new(doc_path),
                data: None, // None indicates document was deleted
                // TODO: Compute actual metadata - has_pending_writes should check local write cache
                // is_from_cache should be true if this is initial data from cache, false for server updates
                // See C++ WatchStream::OnDocumentChange and DocumentSnapshot::FromDocument
                metadata: SnapshotMetadata {
                    has_pending_writes: false, // PLACEHOLDER: should check pending writes
                    is_from_cache: false,      // PLACEHOLDER: should check if from cache or server
                },
            }))
        }
        Some(ResponseType::DocumentRemove(_remove)) => {
            // Document removed from query result set (different from deletion)
            // This happens when a document no longer matches query filters
            // For single document listeners, we filter this out
            // For query listeners, this would update the result set
            Ok(None) // Filter out - not relevant for single document listener
        }
        Some(ResponseType::Filter(filter)) => {
            // Existence filter validates the number of documents in the watch stream
            // If count doesn't match, it indicates the client's view is inconsistent
            // and the watch stream should be reset
            // For now, we log and continue - a full implementation would reset the stream
            if filter.count == 0 {
                // No documents match - this is informational
                // Could return metadata-only snapshot if include_metadata_changes
            }
            // Filter events don't produce snapshots, they're for stream validation
            Ok(None)
        }
        Some(ResponseType::TargetChange(change)) => {
            // Target state changes indicate the watch stream state:
            // - NO_CHANGE (0): No change, initial state
            // - ADD (1): Target was added
            // - REMOVE (2): Target was removed
            // - CURRENT (3): All initial data has been sent
            // - RESET (4): Target was reset (need to refetch)
            
            // Check for errors in the target change
            if let Some(cause) = change.cause {
                let error_msg = format!(
                    "Target change error: code={}, message={}",
                    cause.code,
                    cause.message
                );
                return Err(FirebaseError::internal(error_msg));
            }
            
            // For document listeners, these are informational
            // The actual data comes through DocumentChange events
            // If include_metadata_changes is true, could return metadata-only snapshot
            Ok(None)
        }
        None => Ok(None),
    }
}

/// Convert protobuf fields map to serde_json::Value
///
/// Mirrors C++ Serializer field conversion
fn convert_proto_fields_to_json(
    fields: &HashMap<String, firestore_proto::Value>,
) -> Result<serde_json::Value, FirebaseError> {
    let mut map = serde_json::Map::new();

    for (key, value) in fields {
        let json_value = convert_proto_value_to_json(value)?;
        map.insert(key.clone(), json_value);
    }

    Ok(serde_json::Value::Object(map))
}

/// Convert a single protobuf Value to serde_json::Value
///
/// Mirrors C++ Serializer::DecodeFieldValue
fn convert_proto_value_to_json(
    value: &firestore_proto::Value,
) -> Result<serde_json::Value, FirebaseError> {
    match &value.value_type {
        Some(ValueType::NullValue(_)) => Ok(serde_json::Value::Null),
        Some(ValueType::BooleanValue(b)) => Ok(serde_json::Value::Bool(*b)),
        Some(ValueType::IntegerValue(i)) => Ok(json!(i)),
        Some(ValueType::DoubleValue(d)) => Ok(json!(d)),
        Some(ValueType::StringValue(s)) => Ok(serde_json::Value::String(s.clone())),
        Some(ValueType::ArrayValue(arr)) => {
            let mut json_arr = Vec::new();
            for item in &arr.values {
                json_arr.push(convert_proto_value_to_json(item)?);
            }
            Ok(serde_json::Value::Array(json_arr))
        }
        Some(ValueType::MapValue(map)) => convert_proto_fields_to_json(&map.fields),
        Some(ValueType::TimestampValue(ts)) => {
            let Some(dt) = chrono::DateTime::from_timestamp(ts.seconds, ts.nanos as u32) else {
                return Err(FirebaseError::internal("Invalid timestamp"));
            };
            Ok(serde_json::Value::String(dt.to_rfc3339()))
        }
        Some(ValueType::GeoPointValue(geo)) => Ok(serde_json::json!({
            "latitude": geo.latitude,
            "longitude": geo.longitude,
        })),
        Some(ValueType::ReferenceValue(r)) => Ok(serde_json::Value::String(r.clone())),
        Some(ValueType::BytesValue(b)) => {
            // Encode as hex string
            Ok(serde_json::Value::String(
                b.iter().map(|byte| format!("{:02x}", byte)).collect(),
            ))
        }
        // New value types added in recent Firestore versions
        Some(ValueType::FieldReferenceValue(field_ref)) => {
            Ok(serde_json::Value::String(field_ref.clone()))
        }
        Some(ValueType::FunctionValue(_func)) => {
            // Function values are not directly serializable
            // TODO: Decide if we should error or return a special marker
            todo!("Implement function value serialization or return error")
        }
        Some(ValueType::PipelineValue(_pipeline)) => {
            // Pipeline values are not directly serializable
            // TODO: Decide if we should error or return a special marker
            todo!("Implement pipeline value serialization or return error")
        }
        None => Ok(serde_json::Value::Null),
    }
}

// WASM support will be added using tonic-web-wasm-client
#[cfg(target_arch = "wasm32")]
pub async fn add_document_listener<F>(
    _auth_token: String,
    _project_id: String,
    _database_id: String,
    _document_path: String,
    _options: ListenerOptions,
    _callback: F,
) -> Result<ListenerRegistration, FirebaseError>
where
    F: FnMut(Result<DocumentSnapshot, FirebaseError>) + Send + 'static,
{
    // TODO: Implement using tonic-web-wasm-client
    Err(FirebaseError::internal(
        "WASM listener support not yet implemented",
    ))
}

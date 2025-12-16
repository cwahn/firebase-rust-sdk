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
use proto::google::firestore::v1 as firestore_proto;

use crate::error::FirebaseError;
use crate::firestore::types::DocumentSnapshot;
use futures::stream::StreamExt;
use tokio::sync::mpsc;
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
async fn create_authenticated_channel(
    _auth_token: &str,
) -> Result<Channel, FirebaseError> {
    // Configure TLS with webpki root certificates (similar to C++ SDK's LoadGrpcRootCertificate)
    let tls_config = tonic::transport::ClientTlsConfig::new()
        .with_webpki_roots()
        .domain_name("firestore.googleapis.com");
    
    // Connect to Firestore gRPC endpoint with TLS
    let channel = Channel::from_static("https://firestore.googleapis.com")
        .tls_config(tls_config)
        .map_err(|e| FirebaseError::internal(format!("Failed to configure TLS: {}", e)))?
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10))
        .connect()
        .await
        .map_err(|e| {
            eprintln!("gRPC connection error details: {:?}", e);
            FirebaseError::internal(format!("Failed to connect to Firestore gRPC: {}", e))
        })?;
    
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
    use firestore_proto::firestore_client::FirestoreClient;
    use firestore_proto::{ListenRequest, Target};
    
    // Create gRPC channel
    let channel = create_authenticated_channel(&auth_token).await?;
    
    // Create client with authentication interceptor
    // Mirrors C++ GrpcConnection::CreateContext which adds authorization and x-goog-request-params
    let auth_token_clone = auth_token.clone();
    let project_id_clone = project_id.clone();
    let database_id_clone = database_id.clone();
    let mut client = FirestoreClient::with_interceptor(
        channel,
        move |mut req: Request<()>| {
            // Add Bearer token for authentication
            let token = format!("Bearer {}", auth_token_clone);
            match token.parse() {
                Ok(val) => {
                    req.metadata_mut().insert("authorization", val);
                }
                Err(_) => return Err(tonic::Status::unauthenticated("Invalid token")),
            }
            
            // Add required routing header (mirrors C++ kXGoogRequestParams)
            // Format: "projects/{project_id}/databases/{database_id}"
            let resource_prefix = format!("projects/{}/databases/{}", project_id_clone, database_id_clone);
            match resource_prefix.parse() {
                Ok(val) => {
                    req.metadata_mut().insert("x-goog-request-params", val);
                }
                Err(_) => return Err(tonic::Status::invalid_argument("Invalid resource prefix")),
            }
            
            Ok(req)
        },
    );
    
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
        labels: std::collections::HashMap::new(),
        target_change: Some(firestore_proto::listen_request::TargetChange::AddTarget(target)),
    };
    
    // Start bidirectional streaming
    // Mirrors C++ GrpcConnection::CreateStream + Stream::Start
    // Note: We use a channel to send requests and keep the stream open
    let (mut request_sender, request_receiver) = mpsc::channel(10);
    
    // Send the initial listen request
    request_sender
        .send(request)
        .await
        .map_err(|e| FirebaseError::internal(format!("Failed to send listen request: {}", e)))?;
    
    let response_stream = client
        .listen(tokio_stream::wrappers::ReceiverStream::new(request_receiver))
        .await
        .map_err(|e| FirebaseError::internal(format!("Failed to start listener: {}", e)))?
        .into_inner();
    
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
                    match message {
                        Some(Ok(response)) => {
                            // Process the listen response
                            // Mirrors C++ WatchStream::NotifyStreamResponse
                            match process_listen_response(response, &options, &project_id, &database_id) {
                                Ok(Some(snapshot)) => {
                                    callback(Ok(snapshot));
                                }
                                Ok(None) => {
                                    // Metadata-only change or filtered out
                                    continue;
                                }
                                Err(e) => {
                                    callback(Err(e));
                                    break;
                                }
                            }
                        }
                        Some(Err(e)) => {
                            // Stream error - mirrors C++ Stream::OnStreamFinish
                            callback(Err(FirebaseError::internal(format!("Stream error: {}", e))));
                            break;
                        }
                        None => {
                            // Stream ended normally
                            break;
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
    options: &ListenerOptions,
    _project_id: &str,
    _database_id: &str,
) -> Result<Option<DocumentSnapshot>, FirebaseError> {
    use firestore_proto::listen_response::ResponseType;
    
    match response.response_type {
        Some(ResponseType::DocumentChange(change)) => {
            // Document was added or modified
            match change.document {
                Some(doc) => {
                    // Convert protobuf document to DocumentSnapshot
                    // Mirrors C++ conversion in document_reference_main.cc
                    let path = doc.name.clone();
                    let data = convert_proto_fields_to_json(&doc.fields)?;
                    
                    // Extract just the document path (remove the database prefix)
                    // Path format: "projects/{project}/databases/{database}/documents/{doc_path}"
                    let doc_path = path
                        .split("/documents/")
                        .nth(1)
                        .unwrap_or(&path)
                        .to_string();
                    
                    // Create DocumentSnapshot with simplified DocumentReference
                    // The DocumentReference only needs the path
                    Ok(Some(DocumentSnapshot {
                        reference: crate::firestore::types::DocumentReference::new(doc_path),
                        data: Some(data),
                        metadata: crate::firestore::types::SnapshotMetadata {
                            has_pending_writes: false,
                            is_from_cache: false,
                        },
                    }))
                }
                None => Ok(None),
            }
        }
        Some(ResponseType::DocumentDelete(_delete)) => {
            // Document was deleted
            // For now, return None to indicate deletion
            Ok(None)
        }
        Some(ResponseType::DocumentRemove(_remove)) => {
            // Document removed from query (not applicable for single document)
            Ok(None)
        }
        Some(ResponseType::Filter(_filter)) => {
            // Existence filter - metadata only
            if options.include_metadata_changes {
                Ok(None) // Could return metadata-only snapshot
            } else {
                Ok(None)
            }
        }
        Some(ResponseType::TargetChange(_change)) => {
            // Target state changed - metadata only
            if options.include_metadata_changes {
                Ok(None) // Could return metadata-only snapshot
            } else {
                Ok(None)
            }
        }
        None => Ok(None),
    }
}

/// Convert protobuf fields map to serde_json::Value
///
/// Mirrors C++ Serializer field conversion
fn convert_proto_fields_to_json(
    fields: &std::collections::HashMap<String, firestore_proto::Value>,
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
    use firestore_proto::value::ValueType;
    
    match &value.value_type {
        Some(ValueType::NullValue(_)) => Ok(serde_json::Value::Null),
        Some(ValueType::BooleanValue(b)) => Ok(serde_json::Value::Bool(*b)),
        Some(ValueType::IntegerValue(i)) => Ok(serde_json::json!(i)),
        Some(ValueType::DoubleValue(d)) => Ok(serde_json::json!(d)),
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
            match chrono::DateTime::from_timestamp(ts.seconds, ts.nanos as u32) {
                Some(dt) => Ok(serde_json::Value::String(dt.to_rfc3339())),
                None => Err(FirebaseError::internal("Invalid timestamp")),
            }
        }
        Some(ValueType::GeoPointValue(geo)) => {
            Ok(serde_json::json!({
                "latitude": geo.latitude,
                "longitude": geo.longitude,
            }))
        }
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
            Ok(serde_json::Value::Null)
        }
        Some(ValueType::PipelineValue(_pipeline)) => {
            // Pipeline values are not directly serializable
            Ok(serde_json::Value::Null)
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

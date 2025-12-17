// Firestore Snapshot Listener Implementation using gRPC Streaming
//
// Based on C++ Firebase SDK's WatchStream architecture:
// - Uses bidirectional gRPC streaming via /google.firestore.v1.Firestore/Listen
// - Sends ListenRequest with target (document or query)
// - Receives ListenResponse stream with document changes
// - Returns ListenerRegistration handle for cleanup

#![allow(missing_docs)]
#![allow(clippy::all)]

use crate::error::FirebaseError;
use crate::firestore::firestore::FirestoreInner;
use crate::firestore::{proto, DocumentReference, DocumentSnapshot, SnapshotMetadata};
use crate::firestore::Firestore;
use firestore_proto::firestore_client::FirestoreClient;
use firestore_proto::listen_response::ResponseType;
use firestore_proto::{ListenRequest, Target};
use futures::stream::{Stream, StreamExt};
use proto::google::firestore::v1 as firestore_proto;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Channel;
use tonic::Request;

/// Stream of document snapshots from Firestore listener
///
/// This stream yields `Result<DocumentSnapshot, FirebaseError>` items as the document changes.
/// The stream will continue until dropped or an error occurs.
pub type DocumentSnapshotStream =
    Pin<Box<dyn Stream<Item = Result<DocumentSnapshot, FirebaseError>> + Send>>;

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

/// Creates a real-time listener stream for a Firestore document using gRPC streaming
///
/// This implements the C++ WatchStream pattern but returns a Rust Stream for idiomatic async iteration.
///
/// # Example
/// ```rust,no_run
/// use futures::stream::StreamExt;
///
/// let mut stream = firestore.listen_document(
///     auth_token,
///     project_id,
///     database_id,
///     "users/123",
///     ListenerOptions::default()
/// ).await?;
///
/// while let Some(result) = stream.next().await {
///     match result {
///         Ok(snapshot) => println!("Document updated: {:?}", snapshot),
///         Err(e) => eprintln!("Listener error: {}", e),
///     }
/// }
/// ```
///
/// # Arguments
/// * `firestore` - Firestore instance for creating DocumentReferences
/// * `auth_token` - Bearer token for authentication
/// * `project_id` - Firebase project ID
/// * `database_id` - Firestore database ID (usually "default")
/// * `document_path` - Full document path (e.g., "users/123")
/// * `options` - Listener configuration options
///
/// # Returns
/// Stream of `Result<DocumentSnapshot, FirebaseError>` that yields updates as they occur
#[cfg(not(target_arch = "wasm32"))]
pub async fn listen_document(
    firestore: &Firestore,
    auth_token: String,
    project_id: String,
    database_id: String,
    document_path: String,
    options: ListenerOptions,
) -> Result<DocumentSnapshotStream, FirebaseError> {
    let firestore_inner = Arc::clone(&firestore.inner);
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

    // Create channel for streaming snapshots to caller
    let (snapshot_tx, snapshot_rx) = mpsc::channel::<Result<DocumentSnapshot, FirebaseError>>(100);

    // Spawn task to process gRPC stream and forward to snapshot channel
    tokio::spawn(async move {
        let mut stream = response_stream;
        let _request_sender = request_sender; // Keep sender alive to maintain bidirectional stream

        loop {
            let message = stream.next().await;

            let Some(msg) = message else {
                // Stream ended normally
                break;
            };

            match msg {
                Err(e) => {
                    // Stream error - send error and stop
                    let _ = snapshot_tx
                        .send(Err(FirebaseError::internal(format!("Stream error: {}", e))))
                        .await;
                    break;
                }
                Ok(response) => {
                    // Process the listen response
                    let snapshot = match process_listen_response(
                        response,
                        &options,
                        &firestore_inner,
                        &document_path,
                    ) {
                        Err(e) => {
                            let _ = snapshot_tx.send(Err(e)).await;
                            break;
                        }
                        Ok(result) => result,
                    };

                    let Some(snapshot) = snapshot else {
                        // Metadata-only change or filtered out
                        continue;
                    };

                    // Send snapshot to caller
                    if snapshot_tx.send(Ok(snapshot)).await.is_err() {
                        // Receiver dropped - stop listening
                        break;
                    }
                }
            }
        }
    });

    // Return stream that yields snapshots from the channel
    Ok(Box::pin(ReceiverStream::new(snapshot_rx)))
}

/// Process a ListenResponse and convert to DocumentSnapshot
///
/// Mirrors C++ WatchStream::NotifyStreamResponse and WatchStreamSerializer::DecodeWatchChange
fn process_listen_response(
    response: firestore_proto::ListenResponse,
    _options: &ListenerOptions,
    firestore_inner: &Arc<FirestoreInner>,
    document_path: &str,
) -> Result<Option<DocumentSnapshot>, FirebaseError> {
    use crate::firestore::MapValue;

    match response.response_type {
        Some(ResponseType::DocumentChange(change)) => {
            // Document was added or modified
            let Some(doc) = change.document else {
                return Ok(None);
            };

            // Convert protobuf document to DocumentSnapshot
            // Mirrors C++ conversion in document_reference_main.cc
            let fields = doc.fields; // HashMap<String, Value>

            // Create DocumentReference for this document
            let doc_ref =
                DocumentReference::new(document_path.to_string(), Arc::clone(firestore_inner));

            // Create DocumentSnapshot with MapValue containing the fields
            let snapshot = DocumentSnapshot {
                reference: doc_ref,
                data: Some(MapValue { fields }),
                metadata: SnapshotMetadata {
                    has_pending_writes: false,
                    is_from_cache: false,
                },
            };

            Ok(Some(snapshot))
        }
        Some(ResponseType::DocumentDelete(_delete)) => {
            // Document was deleted - return snapshot with no data
            let doc_ref =
                DocumentReference::new(document_path.to_string(), Arc::clone(firestore_inner));

            // Return a snapshot indicating the document was deleted
            let snapshot = DocumentSnapshot {
                reference: doc_ref,
                data: None,
                metadata: SnapshotMetadata {
                    has_pending_writes: false,
                    is_from_cache: false,
                },
            };

            Ok(Some(snapshot))
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
                    cause.code, cause.message
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

/// Stream of query snapshots from Firestore listener
pub type QuerySnapshotStream =
    Pin<Box<dyn Stream<Item = Result<crate::firestore::QuerySnapshot, FirebaseError>> + Send>>;

/// Creates a real-time listener stream for a Firestore query using gRPC streaming
///
/// Similar to listen_document but tracks all documents matching the query.
/// Accumulates changes and builds QuerySnapshot with document change tracking.
///
/// # Arguments
/// * `firestore` - Firestore instance
/// * `auth_token` - Bearer token for authentication
/// * `project_id` - Firebase project ID
/// * `database_id` - Firestore database ID
/// * `query_state` - Query filters, orders, limits
/// * `options` - Listener configuration options
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn listen_query(
    firestore: &Firestore,
    auth_token: String,
    project_id: String,
    database_id: String,
    query_state: crate::firestore::query::QueryState,
    options: ListenerOptions,
) -> Result<QuerySnapshotStream, FirebaseError> {
    use crate::firestore::QuerySnapshot;
    use std::collections::HashMap;

    let firestore_inner = Arc::clone(&firestore.inner);
    let channel = create_authenticated_channel(&auth_token).await?;

    let auth_token_clone = auth_token.clone();
    let project_id_clone = project_id.clone();
    let database_id_clone = database_id.clone();
    let mut client = FirestoreClient::with_interceptor(channel, move |mut req: Request<()>| {
        let token = format!("Bearer {}", auth_token_clone);
        let Ok(val) = token.parse() else {
            return Err(tonic::Status::unauthenticated("Invalid token"));
        };
        req.metadata_mut().insert("authorization", val);

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

    let database = format!("projects/{}/databases/{}", project_id, database_id);
    let parent = format!("{}/documents", database);

    // Convert QueryState to gRPC StructuredQuery
    let structured_query = query_state_to_structured_query(&query_state);

    // Create query target
    let target = Target {
        target_id: 1,
        once: false,
        expected_count: None,
        target_type: Some(firestore_proto::target::TargetType::Query(
            firestore_proto::target::QueryTarget {
                parent,
                query_type: Some(
                    firestore_proto::target::query_target::QueryType::StructuredQuery(
                        structured_query,
                    ),
                ),
            },
        )),
        resume_type: None,
    };

    let request = ListenRequest {
        database: database.clone(),
        labels: HashMap::new(),
        target_change: Some(firestore_proto::listen_request::TargetChange::AddTarget(
            target,
        )),
    };

    let (request_sender, request_receiver) = mpsc::channel(10);
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

    let (snapshot_tx, snapshot_rx) = mpsc::channel::<Result<QuerySnapshot, FirebaseError>>(100);

    tokio::spawn(async move {
        let mut stream = response_stream;
        let _request_sender = request_sender;

        // Accumulate documents
        let mut documents: HashMap<String, proto::google::firestore::v1::Document> = HashMap::new();
        let mut is_initial_snapshot = true;

        loop {
            let message = stream.next().await;

            let Some(msg) = message else {
                break;
            };

            match msg {
                Err(e) => {
                    let _ = snapshot_tx
                        .send(Err(FirebaseError::internal(format!("Stream error: {}", e))))
                        .await;
                    break;
                }
                Ok(response) => {
                    let should_emit = process_query_listen_response(
                        response,
                        &options,
                        &mut documents,
                        &mut is_initial_snapshot,
                    );

                    if should_emit {
                        // Build QuerySnapshot from current documents
                        let snapshot = QuerySnapshot {
                            documents: documents.values().cloned().collect(),
                            firestore: Arc::clone(&firestore_inner),
                        };

                        if snapshot_tx.send(Ok(snapshot)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    });

    Ok(Box::pin(ReceiverStream::new(snapshot_rx)))
}

/// Convert QueryState to gRPC StructuredQuery
fn query_state_to_structured_query(
    state: &crate::firestore::query::QueryState,
) -> firestore_proto::StructuredQuery {
    use proto::google::firestore::v1 as firestore_proto;

    // Extract collection_id from collection_path
    // collection_path format: "projects/{project_id}/databases/{database_id}/documents/{collection}/{docId}..."
    let collection_id = state
        .collection_path
        .split('/')
        .last()
        .unwrap_or("documents")
        .to_string();

    let mut query = firestore_proto::StructuredQuery {
        select: None, // Return all fields
        from: vec![firestore_proto::structured_query::CollectionSelector {
            collection_id,
            all_descendants: false,
        }],
        r#where: None,
        order_by: Vec::new(),
        start_at: None,
        end_at: None,
        offset: 0,
        limit: None,
        find_nearest: None,
    };

    // Add filters
    if !state.filters.is_empty() {
        if state.filters.len() == 1 {
            let (field, op, value) = &state.filters[0];
            query.r#where = Some(create_field_filter(field, op, value));
        } else {
            // Multiple filters wrapped in AND
            let filters: Vec<_> = state
                .filters
                .iter()
                .map(|(field, op, value)| create_field_filter(field, op, value))
                .collect();

            query.r#where = Some(firestore_proto::structured_query::Filter {
                filter_type: Some(
                    firestore_proto::structured_query::filter::FilterType::CompositeFilter(
                        firestore_proto::structured_query::CompositeFilter {
                            op: firestore_proto::structured_query::composite_filter::Operator::And
                                as i32,
                            filters,
                        },
                    ),
                ),
            });
        }
    }

    // Add ordering
    for (field, direction) in &state.orders {
        query
            .order_by
            .push(firestore_proto::structured_query::Order {
                field: Some(firestore_proto::structured_query::FieldReference {
                    field_path: field.clone(),
                }),
                direction: match direction {
                    crate::firestore::Direction::Ascending => {
                        firestore_proto::structured_query::Direction::Ascending as i32
                    }
                    crate::firestore::Direction::Descending => {
                        firestore_proto::structured_query::Direction::Descending as i32
                    }
                    crate::firestore::Direction::Unspecified => {
                        firestore_proto::structured_query::Direction::Ascending as i32
                    }
                },
            });
    }

    // Add limit
    if let Some(limit) = state.limit_value {
        query.limit = Some(limit);
    }

    query
}

/// Create a field filter from query state format
fn create_field_filter(
    field: &str,
    op: &proto::google::firestore::v1::structured_query::field_filter::Operator,
    value: &proto::google::firestore::v1::Value,
) -> proto::google::firestore::v1::structured_query::Filter {
    use proto::google::firestore::v1 as firestore_proto;

    firestore_proto::structured_query::Filter {
        filter_type: Some(
            firestore_proto::structured_query::filter::FilterType::FieldFilter(
                firestore_proto::structured_query::FieldFilter {
                    field: Some(firestore_proto::structured_query::FieldReference {
                        field_path: field.to_string(),
                    }),
                    op: *op as i32,
                    value: Some(value.clone()),
                },
            ),
        ),
    }
}

/// Process ListenResponse for query listening
fn process_query_listen_response(
    response: firestore_proto::ListenResponse,
    _options: &ListenerOptions,
    documents: &mut HashMap<String, proto::google::firestore::v1::Document>,
    is_initial: &mut bool,
) -> bool {
    use ResponseType::*;

    match response.response_type {
        Some(DocumentChange(change)) => {
            if let Some(doc) = change.document {
                let doc_name = doc.name.clone();
                documents.insert(doc_name, doc);
            }
            // After initial snapshot, emit on every document change
            !*is_initial
        }
        Some(DocumentDelete(delete)) => {
            documents.remove(&delete.document);
            // After initial snapshot, emit on every document deletion
            !*is_initial
        }
        Some(DocumentRemove(remove)) => {
            documents.remove(&remove.document);
            // After initial snapshot, emit on every document removal
            !*is_initial
        }
        Some(TargetChange(change)) => {
            // Check if this is CURRENT state (all initial data received)
            if change.target_change_type
                == firestore_proto::target_change::TargetChangeType::Current as i32
            {
                if *is_initial {
                    *is_initial = false;
                    return true; // Emit initial snapshot
                }
            }

            // Check for errors
            if let Some(cause) = change.cause {
                eprintln!(
                    "Target change error: code={}, message={}",
                    cause.code, cause.message
                );
            }
            false
        }
        Some(Filter(_)) => false,
        None => false,
    }
}

// WASM support will be added using tonic-web-wasm-client
#[cfg(target_arch = "wasm32")]
pub async fn listen_document(
    _firestore: &Firestore,
    _auth_token: String,
    _project_id: String,
    _database_id: String,
    _document_path: String,
    _options: ListenerOptions,
) -> Result<DocumentSnapshotStream, FirebaseError> {
    Err(FirebaseError::internal(
        "WASM listener support not yet implemented",
    ))
}

#[cfg(target_arch = "wasm32")]
pub async fn listen_query(
    _firestore: &Firestore,
    _auth_token: String,
    _project_id: String,
    _database_id: String,
    _query_state: crate::firestore::query::QueryState,
    _options: ListenerOptions,
) -> Result<QuerySnapshotStream, FirebaseError> {
    Err(FirebaseError::internal(
        "WASM listener support not yet implemented",
    ))
}

/// Snapshot listener streams for Firestore.
///
/// This module provides async streams for real-time document and query updates.
/// The streams automatically handle cleanup when dropped.
///
/// # C++ Equivalent Pattern
/// C++ uses callbacks with `ListenerRegistration::Remove()`:
/// - `listener_main.h:69` - `ListenerWithCallback` template
/// - `document_reference.h:265` - `AddSnapshotListener` returns `ListenerRegistration`
///
/// Rust uses async streams that clean up on drop (RAII pattern):
/// - Stream implements `Drop` to cancel the listener
/// - No explicit `remove()` call needed
///
/// # Example
/// ```no_run
/// use firebase_rust_sdk::firestore::Firestore;
/// use futures::StreamExt;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let firestore = Firestore::new("my-project", "(default)", None).await?;
/// let doc_ref = firestore.collection("cities").document("SF");
///
/// let mut stream = doc_ref.listen(None);
/// while let Some(result) = stream.next().await {
///     match result {
///         Ok(snapshot) => println!("Document: {:?}", snapshot.id()),
///         Err(e) => eprintln!("Error: {}", e),
///     }
/// }
/// // Stream automatically cleaned up on drop
/// # Ok(())
/// # }
/// ```

use crate::error::FirebaseError;
use crate::firestore::document_snapshot::DocumentSnapshot;
use crate::firestore::query_snapshot::QuerySnapshot;
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::{mpsc, oneshot};

/// A stream of document snapshot updates.
///
/// This stream receives real-time updates for a single document.
/// When dropped, the underlying listener is automatically canceled.
///
/// # C++ Equivalent
/// C++ callback: `std::function<void(const DocumentSnapshot&, Error, const std::string&)>`
/// Reference: `document_reference.h:265`
pub struct DocumentSnapshotStream {
    receiver: mpsc::UnboundedReceiver<Result<DocumentSnapshot, FirebaseError>>,
    cancel_tx: Option<oneshot::Sender<()>>,
}

impl DocumentSnapshotStream {
    pub(crate) fn new(
        receiver: mpsc::UnboundedReceiver<Result<DocumentSnapshot, FirebaseError>>,
        cancel_tx: oneshot::Sender<()>,
    ) -> Self {
        Self {
            receiver,
            cancel_tx: Some(cancel_tx),
        }
    }
}

impl Stream for DocumentSnapshotStream {
    type Item = Result<DocumentSnapshot, FirebaseError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}

impl Drop for DocumentSnapshotStream {
    fn drop(&mut self) {
        if let Some(tx) = self.cancel_tx.take() {
            // Send cancellation signal - ignore error if receiver already dropped
            let _ = tx.send(());
        }
    }
}

/// A stream of query snapshot updates.
///
/// This stream receives real-time updates for a query result set.
/// When dropped, the underlying listener is automatically canceled.
///
/// # C++ Equivalent
/// C++ callback: `std::function<void(const QuerySnapshot&, Error, const std::string&)>`
/// Reference: `query.h:634`
pub struct QuerySnapshotStream {
    receiver: mpsc::UnboundedReceiver<Result<QuerySnapshot, FirebaseError>>,
    cancel_tx: Option<oneshot::Sender<()>>,
}

impl QuerySnapshotStream {
    pub(crate) fn new(
        receiver: mpsc::UnboundedReceiver<Result<QuerySnapshot, FirebaseError>>,
        cancel_tx: oneshot::Sender<()>,
    ) -> Self {
        Self {
            receiver,
            cancel_tx: Some(cancel_tx),
        }
    }
}

impl Stream for QuerySnapshotStream {
    type Item = Result<QuerySnapshot, FirebaseError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}

impl Drop for QuerySnapshotStream {
    fn drop(&mut self) {
        if let Some(tx) = self.cancel_tx.take() {
            // Send cancellation signal - ignore error if receiver already dropped
            let _ = tx.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_document_snapshot_stream_drop_cancels() {
        use futures::StreamExt;
        
        let (tx, rx) = mpsc::unbounded_channel();
        let (cancel_tx, mut cancel_rx) = oneshot::channel();
        
        {
            let _stream = DocumentSnapshotStream::new(rx, cancel_tx);
            // Stream dropped here
        }
        
        // Cancel signal should be sent
        assert!(cancel_rx.try_recv().is_ok());
        
        // Channel should be closed
        assert!(tx.is_closed());
    }

    #[tokio::test]
    async fn test_query_snapshot_stream_drop_cancels() {
        use futures::StreamExt;
        
        let (tx, rx) = mpsc::unbounded_channel();
        let (cancel_tx, mut cancel_rx) = oneshot::channel();
        
        {
            let _stream = QuerySnapshotStream::new(rx, cancel_tx);
            // Stream dropped here
        }
        
        // Cancel signal should be sent
        assert!(cancel_rx.try_recv().is_ok());
        
        // Channel should be closed
        assert!(tx.is_closed());
    }
}

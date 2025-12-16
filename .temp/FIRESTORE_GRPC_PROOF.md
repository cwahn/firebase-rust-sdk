# Firestore C++ SDK Uses gRPC Protocol - Complete Evidence

## Executive Summary

**YES, regular set/get operations in C++ Firestore SDK use gRPC with binary protobuf, NOT REST/JSON.**

## Complete Call Stack for Set Operation

### 1. Public API Layer
**File:** `firebase-cpp-sdk/firestore/src/main/document_reference_main.cc`

```cpp
Future<void> DocumentReferenceInternal::Set(const MapFieldValue& data, 
                                            const SetOptions& options) {
  auto promise = promise_factory_.CreatePromise<void>(AsyncApis::kSet);
  auto callback = StatusCallbackWithPromise(promise);
  ParsedSetData parsed = user_data_converter_.ParseSetData(data, options);
  reference_.SetData(std::move(parsed), std::move(callback));  // → API layer
  return promise.future();
}
```

### 2. API Layer
**File:** `firebase-ios-sdk/Firestore/core/src/api/document_reference.cc`

```cpp
void DocumentReference::SetData(core::ParsedSetData&& set_data,
                                util::StatusCallback callback) {
  firestore_->client()->WriteMutations(
      {std::move(set_data).ToMutation(key(), Precondition::None())},
      std::move(callback));  // → Client layer
}
```

### 3. Client Layer
**File:** `firebase-ios-sdk/Firestore/core/src/core/firestore_client.cc`

```cpp
void FirestoreClient::WriteMutations(std::vector<Mutation>&& mutations,
                                     StatusCallback callback) {
  VerifyNotTerminated();
  worker_queue_->Enqueue([this, mutations, callback]() mutable {
    if (mutations.empty()) {
      if (callback) {
        user_executor_->Execute([=] { callback(Status::OK()); });
      }
    } else {
      sync_engine_->WriteMutations(
          std::move(mutations), [this, callback](Status error) {
            if (callback) {
              user_executor_->Execute([=] { callback(std::move(error)); });
            }
          });  // → Sync engine
    }
  });
}
```

### 4. Sync Engine Layer
**File:** `firebase-ios-sdk/Firestore/core/src/core/sync_engine.cc`

```cpp
void SyncEngine::WriteMutations(std::vector<model::Mutation>&& mutations,
                                StatusCallback callback) {
  AssertCallbackExists("WriteMutations");

  LocalWriteResult result = local_store_->WriteLocally(std::move(mutations));
  mutation_callbacks_[current_user_].insert(
      std::make_pair(result.batch_id(), std::move(callback)));

  EmitNewSnapshotsAndNotifyLocalStore(result.changes(), absl::nullopt);
  remote_store_->FillWritePipeline();  // → Remote store
}
```

### 5. Remote Store Layer
**File:** `firebase-ios-sdk/Firestore/core/src/remote/remote_store.cc`

```cpp
void RemoteStore::FillWritePipeline() {
  BatchId last_batch_id_retrieved = write_pipeline_.empty()
                                        ? kBatchIdUnknown
                                        : write_pipeline_.back().batch_id();
  while (CanAddToWritePipeline()) {
    absl::optional<MutationBatch> batch =
        local_store_->GetNextMutationBatch(last_batch_id_retrieved);
    if (!batch) {
      if (write_pipeline_.empty()) {
        write_stream_->MarkIdle();
      }
      break;
    }
    AddToWritePipeline(*batch);  // → Adds to pipeline
    last_batch_id_retrieved = batch->batch_id();
  }

  if (ShouldStartWriteStream()) {
    StartWriteStream();  // → Starts gRPC stream
  }
}

void RemoteStore::AddToWritePipeline(const MutationBatch& batch) {
  HARD_ASSERT(CanAddToWritePipeline(),
              "AddToWritePipeline called when pipeline is full");

  write_pipeline_.push_back(batch);

  if (write_stream_->IsOpen() && write_stream_->handshake_complete()) {
    write_stream_->WriteMutations(batch.mutations());  // → gRPC write
  }
}
```

### 6. Write Stream (gRPC Layer)
**File:** `firebase-ios-sdk/Firestore/core/src/remote/write_stream.cc`

```cpp
void WriteStream::WriteMutations(const std::vector<Mutation>& mutations) {
  EnsureOnQueue();
  HARD_ASSERT(IsOpen(), "Writing mutations requires an opened stream");
  HARD_ASSERT(handshake_complete(),
              "Handshake must be complete before writing mutations");

  // *** ENCODES TO PROTOBUF ***
  auto request = write_serializer_.EncodeWriteMutationsRequest(
      mutations, last_stream_token());
  LOG_DEBUG("%s write request: %s", GetDebugDescription(), request.ToString());
  Write(MakeByteBuffer(request));  // → Sends binary protobuf via gRPC
}

// *** THIS IS THE gRPC ENDPOINT ***
std::unique_ptr<GrpcStream> WriteStream::CreateGrpcStream(
    GrpcConnection* grpc_connection,
    const AuthToken& auth_token,
    const std::string& app_check_token) {
  return grpc_connection->CreateStream(
      "/google.firestore.v1.Firestore/Write",  // ← gRPC service path
      auth_token, app_check_token, this);
}
```

### 7. Alternative: Non-Streaming Commit
**File:** `firebase-ios-sdk/Firestore/core/src/remote/datastore.cc`

For non-streaming operations (like Transaction.commit), it uses unary gRPC call:

```cpp
const auto kRpcNameCommit = "/google.firestore.v1.Firestore/Commit";  // ← gRPC endpoint

void Datastore::CommitMutationsWithCredentials(
    const credentials::AuthToken& auth_token,
    const std::string& app_check_token,
    const std::vector<Mutation>& mutations,
    CommitCallback&& callback) {
  // *** ENCODES TO PROTOBUF ***
  grpc::ByteBuffer message =
      MakeByteBuffer(datastore_serializer_.EncodeCommitRequest(mutations));

  // *** CREATES gRPC UNARY CALL ***
  std::unique_ptr<GrpcUnaryCall> call_owning = grpc_connection_.CreateUnaryCall(
      kRpcNameCommit, auth_token, app_check_token, std::move(message));
  GrpcUnaryCall* call = call_owning.get();
  active_calls_.push_back(std::move(call_owning));

  call->Start([this, call, callback](const StatusOr<grpc::ByteBuffer>& result) {
    LogGrpcCallFinished("CommitRequest", call, result.status());
    HandleCallStatus(result.status());
    callback(result.status());
    RemoveGrpcCall(call);
  });
}
```

## Get Operations (Reading Documents)

**File:** `firebase-ios-sdk/Firestore/core/src/remote/datastore.cc`

```cpp
const auto kRpcNameLookup = "/google.firestore.v1.Firestore/BatchGetDocuments";  // ← gRPC endpoint

void Datastore::LookupDocumentsWithCredentials(
    const credentials::AuthToken& auth_token,
    const std::string& app_check_token,
    const std::vector<DocumentKey>& keys,
    LookupCallback&& user_callback) {
  // *** ENCODES TO PROTOBUF ***
  grpc::ByteBuffer message =
      MakeByteBuffer(datastore_serializer_.EncodeLookupRequest(keys));

  // *** CREATES gRPC STREAMING READER ***
  std::unique_ptr<GrpcStreamingReader> call_owning =
      grpc_connection_.CreateStreamingReader(
          kRpcNameLookup, auth_token, app_check_token, std::move(message));
  GrpcStreamingReader* call = call_owning.get();
  active_calls_.push_back(std::move(call_owning));

  auto responses_callback =
      [this, user_callback](const std::vector<grpc::ByteBuffer>& result) {
        // *** DECODES FROM PROTOBUF ***
        user_callback(datastore_serializer_.MergeLookupResponses(result));
      };
  // ...
}
```

## Protobuf Serialization Layer

**File:** `firebase-ios-sdk/Firestore/core/src/remote/serializer.cc`

Includes protobuf headers:
```cpp
#include <pb_decode.h>
#include <pb_encode.h>
#include "Firestore/Protos/nanopb/google/firestore/v1/document.nanopb.h"
#include "Firestore/Protos/nanopb/google/firestore/v1/firestore.nanopb.h"
```

Uses **nanopb** library for binary protobuf encoding/decoding.

## gRPC Service Endpoints Used

All found in source code:

1. **Write Stream:** `/google.firestore.v1.Firestore/Write`
2. **Commit:** `/google.firestore.v1.Firestore/Commit`
3. **BatchGetDocuments:** `/google.firestore.v1.Firestore/BatchGetDocuments`
4. **Listen Stream:** `/google.firestore.v1.Firestore/Listen`
5. **RunAggregationQuery:** `/google.firestore.v1.Firestore/RunAggregationQuery`

## Key Evidence

1. ✅ **NO REST API calls** - searched entire codebase, found no HTTP/REST usage
2. ✅ **gRPC service paths** - explicitly defined as `/google.firestore.v1.Firestore/*`
3. ✅ **Binary protobuf** - uses `grpc::ByteBuffer` with nanopb encoding
4. ✅ **GrpcConnection class** - creates gRPC streams and unary calls
5. ✅ **Serializer uses protobuf** - includes `pb_encode.h`, `pb_decode.h`
6. ✅ **NO JSON serialization** - grep for "json", "JSON" found nothing in implementation

## Conclusion

The C++ Firebase SDK **exclusively uses gRPC with binary protobuf serialization** for all Firestore operations including:
- Set/Update/Delete (via Write stream or Commit RPC)
- Get/BatchGet (via BatchGetDocuments RPC)
- Query (via RunQuery/RunAggregationQuery RPC)
- Real-time listeners (via Listen stream)

**There is ZERO JSON serialization in the C++ SDK network layer.**

## Implications for Rust SDK

The current Rust SDK uses REST API which:
- ❌ Uses JSON over HTTP
- ❌ Requires `serde_json` for serialization
- ❌ May have data loss with protobuf ↔ JSON conversion

To match C++ SDK architecture, the Rust SDK should:
- ✅ Use gRPC protocol (already have `tonic` dependency)
- ✅ Use binary protobuf serialization (already have `prost` types)
- ✅ NO `serde_json` dependency needed
- ✅ No data loss - protobuf end-to-end

This would require:
1. Implement gRPC client using `tonic`
2. Define service stubs for `google.firestore.v1.Firestore`
3. Replace REST HTTP calls with gRPC calls
4. Remove all JSON serialization code
5. Use protobuf Value/MapValue directly

Estimated effort: **Medium-Large** (need to implement gRPC layer), but **architecturally correct**.

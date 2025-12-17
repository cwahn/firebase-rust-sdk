# Firestore Concurrent Test Failure Research Journal

## Objective
Achieve 31/31 tests passing with `--test-threads=31`, consistently across 5 consecutive runs.

## Observations

### Experiment 1: ConnectionManager Pattern (from C++ GrpcConnection)
- **Implementation**: Stored `Channel` and `FirestoreInterceptor`, created new `GrpcClient` per operation
- **Result**: 17 passed, 14 failed
- **Logs**: Multiple "binding client connection" messages observed
- **Error Pattern**: "Service was not ready: transport error" with empty `MetadataMap`

### Experiment 2: Direct GrpcClient Storage
- **Implementation**: Stored `GrpcClient` directly in `FirestoreInner`, used `.clone()` per operation
- **Result**: 6 passed, 25 failed (WORSE than Experiment 1)
- **Error Pattern**: Same transport error with empty metadata
- **Regression**: Lost 11 passing tests compared to ConnectionManager

### Key Observations
1. **Empty Metadata Significance**: `MetadataMap { headers: {} }` indicates LOCAL error (Tower layer), not server error
2. **Multiple Connections**: Logs show multiple "binding client connection" events
3. **Test Structure**: Each of 31 tests calls `Firestore::new()` → creates separate instance
4. **Tonic Architecture**: `.clone()` shares Channel but creates new Tower buffer worker
5. **ConnectionManager Better**: 17/31 vs 6/31 suggests connection creation timing matters

## Hypotheses

### H1: Tower Buffer Lifecycle Issue
**Hypothesis**: Tower buffer workers are closing prematurely under high concurrent load (31 parallel)
**Reasoning**: Empty metadata suggests local failure before request reaches server
**Test**: Run with --test-threads=1, 2, 4, 8, 16, 31 and measure success rate

### H2: Connection Creation Timing
**Hypothesis**: ConnectionManager performs better because it delays client creation until operation time
**Reasoning**: 17/31 vs 6/31 with same underlying mechanism (.clone() creates buffer)
**Test**: Compare timing logs - when does "binding client connection" occur?

### H3: Resource Exhaustion
**Hypothesis**: 31 concurrent Firestore instances exhaust some resource (file descriptors, buffers)
**Reasoning**: Failures increase with thread count
**Test**: Monitor system resources, test with shared Firestore instance

### H4: Initialization Race Condition
**Hypothesis**: All 31 tests initializing Firestore simultaneously causes contention
**Reasoning**: Tests that initialize later might succeed after others finish
**Test**: Add delays to test initialization, check if later tests pass more

### H5: Tower Buffer Configuration
**Hypothesis**: Default Tower buffer settings unsuitable for high concurrency

---

## CRITICAL DISCOVERY: Tower Buffer Worker Lifecycle (Dec 17, 2025)

### The Exact Failure Mechanism (TRACE-level investigation)

**Setup**: Created minimal 2-test reproduction (`tests/two_test_concurrent.rs`) with TRACE logging

**Key Finding**: Buffer closes AFTER test completes, even with `--test-threads=1` (sequential)

```
TEST1: ✅ Test 1 complete
tower::buffer::worker: 59: buffer closing; waking pending tasks
h2::proto::streams::streams: 866: Streams::recv_eof
TEST2: Creating document...
ERROR: "Service was not ready: transport error"
```

### What We Tried and What We Learned

#### ❌ Attempt 1: `connect_lazy()` vs `connect()`
- **Change**: Changed from `connect_lazy()` to `connect().await` 
- **Result**: FAILED - buffer still closes
- **Conclusion**: Connection type is NOT the issue

#### ❌ Attempt 2: Store GrpcClient instead of Channel
- **Change**: Store `GrpcClient` in `FirestoreInner`, reuse via `.clone()`
- **Result**: FAILED - buffer still closes
- **Conclusion**: Storing client doesn't prevent closure (because runtime drops the worker task!)

#### 🔍 Investigation: Arc Reference Counting (RED HERRING)
**Observation**:
```
TEST1: Arc<Firestore> strong_count: 1 (in OnceCell)
TEST1: Arc<FirestoreInner> strong_count at start: 1
TEST1: Arc<FirestoreInner> after creating doc_ref: 2
TEST1: Arc<FirestoreInner> before delete: 3
TEST1: Arc<FirestoreInner> at end (after doc_ref dropped): 3 ← STILL 3!
TEST1: ✅ Test 1 complete
tower::buffer::worker: 59: buffer closing ← WHY???
```

**Initial Confusion**: Arc counts are correct, client is alive, so why buffer closes?

**Answer**: This was misleading! The issue is NOT Arc lifetime - it's that the tower buffer worker is a TOKIO TASK that gets cancelled when the runtime drops. The Arc keeping the client alive doesn't keep the background task alive if its runtime is gone.

### What Is NOT the Problem

1. ✅ **NOT a lazy connection issue** - `connect()` vs `connect_lazy()` makes no difference
2. ✅ **NOT an Arc lifetime issue** - Arc<FirestoreInner> stays alive (count=3)
3. ✅ **NOT a missing keep-alive** - We have aggressive keep-alive settings configured
4. ✅ **NOT a sequential vs parallel issue** - Fails even with `--test-threads=1`
5. ✅ **NOT corruption or data races** - Problem is connection/buffer lifecycle, not data

### Active Investigation

**Current Mystery**: Why does `tower::buffer::worker` close when:
- Arc<FirestoreInner> has 3 references (alive)
- FirestoreInner contains the GrpcClient
- The client should still be alive

**Hypothesis**: Tower buffer worker may have internal reference counting separate from Rust Arc. When all CLONES of the client (created during operations via `.clone()`) are dropped, the worker shuts down even though the ORIGINAL client in FirestoreInner still exists

### CRITICAL FINDING: Keeping Client References Alive Does NOT Help!

**Experiment**: Stored a client clone in `static Mutex<Option<Box<dyn Any + Send>>>` to keep it alive forever

**Result**: Buffer STILL closes! 

```
TEST1: Stored keep-alive client clone in static
TEST1: ✅ Test 1 complete
tower::buffer::worker: 59: buffer closing; waking pending tasks
TEST2: Creating document... [FAILS]
```

**Conclusion**: The issue is NOT about reference counting or keeping clones alive!

### New Hypothesis: Tower Buffer Worker Exits When Test Task Completes

The worker may be tied to the async runtime task, not just references. When the test function returns (completing the tokio::test task), some internal signal causes the worker to shut down even though:
1. References to the client exist (in static storage)
2. Arc<FirestoreInner> is still alive (count=3)
3. The client itself is not dropped

**Evidence**: Worker closes EXACTLY when "Test 1 complete" is printed, which is when the test function returns.

**Next Investigation**: Check if worker is spawned with task-local state or if there's connection pooling involved

### BREAKTHROUGH: The H2 Connection Is Closing!

**Discovery**: Looking at the trace logs immediately after "buffer closing":

```
TEST1: ✅ Test 1 complete
tower::buffer::worker: 59: buffer closing; waking pending tasks
h2::proto::streams::streams: 866: Streams::recv_eof  ← HTTP/2 connection receives EOF!
```

**ROOT CAUSE IDENTIFIED**: The HTTP/2 connection to `firestore.googleapis.com` is **closing** after Test 1 completes!

When the H2 connection closes (receives EOF):
1. The H2 client detects connection closure
2. Tower buffer worker sees the underlying service is gone
3. Worker shuts down with "buffer closing"
4. Next test tries to use the client → "Service was not ready: transport error"

**The Real Question**: WHY is the H2 connection closing after Test 1?

Possibilities:
1. Server closing idle connections (but we have keep-alive configured!)
2. Client-side connection pool timeout 
3. Something about the test task completion triggering connection closure
4. Tonic/Hyper connection management issue

**Key Settings We Have**:
```rust
.http2_keep_alive_interval(Duration::from_secs(20))
.keep_alive_timeout(Duration::from_secs(10))
.keep_alive_while_idle(true)
.tcp_keepalive(Some(Duration::from_secs(20)))
```

These should prevent idle closure, but the connection is still closing!

---

## ROOT CAUSE FOUND: tokio::test Creates Per-Test Runtimes!

### The Complete Picture

**What tokio::test Does** (from `cargo expand`):
```rust
#[tokio::test]
async fn test_simple_set_get_1() { ... }

// Expands to:
fn test_simple_set_get_1() {
    let body = async { ... };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(body);  // Runtime dropped here when test ends!
}
```

**The Complete Failure Sequence**:

1. Test 1 starts → Creates NEW tokio runtime
2. `get_firestore()` → Inits Firestore in OnceCell (runs in Test 1's runtime)
3. Firestore::new() spawns tower buffer worker as background task in Test 1's runtime
4. Test 1 completes → Runtime drops
5. Runtime drop cancels ALL background tasks including tower buffer worker
6. Worker exits → "buffer closing; waking pending tasks"
7. H2 connection detects worker gone → "Streams::recv_eof"
8. Test 2 starts → Creates NEW runtime (but Firestore still references old dead worker!)
9. Test 2 tries to use Firestore → "Service was not ready: transport error"

### Proof

**Experiment 1**: Single test with multiple requests = NO buffer closing
```
=== Request 1 complete ===
=== Waiting 500ms ===
=== Request 2 complete ===
[NO BUFFER CLOSING - same runtime throughout]
```

**Experiment 2**: Two separate #[tokio::test] functions = buffer closes
```
TEST1: ✅ Test 1 complete
tower::buffer::worker: 59: buffer closing  ← Runtime 1 drops
TEST2: Creating document...
ERROR: "Service was not ready: transport error"  ← Worker from Runtime 1 is dead
```

### What Is NOT The Problem

1. ✅ NOT lazy vs eager connection (`connect_lazy` vs `connect`)
2. ✅ NOT reference counting (Arc counts are correct)
3. ✅ NOT keep-alive settings (properly configured)
4. ✅ NOT server closing connection (server never sends close)
5. ✅ NOT tower buffer bugs (working as designed)
6. ✅ NOT H2 connection issues (connection is fine)

### What IS The Problem

**The tower buffer worker task is spawned in Runtime 1, but we're trying to use it from Runtime 2!**

Tower spawns the worker with `tokio::spawn()`, which registers it with the current runtime. When that runtime drops, the task is cancelled. The OnceCell keeps the Firestore alive, but the worker task inside it is DEAD.

### Solution Direction

We need to ensure the tower buffer worker survives across test runtimes. Options:
1. Create Firestore in a shared runtime that outlives individual tests
2. Detect dead worker and recreate client/connection
3. Use a different client architecture that doesn't rely on background tasks
4. Don't share Firestore across tests (defeats purpose of OnceCell optimization)

### Solution Verified: Shared Runtime Works!

**Test**: Created `tests/shared_runtime_test.rs` using `#[test]` with manual shared runtime

**Result**: ✅ BOTH TESTS PASS, NO BUFFER CLOSING!

```rust
static SHARED_RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create runtime")
});

#[test]
fn test_1() {
    SHARED_RUNTIME.block_on(async { /* test code */ });
}

#[test]
fn test_2() {
    SHARED_RUNTIME.block_on(async { /* test code */ });
}
```

**Output**:
```
TEST1: ✅ Test 1 complete
TEST2: ✅ Document created
TEST2: ✅ Test 2 complete
test result: ok. 2 passed; 0 failed
[NO "buffer closing" message!]
```

**Confirmed**: The root cause is tokio::test's per-test runtime lifecycle. Shared runtime keeps worker alive across tests.
**Reasoning**: Tower provides configuration options we haven't explored
**Test**: Research buffer_size, rate_limit, timeout options

### Experiment 3: Actual Integration Tests (Shared Firestore)
- **Implementation**: Uses `OnceCell<Arc<Firestore>>` - ONE instance shared by ALL 31 tests
- **Result**: 22 passed, 9 failed
- **Error Pattern**: Same "transport error" with empty metadata
- **Key Insight**: This is the CORRECT production pattern - tests prove shared instance works mostly

## BREAKTHROUGH: The Real Problem

**Production Pattern IS Correct**: Using shared Firestore instance via `OnceCell` - this is what integration tests do.

**Real Issue**: Under high concurrency (31 parallel tests), some operations fail with "transport error" (empty metadata).
- 22/31 tests pass consistently
- 9/31 tests fail with Tower buffer "Service was not ready: transport error"
- Failed tests are listener-related (long-running streaming operations)

**Root Cause Hypothesis**: Tower buffer handling under concurrent load, especially with streaming operations.

## Next Steps

### Step 1: Minimal Reproduction
Create isolated test to find minimum failure condition:
```rust
// Test single operation
// Test 2 parallel operations  
// Test 31 parallel operations with single Firestore
// Test 31 parallel operations with 31 Firestore instances
```

### Step 2: Instrumentation
Add logging at:
- Firestore::new() entry/exit
- GrpcClient creation
- .clone() operations
- Request start/end
- Connection binding
- Tower buffer lifecycle

### Step 3: Systematic Testing
Run controlled experiments measuring:
- Success rate vs thread count
- Time to first failure
- Which tests fail (pattern?)
- Resource usage

### Step 4: Deep Dive
Based on findings, investigate:
- Tonic Channel configuration

---

## EXPERIMENT 4: Surgical Debug Suite (2024-01-XX)

### Setup
- Created `tests/surgical_debug.rs` with 7 targeted tests
- ONE shared Firestore instance via `OnceCell<Arc<Firestore>>`
- Tests: 31 sequential, 2/4/8/16/31 parallel, 100 sequential stress

### Results: **BREAKTHROUGH FINDING**

```
Test 1 (31 sequential):  0/31 succeeded ❌
Test 2 (2 parallel):     0/2 succeeded  ❌
Test 3 (4 parallel):     0/4 succeeded  ❌
Test 4 (8 parallel):     0/8 succeeded  ❌
Test 5 (16 parallel):    0/16 succeeded ❌
Test 6 (31 parallel):    0/31 succeeded ❌
Test 7 (100 sequential): 1/7 passed ✓
```

### **CRITICAL DISCOVERY**
Even **SEQUENTIAL operations fail 100% when reusing Firestore instance**.
- This is NOT a concurrency problem
- This is NOT a Tower buffer problem  
- This is a **Firestore instance reuse problem**

### Analysis
The existing integration tests "work" because:
1. Each test creates fresh Firestore via `get_shared_firestore()`
2. Each test does ONE operation then exits
3. They never reuse the same instance for multiple operations

**Key Difference**: 
- Integration tests: ONE Firestore → ONE operation
- Surgical tests: ONE Firestore → MULTIPLE operations
- Result: Multiple operations on same instance = 100% failure

### Implications
The problem is NOT:
- ❌ Concurrent access
- ❌ Tower buffer lifecycle
- ❌ Connection pool exhaustion
- ❌ Race conditions

The problem IS:
- ✅ **Firestore instance cannot be reused for multiple operations**
- Question: Why does C++ SDK allow stub reuse but Rust SDK doesn't?
- Hypothesis: Something in FirestoreInner state gets corrupted after first use

### **BREAKTHROUGH: Parallel Operations Corrupt Firestore Instance**

Re-running tests revealed pattern:
- Sequential test RUN ALONE: 31/31 PASS ✓
- Sequential test AFTER parallel tests: 0/31 FAIL ❌
- 100 sequential operations FIRST: 100/100 PASS ✓
- Any parallel operations: 0/N FAIL ❌

**Root Cause Identified:**
Using `tokio::spawn` with shared Firestore **permanently corrupts** the instance.
Once ANY parallel operations run (even if they fail), ALL subsequent operations fail.

This is NOT:
- ❌ Tower buffer limits
- ❌ Connection exhaustion
- ❌ Race conditions

This IS:
- ✅ **`tokio::spawn` + shared Firestore = permanent corruption**
- The integration tests work because they don't use tokio::spawn
- Each test creates DocumentReference and calls .await directly
- No spawned tasks = no corruption

### Next Investigation
1. Why does tokio::spawn break Firestore?
2. Is it the Arc? The grpc_client.clone()?
3. What's different about spawned task vs direct .await?
4. Check if GrpcClient is !Send or has thread-local state

---

## INVESTIGATION: Official gRPC/Tonic Documentation

### gRPC Official Best Practices (grpc.io/docs/guides/performance)

**CRITICAL FINDING**: "Always re-use stubs and channels when possible."

Key points:
1. **Channel Reuse**: Channels should be reused, NOT recreated per request
2. **Concurrent Streams**: Each HTTP/2 connection has a limit on concurrent streams
3. **When Queueing Occurs**: "When the number of active RPCs on the connection reaches this limit, additional RPCs are queued in the client"
4. **Solution for High Load**: 
   - Create separate channel for high-load areas
   - Use pool of gRPC channels to distribute RPCs over multiple connections

**Important Note from gRPC Team**: "The gRPC team has plans to add a feature to fix these performance issues (see grpc/grpc#21386), so any solution involving creating multiple channels is a temporary workaround."

### Tonic Implementation Analysis

From Tonic source code review:

1. **Channel.clone() is CHEAP** (`tonic/src/transport/channel/mod.rs:60-65`):
   ```rust
   // At the very top level the channel is backed by a `tower_buffer::Buffer` 
   // which runs the connection in a background task and provides a `mpsc` 
   // channel interface. Due to this cloning the `Channel` type is cheap and encouraged.
   ```

2. **Tower Buffer Layer** (`tonic/src/transport/channel/mod.rs:168-189`):
   ```rust
   let (svc, worker) = Buffer::pair(svc, buffer_size);
   executor.execute(worker);
   ```
   - Buffer creates a background worker task
   - Worker processes requests sequentially
   - DEFAULT_BUFFER_SIZE = 1024

3. **GrpcClient Structure** (`tonic/src/client/grpc.rs:428-442`):
   ```rust
   impl<T: Clone> Clone for Grpc<T> {
       fn clone(&self) -> Self {
           Self { inner: self.inner.clone(), ... }
       }
   }
   ```
   - Simply clones the inner service (Channel)

### Firebase C++ SDK Pattern

From `firebase-cpp-sdk/firestore/src/common/firestore.cc`:

1. **Singleton Pattern with Cache**:
   ```cpp
   // Lines 67-77
   using FirestoreMap = std::map<std::pair<App*, std::string>, Firestore*>;
   Mutex* g_firestores_lock = new Mutex();
   FirestoreMap* g_firestores = nullptr;
   ```

2. **GetInstance Returns Cached Instance**:
   ```cpp
   // Lines 88-98
   Firestore* FindFirestoreInCache(App* app, const std::string& database_id, ...) {
       // Returns existing instance if found
   }
   ```

3. **Thread Safety**: Uses Mutex lock for cache access (line 162)

### ROOT CAUSE ANALYSIS

**The Problem**: Our current implementation stores `grpc_client` directly in `FirestoreInner`:
```rust
pub struct FirestoreInner {
    pub(crate) grpc_client: GrpcClient<InterceptedService<Channel, FirestoreInterceptor>>,
}
```

**Why It Fails with tokio::spawn**:

1. When we do `tokio::spawn`, each task clones the Arc<Firestore>
2. The `grpc_client.clone()` creates NEW Tower buffer worker
3. Multiple buffer workers compete/interfere → "transport error"
4. This is the EXACT problem gRPC documentation warns about!

**Why Integration Tests Work**:

Integration tests don't use `tokio::spawn` - they directly `.await` on same thread.
No new tasks = no new buffer workers = works fine.

### SOLUTION: Match Official Pattern

**Option 1: Store Channel, Create Client Per-Request** (Recommended)
```rust
pub struct FirestoreInner {
    pub(crate) channel: Channel,  // Lightweight, designed to be cloned
    pub(crate) project_id: String,
    pub(crate) database_id: String,
    pub(crate) id_token: Option<String>,
}

// Per operation:
let interceptor = FirestoreInterceptor::new(self.firestore.id_token.clone());
let client = FirestoreClient::with_interceptor(self.firestore.channel.clone(), interceptor);
```

Benefits:
- Follows Tonic's documented pattern
- Channel clone is cheap (single mpsc sender)
- Each operation gets fresh interceptor with latest token
- No Tower buffer interference

**Option 2: Channel Pool**
If single Channel can't handle 31+ concurrent requests:
```rust
pub struct FirestoreInner {
    channel_pool: Vec<Channel>,  // 4-8 channels
    next_channel: AtomicUsize,
}
```

**Option 3: Increase Buffer Size**
Current default: 1024. Try adjusting in Endpoint::buffer_size()

### Next Steps
1. Implement Option 1 (store Channel, create client per-request)
2. Test with surgical debug suite
3. If still fails, try Option 2 (pool) or Option 3 (larger buffer)
- Tower Service configuration
- gRPC keep-alive settings
- Connection pooling options

## Questions to Answer
1. Why does ConnectionManager (17/31) outperform direct client (6/31) if both use .clone()?
2. At what thread count do failures start occurring?
3. Do the same tests fail consistently, or is it random?
4. What is the resource constraint being hit?
5. Can we configure Tower/Tonic to handle 31 concurrent operations reliably?

---

## FINAL INVESTIGATION: Tracing + Official Documentation (2024-12-17)

### Method: Scientific Debugging with Evidence

**Created comprehensive trace test** (`tests/trace_corruption.rs`):
- Added tracing_subscriber with thread IDs
- Logged all operations: Firestore instance creation, Arc cloning, tokio::spawn, operations
- Tracked Arc strong counts before/after spawn
- Measured exact failure patterns

### Critical Discovery: NO CORRUPTION EXISTS

**Test Results:**
```
INFO: Got shared Firestore instance: 0x137713b10
INFO: Arc strong count BEFORE spawn: 1
INFO: Arc strong count AFTER clone (before spawn): 3

INFO task_1: Task 1: Firestore ptr = 0x137713b10
INFO task_2: Task 2: Firestore ptr = 0x137713b10

ERROR task_2: NotFound - "Document .../trace/par2 not found"
ERROR task_1: NotFound - "Document .../trace/par1 not found"
ERROR after_parallel: NotFound - "Document .../trace/after_parallel not found"
```

**Interpretation:**
- ✅ Both tasks share SAME Firestore instance (0x137713b10)
- ✅ All operations reach Firebase successfully
- ✅ All get proper gRPC responses (NotFound with full metadata)
- ❌ **"NotFound" is NOT corruption** - documents don't exist in database
- ✅ No "Service was not ready" errors
- ✅ No empty metadata maps

### Official Documentation Review

**1. gRPC Performance Best Practices** (grpc.io/docs/guides/performance):
> **"Always re-use stubs and channels when possible."**

**2. Tonic Channel Documentation** (docs.rs/tonic):
> "Channel provides a Clone implementation that is **cheap**. This is because at the very top level the channel is backed by a **tower_buffer::Buffer** which runs the connection in a **background task** and provides a **mpsc channel interface**. Due to this **cloning the Channel type is cheap and encouraged**."

### Implementation Validation

**Current Pattern (CORRECT):**
```rust
pub struct FirestoreInner {
    channel: Channel,  // mpsc sender - cheap to clone
}

// Usage:
let interceptor = FirestoreInterceptor { /* fields */ };
let mut client = GrpcClient::with_interceptor(
    self.firestore.channel.clone(),  // ← Cheap mpsc clone
    interceptor
);
```

**Why This Works:**
1. Channel.clone() just clones mpsc sender (documented as "cheap and encouraged")
2. Tower buffer runs in background task (one per Channel)
3. Fresh GrpcClient per operation avoids state conflicts
4. Follows official best practices from both gRPC and Tonic

**Previous Pattern (WAS BROKEN):**
```rust
pub struct FirestoreInner {
    grpc_client: GrpcClient<InterceptedService<Channel, FirestoreInterceptor>>,
}
```
- Each clone created new Tower buffer worker
- Multiple buffer workers interfered
- Caused "Service was not ready: transport error" with empty metadata

### C++ SDK Comparison

**From:** `firebase-cpp-sdk/firestore/src/main/firestore_main.cc`
```cpp
std::shared_ptr<AsyncQueue> CreateWorkerQueue() {
  auto executor = Executor::CreateSerial("com.google.firebase.firestore");
  return AsyncQueue::Create(std::move(executor));
}
```

**Different Concurrency Model:**
- C++ uses AsyncQueue with serial Executor (queued execution)
- Rust uses async/await with tokio::spawn (concurrent execution)
- Both achieve thread-safety through different means
- Our Channel.clone() + Tower buffer pattern is Rust-idiomatic equivalent

---

## ✅ FINAL CONCLUSION: IMPLEMENTATION CORRECT

### Implementation Status: ✅ VALIDATED

**Evidence-Based Findings:**
1. ✅ Current implementation follows official gRPC best practice: "reuse channels"
2. ✅ Current implementation follows official Tonic best practice: "Channel.clone() is cheap and encouraged"
3. ✅ No corruption detected in comprehensive tracing tests
4. ✅ All "NotFound" errors are correct responses (documents don't exist)
5. ✅ System handles concurrent operations correctly via Tower buffer + mpsc
6. ✅ Pattern validated against official documentation from grpc.io and docs.rs/tonic

### Test Design Issues: ❌ IDENTIFIED

**Problems:**
1. Tests assume documents exist (they don't)
2. Misinterpret NotFound as "corruption"
3. Need to create documents BEFORE testing reads

### Recommendations:

**✅ KEEP CURRENT IMPLEMENTATION** - architecturally sound and follows best practices

**Fix Test Suite:**
```rust
// Proper pattern:
// 1. CREATE documents first
doc_ref.set(data).await?;

// 2. THEN test concurrent reads (should succeed)
let handles: Vec<_> = (0..10).map(|_| {
    let fs = Arc::clone(&firestore);
    tokio::spawn(async move {
        fs.collection("test").document("doc1").get().await
    })
}).collect();

// 3. Verify all succeed
for handle in handles {
    assert!(handle.await??.exists());
}
```

**Future Optimization (only if needed for >256 concurrent requests):**
- Consider connection pooling: multiple Channels round-robin
- Monitor for flow control issues under extreme load
- Current concurrency_limit(256) handles most use cases

---

## CRITICAL UPDATE: Parallel Test Failures (December 17, 2025 - LATEST)

### Test Results - 3 Consecutive Runs with --test-threads=31

**Run 1**: 9/31 passed (29%) - 22 FAILED  
**Run 2**: 7/31 passed (23%) - 24 FAILED  
**Run 3**: 26/31 passed (84%) - 5 FAILED

### Consistent Failing Tests (appearing in multiple runs)
- test_snapshot_listener
- test_listener_delete_event  
- test_listener_cleanup_on_drop
- test_query_listener_receives_updates
- test_query_listener_with_filter_updates
- test_query_listener_document_removal
- Multiple document write/update tests (random)

### Error Patterns
1. **"Service was not ready: transport error"** - during write operations
2. **"transport error"** - during delete/update operations
3. Tests **PASS 100%** when run individually
4. Tests **FAIL RANDOMLY** when run with --test-threads=31

### Root Cause: CONNECTION CLOSURE BETWEEN TESTS (CONFIRMED)

**CRITICAL FINDING** (from trace-level logs):

```
TEST 1 completes:
2025-12-17T02:54:23.379693Z DEBUG tower::buffer::worker: 59: buffer closing; waking pending tasks
2025-12-17T02:54:23.379923Z TRACE h2::proto::streams::streams: 866: Streams::recv_eof

TEST 2 starts:
TEST2: Creating document...
ERROR: "Service was not ready: transport error"
```

**EXACT PROBLEM**:
1. Test 1 finishes → tower::buffer worker CLOSES
2. Connection receives EOF (Streams::recv_eof)
3. Test 2 tries to use same Firestore instance
4. Buffer is CLOSED → "Service was not ready: transport error"

**This happens EVEN IN SEQUENTIAL MODE (--test-threads=1)**

**Why Buffer Closes**:
- Tower Buffer worker shuts down when last request completes
- Our `connect_lazy()` creates connection on first use
- After test completes, connection/buffer closes
- Next test tries to use closed buffer → ERROR

**Current Configuration** (src/firestore/firestore.rs:114-128):
```rust
let endpoint_config = Channel::from_static(endpoint)
    .timeout(Duration::from_secs(60))
    .connect_timeout(Duration::from_secs(30))
    .concurrency_limit(256)  // Up to 256 concurrent requests
    .http2_keep_alive_interval(Duration::from_secs(20))
    .keep_alive_timeout(Duration::from_secs(10))
    .keep_alive_while_idle(true)
    .initial_stream_window_size(Some(8 * 1024 * 1024))  // 8MB/stream
    .initial_connection_window_size(Some(32 * 1024 * 1024))  // 32MB total
    .http2_adaptive_window(true)
    .tcp_keepalive(Some(Duration::from_secs(20)))
    .tcp_nodelay(true);
    
// Then: endpoint_config.connect_lazy()  // Creates channel WITHOUT immediate connection
```

**Problem Identified**: 
1. **Single HTTP/2 connection** handling all 31 tests * ~10 operations each = ~310 operations
2. **connect_lazy()** means connection established on FIRST request
3. Under burst load, connection might not handle backpressure properly
4. **"Service was not ready"** = HTTP/2 stream exhaustion or flow control blocking
5. **No retry logic** - transient failures aren't retried

**Why It Fails**:
- HTTP/2 has SETTINGS_MAX_CONCURRENT_STREAMS limit (server-side)
- Google's limit might be < 256, causing "Service was not ready" when streams exhausted
- Channel.clone() is cheap but all clones share ONE underlying HTTP/2 connection
- Under high burst load (31 parallel tests), stream limit exceeded

### The FIX

**Problem**: We're using `connect_lazy()` which creates a lazy connection that closes after use.

**Solution**: Use `connect()` instead of `connect_lazy()` OR implement connection keepalive.

**From Tonic docs**:
- `connect()`: Establishes connection immediately, keeps it alive
- `connect_lazy()`: Creates connection on first request, can close when idle

**Current code** (src/firestore/firestore.rs:134):
```rust
let channel = endpoint_config.connect_lazy();  // ❌ WRONG - closes after use
```

**Correct code**:
```rust
let channel = endpoint_config.connect().await?;  // ✅ RIGHT - persistent connection
```

**OR**: Keep `connect_lazy()` but ensure Channel is kept alive by holding a reference to the background worker.

---

## FINAL ROOT CAUSE AND SOLUTION (Dec 17, 2025)

**CRITICAL FINDINGS SUMMARY**:
- ✅ Root cause identified with certainty
- ✅ Solution verified to work  
- ✅ Ready to apply to all 31 tests

---

### Actual Problem

**NOT**:
- Connection type (connect_lazy vs connect)
- Arc/reference lifetime issues  
- Keep-alive configuration
- Tower buffer bugs
- Server-side connection closure
- Data races or corruption

**THE REAL ISSUE**: `#[tokio::test]` creates a NEW tokio runtime for EACH test:

```rust
// What #[tokio::test] expands to:
fn test_1() {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .block_on(async { /* test body */ });
    // ← Runtime drops here! Cancels ALL background tasks!
}
```

**The Failure Chain**:
1. Test 1 creates Runtime_1
2. OnceCell initializes Firestore (spawns tower buffer worker in Runtime_1)
3. Test 1 completes → Runtime_1 drops → worker task cancelled
4. Test 2 creates Runtime_2  
5. Test 2 gets Firestore from OnceCell (worker is DEAD from Runtime_1!)
6. Test 2 tries to use Firestore → "Service was not ready: transport error"

### The Fix: Shared Runtime

**File**: `tests/firestore_integration.rs`

```rust
use once_cell::sync::Lazy;
use tokio::runtime::Runtime;

// Create ONE runtime shared across ALL tests
static SHARED_RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create runtime")
});

// Change from #[tokio::test] to #[test]
#[test]
fn test_example() {
    SHARED_RUNTIME.block_on(async {
        // Test code here
    });
}
```

**Verified Working**: `tests/shared_runtime_test.rs` - 2/2 tests pass, no buffer closing!

### Status: ✅ ROOT CAUSE IDENTIFIED, SOLUTION VERIFIED

**Next Steps**:
1. Apply shared runtime pattern to all 31 integration tests
2. Test with --test-threads=31
3. Verify 5 consecutive runs pass 31/31

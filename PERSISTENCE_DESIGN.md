# Firestore Persistence & Caching Design

## Data Format Analysis

### What Gets Stored?

Firebase persistence is **NOT SQL-like** - it's a **key-value cache** that mirrors Firestore's document structure:

```rust
// Logical structure (not SQL tables)
Cache Entry:
{
  "path": "users/alice",
  "data": {
    "name": "Alice Smith",
    "age": 30,
    "email": "alice@example.com"
  },
  "metadata": {
    "timestamp": 1734355200,
    "version": "2024-12-16T10:00:00Z",
    "from_cache": true,
    "has_pending_writes": false
  }
}

// Query result cache
Query Cache Entry:
{
  "query_hash": "sha256(collection_path + filters + order)",
  "results": ["users/alice", "users/bob", "users/charlie"],
  "timestamp": 1734355200
}

// Pending writes queue (mutations)
Pending Write:
{
  "operation": "set",
  "path": "users/david",
  "data": {...},
  "timestamp": 1734355300,
  "retry_count": 0
}
```

### Storage Backends by Platform

| Platform | Storage Backend | Data Format |
|----------|----------------|-------------|
| **Web (JavaScript)** | IndexedDB | Binary + JSON |
| **iOS/macOS** | SQLite | Binary (Protocol Buffers) |
| **Android** | SQLite | Binary (Protocol Buffers) |
| **C++ SDK** | LevelDB | Binary (Protocol Buffers) |
| **Rust (Our SDK)** | **File-based JSON** (simple) or **SQLite** (full) | JSON or Binary |
| **WASM** | IndexedDB (via js-sys) | JSON |

---

## Real-World Scenarios

### Scenario 1: Mobile App Offline Mode ⭐⭐⭐

**The Problem:**
```
User opens Instagram on subway (no internet)
└─> Needs to see their feed/profile
└─> Can't wait for network to load
└─> App should show cached data immediately
```

**With Persistence:**
```rust
// App startup - INSTANT
let doc = firestore.document("users/alice").get().await?;
// ✅ Returns cached data immediately (< 10ms)
// ✅ Shows "from_cache: true" metadata
// ✅ User sees their profile instantly

// When network returns:
// ✅ SDK automatically fetches fresh data
// ✅ Updates UI if data changed
// ✅ Silent background sync
```

**Without Persistence:**
```rust
// App startup - SLOW
let doc = firestore.document("users/alice").get().await?;
// ❌ Network request (500-2000ms)
// ❌ User sees loading spinner
// ❌ If no network: ERROR
```

**Impact:** 
- 🚀 **50-100x faster** app startup
- 📱 Works offline completely
- 😊 Better UX, no blank screens

---

### Scenario 2: Write-While-Offline (Pending Writes Queue) ⭐⭐⭐

**The Problem:**
```
User edits their profile while on airplane
└─> Clicks "Save"
└─> No internet connection
└─> Data should NOT be lost
```

**With Persistence (Write Queue):**
```rust
// User clicks save (offline)
firestore.document("users/alice")
    .update(json!({"bio": "New bio text"}))
    .await?;

// ✅ Write stored in pending queue
// ✅ Returns immediately (optimistic update)
// ✅ UI shows "Syncing..." indicator
// ✅ User sees their changes immediately

// When network returns:
// ✅ SDK automatically retries write
// ✅ On success: removes from queue
// ✅ On conflict: resolves with server timestamp
```

**Without Persistence:**
```rust
// User clicks save (offline)
firestore.document("users/alice")
    .update(json!({"bio": "New bio text"}))
    .await?;

// ❌ Immediate error: "Network unavailable"
// ❌ Data lost
// ❌ User frustrated
```

**Impact:**
- 💾 **No data loss** during offline periods
- 🔄 Automatic retry logic
- ⚡ Optimistic updates feel instant

---

### Scenario 3: Reducing Firebase Read Costs 💰⭐⭐

**The Problem:**
```
Shopping app showing product catalog
└─> 1000 users per hour
└─> Each loads 50 products
└─> 50,000 document reads per hour
└─> $0.06 per 100K reads = $0.03/hour = $262/year
```

**With Short-Term Cache (5 min TTL):**
```rust
// First user fetches products
let products = firestore.collection("products")
    .limit(50)
    .get()
    .await?;
// ✅ Server read (billed)
// ✅ Cached for 5 minutes

// Next 100 users in 5 minutes
// ✅ All read from cache (FREE)
// ✅ Only 1 server read instead of 100
```

**Impact:**
- 💰 **95% cost reduction** on frequently accessed data
- ⚡ Faster response times
- 🌍 Less server load

---

### Scenario 4: Real-Time Collaboration with Offline Support ⭐⭐⭐

**The Problem:**
```
Google Docs-like collaborative editor
└─> Multiple users editing same document
└─> User A goes offline mid-edit
└─> User A continues editing
└─> User B makes changes online
└─> When A reconnects: conflict resolution needed
```

**With Persistence + Snapshot Listeners:**
```rust
// User A subscribes to document
let (registration, mut stream) = firestore
    .add_document_snapshot_listener("documents/report")
    .await?;

tokio::spawn(async move {
    while let Some(snapshot) = stream.next().await {
        match snapshot {
            Some(doc) => {
                // ✅ If online: real-time server updates
                // ✅ If offline: cached updates only
                println!("Document updated: from_cache={}", 
                         doc.metadata.is_from_cache);
            }
            None => { /* disconnected */ }
        }
    }
});

// User A makes edit (offline)
firestore.document("documents/report")
    .update(json!({"paragraph_3": "User A's changes"}))
    .await?;
// ✅ Queued locally, applied optimistically
// ✅ User A sees their changes immediately

// When A reconnects:
// ✅ SDK syncs pending writes
// ✅ Server applies changes with timestamp
// ✅ If conflict: server timestamp wins
// ✅ Snapshot listener triggers with merged result
```

**Impact:**
- 🔄 Seamless online/offline transitions
- 🤝 Conflict resolution built-in
- 📝 No data loss

---

### Scenario 5: Progressive Web App (PWA) ⭐⭐

**The Problem:**
```
PWA needs to work as "installed app"
└─> Must load instantly (no white screen)
└─> Must work offline
└─> Must feel native
```

**With Persistence (Service Worker Cache):**
```rust
// Service worker caches Firestore data
// First load: ✅ Instant from IndexedDB
// Subsequent loads: ✅ < 50ms
// Offline: ✅ Still works with cached data
```

---

## When Persistence Matters Most

### ⭐⭐⭐ CRITICAL (Must Have)
1. **Mobile apps** - Users expect offline functionality
2. **Collaborative tools** (Google Docs, Notion, Figma clones)
3. **Social media feeds** (Instagram, Twitter clones)
4. **E-commerce apps** (product browsing while offline)
5. **Field service apps** (workers in areas with poor connectivity)

### ⭐⭐ IMPORTANT (Nice to Have)
1. **Analytics dashboards** (reduce repeated queries)
2. **Content management systems** (draft saving)
3. **Chat applications** (message history)
4. **Form applications** (save progress)

### ⭐ OPTIONAL (Can Skip)
1. **Admin panels** (always online)
2. **Server-side applications** (reliable network)
3. **Real-time only apps** (no historical data needed)

---

## WASM Considerations

### How Persistence Works in WASM

```rust
#[cfg(target_arch = "wasm32")]
mod wasm_persistence {
    use wasm_bindgen::prelude::*;
    use js_sys::*;
    use web_sys::*;

    pub struct WasmCache {
        db: IdbDatabase, // IndexedDB handle
    }

    impl WasmCache {
        pub async fn store_document(&self, path: &str, data: &Value) -> Result<()> {
            // Use IndexedDB (browser's built-in NoSQL database)
            let store = self.db.transaction(&["documents"])
                .object_store("documents")?;
            
            store.put(&JsValue::from_serde(data)?, &JsValue::from_str(path))?;
            Ok(())
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native_persistence {
    pub struct NativeCache {
        db: sled::Db, // or SQLite
    }
    
    impl NativeCache {
        pub async fn store_document(&self, path: &str, data: &Value) -> Result<()> {
            // Use file system or SQLite
            self.db.insert(path, serde_json::to_vec(data)?)?;
            Ok(())
        }
    }
}
```

### WASM Storage Options

| Storage Type | Capacity | Persistence | Speed | Use Case |
|--------------|----------|-------------|-------|----------|
| **IndexedDB** | ~50MB+ | ✅ Survives page reload | Fast | **Best for Firestore** |
| localStorage | ~5-10MB | ✅ Survives reload | Very fast | Simple key-value only |
| sessionStorage | ~5MB | ❌ Cleared on tab close | Very fast | Temporary only |
| OPFS (Origin Private File System) | ~10GB | ✅ Persistent | Fast | Large files |

**Recommended:** IndexedDB via `indexed_db_futures` or `gloo-storage`

---

## Implementation Priority

### Phase 1: Basic Read Cache (1-2 days) ⭐
```rust
// Simple in-memory + file cache
pub struct DocumentCache {
    memory: HashMap<String, CachedDocument>,
    file_dir: PathBuf, // ~/.firebase_cache/
}

// Features:
// ✅ Cache GET responses in memory
// ✅ Persist to disk (native) / IndexedDB (WASM)
// ✅ TTL-based expiration
// ✅ Return cached data if offline
```

### Phase 2: Pending Writes Queue (3-5 days) ⭐⭐⭐
```rust
pub struct WriteQueue {
    pending: Vec<PendingWrite>,
    storage: Box<dyn PersistenceBackend>,
}

// Features:
// ✅ Queue SET/UPDATE/DELETE operations
// ✅ Retry on network reconnect
// ✅ Conflict resolution with server timestamps
```

### Phase 3: Query Result Cache (2-3 days) ⭐⭐
```rust
// Cache query results
// Hash: sha256(collection + filters + order)
pub struct QueryCache {
    results: HashMap<String, Vec<DocumentPath>>,
}
```

### Phase 4: Advanced Features (1-2 weeks) ⭐
- Garbage collection (LRU eviction)
- Cache size limits
- Multi-tab synchronization (WASM)
- Encryption at rest

---

## Recommendation

**For your SDK right now (96% complete, 90 tests passing):**

### Option 1: Minimal Cache (Recommended) - 1-2 days
```rust
// Just enough to say "persistence enabled"
impl Firestore {
    pub fn enable_persistence(&mut self) -> Result<()> {
        // Basic document read cache
        // - Cache successful GET responses
        // - Return cached data when offline
        // - Simple TTL (5 min default)
    }
}
```
**Gets you to 100% feature checklist ✅**

### Option 2: Full Implementation - 2-3 weeks
- Pending writes queue
- Query caching
- Conflict resolution
- Production-grade

---

## Key Takeaways

1. **Persistence ≠ SQL Database**
   - It's a key-value cache + mutation queue
   - Mirrors Firestore's document model

2. **Critical for Mobile/Offline Apps**
   - Instagram, WhatsApp, Google Docs all use it
   - 50-100x faster app startup
   - No data loss during offline periods

3. **WASM Uses IndexedDB**
   - Browser's built-in NoSQL database
   - ~50MB+ storage
   - Same API across browsers

4. **Two Separate Features:**
   - **Read Cache:** Fast reads, reduce costs
   - **Write Queue:** No data loss, optimistic updates

5. **Start Simple:**
   - Basic read cache gets you 80% of value
   - Can enhance later if needed

**Question for you:** Given you're at 96% completion with excellent test coverage, do you want:
- **A) Minimal read cache** (1-2 days, completes checklist)
- **B) Full implementation** (2-3 weeks, production-grade)
- **C) Skip for now** (already production-ready for online use)

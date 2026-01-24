# proxy-spider Architecture

This document provides an overview of the proxy-spider architecture, design decisions, and implementation details.

## 📐 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Main Process                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Config     │  │  HTTP Client │  │ DNS Resolver │      │
│  │   Loader     │  │   Builder    │  │   (Hickory)  │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        │                   │                   │
        ▼                   ▼                   ▼
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│   Scraper    │    │   Checker    │    │   Output     │
│   Module     │───▶│   Module     │───▶│   Module     │
└──────────────┘    └──────────────┘    └──────────────┘
        │                   │                   │
        ▼                   ▼                   ▼
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│  Proxy List  │    │  Validated   │    │  JSON/TXT    │
│  (HashSet)   │    │  Proxies     │    │   Files      │
└──────────────┘    └──────────────┘    └──────────────┘
```

## 🏗️ Core Components

### 1. Configuration System

**Files:** `src/config.rs`, `src/raw_config.rs`, `src/validation.rs`

**Responsibilities:**
- Load and parse TOML configuration
- Validate configuration values
- Convert raw config to processed config
- Handle platform-specific paths (Docker vs native)

**Design Decisions:**
- Two-stage config: `RawConfig` (deserialized) → `Config` (processed)
- Validation happens before processing
- Immutable config wrapped in `Arc` for sharing

**Data Flow:**
```
config.toml → RawConfig → Validation → Config → Arc<Config>
```

### 2. Scraping Engine

**File:** `src/scraper.rs`

**Responsibilities:**
- Fetch proxy lists from multiple sources concurrently
- Parse proxies using regex
- Deduplicate proxies
- Handle various source types (HTTP, HTTPS, file://, local files)

**Architecture:**
```
┌─────────────────────────────────────────┐
│         Scraper Coordinator             │
│  ┌─────────────────────────────────┐   │
│  │  JoinSet<ScraperTask>           │   │
│  │  ┌──────┐ ┌──────┐ ┌──────┐    │   │
│  │  │Task 1│ │Task 2│ │Task N│    │   │
│  │  └──────┘ └──────┘ └──────┘    │   │
│  └─────────────────────────────────┘   │
│              │                          │
│              ▼                          │
│  ┌─────────────────────────────────┐   │
│  │  Shared HashSet<Proxy>          │   │
│  │  (Mutex-protected)               │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

**Key Features:**
- Concurrent scraping with configurable limits
- Automatic deduplication using `HashSet`
- Regex-based parsing supporting multiple formats
- Support for authenticated sources

### 3. Checking Engine

**File:** `src/checker.rs`

**Responsibilities:**
- Validate proxies by making test requests
- Measure response times
- Extract exit IP addresses
- Handle concurrent checking with worker pool

**Architecture:**
```
┌─────────────────────────────────────────┐
│         Checker Coordinator             │
│  ┌─────────────────────────────────┐   │
│  │  Worker Pool (JoinSet)          │   │
│  │  ┌──────┐ ┌──────┐ ┌──────┐    │   │
│  │  │Worker│ │Worker│ │Worker│    │   │
│  │  │  1   │ │  2   │ │  N   │    │   │
│  │  └──────┘ └──────┘ └──────┘    │   │
│  └─────────────────────────────────┘   │
│       │         │         │             │
│       ▼         ▼         ▼             │
│  ┌─────────────────────────────────┐   │
│  │  Shared Queue (Mutex<Vec>)      │   │
│  └─────────────────────────────────┘   │
│              │                          │
│              ▼                          │
│  ┌─────────────────────────────────┐   │
│  │  Checked Proxies (Mutex<Vec>)   │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

**Key Features:**
- Worker pool pattern for efficient concurrency
- Per-proxy HTTP client with custom DNS resolver
- Timeout handling at multiple levels
- Graceful cancellation support

### 4. Output System

**File:** `src/output.rs`

**Responsibilities:**
- Sort proxies by speed or IP
- Generate JSON output with metadata
- Generate plain text output
- Enrich with ASN and geolocation data

**Output Formats:**

**JSON:**
```json
{
  "protocol": "http",
  "host": "192.168.1.1",
  "port": 8080,
  "timeout": 1.23,
  "exit_ip": "1.2.3.4",
  "asn": { ... },
  "geolocation": { ... }
}
```

**Text:**
```
http://192.168.1.1:8080
socks5://user:pass@10.0.0.1:1080
```

### 5. HTTP Client & Retry Middleware

**File:** `src/http.rs`

**Responsibilities:**
- Create configured HTTP clients
- Implement retry logic with exponential backoff
- Custom DNS resolution using Hickory
- Handle various HTTP error scenarios

**Retry Strategy:**
```
Request → Error?
    ├─ Yes → Retryable?
    │   ├─ Yes → Wait (exponential backoff) → Retry
    │   └─ No → Return error
    └─ No → Return response
```

**Retryable Conditions:**
- Connection errors
- Timeout errors
- 5xx server errors
- 429 Too Many Requests
- 408 Request Timeout

### 6. IP Database Integration

**File:** `src/ipdb.rs`

**Responsibilities:**
- Download MaxMind GeoLite2 databases
- Cache databases locally
- Use ETag for efficient updates
- Memory-map databases for fast lookups

**Database Types:**
- ASN (Autonomous System Number)
- Geolocation (City-level)

## 🔄 Data Flow

### Complete Workflow

```
1. Load Config
   config.toml → RawConfig → Validate → Config

2. Initialize
   Create HTTP Client
   Create DNS Resolver
   Download IP Databases (if needed)

3. Scrape
   For each source (parallel):
     Fetch content
     Parse proxies with regex
     Add to shared HashSet (dedup)

4. Check
   Create worker pool
   For each proxy (concurrent):
     Create HTTP client with proxy
     Make test request
     Measure timeout
     Extract exit IP
     Add to checked list

5. Enrich
   For each checked proxy:
     Lookup ASN (if enabled)
     Lookup geolocation (if enabled)

6. Output
   Sort proxies
   Generate JSON (if enabled)
   Generate TXT (if enabled)
   Write to disk
```

## 🧵 Concurrency Model

### Async Runtime

- **Runtime:** Tokio with full features
- **Executor:** Multi-threaded work-stealing
- **Reactor:** Epoll (Linux), Kqueue (macOS), IOCP (Windows)

### Synchronization Primitives

- `Arc<T>`: Shared ownership across tasks
- `Mutex<T>`: Mutual exclusion (parking_lot for performance)
- `JoinSet`: Managing multiple concurrent tasks
- `CancellationToken`: Graceful shutdown

### Concurrency Patterns

**Worker Pool:**
```rust
let queue = Arc::new(Mutex::new(items));
let results = Arc::new(Mutex::new(Vec::new()));

for _ in 0..worker_count {
    let queue = Arc::clone(&queue);
    let results = Arc::clone(&results);
    
    spawn(async move {
        loop {
            let item = queue.lock().pop();
            if item.is_none() { break; }
            
            let result = process(item).await;
            results.lock().push(result);
        }
    });
}
```

**Fan-out/Fan-in:**
```rust
let mut join_set = JoinSet::new();

for source in sources {
    join_set.spawn(scrape_source(source));
}

while let Some(result) = join_set.join_next().await {
    handle_result(result);
}
```

## 🎯 Design Principles

### 1. Performance First

- Zero-copy where possible
- Memory-mapped databases
- Custom allocator support (mimalloc)
- Efficient data structures (foldhash)
- Minimal allocations

### 2. Reliability

- Comprehensive error handling
- Graceful degradation
- Retry logic for transient failures
- Timeout protection
- Cancellation support

### 3. Usability

- Clear error messages
- Sensible defaults
- Extensive configuration options
- Multiple output formats
- Optional TUI for monitoring

### 4. Maintainability

- Modular architecture
- Clear separation of concerns
- Type safety
- Comprehensive testing
- Good documentation

## 🔐 Security Considerations

### Input Validation

- URL validation for all sources
- Proxy format validation
- Configuration value validation
- Timeout limits

### Network Security

- HTTPS by default for check URLs
- TLS verification
- No automatic redirects
- Timeout protection

### Resource Limits

- Configurable concurrency limits
- Memory limits via allocator
- File descriptor limits
- Timeout limits

## 📊 Performance Characteristics

### Time Complexity

- Proxy parsing: O(n) where n = input size
- Deduplication: O(n) with HashSet
- Sorting: O(n log n)
- Checking: O(n/w) where w = worker count

### Space Complexity

- Proxy storage: O(n) where n = unique proxies
- Configuration: O(1)
- Databases: O(1) with mmap

### Scalability

- **Horizontal:** Can process multiple sources in parallel
- **Vertical:** Scales with CPU cores and memory
- **Limits:** File descriptors, network bandwidth

## 🔧 Extension Points

### Adding New Proxy Sources

1. Add URL to config
2. Ensure format is supported by regex
3. (Optional) Add custom headers/auth

### Adding New Output Formats

1. Implement serialization in `output.rs`
2. Add config option
3. Update output logic

### Adding New Validation Methods

1. Extend `Proxy::check()` method
2. Add configuration options
3. Update checker logic

## 📚 Dependencies

### Core Dependencies

- **tokio**: Async runtime
- **reqwest**: HTTP client
- **hickory-resolver**: DNS resolution
- **serde**: Serialization
- **maxminddb**: IP database lookups

### Performance Dependencies

- **mimalloc**: Fast allocator
- **parking_lot**: Fast synchronization
- **foldhash**: Fast hashing

### UI Dependencies

- **ratatui**: Terminal UI
- **crossterm**: Terminal control

## 🎓 Learning Resources

- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Async Book](https://rust-lang.github.io/async-book/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [MaxMind GeoIP2](https://dev.maxmind.com/geoip/docs/databases)

## 🤝 Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on contributing to the architecture.

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## About

Rust implementation of a basic SQL database, following Edward Sciore's book _Database Design and Implementation_ (Java reference: https://link.springer.com/book/10.1007/978-3-030-33836-7). README.md maps implemented features to book chapters.

Two deliberate deviations from the book:

- **B-tree index** — single file, non-recursive `insert`/`get`/`delete`.
- **JOINs** — both `SELECT a, b FROM a, b` and `SELECT a, b FROM a JOIN b ON a = b` are accepted.

## Commands

```bash
cargo build
cargo test                          # all crates
cargo test -p engine                # one crate
cargo test -p engine select_with_index   # one test by name
cargo check

# CI gates — both must pass, clippy warnings are errors
cargo fmt --all -- --check
cargo clippy --all-features -- -D warnings

# Benchmarks (criterion; CI fails on >150% regression)
cargo bench -p engine --bench db_benchmark

# Embedded REPL (`sql>` prompt, data dir defaults to ./data)
cargo run -p server --bin cli -- --path ./data

# TCP server speaking the `protocol` wire format
cargo run -p server --bin server -- --port 6543 --path ./data
```

CI (`.github/workflows/rust.yml`) runs on a **self-hosted** runner. The bench job clears `target/criterion` first because the workspace persists between runs.

## Crate Architecture

Workspace is layered bottom-up; higher crates depend on lower ones:

```
server (cli + server binaries)
  ├── protocol (binary wire frames)
  └── engine (core DB + SQL layer, incl. B-tree index)
        └── transaction
              ├── buffer
              │     └── log
              │           └── file
              └── common (DbError / DbResult / timed locks)
```

- **`common`** — `DbError` (via `thiserror`), `DbResult<T>`, and `locks` (see Key Patterns).
- **`file`** — `FileMgr` manages named files as fixed-size block sequences; `Page` is an in-memory byte buffer (`bytes::BytesMut`); `BlockId` is a file+block pair. `FileHolder` is a private LRU cache capped at `MAX_OPEN_FILES = 100` — external merge sort spills each run to a uniquely-named temp table, so uncapped handles hit the OS limit. Evicted files reopen transparently.
- **`log`** — write-ahead log (`LogMgr`) over `FileMgr`.
- **`buffer`** — `BufferMgr` pins/unpins `Buffer`s backed by `Page`s, flushing to log on eviction.
- **`transaction`** — `Transaction` wraps `ConcurrencyMgr` (S/X locks over `LockTable`), `RecoveryMgr` (undo logging), and `BufferList` (per-tx pinned buffers).
- **`engine`** — everything above the transaction layer: storage, scans, plans, index, and the SQL layer. This is where nearly all the work happens.
- **`protocol`** — `DbRequest` (`Query` / `Execute` / `Next` / `GetField`) and `DbResponse` (`Ok` / `Err` / `Execute` / `HasNext` / `Value`), each implementing the `Frame` trait's `read`/`write` over any `Read`/`Write`. Single-byte type tags, length-prefixed strings. Depends on `engine` for `Value`.
- **`server`** — `cli.rs` is an embedded REPL (routes `select` to `db.query()` and prints a formatted table, everything else to `db.execute()`). `server.rs` binds a `TcpListener` and spawns a thread per connection; `session.rs` holds one `Transaction` plus an optional open `Scan` per client, so `Next`/`GetField` step a cursor left open by a prior `Query`.

### Inside `engine`

`SimpleDB` (`engine/src/lib.rs`) is the handle. `new(dir)` uses defaults (8 KiB blocks, 1024 buffers, `wal.log`); `configured(dir, block_size, num_buffers)` overrides them — tests use small values like `configured(dir, 512, 8)` to force page splits and buffer pressure. Construction recovers an existing database or bootstraps a new one, then commits. `query()` returns `Rc<dyn Scan>`, `execute()` returns an affected-row count; both take `&Arc<Transaction>`.

**`Element` is the identifier type used everywhere** (`element.rs`) — the most connected type in the codebase, and the thing to understand first:

```rust
Element::Raw("id")          // bare identifier
Element::Spec("t", "id")    // qualified: t.id
Element::View("users", "u") // aliased: users u
Element::Array(vec![..])    // a list, e.g. multiple tables in FROM
```

`Scan`, `Plan`, `Schema`, and `Predicate` all take `&Element`, never `&str`. Field lookup is alias-aware: `SchemaInner::info` resolves `Spec(table, field)` against its own table name. `SchemaMapping` (private, `schema_mapping.rs`) carries the alias→source resolution built by the parser through `QueryData` into the planners and `ProjectPlan`.

`Value` (`value.rs`) is the runtime cell value: `Integer(i32)` | `Varchar(String)`.

**Storage** — `Schema` (immutable `Arc<SchemaInner>`, built via `SchemaBuilder`), `Layout` (byte offsets of fields within a slot), `RecordPage` (fixed-length slots on one `BlockId` through a `Transaction`), `RID` (block number + slot number).

**Scan / Plan** (`scan.rs`, `plan.rs`) — `Plan` estimates cost (`blocks_accessed`, `records_output`, `distinct_values`) and opens an `Rc<dyn Scan>`. `Scan` is a cursor: read methods are required, write methods (`set_*`, `insert`, `delete`, `move_to_rid`, `save_position`) default to `Err`, so read-only scans get them for free. Every plan module under `plan/` has a matching scan under `scan/`.

**Query operators** — `index/b_tree/` (`BTreePage` is an enum of `Metadata` / `Node` / `Leaf`; duplicate keys spill to overflow pages), `plan/order.rs` + `sort_by.rs` (external merge sort backing `ORDER BY`), `plan/group.rs` + `scan/group.rs` (`GROUP BY` with `AggregationFn`), `plan/merge.rs` (merge join), `plan/multibuffer.rs` + `scan/chunk.rs` (multibuffer product), `plan/materialize.rs`, `temp.rs` (private; temp tables named `temp_{N}` from a global atomic counter), `buffer_needs.rs` (`best_root` / `best_factor` for per-operator buffer reservation).

**SQL layer** (`engine/src/query/`) — `tokenizer` → `lexer` → `parser` → `Command` (an enum over `SELECT`, `INSERT`, `UPDATE`, `DELETE`, `CREATE TABLE/VIEW/INDEX`). `Planner` parses, then dispatches to `QueryPlanner` (→ `Rc<dyn Plan>`) or `UpdatePlanner` (→ affected rows). The only implementations are `HeuristicQueryPlanner` and `HeuristicUpdatePlanner` in `query/heuristic_planner/`.

`TablePlanner` (`query/table_planner.rs`) is the optimizer's unit of work: one per table in the `FROM`, holding that table's `TablePlan`, predicate, and index info. `HeuristicQueryPlanner` greedily takes the lowest-`records_output` select plan, then repeatedly joins in whichever remaining table produces the fewest rows — preferring an index join, falling back to multibuffer product — before layering `GroupByPlan`, `SortPlan`, and finally `ProjectPlan`.

**Metadata** (`metadata_mgr.rs`) — `MetadataMgr` aggregates `TableMgr`, `ViewMgr`, `StatMgr`, `IndexMgr`. Catalog tables (`table_catalog`, `field_catalog`, …) are ordinary tables on disk.

**Predicates** (`predicate.rs`) — `Predicate` is a conjunction of `Term`s comparing two `Expression`s. `join_sub_pred` / `select_sub_pred` extract the portion of a predicate applicable to a given schema pair; this is what drives join selection.

## Key Patterns

- All fallible operations return `DbResult<T>`.
- **Never use `std::sync::Mutex`/`RwLock` directly for DB state.** Use `common::locks::{TimedMutex, TimedRwLock}` (or `lock_with_timeout`), which spin with a 10 s timeout (1 s under `cfg!(test)`) and return `DbError::LockTimeout` instead of deadlocking.
- **Interior mutability via a private `*Lock` inner struct.** `Scan` and `Index` methods take `&self`, so mutable state lives behind a `RefCell`/`Mutex` wrapper — e.g. `pub struct ChunkScan(RefCell<ChunkScanLock>)`, `BufferListLock`, `StatMgrLock`, `MergeJoinScanLock`, `BTreeIndexInner`. Follow this shape when adding a scan.
- **`Rc` for query execution, `Arc` for managers.** Plans and scans are single-threaded (`Rc<dyn Plan>` / `Rc<dyn Scan>`); `FileMgr`, `LogMgr`, `BufferMgr`, `LockTable`, `Transaction` are shared across threads (`Arc`). A `Session` is pinned to one thread for exactly this reason.
- Tests use `tempfile::tempdir()`, so no cleanup is needed. `engine/src/lib.rs` exposes `init()` / `init_with_size(block_size)` test helpers that build a `Transaction` over a temp dir.
- `tracing` for logging, not `println!` (the CLI's table output is the exception).

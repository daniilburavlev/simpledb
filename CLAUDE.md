# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## About

Rust implementation of a basic SQL database, following Edward Sciore's book _Database Design and Implementation_. The original Java reference is available at https://link.springer.com/book/10.1007/978-3-030-33836-7.

## Commands

```bash
# Build
cargo build

# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p table

# Run a single test by name
cargo test -p table schema_test_name

# Check without building
cargo check

# Run the interactive SQL REPL (data dir defaults to ./data)
cargo run -p server --bin cli -- --path ./data
```

## Crate Architecture

The workspace is layered bottom-up; higher crates depend on lower ones:

```
server (CLI / REPL)          engine, network (stubs)
  └── table (core DB logic + SQL layer, incl. B-tree index)
        └── transaction
              ├── buffer
              │     └── log
              │           └── file
              └── common (DbError / DbResult)
```

### Layer responsibilities

- **`common`** — `DbError` enum (via `thiserror`) and `DbResult<T>` alias used everywhere.
- **`file`** — Block-based disk I/O: `FileMgr` manages named files as sequences of fixed-size blocks; `Page` is an in-memory byte buffer (`bytes::BytesMut`); `BlockId` identifies a file+block pair.
- **`log`** — Write-ahead log (`LogMgr`) on top of `FileMgr`.
- **`buffer`** — Buffer pool (`BufferMgr`) that pins/unpins `Buffer` objects backed by `Page`s, with flush-to-log on eviction.
- **`transaction`** — ACID transactions: `Transaction` wraps `ConcurrencyMgr` (S/X locks via `LockTable`), `RecoveryMgr` (undo logging), and `BufferList` (per-tx pinned buffers). `TxNumGenerator` produces monotone transaction IDs atomically.
- **`table`** — Everything above the transaction layer, including the B-tree index (`table/src/index/`) and the SQL layer (see below).
- **`server`** — The actual entry point: `server/src/bin/cli.rs` is an interactive `sql>` REPL. It opens a `SimpleDB`, then routes each line to `db.query()` (for `SELECT`, prints a formatted table) or `db.execute()` (for DML/DDL, prints affected-row count). Run with `cargo run -p server --bin cli -- --path <dir>` (defaults to `./data`).
- **`engine`** / **`network`** — Empty stubs (placeholder `fn main` / `add`), reserved for future work. The CLI does **not** live in `engine` despite its `engine/src/bin/cli.rs` (also a stub).

### Inside `table`

`SimpleDB` (in `table/src/lib.rs`) is the database handle. `new(dir)` uses defaults; `configured(dir, block_size, num_buffers)` overrides them. It owns `Arc` references to `FileMgr`, `LogMgr`, `BufferMgr`, `LockTable`, and `MetadataMgr`, and produces new `Transaction`s via `get_tx()`. The two SQL entry points take a `&Arc<Transaction>`: `query()` returns `Rc<dyn Scan>` for `SELECT`, `execute()` returns the affected-row count for everything else.

**Storage layer**

- `Schema` — field names and types (`INTEGER` / `VARCHAR(n)`), protected internally by `RwLock`.
- `Layout` — computes byte offsets for fields within a slot; built from a `Schema`.
- `RecordPage` — reads/writes fixed-length slots on a single `BlockId` through a `Transaction`.
- `RID` — record identifier: (block number, slot number).

**Scan / Plan traits** (`table/src/scan.rs`, `table/src/plan.rs`)

- `Scan` — cursor over records; read methods + optional write methods that default to `Err`.
- `Plan` — cost-estimation node + `open() -> Rc<dyn Scan>`. Implementations: `TablePlan`, `SelectPlan`, `ProjectPlan`, `ProductPlan`, `IndexPlan`.
- Scan implementations mirror plans: `TableScan`, `SelectScan`, `ProjectScan`, `ProductScan`, `IndexScan`.
- Plans use `Rc<dyn Plan>` / `Rc<dyn Scan>` (single-threaded query execution); managers use `Arc`.

**Query operators** (beyond the core scans above)

- `index/` — B-tree index: `BTreeIndex` over `BTreePage` (leaf/dir pages), with `BTreeLeaf`, `BTreeDir`, and `DirEntry`. `IndexMgr` (`index_mgr.rs`) tracks index metadata.
- `sort/` + `sort_by.rs` — external merge sort (`SortPlan`/`SortScan`), backing `ORDER BY`. Spills runs to temp tables.
- `group/` + `group_by.rs` — `GROUP BY` with aggregation functions (`AggregationFn`).
- `merge/` — merge join (`MergeJoinPlan`/`MergeJoinScan`).
- `temp/` (private `temp` mod) — temporary materialized tables used by sort/merge.
- `constant.rs` (`Constant`), `field_info.rs` (`FieldInfo`: `Integer` / `Varchar(n)`), `buffer_needs.rs` (per-operator buffer reservation).

**SQL layer** (`table/src/query/`)

- `tokenizer` → `lexer` → `parser` → `Command` (an enum covering `SELECT`, `INSERT`, `UPDATE`, `DELETE`, `CREATE TABLE/VIEW/INDEX`).
- `Planner` dispatches to `QueryPlanner` (returns a `Plan`) or `UpdatePlanner` (executes DML, returns affected row count).
- `basic_planner` contains the default `BasicQueryPlanner` / `BasicUpdatePlanner` implementations.

**Metadata** (`table/src/metadata_mgr.rs`)

- `MetadataMgr` aggregates `TableMgr`, `ViewMgr`, `StatMgr`, and `IndexMgr`.
- Catalog tables (`table_catalog`, `field_catalog`, etc.) are stored as regular tables on disk.

**Predicates** (`table/src/predicate.rs`)

- `Predicate` is a conjunction of `Term`s; each `Term` compares two `Expression`s (field name or `Constant`).
- `Constant` is either `Integer(i32)` or `Str(String)`.

## Key Patterns

- All fallible operations return `DbResult<T>` (`Result<T, DbError>`).
- Interior mutability in `Schema` uses `RwLock`; lock poisoning maps to `DbError::Lock`.
- Integration tests use `tempfile::tempdir()` so no cleanup is needed.
- `tracing` is used for debug/info logging (not `println!`).


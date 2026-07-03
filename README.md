# SimpleDB

## Overview

Rust implementation of basic SQL database, originally implemented in Java by the Edward Sciore's book ["Database Design and implementation"](https://link.springer.com/book/10.1007/978-3-030-33836-7).

## SQL

simpledb is a database engine that can handle a subset of SQL. The following is the examples of commands which can be accepted by simpledb.

```sql
-- Creating tables
CREATE TABLE users(id INT, name VARCHAR(16));
-- Insertions
INSERT INTO users(id, name) VALUES(1, 'User');
-- Queries
SELECT u.id, u.name FROM users u where u.id=1;
-- Removal
DELETE FROM users WHERE id=1;
-- Updating
UPDATE users SET name='Other' WHERE id=1;
-- Index creation
CREATE INDEX users_ids ON users(id);
-- View for joins
CREATE TABLE salaries(user_id INT, amount INT);
CREATE VIEW users_salaries AS select id, amount FROM users, salaries WHERE id=user_id;
```

## Features
| Book Chapter | Feature                                    | Implemented        |
|--------------|--------------------------------------------|--------------------|
| 3            | File Manager                               | :heavy_check_mark: |
| 4            | Log Manager                                | :heavy_check_mark: |
| 4            | Buffer Manager                             | :heavy_check_mark: |
| 5            | Recovery Manager                           | :heavy_check_mark: |
| 5            | Concurrency Manager                        | :heavy_check_mark: |
| 5            | Transaction                                | :heavy_check_mark: |
| 6            | Record Pages                               | :heavy_check_mark: |
| 6            | Table Scans                                | :heavy_check_mark: |
| 7            | Metadata Manager                           | :heavy_check_mark: |
| 8            | Select Scans, Project Scans, Product Scans | :heavy_check_mark: |
| 9            | Parser                                     | :heavy_check_mark: |
| 10           | Planner                                    | :heavy_check_mark: |
| 11           | Embedded JDBC Interface                    | :heavy_check_mark: |
| 11           | Remote JDBC Interface                      | :x:                |
| 12           | Static Hash Indexes                        | :x:                |
| 12           | Btree Indexes                              | :heavy_check_mark: |
| 13           | Materialization and Sorting                | :heavy_check_mark: |
| 14           | MultiBuffer Sorting/Product                | :heavy_check_mark: |
| 15           | Query Optimization                         | :heavy_check_mark: |

The Remote JDBC Interface and network interface is in development(DB_00003 branch)

## Key differences

- `B-Tree Index` - implemented as single file structure, without using the recursion on `insert`, `get` and `delete`
- `JOINs` - can be executed in two types of queries `SELECT a, b FROM a, b` and `SELECT a, b FROM a JOIN b ON a = b`

# Getting started

## Build

Clone this repository
```bash
cargo build --release
```

Then the executables will be generated in the `target/release` folder.

## Usage

cd target/release

```bash
./cli
```

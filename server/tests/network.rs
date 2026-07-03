//! End-to-end test of the server's TCP protocol: frame requests, read framed
//! responses, and check that a committed write survives across connections.
//!
//! `Runner` is `!Send` (the engine is `Rc`-based), so the server runs on the
//! test's main thread and the client drives it from a spawned thread.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicU16, Ordering};

use engine::value::Value;
use network::model::{DbRequest, DbResponse};
use server::Runner;

/// Hands out distinct ports so concurrently-run tests don't collide.
static NEXT_PORT: AtomicU16 = AtomicU16::new(15_432);

fn write_frame(stream: &mut TcpStream, payload: &[u8]) {
    stream
        .write_all(&(payload.len() as u32).to_be_bytes())
        .unwrap();
    stream.write_all(payload).unwrap();
    stream.flush().unwrap();
}

fn read_frame(stream: &mut TcpStream) -> Vec<u8> {
    let mut len = [0u8; 4];
    stream.read_exact(&mut len).unwrap();
    let mut buf = vec![0u8; u32::from_be_bytes(len) as usize];
    stream.read_exact(&mut buf).unwrap();
    buf
}

fn call(stream: &mut TcpStream, request: DbRequest) -> DbResponse {
    write_frame(stream, &request.to_bytes());
    DbResponse::from_bytes(&read_frame(stream)).unwrap()
}

fn tag(response: &DbResponse) -> &'static str {
    match response {
        DbResponse::Field(_) => "Field",
        DbResponse::Int(_) => "Int",
        DbResponse::String(_) => "String",
        DbResponse::HasNext(_) => "HasNext",
        DbResponse::Executed(_) => "Executed",
    }
}

#[test]
fn round_trip_over_tcp() {
    let dir = tempfile::tempdir().unwrap();
    let port = NEXT_PORT.fetch_add(1, Ordering::SeqCst);
    // The listener is bound here, so client connects queue in the backlog even
    // before the server calls `accept` — no startup sleep needed.
    let runner = Runner::new(port, dir.path()).unwrap();

    let client = std::thread::spawn(move || {
        let addr = ("127.0.0.1", port);

        // Connection 1: DDL + DML, then disconnect to commit the transaction.
        let mut writer = TcpStream::connect(addr).unwrap();
        assert!(matches!(
            call(
                &mut writer,
                DbRequest::Execute("CREATE TABLE t(id INT, name VARCHAR(16))".into()),
            ),
            DbResponse::Executed(_)
        ));
        assert!(matches!(
            call(
                &mut writer,
                DbRequest::Execute("INSERT INTO t(id, name) VALUES(7, 'Alice')".into()),
            ),
            DbResponse::Executed(1)
        ));
        drop(writer);

        // Connection 2: read the committed row back and exercise every getter.
        let mut reader = TcpStream::connect(addr).unwrap();
        assert!(matches!(
            call(
                &mut reader,
                DbRequest::Query("SELECT id, name FROM t WHERE id = 7".into()),
            ),
            DbResponse::Executed(0)
        ));
        assert!(matches!(
            call(&mut reader, DbRequest::NextRow),
            DbResponse::HasNext(true)
        ));
        assert!(matches!(
            call(&mut reader, DbRequest::GetInt("id".into())),
            DbResponse::Int(7)
        ));
        match call(&mut reader, DbRequest::GetString("name".into())) {
            DbResponse::String(name) => assert_eq!(name, "Alice"),
            other => panic!("expected string, got {}", tag(&other)),
        }
        assert!(matches!(
            call(&mut reader, DbRequest::GetField("id".into())),
            DbResponse::Field(Value::Integer(7))
        ));
        assert!(matches!(
            call(&mut reader, DbRequest::NextRow),
            DbResponse::HasNext(false)
        ));
    });

    // Serve exactly the two connections the client opens; each returns once the
    // client drops its end.
    runner.serve_one().unwrap();
    runner.serve_one().unwrap();

    client.join().unwrap();
}
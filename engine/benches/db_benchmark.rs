use criterion::{Criterion, criterion_group, criterion_main};
use engine::SimpleDB;
use rand::RngExt;
use tempfile::tempdir;

fn bench_insert(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let db = SimpleDB::new(dir.path()).unwrap();
    let tx = db.get_tx().unwrap();
    db.execute(&tx, "CREATE TABLE test(id INT)").unwrap();
    tx.commit().unwrap();
    let mut rng = rand::rng();
    c.bench_function("insert", |b| {
        b.iter(|| {
            let id = rng.random::<i32>();
            db.execute(&tx, &format!("INSERT INTO test(id) VALUES({})", id))
                .unwrap();
            db.query(&tx, "SELECT * FROM test").unwrap();
        });
    });
}

fn bench_index_insert(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let db = SimpleDB::new(dir.path()).unwrap();
    let tx = db.get_tx().unwrap();
    db.execute(&tx, "CREATE TABLE test(id INT)").unwrap();
    db.execute(&tx, "CREATE INDEX test_ids ON test(id)")
        .unwrap();
    tx.commit().unwrap();
    let mut rng = rand::rng();
    c.bench_function("index_insert", |b| {
        b.iter(|| {
            let id = rng.random::<i32>();
            db.execute(&tx, &format!("INSERT INTO test(id) VALUES({})", id))
                .unwrap();
            db.query(&tx, &format!("SELECT id FROM test WHERE id={}", id))
                .unwrap();
        });
    });
}

fn bench_join(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let db = SimpleDB::new(dir.path()).unwrap();
    let tx = db.get_tx().unwrap();
    db.execute(&tx, "CREATE TABLE t1(i1 INT)").unwrap();
    db.execute(&tx, "CREATE TABLE t2(i2 INT)").unwrap();
    tx.commit().unwrap();
    for i in 0..100 {
        db.execute(&tx, &format!("INSERT INTO t1(i1) VALUES({})", i))
            .unwrap();
        db.execute(&tx, &format!("INSERT INTO t2(i2) VALUES({})", i))
            .unwrap();
    }
    tx.commit().unwrap();
    c.bench_function("join", |b| {
        b.iter(|| {
            for i in 0..10 {
                let result = db
                    .query(
                        &tx,
                        &format!("SELECT i1, i2 FROM t1, t2 WHERE i1 = i2 AND i1 = {}", i),
                    )
                    .unwrap();
                result.close().unwrap();
            }
        });
    });
}

criterion_group!(benches, bench_insert, bench_join, bench_index_insert);
criterion_main!(benches);

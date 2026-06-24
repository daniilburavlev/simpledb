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

criterion_group!(benches, bench_insert);
criterion_main!(benches);

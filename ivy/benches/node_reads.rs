use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ivy::{IvyBuilder, NodeId, Value};
use tempfile::tempdir;

fn bench_single_get_node(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let ids: Vec<NodeId> = (0..1000)
        .map(|i| {
            db.add_node(
                "Person",
                vec![
                    ("name".to_string(), Value::String(format!("user_{i}"))),
                    ("age".to_string(), Value::Int(i)),
                    (
                        "bio".to_string(),
                        Value::String("A somewhat long biography string for realism.".to_string()),
                    ),
                ],
            )
            .unwrap()
        })
        .collect();

    let mut idx = 0usize;
    c.bench_function("node_get_single", |b| {
        b.iter(|| {
            let id = ids[idx % ids.len()];
            idx += 1;
            db.get_node(id).unwrap();
        });
    });
}

fn bench_bulk_get_nodes(c: &mut Criterion) {
    let mut group = c.benchmark_group("node_get_bulk");

    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let ids: Vec<NodeId> = (0..1000)
        .map(|i| {
            db.add_node(
                "Person",
                vec![
                    ("name".to_string(), Value::String(format!("user_{i}"))),
                    ("age".to_string(), Value::Int(i)),
                ],
            )
            .unwrap()
        })
        .collect();

    for size in [10, 100, 1000] {
        let batch: Vec<NodeId> = ids.iter().take(size).copied().collect();
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &batch, |b, batch| {
            b.iter(|| {
                db.get_nodes(batch).unwrap();
            });
        });
    }
    group.finish();
}

fn bench_read_transaction(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let ids: Vec<NodeId> = (0..500)
        .map(|i| {
            db.add_node(
                "Person",
                vec![
                    ("name".to_string(), Value::String(format!("user_{i}"))),
                    ("age".to_string(), Value::Int(i)),
                ],
            )
            .unwrap()
        })
        .collect();

    c.bench_function("read_txn_multi_get", |b| {
        b.iter(|| {
            db.read(|tx| {
                for &id in &ids[..50] {
                    tx.get_node(id)?;
                }
                Ok(())
            })
            .unwrap();
        });
    });
}

criterion_group!(
    benches,
    bench_single_get_node,
    bench_bulk_get_nodes,
    bench_read_transaction
);
criterion_main!(benches);

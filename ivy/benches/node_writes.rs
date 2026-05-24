use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ivy::{IvyBuilder, Value};
use tempfile::tempdir;

fn bench_single_node_insert(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let mut i = 0u64;
    c.bench_function("node_insert_single", |b| {
        b.iter(|| {
            i += 1;
            db.add_node(
                "Person",
                vec![
                    ("name".to_string(), Value::String(format!("user_{i}"))),
                    ("age".to_string(), Value::Int(25)),
                ],
            )
            .unwrap();
        });
    });
}

fn bench_batch_node_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("node_insert_batch");

    for size in [100, 1_000, 10_000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let dir = tempdir().unwrap();
                    let db = IvyBuilder::new().open(dir.path()).unwrap();
                    (dir, db)
                },
                |(_dir, db)| {
                    db.batch(|tx| {
                        for i in 0..size {
                            tx.add_node(
                                "Person",
                                vec![
                                    ("name".to_string(), Value::String(format!("user_{i}"))),
                                    ("age".to_string(), Value::Int(i as i64)),
                                    (
                                        "email".to_string(),
                                        Value::String(format!("user_{i}@example.com")),
                                    ),
                                ],
                            )?;
                        }
                        Ok(())
                    })
                    .unwrap();
                },
                criterion::BatchSize::PerIteration,
            );
        });
    }
    group.finish();
}

fn bench_node_insert_with_index(c: &mut Criterion) {
    let mut group = c.benchmark_group("node_insert_with_property_index");

    for size in [100, 1_000, 5_000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let dir = tempdir().unwrap();
                    let db = IvyBuilder::new().open(dir.path()).unwrap();
                    db.add_node(
                        "Person",
                        vec![("name".to_string(), Value::String("seed".to_string()))],
                    )
                    .unwrap();
                    db.indexes()
                        .nodes()
                        .create_property("Person", "name")
                        .unwrap();
                    (dir, db)
                },
                |(_dir, db)| {
                    db.batch(|tx| {
                        for i in 0..size {
                            tx.add_node(
                                "Person",
                                vec![
                                    ("name".to_string(), Value::String(format!("user_{i}"))),
                                    ("age".to_string(), Value::Int(i as i64)),
                                ],
                            )?;
                        }
                        Ok(())
                    })
                    .unwrap();
                },
                criterion::BatchSize::PerIteration,
            );
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_single_node_insert,
    bench_batch_node_insert,
    bench_node_insert_with_index
);
criterion_main!(benches);

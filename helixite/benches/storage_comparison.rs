use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use helixite::storage::MemoryStorage;
use helixite::{Helixite, HelixiteBuilder, NodeId, Value};
use tempfile::TempDir;

fn open_lmdb() -> (TempDir, Helixite) {
    let dir = tempfile::tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();
    (dir, db)
}

fn open_memory() -> Helixite<MemoryStorage> {
    let storage = MemoryStorage::new();
    HelixiteBuilder::new()
        .storage(storage)
        .open("/dev/null")
        .unwrap()
}

fn bench_write_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_write_comparison");
    group.throughput(Throughput::Elements(500));

    group.bench_function("lmdb_500_nodes", |b| {
        b.iter_batched(
            open_lmdb,
            |(_dir, db)| {
                db.batch(|tx| {
                    for i in 0..500 {
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

    group.bench_function("memory_500_nodes", |b| {
        b.iter_batched(
            open_memory,
            |db| {
                db.batch(|tx| {
                    for i in 0..500 {
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

    group.finish();
}

fn bench_read_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_read_comparison");
    group.throughput(Throughput::Elements(100));

    group.bench_function("lmdb_100_reads", |b| {
        let (_dir, db) = open_lmdb();
        let ids: Vec<NodeId> = (0..100)
            .map(|i| {
                db.add_node(
                    "Person",
                    vec![("name".to_string(), Value::String(format!("user_{i}")))],
                )
                .unwrap()
            })
            .collect();

        b.iter(|| {
            for &id in &ids {
                db.get_node(id).unwrap();
            }
        });
    });

    group.bench_function("memory_100_reads", |b| {
        let db = open_memory();
        let ids: Vec<NodeId> = (0..100)
            .map(|i| {
                db.add_node(
                    "Person",
                    vec![("name".to_string(), Value::String(format!("user_{i}")))],
                )
                .unwrap()
            })
            .collect();

        b.iter(|| {
            for &id in &ids {
                db.get_node(id).unwrap();
            }
        });
    });

    group.finish();
}

fn bench_traversal_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_traversal_comparison");

    group.bench_function("lmdb_traversal", |b| {
        let (_dir, db) = open_lmdb();
        let center = db
            .add_node(
                "Person",
                vec![("name".to_string(), Value::String("center".to_string()))],
            )
            .unwrap();
        db.batch(|tx| {
            for i in 0..50 {
                let other = tx.add_node(
                    "Person",
                    vec![("name".to_string(), Value::String(format!("friend_{i}")))],
                )?;
                tx.add_edge(center, other, "knows", Vec::new())?;
            }
            Ok(())
        })
        .unwrap();

        b.iter(|| {
            let nodes = db.node(center).outgoing("knows").nodes().unwrap();
            assert_eq!(nodes.len(), 50);
            nodes
        });
    });

    group.bench_function("memory_traversal", |b| {
        let db = open_memory();
        let center = db
            .add_node(
                "Person",
                vec![("name".to_string(), Value::String("center".to_string()))],
            )
            .unwrap();
        db.batch(|tx| {
            for i in 0..50 {
                let other = tx.add_node(
                    "Person",
                    vec![("name".to_string(), Value::String(format!("friend_{i}")))],
                )?;
                tx.add_edge(center, other, "knows", Vec::new())?;
            }
            Ok(())
        })
        .unwrap();

        b.iter(|| {
            let nodes = db.node(center).outgoing("knows").nodes().unwrap();
            assert_eq!(nodes.len(), 50);
            nodes
        });
    });

    group.finish();
}

fn bench_query_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_query_comparison");

    for label in ["lmdb", "memory"] {
        group.bench_with_input(
            BenchmarkId::new("label_query_1000", label),
            &label,
            |b, &backend| match backend {
                "lmdb" => {
                    let (_dir, db) = open_lmdb();
                    db.batch(|tx| {
                        for i in 0..1000 {
                            tx.add_node(
                                "Person",
                                vec![("name".to_string(), Value::String(format!("u{i}")))],
                            )?;
                        }
                        Ok(())
                    })
                    .unwrap();

                    b.iter(|| {
                        let count = db.nodes().label("Person").count().unwrap();
                        assert_eq!(count, 1000);
                        count
                    });
                }
                "memory" => {
                    let db = open_memory();
                    db.batch(|tx| {
                        for i in 0..1000 {
                            tx.add_node(
                                "Person",
                                vec![("name".to_string(), Value::String(format!("u{i}")))],
                            )?;
                        }
                        Ok(())
                    })
                    .unwrap();

                    b.iter(|| {
                        let count = db.nodes().label("Person").count().unwrap();
                        assert_eq!(count, 1000);
                        count
                    });
                }
                _ => unreachable!(),
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_write_comparison,
    bench_read_comparison,
    bench_traversal_comparison,
    bench_query_comparison
);
criterion_main!(benches);

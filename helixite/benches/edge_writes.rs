use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use helixite::{HelixiteBuilder, Value};
use tempfile::tempdir;

fn bench_single_edge_insert(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let nodes: Vec<_> = (0..1000)
        .map(|i| {
            db.add_node(
                "Person",
                vec![("name".to_string(), Value::String(format!("u{i}")))],
            )
            .unwrap()
        })
        .collect();

    let mut idx = 0usize;
    c.bench_function("edge_insert_single", |b| {
        b.iter(|| {
            let from = nodes[idx % nodes.len()];
            let to = nodes[(idx + 1) % nodes.len()];
            idx += 1;
            db.add_edge(
                from,
                to,
                "knows",
                vec![("weight".to_string(), Value::Float(0.5))],
            )
            .unwrap();
        });
    });
}

fn bench_batch_edge_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("edge_insert_batch");

    for size in [100, 1_000, 10_000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let dir = tempdir().unwrap();
                    let db = HelixiteBuilder::new().open(dir.path()).unwrap();
                    let nodes: Vec<_> = (0..size)
                        .map(|i| {
                            db.add_node(
                                "Person",
                                vec![("name".to_string(), Value::String(format!("u{i}")))],
                            )
                            .unwrap()
                        })
                        .collect();
                    (dir, db, nodes)
                },
                |(_dir, db, nodes)| {
                    db.batch(|tx| {
                        for i in 0..size {
                            let from = nodes[i % nodes.len()];
                            let to = nodes[(i + 1) % nodes.len()];
                            tx.add_edge(
                                from,
                                to,
                                "knows",
                                vec![("since".to_string(), Value::Int(2020 + (i as i64 % 5)))],
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

fn bench_edge_insert_with_property_index(c: &mut Criterion) {
    let mut group = c.benchmark_group("edge_insert_with_property_index");

    for size in [100, 1_000, 5_000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let dir = tempdir().unwrap();
                    let db = HelixiteBuilder::new().open(dir.path()).unwrap();
                    let seed_a = db
                        .add_node(
                            "Person",
                            vec![("name".to_string(), Value::String("sa".to_string()))],
                        )
                        .unwrap();
                    let seed_b = db
                        .add_node(
                            "Person",
                            vec![("name".to_string(), Value::String("sb".to_string()))],
                        )
                        .unwrap();
                    db.add_edge(
                        seed_a,
                        seed_b,
                        "knows",
                        vec![("since".to_string(), Value::Int(2020))],
                    )
                    .unwrap();
                    db.indexes()
                        .edges()
                        .create_property("knows", "since")
                        .unwrap();
                    let nodes: Vec<_> = (0..size)
                        .map(|i| {
                            db.add_node(
                                "Person",
                                vec![("name".to_string(), Value::String(format!("u{i}")))],
                            )
                            .unwrap()
                        })
                        .collect();
                    (dir, db, nodes)
                },
                |(_dir, db, nodes)| {
                    db.batch(|tx| {
                        for i in 0..size {
                            let from = nodes[i % nodes.len()];
                            let to = nodes[(i + 1) % nodes.len()];
                            tx.add_edge(
                                from,
                                to,
                                "knows",
                                vec![("since".to_string(), Value::Int(2020 + (i as i64 % 5)))],
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
    bench_single_edge_insert,
    bench_batch_edge_insert,
    bench_edge_insert_with_property_index
);
criterion_main!(benches);

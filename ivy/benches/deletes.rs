use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ivy::{IvyBuilder, NodeId, Value};

fn bench_delete_node_no_edges(c: &mut Criterion) {
    c.bench_function("delete_node_no_edges", |b| {
        b.iter_batched(
            || {
                let dir = tempfile::tempdir().unwrap();
                let db = IvyBuilder::new().open(dir.path()).unwrap();
                let ids: Vec<NodeId> = (0..100)
                    .map(|i| {
                        db.add_node(
                            "Person",
                            vec![("name".to_string(), Value::String(format!("user_{i}")))],
                        )
                        .unwrap()
                    })
                    .collect();
                (dir, db, ids)
            },
            |(_dir, db, ids)| {
                for id in ids {
                    db.delete_node(id).unwrap();
                }
            },
            criterion::BatchSize::PerIteration,
        );
    });
}

fn bench_delete_node_with_edges(c: &mut Criterion) {
    let mut group = c.benchmark_group("delete_node_with_edges");

    for edge_count in [5, 20, 50] {
        group.bench_with_input(
            BenchmarkId::new("edges_per_node", edge_count),
            &edge_count,
            |b, &edge_count| {
                b.iter_batched(
                    || {
                        let dir = tempfile::tempdir().unwrap();
                        let db = IvyBuilder::new().open(dir.path()).unwrap();

                        let center = db
                            .add_node(
                                "Person",
                                vec![("name".to_string(), Value::String("center".to_string()))],
                            )
                            .unwrap();

                        db.batch(|tx| {
                            for i in 0..edge_count {
                                let other = tx.add_node(
                                    "Person",
                                    vec![("name".to_string(), Value::String(format!("other_{i}")))],
                                )?;
                                tx.add_edge(center, other, "knows", Vec::new())?;
                                tx.add_edge(other, center, "follows", Vec::new())?;
                            }
                            Ok(())
                        })
                        .unwrap();

                        (dir, db, center)
                    },
                    |(_dir, db, center)| {
                        db.delete_node(center).unwrap();
                    },
                    criterion::BatchSize::PerIteration,
                );
            },
        );
    }
    group.finish();
}

fn bench_delete_edge(c: &mut Criterion) {
    let mut group = c.benchmark_group("delete_edge");
    group.throughput(Throughput::Elements(100));

    group.bench_function("batch_100", |b| {
        b.iter_batched(
            || {
                let dir = tempfile::tempdir().unwrap();
                let db = IvyBuilder::new().open(dir.path()).unwrap();
                let nodes: Vec<NodeId> = (0..101)
                    .map(|i| {
                        db.add_node(
                            "Person",
                            vec![("name".to_string(), Value::String(format!("u{i}")))],
                        )
                        .unwrap()
                    })
                    .collect();

                let edge_ids: Vec<_> = (0..100)
                    .map(|i| {
                        db.add_edge(nodes[i], nodes[i + 1], "knows", Vec::new())
                            .unwrap()
                    })
                    .collect();

                (dir, db, edge_ids)
            },
            |(_dir, db, edge_ids)| {
                for eid in edge_ids {
                    db.delete_edge(eid).unwrap();
                }
            },
            criterion::BatchSize::PerIteration,
        );
    });

    group.finish();
}

fn bench_delete_node_with_vector_index(c: &mut Criterion) {
    c.bench_function("delete_node_with_vector_index", |b| {
        b.iter_batched(
            || {
                let dir = tempfile::tempdir().unwrap();
                let db = IvyBuilder::new().open(dir.path()).unwrap();

                db.indexes()
                    .vectors()
                    .create("Chunk", "embedding", 64, ivy::HnswConfig::cosine())
                    .unwrap();

                let ids: Vec<NodeId> = (0..50)
                    .map(|i| {
                        let mut vec = vec![0.0f32; 64];
                        vec[i % 64] = 1.0;
                        db.add_node("Chunk", vec![("embedding".to_string(), Value::Vector(vec))])
                            .unwrap()
                    })
                    .collect();

                (dir, db, ids)
            },
            |(_dir, db, ids)| {
                for id in ids.into_iter().take(10) {
                    db.delete_node(id).unwrap();
                }
            },
            criterion::BatchSize::PerIteration,
        );
    });
}

criterion_group!(
    benches,
    bench_delete_node_no_edges,
    bench_delete_node_with_edges,
    bench_delete_edge,
    bench_delete_node_with_vector_index
);
criterion_main!(benches);

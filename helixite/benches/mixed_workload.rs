use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use helixite::{HelixiteBuilder, NodeId, Value};
use tempfile::TempDir;

fn setup_social_graph(
    node_count: usize,
    edges_per_node: usize,
) -> (TempDir, helixite::Helixite, Vec<NodeId>) {
    let dir = tempfile::tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let nodes: Vec<NodeId> = db
        .batch(|tx| {
            let mut ids = Vec::with_capacity(node_count);
            for i in 0..node_count {
                let id = tx.add_node(
                    "User",
                    vec![
                        ("name".to_string(), Value::String(format!("user_{i}"))),
                        ("age".to_string(), Value::Int((i % 60 + 18) as i64)),
                        (
                            "city".to_string(),
                            Value::String(
                                ["London", "Paris", "Berlin", "Tokyo", "NYC"][i % 5].to_string(),
                            ),
                        ),
                    ],
                )?;
                ids.push(id);
            }
            Ok(ids)
        })
        .unwrap();

    db.batch(|tx| {
        for i in 0..node_count {
            for j in 0..edges_per_node {
                let target_idx = (i + j + 1) % node_count;
                tx.add_edge(
                    nodes[i],
                    nodes[target_idx],
                    "follows",
                    vec![("since".to_string(), Value::Int(2015 + (j as i64 % 10)))],
                )?;
            }
        }
        Ok(())
    })
    .unwrap();

    (dir, db, nodes)
}

fn bench_social_graph_mixed_ops(c: &mut Criterion) {
    let ops_per_iter = 100u64;
    let mut group = c.benchmark_group("mixed_workload_social");
    group.throughput(Throughput::Elements(ops_per_iter));

    group.bench_function("80read_20write", |b| {
        let (_dir, db, nodes) = setup_social_graph(500, 5);
        let mut op_counter = 0usize;

        b.iter(|| {
            for _ in 0..ops_per_iter {
                let idx = op_counter % 100;
                op_counter += 1;

                if idx < 40 {
                    // 40% traversal reads
                    let node_id = nodes[op_counter % nodes.len()];
                    let _ = db.node(node_id).outgoing("follows").nodes().unwrap();
                } else if idx < 60 {
                    // 20% point lookups
                    let node_id = nodes[op_counter % nodes.len()];
                    let _ = db.get_node(node_id).unwrap();
                } else if idx < 80 {
                    // 20% query reads
                    let _ = db.nodes().label("User").limit(10).collect().unwrap();
                } else if idx < 90 {
                    // 10% node writes
                    let _ = db
                        .add_node(
                            "User",
                            vec![
                                (
                                    "name".to_string(),
                                    Value::String(format!("new_user_{op_counter}")),
                                ),
                                ("age".to_string(), Value::Int(25)),
                            ],
                        )
                        .unwrap();
                } else {
                    // 10% edge writes
                    let from = nodes[op_counter % nodes.len()];
                    let to = nodes[(op_counter + 7) % nodes.len()];
                    let _ = db.add_edge(from, to, "likes", vec![]).unwrap();
                }
            }
        });
    });

    group.finish();
}

fn bench_read_heavy_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_workload_read_heavy");
    group.throughput(Throughput::Elements(100));

    group.bench_function("95read_5write", |b| {
        let (_dir, db, nodes) = setup_social_graph(1000, 3);
        let mut op_counter = 0usize;

        b.iter(|| {
            for _ in 0..100 {
                let idx = op_counter % 100;
                op_counter += 1;

                if idx < 50 {
                    let node_id = nodes[op_counter % nodes.len()];
                    let _ = db.node(node_id).outgoing("follows").nodes().unwrap();
                } else if idx < 70 {
                    let node_id = nodes[op_counter % nodes.len()];
                    let _ = db.get_node(node_id).unwrap();
                } else if idx < 95 {
                    let _ = db.nodes().label("User").limit(20).collect().unwrap();
                } else {
                    let from = nodes[op_counter % nodes.len()];
                    let to = nodes[(op_counter + 3) % nodes.len()];
                    let _ = db.add_edge(from, to, "mentions", vec![]).unwrap();
                }
            }
        });
    });

    group.finish();
}

fn bench_write_heavy_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_workload_write_heavy");
    group.throughput(Throughput::Elements(100));

    group.bench_function("30read_70write", |b| {
        b.iter_batched(
            || {
                let dir = tempfile::tempdir().unwrap();
                let db = HelixiteBuilder::new().open(dir.path()).unwrap();
                let nodes: Vec<NodeId> = (0..100)
                    .map(|i| {
                        db.add_node(
                            "User",
                            vec![("name".to_string(), Value::String(format!("seed_{i}")))],
                        )
                        .unwrap()
                    })
                    .collect();
                (dir, db, nodes)
            },
            |(_dir, db, nodes)| {
                let mut op_counter = 0usize;
                for _ in 0..100 {
                    let idx = op_counter % 100;
                    op_counter += 1;

                    if idx < 15 {
                        let node_id = nodes[op_counter % nodes.len()];
                        let _ = db.get_node(node_id).unwrap();
                    } else if idx < 30 {
                        let node_id = nodes[op_counter % nodes.len()];
                        let _ = db.node(node_id).outgoing_any().count().unwrap();
                    } else if idx < 65 {
                        let _ = db
                            .add_node(
                                "Post",
                                vec![
                                    (
                                        "title".to_string(),
                                        Value::String(format!("post_{op_counter}")),
                                    ),
                                    ("likes".to_string(), Value::Int(op_counter as i64)),
                                ],
                            )
                            .unwrap();
                    } else {
                        let from = nodes[op_counter % nodes.len()];
                        let to = nodes[(op_counter + 1) % nodes.len()];
                        let _ = db.add_edge(from, to, "wrote", vec![]).unwrap();
                    }
                }
            },
            criterion::BatchSize::PerIteration,
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_social_graph_mixed_ops,
    bench_read_heavy_workload,
    bench_write_heavy_workload
);
criterion_main!(benches);

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use ivy::{IvyBuilder, NodeId, Value};
use tempfile::TempDir;

fn setup_star_graph(fan_out: usize) -> (TempDir, ivy::Ivy, NodeId) {
    let dir = tempfile::tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let center = db
        .add_node(
            "Person",
            vec![("name".to_string(), Value::String("center".to_string()))],
        )
        .unwrap();

    db.batch(|tx| {
        for i in 0..fan_out {
            let target = tx.add_node(
                "Person",
                vec![("name".to_string(), Value::String(format!("friend_{i}")))],
            )?;
            tx.add_edge(
                center,
                target,
                "knows",
                vec![("weight".to_string(), Value::Float(i as f64 * 0.01))],
            )?;
        }
        Ok(())
    })
    .unwrap();

    (dir, db, center)
}

fn setup_chain_graph(depth: usize, width: usize) -> (TempDir, ivy::Ivy, NodeId) {
    let dir = tempfile::tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let root = db
        .add_node(
            "Person",
            vec![("name".to_string(), Value::String("root".to_string()))],
        )
        .unwrap();

    db.batch(|tx| {
        let mut current_layer = vec![root];
        for d in 0..depth {
            let mut next_layer = Vec::new();
            for &parent in &current_layer {
                for w in 0..width {
                    let child = tx.add_node(
                        "Person",
                        vec![("name".to_string(), Value::String(format!("d{d}_w{w}")))],
                    )?;
                    tx.add_edge(parent, child, "knows", Vec::new())?;
                    next_layer.push(child);
                }
            }
            current_layer = next_layer;
        }
        Ok(())
    })
    .unwrap();

    (dir, db, root)
}

fn bench_single_hop_outgoing(c: &mut Criterion) {
    let mut group = c.benchmark_group("traversal_single_hop_out");

    for fan_out in [5, 50, 500] {
        group.bench_with_input(
            BenchmarkId::new("fan_out", fan_out),
            &fan_out,
            |b, &fan_out| {
                let (_dir, db, center) = setup_star_graph(fan_out);
                b.iter(|| {
                    let nodes = db.node(center).outgoing("knows").nodes().unwrap();
                    assert_eq!(nodes.len(), fan_out);
                    nodes
                });
            },
        );
    }
    group.finish();
}

fn bench_single_hop_incoming(c: &mut Criterion) {
    let mut group = c.benchmark_group("traversal_single_hop_in");

    for fan_in in [5, 50, 500] {
        group.bench_with_input(BenchmarkId::new("fan_in", fan_in), &fan_in, |b, &fan_in| {
            let dir = tempfile::tempdir().unwrap();
            let db = IvyBuilder::new().open(dir.path()).unwrap();

            let target = db
                .add_node(
                    "Person",
                    vec![("name".to_string(), Value::String("target".to_string()))],
                )
                .unwrap();

            db.batch(|tx| {
                for i in 0..fan_in {
                    let src = tx.add_node(
                        "Person",
                        vec![("name".to_string(), Value::String(format!("src_{i}")))],
                    )?;
                    tx.add_edge(src, target, "follows", Vec::new())?;
                }
                Ok(())
            })
            .unwrap();

            b.iter(|| {
                let nodes = db.node(target).incoming("follows").nodes().unwrap();
                assert_eq!(nodes.len(), fan_in);
                nodes
            });
        });
    }
    group.finish();
}

fn bench_single_hop_edges(c: &mut Criterion) {
    let (_dir, db, center) = setup_star_graph(100);

    c.bench_function("traversal_single_hop_edges", |b| {
        b.iter(|| {
            let edges = db.node(center).outgoing("knows").edges().unwrap();
            assert_eq!(edges.len(), 100);
            edges
        });
    });
}

fn bench_single_hop_count(c: &mut Criterion) {
    let (_dir, db, center) = setup_star_graph(500);

    c.bench_function("traversal_single_hop_count", |b| {
        b.iter(|| {
            let count = db.node(center).outgoing("knows").count().unwrap();
            assert_eq!(count, 500);
            count
        });
    });
}

fn bench_multi_hop(c: &mut Criterion) {
    let mut group = c.benchmark_group("traversal_multi_hop");

    for depth in [2, 3] {
        group.bench_with_input(
            BenchmarkId::new("depth_width5", depth),
            &depth,
            |b, &depth| {
                let (_dir, db, root) = setup_chain_graph(depth, 5);
                b.iter(|| {
                    let mut query = db.node(root).then_outgoing("knows");
                    for _ in 1..depth {
                        query = query.then_outgoing("knows");
                    }
                    let nodes = query.nodes().unwrap();
                    assert!(!nodes.is_empty());
                    nodes
                });
            },
        );
    }
    group.finish();
}

fn bench_multi_hop_wide(c: &mut Criterion) {
    let mut group = c.benchmark_group("traversal_multi_hop_wide");

    for width in [10, 20] {
        group.bench_with_input(
            BenchmarkId::new("depth2_width", width),
            &width,
            |b, &width| {
                let (_dir, db, root) = setup_chain_graph(2, width);
                b.iter(|| {
                    let nodes = db
                        .node(root)
                        .then_outgoing("knows")
                        .then_outgoing("knows")
                        .nodes()
                        .unwrap();
                    assert!(!nodes.is_empty());
                    nodes
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_single_hop_outgoing,
    bench_single_hop_incoming,
    bench_single_hop_edges,
    bench_single_hop_count,
    bench_multi_hop,
    bench_multi_hop_wide
);
criterion_main!(benches);

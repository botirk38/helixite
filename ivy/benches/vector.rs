use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ivy::{IvyBuilder, HnswConfig, Value};
use tempfile::TempDir;

fn random_vector(dim: usize, seed: u64) -> Vec<f32> {
    let mut v = Vec::with_capacity(dim);
    let mut state = seed;
    for _ in 0..dim {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let f = ((state >> 33) as f32) / (u32::MAX as f32) * 2.0 - 1.0;
        v.push(f);
    }
    v
}

fn setup_vector_db(count: usize, dim: usize) -> (TempDir, ivy::Ivy) {
    let dir = tempfile::tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    db.indexes()
        .vectors()
        .create("Chunk", "embedding", dim, HnswConfig::cosine())
        .unwrap();

    db.batch(|tx| {
        for i in 0..count {
            tx.add_node(
                "Chunk",
                vec![
                    (
                        "embedding".to_string(),
                        Value::Vector(random_vector(dim, i as u64)),
                    ),
                    ("text".to_string(), Value::String(format!("chunk_{i}"))),
                ],
            )?;
        }
        Ok(())
    })
    .unwrap();

    (dir, db)
}

fn bench_vector_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("vector_insert");

    for dim in [128, 384] {
        group.throughput(Throughput::Elements(100));
        group.bench_with_input(BenchmarkId::new("dim", dim), &dim, |b, &dim| {
            b.iter_batched(
                || {
                    let dir = tempfile::tempdir().unwrap();
                    let db = IvyBuilder::new().open(dir.path()).unwrap();
                    db.indexes()
                        .vectors()
                        .create("Chunk", "embedding", dim, HnswConfig::cosine())
                        .unwrap();
                    (dir, db)
                },
                |(_dir, db)| {
                    db.batch(|tx| {
                        for i in 0..100 {
                            tx.add_node(
                                "Chunk",
                                vec![(
                                    "embedding".to_string(),
                                    Value::Vector(random_vector(dim, i)),
                                )],
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

fn bench_vector_insert_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("vector_insert_scaling");
    group.sample_size(10);

    for count in [100, 500, 1_000] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("count_dim128", count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let dir = tempfile::tempdir().unwrap();
                        let db = IvyBuilder::new().open(dir.path()).unwrap();
                        db.indexes()
                            .vectors()
                            .create("Chunk", "embedding", 128, HnswConfig::cosine())
                            .unwrap();
                        (dir, db)
                    },
                    |(_dir, db)| {
                        db.batch(|tx| {
                            for i in 0..count {
                                tx.add_node(
                                    "Chunk",
                                    vec![(
                                        "embedding".to_string(),
                                        Value::Vector(random_vector(128, i as u64)),
                                    )],
                                )?;
                            }
                            Ok(())
                        })
                        .unwrap();
                    },
                    criterion::BatchSize::PerIteration,
                );
            },
        );
    }
    group.finish();
}

fn bench_vector_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("vector_search_knn");

    for (count, dim) in [(500, 128), (1_000, 128), (500, 384)] {
        for k in [10, 50] {
            group.bench_with_input(
                BenchmarkId::new(format!("n{count}_d{dim}_k{k}"), k),
                &(count, dim, k),
                |b, &(count, dim, k)| {
                    let (_dir, db) = setup_vector_db(count, dim);
                    let query_vec = random_vector(dim, 999_999);

                    b.iter(|| {
                        let results = db
                            .nodes()
                            .label("Chunk")
                            .nearest("embedding", query_vec.clone(), k)
                            .collect()
                            .unwrap();
                        assert!(!results.is_empty());
                        results
                    });
                },
            );
        }
    }
    group.finish();
}

fn bench_vector_search_ids_only(c: &mut Criterion) {
    let (_dir, db) = setup_vector_db(1_000, 128);
    let query_vec = random_vector(128, 42);

    c.bench_function("vector_search_ids_only_n1000_d128_k10", |b| {
        b.iter(|| {
            let ids = db
                .nodes()
                .label("Chunk")
                .nearest("embedding", query_vec.clone(), 10)
                .ids()
                .unwrap();
            assert!(!ids.is_empty());
            ids
        });
    });
}

fn bench_vector_similarity_functions(c: &mut Criterion) {
    let mut group = c.benchmark_group("vector_similarity_comparison");

    for similarity in ["cosine", "euclidean", "dot_product"] {
        group.bench_with_input(
            BenchmarkId::new("search_n500_d128_k10", similarity),
            &similarity,
            |b, &similarity| {
                let dir = tempfile::tempdir().unwrap();
                let db = IvyBuilder::new().open(dir.path()).unwrap();

                let config = match similarity {
                    "cosine" => HnswConfig::cosine(),
                    "euclidean" => HnswConfig::euclidean(),
                    "dot_product" => HnswConfig::dot_product(),
                    _ => unreachable!(),
                };

                db.indexes()
                    .vectors()
                    .create("Chunk", "embedding", 128, config)
                    .unwrap();

                db.batch(|tx| {
                    for i in 0..500 {
                        tx.add_node(
                            "Chunk",
                            vec![(
                                "embedding".to_string(),
                                Value::Vector(random_vector(128, i)),
                            )],
                        )?;
                    }
                    Ok(())
                })
                .unwrap();

                let query_vec = random_vector(128, 77777);

                b.iter(|| {
                    db.nodes()
                        .label("Chunk")
                        .nearest("embedding", query_vec.clone(), 10)
                        .ids()
                        .unwrap()
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_vector_insert,
    bench_vector_insert_scaling,
    bench_vector_search,
    bench_vector_search_ids_only,
    bench_vector_similarity_functions
);
criterion_main!(benches);

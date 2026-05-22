use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use helixite::{HelixiteBuilder, Value};
use tempfile::TempDir;

fn setup_db(count: usize) -> (TempDir, helixite::Helixite) {
    let dir = tempfile::tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    db.batch(|tx| {
        for i in 0..count {
            let label = if i % 3 == 0 { "Person" } else { "Document" };
            tx.add_node(
                label,
                vec![
                    ("name".to_string(), Value::String(format!("item_{i}"))),
                    ("idx".to_string(), Value::Int(i as i64)),
                ],
            )?;
        }
        Ok(())
    })
    .unwrap();

    (dir, db)
}

fn bench_query_by_label_collect(c: &mut Criterion) {
    let mut group = c.benchmark_group("label_query_collect");

    for total_size in [300, 3_000, 30_000] {
        let expected_count = total_size / 3;
        group.throughput(Throughput::Elements(expected_count as u64));
        group.bench_with_input(
            BenchmarkId::new("Person_nodes", expected_count),
            &total_size,
            |b, &total_size| {
                let (_dir, db) = setup_db(total_size);
                b.iter(|| {
                    let nodes = db.nodes().label("Person").collect().unwrap();
                    assert!(!nodes.is_empty());
                    nodes
                });
            },
        );
    }
    group.finish();
}

fn bench_query_by_label_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("label_query_count");

    for total_size in [300, 3_000, 30_000] {
        let expected_count = total_size / 3;
        group.throughput(Throughput::Elements(expected_count as u64));
        group.bench_with_input(
            BenchmarkId::new("Person_count", expected_count),
            &total_size,
            |b, &total_size| {
                let (_dir, db) = setup_db(total_size);
                b.iter(|| {
                    let count = db.nodes().label("Person").count().unwrap();
                    assert!(count > 0);
                    count
                });
            },
        );
    }
    group.finish();
}

fn bench_query_by_label_ids(c: &mut Criterion) {
    let mut group = c.benchmark_group("label_query_ids");

    for total_size in [300, 3_000, 30_000] {
        let expected_count = total_size / 3;
        group.throughput(Throughput::Elements(expected_count as u64));
        group.bench_with_input(
            BenchmarkId::new("Person_ids", expected_count),
            &total_size,
            |b, &total_size| {
                let (_dir, db) = setup_db(total_size);
                b.iter(|| {
                    let ids = db.nodes().label("Person").ids().unwrap();
                    assert!(!ids.is_empty());
                    ids
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_query_by_label_collect,
    bench_query_by_label_count,
    bench_query_by_label_ids
);
criterion_main!(benches);

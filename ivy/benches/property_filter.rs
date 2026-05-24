use criterion::{Criterion, criterion_group, criterion_main};
use ivy::{IvyBuilder, Value};
use tempfile::TempDir;

fn setup_indexed_db(count: usize) -> (TempDir, ivy::Ivy) {
    let dir = tempfile::tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    db.add_node(
        "Person",
        vec![
            ("age".to_string(), Value::Int(0)),
            ("city".to_string(), Value::String("seed".to_string())),
        ],
    )
    .unwrap();

    db.indexes()
        .nodes()
        .create_property("Person", "age")
        .unwrap();
    db.indexes()
        .nodes()
        .create_property("Person", "city")
        .unwrap();

    db.batch(|tx| {
        let cities = ["London", "Paris", "Berlin", "Tokyo", "NYC"];
        for i in 0..count {
            tx.add_node(
                "Person",
                vec![
                    ("name".to_string(), Value::String(format!("user_{i}"))),
                    ("age".to_string(), Value::Int((i % 80) as i64 + 18)),
                    (
                        "city".to_string(),
                        Value::String(cities[i % cities.len()].to_string()),
                    ),
                    ("score".to_string(), Value::Float(i as f64 * 0.1)),
                ],
            )?;
        }
        Ok(())
    })
    .unwrap();

    (dir, db)
}

fn bench_eq_filter(c: &mut Criterion) {
    let (_dir, db) = setup_indexed_db(10_000);

    c.bench_function("property_filter_eq_indexed", |b| {
        b.iter(|| {
            let nodes = db
                .nodes()
                .label("Person")
                .eq("city", Value::String("London".to_string()))
                .collect()
                .unwrap();
            assert!(!nodes.is_empty());
            nodes
        });
    });
}

fn bench_eq_filter_int(c: &mut Criterion) {
    let (_dir, db) = setup_indexed_db(10_000);

    c.bench_function("property_filter_eq_int_indexed", |b| {
        b.iter(|| {
            db.nodes()
                .label("Person")
                .eq("age", Value::Int(30))
                .collect()
                .unwrap()
        });
    });
}

fn bench_range_filter(c: &mut Criterion) {
    let (_dir, db) = setup_indexed_db(10_000);

    c.bench_function("property_filter_range_gt_lt", |b| {
        b.iter(|| {
            db.nodes()
                .label("Person")
                .gt("age", Value::Int(40))
                .lt("age", Value::Int(60))
                .collect()
                .unwrap()
        });
    });
}

fn bench_gte_lte_filter(c: &mut Criterion) {
    let (_dir, db) = setup_indexed_db(10_000);

    c.bench_function("property_filter_gte_lte", |b| {
        b.iter(|| {
            db.nodes()
                .label("Person")
                .gte("age", Value::Int(25))
                .lte("age", Value::Int(35))
                .collect()
                .unwrap()
        });
    });
}

fn bench_in_filter(c: &mut Criterion) {
    let (_dir, db) = setup_indexed_db(10_000);

    c.bench_function("property_filter_in", |b| {
        b.iter(|| {
            let nodes = db
                .nodes()
                .label("Person")
                .r#in(
                    "city",
                    vec![
                        Value::String("London".to_string()),
                        Value::String("Tokyo".to_string()),
                    ],
                )
                .collect()
                .unwrap();
            assert!(!nodes.is_empty());
            nodes
        });
    });
}

fn bench_ne_filter(c: &mut Criterion) {
    let (_dir, db) = setup_indexed_db(10_000);

    c.bench_function("property_filter_ne", |b| {
        b.iter(|| {
            db.nodes()
                .label("Person")
                .ne("city", Value::String("London".to_string()))
                .collect()
                .unwrap()
        });
    });
}

fn bench_combined_filters(c: &mut Criterion) {
    let (_dir, db) = setup_indexed_db(10_000);

    c.bench_function("property_filter_combined", |b| {
        b.iter(|| {
            db.nodes()
                .label("Person")
                .eq("city", Value::String("Berlin".to_string()))
                .gt("age", Value::Int(30))
                .collect()
                .unwrap()
        });
    });
}

criterion_group!(
    benches,
    bench_eq_filter,
    bench_eq_filter_int,
    bench_range_filter,
    bench_gte_lte_filter,
    bench_in_filter,
    bench_ne_filter,
    bench_combined_filters
);
criterion_main!(benches);

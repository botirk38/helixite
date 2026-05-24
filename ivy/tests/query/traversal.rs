use ivy::IvyBuilder;
use ivy::value::Value;
use tempfile::tempdir;

#[test]
fn test_node_traversal_out() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", Vec::new()).unwrap();
    db.add_edge(a, c, "knows", Vec::new()).unwrap();

    let neighbors = db.node(a).outgoing("knows").nodes().unwrap();
    assert_eq!(neighbors.len(), 2);

    let edges = db.node(a).outgoing("knows").edges().unwrap();
    assert_eq!(edges.len(), 2);
}

#[test]
fn test_node_traversal_in() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, c, "knows", Vec::new()).unwrap();
    db.add_edge(b, c, "knows", Vec::new()).unwrap();

    let neighbors = db.node(c).incoming("knows").nodes().unwrap();
    assert_eq!(neighbors.len(), 2);
}

#[test]
fn test_node_traversal_any() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", Vec::new()).unwrap();
    db.add_edge(a, b, "follows", Vec::new()).unwrap();

    let out_count = db.node(a).outgoing_any().count().unwrap();
    assert_eq!(out_count, 2);
}

#[test]
fn test_multi_hop_then_outgoing() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", Vec::new()).unwrap();
    db.add_edge(b, c, "knows", Vec::new()).unwrap();

    let nodes = db
        .node(a)
        .outgoing("knows")
        .then_outgoing("knows")
        .nodes()
        .unwrap();

    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id, c);
}

#[test]
fn test_multi_hop_then_incoming() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", Vec::new()).unwrap();
    db.add_edge(c, b, "follows", Vec::new()).unwrap();

    let nodes = db
        .node(a)
        .outgoing("knows")
        .then_incoming("follows")
        .nodes()
        .unwrap();

    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id, c);
}

#[test]
fn test_multi_hop_from_node_ref_query() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", Vec::new()).unwrap();
    db.add_edge(b, c, "likes", Vec::new()).unwrap();

    let ids = db
        .node(a)
        .then_outgoing("knows")
        .then_outgoing("likes")
        .ids()
        .unwrap();

    assert_eq!(ids, vec![c]);
}

#[test]
fn test_multi_hop_count_limit_and_first() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();
    let d = db.add_node("D", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", Vec::new()).unwrap();
    db.add_edge(a, c, "knows", Vec::new()).unwrap();
    db.add_edge(b, d, "knows", Vec::new()).unwrap();
    db.add_edge(c, d, "knows", Vec::new()).unwrap();

    let query = db.node(a).then_outgoing("knows").then_outgoing("knows");

    assert_eq!(query.count().unwrap(), 1);
    assert_eq!(
        db.node(a)
            .then_outgoing("knows")
            .then_outgoing("knows")
            .limit(1)
            .ids()
            .unwrap(),
        vec![d]
    );
    assert_eq!(
        db.node(a)
            .then_outgoing("knows")
            .then_outgoing("knows")
            .first_node()
            .unwrap()
            .unwrap()
            .id,
        d
    );
}

#[test]
fn test_traversal_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = IvyBuilder::new().open(path).unwrap();
        let a = db.add_node("A", Vec::new()).unwrap();
        let b = db.add_node("B", Vec::new()).unwrap();
        db.add_edge(a, b, "knows", Vec::new()).unwrap();
    }

    let db = IvyBuilder::new().open(path).unwrap();
    let neighbors = db.node(1).outgoing("knows").nodes().unwrap();
    assert_eq!(neighbors.len(), 1);
    assert_eq!(neighbors[0].id, 2);
}

#[test]
fn test_traversal_where_eq_indexed() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", [("weight".to_string(), Value::Float(1.0))])
        .unwrap();
    db.add_edge(a, c, "knows", [("weight".to_string(), Value::Float(2.0))])
        .unwrap();
    db.add_edge(a, c, "knows", [("weight".to_string(), Value::Float(1.0))])
        .unwrap();

    db.indexes()
        .edges()
        .create_property("knows", "weight")
        .unwrap();

    let edges = db
        .node(a)
        .outgoing("knows")
        .eq("weight", Value::Float(1.0))
        .edges()
        .unwrap();
    assert_eq!(edges.len(), 2);

    let nodes = db
        .node(a)
        .outgoing("knows")
        .eq("weight", Value::Float(2.0))
        .nodes()
        .unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id, c);

    let count = db
        .node(a)
        .outgoing("knows")
        .eq("weight", Value::Float(1.0))
        .count()
        .unwrap();
    assert_eq!(count, 2);
}

#[test]
fn test_traversal_where_comparison_filters() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", [("weight".to_string(), Value::Float(1.0))])
        .unwrap();
    db.add_edge(a, c, "knows", [("weight".to_string(), Value::Float(2.0))])
        .unwrap();

    db.indexes()
        .edges()
        .create_property("knows", "weight")
        .unwrap();

    let nodes = db
        .node(a)
        .outgoing("knows")
        .gt("weight", Value::Float(1.0))
        .nodes()
        .unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id, c);

    let edges = db
        .node(a)
        .outgoing("knows")
        .lte("weight", Value::Float(1.0))
        .edges()
        .unwrap();
    assert_eq!(edges.len(), 1);
}

#[test]
fn test_traversal_where_ne_and_in_filters() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();
    let d = db.add_node("D", Vec::new()).unwrap();

    db.add_edge(
        a,
        b,
        "knows",
        [("kind".to_string(), Value::String("work".into()))],
    )
    .unwrap();
    db.add_edge(
        a,
        c,
        "knows",
        [("kind".to_string(), Value::String("home".into()))],
    )
    .unwrap();
    db.add_edge(
        a,
        d,
        "knows",
        [("kind".to_string(), Value::String("gym".into()))],
    )
    .unwrap();

    db.indexes()
        .edges()
        .create_property("knows", "kind")
        .unwrap();

    let not_work = db
        .node(a)
        .outgoing("knows")
        .ne("kind", Value::String("work".into()))
        .count()
        .unwrap();
    assert_eq!(not_work, 2);

    let selected = db
        .node(a)
        .outgoing("knows")
        .r#in(
            "kind",
            [Value::String("work".into()), Value::String("gym".into())],
        )
        .nodes()
        .unwrap();
    assert_eq!(selected.len(), 2);
}

#[test]
fn test_traversal_where_eq_not_indexed() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", [("weight".to_string(), Value::Float(1.0))])
        .unwrap();

    let result = db
        .node(a)
        .outgoing("knows")
        .eq("weight", Value::Float(1.0))
        .edges();
    assert!(result.is_err());
}

#[test]
fn test_traversal_where_eq_no_label() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", [("weight".to_string(), Value::Float(1.0))])
        .unwrap();
    db.indexes()
        .edges()
        .create_property("knows", "weight")
        .unwrap();

    let result = db
        .node(a)
        .outgoing_any()
        .eq("weight", Value::Float(1.0))
        .edges();
    assert!(result.is_err());
}

#[test]
fn test_traversal_where_eq_multiple_filters() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(
        a,
        b,
        "knows",
        [
            ("weight".to_string(), Value::Float(1.0)),
            ("status".to_string(), Value::String("active".to_string())),
        ],
    )
    .unwrap();
    db.add_edge(
        a,
        c,
        "knows",
        [
            ("weight".to_string(), Value::Float(1.0)),
            ("status".to_string(), Value::String("inactive".to_string())),
        ],
    )
    .unwrap();
    db.add_edge(
        a,
        c,
        "knows",
        [
            ("weight".to_string(), Value::Float(2.0)),
            ("status".to_string(), Value::String("active".to_string())),
        ],
    )
    .unwrap();

    db.indexes()
        .edges()
        .create_property("knows", "weight")
        .unwrap();
    db.indexes()
        .edges()
        .create_property("knows", "status")
        .unwrap();

    let edges = db
        .node(a)
        .outgoing("knows")
        .eq("weight", Value::Float(1.0))
        .eq("status", Value::String("active".to_string()))
        .edges()
        .unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].to, b);
}

#[test]
fn test_traversal_where_eq_in_direction() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, c, "knows", [("weight".to_string(), Value::Float(1.0))])
        .unwrap();
    db.add_edge(b, c, "knows", [("weight".to_string(), Value::Float(2.0))])
        .unwrap();

    db.indexes()
        .edges()
        .create_property("knows", "weight")
        .unwrap();

    let nodes = db
        .node(c)
        .incoming("knows")
        .eq("weight", Value::Float(1.0))
        .nodes()
        .unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id, a);
}

#[test]
fn test_traversal_reflects_edge_mutation() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", [("weight".to_string(), Value::Float(1.0))])
        .unwrap();
    db.add_edge(a, c, "knows", [("weight".to_string(), Value::Float(2.0))])
        .unwrap();

    db.indexes()
        .edges()
        .create_property("knows", "weight")
        .unwrap();

    let edges_before = db
        .node(a)
        .outgoing("knows")
        .eq("weight", Value::Float(1.0))
        .edges()
        .unwrap();
    assert_eq!(edges_before.len(), 1);

    let edge_id = edges_before[0].id;
    db.batch(|tx| {
        tx.edge(edge_id)
            .set_property("weight", Value::Float(3.0))
            .apply()
    })
    .unwrap();

    let edges_after_old = db
        .node(a)
        .outgoing("knows")
        .eq("weight", Value::Float(1.0))
        .edges()
        .unwrap();
    assert_eq!(edges_after_old.len(), 0);

    let edges_after_new = db
        .node(a)
        .outgoing("knows")
        .eq("weight", Value::Float(3.0))
        .edges()
        .unwrap();
    assert_eq!(edges_after_new.len(), 1);
}

#[test]
fn test_traversal_first_node() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", Vec::new()).unwrap();
    db.add_edge(a, c, "knows", Vec::new()).unwrap();

    let node = db.node(a).outgoing("knows").first_node().unwrap();
    assert!(node.is_some());
}

#[test]
fn test_traversal_first_edge() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", Vec::new()).unwrap();

    let edge = db.node(a).outgoing("knows").first_edge().unwrap();
    assert!(edge.is_some());
    assert_eq!(edge.unwrap().to, b);
}

#[test]
fn test_traversal_first_empty() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();

    let node = db.node(a).outgoing("knows").first_node().unwrap();
    assert!(node.is_none());
}

#[test]
fn test_traversal_limit() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();
    let d = db.add_node("D", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", Vec::new()).unwrap();
    db.add_edge(a, c, "knows", Vec::new()).unwrap();
    db.add_edge(a, d, "knows", Vec::new()).unwrap();

    let nodes = db.node(a).outgoing("knows").limit(2).nodes().unwrap();
    assert_eq!(nodes.len(), 2);

    let edges = db.node(a).outgoing("knows").limit(2).edges().unwrap();
    assert_eq!(edges.len(), 2);
}

#[test]
fn test_traversal_limit_with_filters() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", [("weight".to_string(), Value::Float(1.0))])
        .unwrap();
    db.add_edge(a, c, "knows", [("weight".to_string(), Value::Float(1.0))])
        .unwrap();

    db.indexes()
        .edges()
        .create_property("knows", "weight")
        .unwrap();

    let edges = db
        .node(a)
        .outgoing("knows")
        .eq("weight", Value::Float(1.0))
        .limit(1)
        .edges()
        .unwrap();
    assert_eq!(edges.len(), 1);
}

#[test]
fn test_traversal_edges_page_limit_conflict() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    db.add_edge(a, b, "knows", Vec::new()).unwrap();

    let err = db
        .node(a)
        .outgoing("knows")
        .limit(10)
        .edges_page(2)
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("limit() cannot be used with page()")
    );
}

#[test]
fn test_traversal_edges_page_first_page() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    for _i in 0..5 {
        let b = db.add_node("B", Vec::new()).unwrap();
        db.add_edge(a, b, "knows", Vec::new()).unwrap();
    }

    let page = db.node(a).outgoing("knows").edges_page(2).unwrap();
    assert_eq!(page.items.len(), 2);
    assert!(page.next_cursor.is_some());
}

#[test]
fn test_traversal_edges_page_second_page() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    for _ in 0..5 {
        let b = db.add_node("B", Vec::new()).unwrap();
        db.add_edge(a, b, "knows", Vec::new()).unwrap();
    }

    let page1 = db.node(a).outgoing("knows").edges_page(2).unwrap();
    assert_eq!(page1.items.len(), 2);
    let cursor = page1.next_cursor.unwrap();

    let page2 = db
        .node(a)
        .outgoing("knows")
        .after(cursor)
        .edges_page(2)
        .unwrap();
    assert_eq!(page2.items.len(), 2);
    assert!(page2.next_cursor.is_some());

    let page1_ids: std::collections::HashSet<_> = page1.items.iter().map(|e| e.id).collect();
    let page2_ids: std::collections::HashSet<_> = page2.items.iter().map(|e| e.id).collect();
    assert!(page1_ids.is_disjoint(&page2_ids));
}

#[test]
fn test_traversal_edges_page_last_page_no_next_cursor() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    for _ in 0..3 {
        let b = db.add_node("B", Vec::new()).unwrap();
        db.add_edge(a, b, "knows", Vec::new()).unwrap();
    }

    let page1 = db.node(a).outgoing("knows").edges_page(2).unwrap();
    assert_eq!(page1.items.len(), 2);
    let cursor = page1.next_cursor.unwrap();

    let page2 = db
        .node(a)
        .outgoing("knows")
        .after(cursor)
        .edges_page(2)
        .unwrap();
    assert_eq!(page2.items.len(), 1);
    assert!(page2.next_cursor.is_none());
}

#[test]
fn test_traversal_nodes_page_first_page() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();
    let d = db.add_node("D", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", Vec::new()).unwrap();
    db.add_edge(a, c, "knows", Vec::new()).unwrap();
    db.add_edge(a, d, "knows", Vec::new()).unwrap();

    let page = db.node(a).outgoing("knows").nodes_page(2).unwrap();
    assert_eq!(page.items.len(), 2);
    assert!(page.next_cursor.is_some());
}

#[test]
fn test_traversal_nodes_page_second_page() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    for _ in 0..5 {
        let b = db.add_node("B", Vec::new()).unwrap();
        db.add_edge(a, b, "knows", Vec::new()).unwrap();
    }

    let page1 = db.node(a).outgoing("knows").nodes_page(2).unwrap();
    assert_eq!(page1.items.len(), 2);
    let cursor = page1.next_cursor.unwrap();

    let page2 = db
        .node(a)
        .outgoing("knows")
        .after(cursor)
        .nodes_page(2)
        .unwrap();
    assert_eq!(page2.items.len(), 2);
    assert!(page2.next_cursor.is_some());
}

#[test]
fn test_traversal_edges_page_invalid_cursor_format() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    db.add_edge(a, b, "knows", Vec::new()).unwrap();

    let err = db
        .node(a)
        .outgoing("knows")
        .after("bad_cursor")
        .edges_page(2)
        .unwrap_err();
    assert!(err.to_string().contains("edge cursor must start with 'e:'"));
}

#[test]
fn test_traversal_edges_page_stale_cursor() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    db.add_edge(a, b, "knows", Vec::new()).unwrap();

    let stale_cursor = "e:999999";

    let err = db
        .node(a)
        .outgoing("knows")
        .after(stale_cursor)
        .edges_page(2)
        .unwrap_err();
    assert!(err.to_string().contains("cursor not found"));
}

#[test]
fn test_traversal_count_rejects_after() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    for _ in 0..5 {
        let b = db.add_node("B", Vec::new()).unwrap();
        db.add_edge(a, b, "knows", Vec::new()).unwrap();
    }

    let err = db
        .node(a)
        .outgoing("knows")
        .after("e:1")
        .count()
        .unwrap_err();
    assert!(err.to_string().contains("after() requires page()"));
}

#[test]
fn test_traversal_edges_page_zero_size_rejected() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    db.add_edge(a, b, "knows", Vec::new()).unwrap();

    let err = db.node(a).outgoing("knows").edges_page(0).unwrap_err();
    assert!(err.to_string().contains("page size must be greater than 0"));
}

#[test]
fn test_traversal_nodes_page_zero_size_rejected() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    db.add_edge(a, b, "knows", Vec::new()).unwrap();

    let err = db.node(a).outgoing("knows").nodes_page(0).unwrap_err();
    assert!(err.to_string().contains("page size must be greater than 0"));
}

#[test]
fn test_traversal_edges_rejects_after() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    db.add_edge(a, b, "knows", Vec::new()).unwrap();

    let err = db
        .node(a)
        .outgoing("knows")
        .after("e:1")
        .edges()
        .unwrap_err();
    assert!(err.to_string().contains("after() requires page()"));
}

#[test]
fn test_traversal_nodes_rejects_after() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    db.add_edge(a, b, "knows", Vec::new()).unwrap();

    let err = db
        .node(a)
        .outgoing("knows")
        .after("e:1")
        .nodes()
        .unwrap_err();
    assert!(err.to_string().contains("after() requires page()"));
}

#[test]
fn test_traversal_from_deleted_node() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    db.add_edge(a, b, "knows", Vec::new()).unwrap();

    db.delete_node(a).unwrap();

    let edges = db.node(a).outgoing("knows").edges().unwrap();
    assert!(edges.is_empty());

    let nodes = db.node(a).outgoing("knows").nodes().unwrap();
    assert!(nodes.is_empty());

    let count = db.node(a).outgoing("knows").count().unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_traversal_with_self_loop() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    db.add_edge(a, a, "self_ref", Vec::new()).unwrap();

    let out_edges = db.node(a).outgoing("self_ref").edges().unwrap();
    assert_eq!(out_edges.len(), 1);
    assert_eq!(out_edges[0].from, a);
    assert_eq!(out_edges[0].to, a);

    let in_nodes = db.node(a).incoming("self_ref").nodes().unwrap();
    assert_eq!(in_nodes.len(), 1);
    assert_eq!(in_nodes[0].id, a);

    let out_count = db.node(a).outgoing("self_ref").count().unwrap();
    assert_eq!(out_count, 1);
}

#[test]
fn test_traversal_no_edges_for_label() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    db.add_edge(a, b, "knows", Vec::new()).unwrap();

    let edges = db.node(a).outgoing("follows").edges().unwrap();
    assert!(edges.is_empty());

    let count = db.node(a).outgoing("follows").count().unwrap();
    assert_eq!(count, 0);
}

use std::collections::{BinaryHeap, HashSet};

use crate::error::Result;
use crate::id::NodeId;
use crate::storage::ReadTxn;
use crate::storage::WriteTxn;
use crate::storage::engine::{Db, Scan};

use super::keys::*;
use super::similarity::SimilarityKind;

pub(crate) struct Hnsw;

impl Hnsw {
    pub(crate) fn insert(
        txn: &mut dyn WriteTxn,
        label: &str,
        property: &str,
        node_id: NodeId,
        vector: &[f32],
        meta: &VectorIndexMeta,
    ) -> Result<()> {
        let level = random_level(node_id, meta.m);

        txn.put(
            Db::VectorIndexes,
            &vec_key(label, property, node_id),
            &serialize_vector(vector),
        )?;
        txn.put(
            Db::VectorIndexes,
            &lvl_key(label, property, node_id),
            &[level],
        )?;

        let entry_point = match meta.entry_point {
            Some(ep) => ep,
            None => {
                txn.put(
                    Db::VectorIndexes,
                    &ep_key(label, property),
                    &node_id.to_le_bytes(),
                )?;
                let new_meta = VectorIndexMeta {
                    entry_point: Some(node_id),
                    max_level: level,
                    ..meta.clone()
                };
                txn.put(
                    Db::VectorIndexes,
                    &meta_key(label, property),
                    &new_meta.serialize(),
                )?;
                return Ok(());
            }
        };

        let ep_level = {
            let lvl_bytes = txn
                .get(Db::VectorIndexes, &lvl_key(label, property, entry_point))?
                .ok_or_else(|| {
                    crate::error::HelixiteError::Storage("missing entry point level".into())
                })?;
            lvl_bytes[0]
        };

        let mut current = entry_point;
        let mut new_entry_point = entry_point;
        if level > ep_level {
            for l in ((ep_level + 1)..=level).rev() {
                let ctx = SearchCtx {
                    label,
                    property,
                    query: vector,
                    ef: 1,
                    level: l,
                    similarity: meta.similarity,
                };
                let neighbors = search_layer(txn, current, &ctx)?;
                if let Some(&best) = neighbors.first() {
                    current = best;
                }
            }
            new_entry_point = node_id;
        } else if ep_level > level {
            for l in ((level + 1)..=ep_level).rev() {
                let ctx = SearchCtx {
                    label,
                    property,
                    query: vector,
                    ef: 1,
                    level: l,
                    similarity: meta.similarity,
                };
                let neighbors = search_layer(txn, current, &ctx)?;
                if let Some(&best) = neighbors.first() {
                    current = best;
                }
            }
        }

        for l in (0..=level.min(ep_level)).rev() {
            let ctx = SearchCtx {
                label,
                property,
                query: vector,
                ef: meta.ef_construction,
                level: l,
                similarity: meta.similarity,
            };
            let neighbors = search_layer(txn, current, &ctx)?;
            let selected = select_neighbors(
                txn,
                label,
                property,
                vector,
                &neighbors,
                meta.m,
                meta.similarity,
            )?;

            for &neighbor_id in &selected {
                add_link(txn, label, property, l, neighbor_id, node_id, meta)?;
                add_link(txn, label, property, l, node_id, neighbor_id, meta)?;
            }

            if let Some(&first) = selected.first() {
                current = first;
            }
        }

        let new_meta = VectorIndexMeta {
            entry_point: Some(new_entry_point),
            max_level: level.max(meta.max_level),
            ..meta.clone()
        };
        txn.put(
            Db::VectorIndexes,
            &meta_key(label, property),
            &new_meta.serialize(),
        )?;

        Ok(())
    }

    pub(crate) fn delete(
        txn: &mut dyn WriteTxn,
        label: &str,
        property: &str,
        node_id: NodeId,
        meta: &VectorIndexMeta,
    ) -> Result<()> {
        let node_level = match txn.get(Db::VectorIndexes, &lvl_key(label, property, node_id))? {
            Some(bytes) => bytes[0],
            None => return Ok(()),
        };

        for l in 0..=node_level {
            let prefix = lnk_prefix(label, property, l, node_id);
            let entries = txn.scan(Db::VectorIndexes, Scan::Prefix(&prefix), None)?;
            let keys: Vec<Vec<u8>> = entries.iter().map(|e| e.key.to_vec()).collect();
            for key in keys {
                txn.delete(Db::VectorIndexes, &key)?;
            }

            let level_prefix = lnk_level_prefix(label, property, l);
            let entries = txn.scan(Db::VectorIndexes, Scan::Prefix(&level_prefix), None)?;
            let keys: Vec<(Vec<u8>, Vec<u8>)> = entries
                .iter()
                .map(|e| (e.key.to_vec(), e.value.to_vec()))
                .collect();
            for (key, _) in &keys {
                let Some((_, _, target)) = decode_link_from_lnk_key(key) else {
                    continue;
                };
                if target != node_id {
                    continue;
                }
                txn.delete(Db::VectorIndexes, key)?;
            }
        }

        txn.delete(Db::VectorIndexes, &vec_key(label, property, node_id))?;
        txn.delete(Db::VectorIndexes, &lvl_key(label, property, node_id))?;

        if meta.entry_point == Some(node_id) {
            let mut new_ep = None;
            let mut new_max_level = 0u8;

            let vec_prefix = vec_prefix(label, property);
            let entries = txn.scan(Db::VectorIndexes, Scan::Prefix(&vec_prefix), None)?;
            for entry in &entries {
                let Some(nid) = decode_node_id_from_vec_key(entry.key) else {
                    continue;
                };
                if nid == node_id {
                    continue;
                }
                let Some(lvl_bytes) = txn.get(Db::VectorIndexes, &lvl_key(label, property, nid))?
                else {
                    continue;
                };
                let lvl = lvl_bytes[0];
                if lvl > new_max_level {
                    new_max_level = lvl;
                    new_ep = Some(nid);
                }
            }

            let new_meta = VectorIndexMeta {
                entry_point: new_ep,
                max_level: new_max_level,
                ..meta.clone()
            };
            txn.put(
                Db::VectorIndexes,
                &meta_key(label, property),
                &new_meta.serialize(),
            )?;
        }

        Ok(())
    }

    pub(crate) fn search(
        txn: &dyn ReadTxn,
        label: &str,
        property: &str,
        query: &[f32],
        k: usize,
        meta: &VectorIndexMeta,
    ) -> Result<Vec<(NodeId, f32)>> {
        let entry_point = match meta.entry_point {
            Some(ep) => ep,
            None => return Ok(Vec::new()),
        };

        let ep_level = {
            let lvl_bytes = txn
                .get(Db::VectorIndexes, &lvl_key(label, property, entry_point))?
                .ok_or_else(|| {
                    crate::error::HelixiteError::Storage("missing entry point level".into())
                })?;
            lvl_bytes[0]
        };

        let mut current = entry_point;
        for l in (0..=ep_level).rev() {
            let ctx = SearchCtx {
                label,
                property,
                query,
                ef: 1,
                level: l,
                similarity: meta.similarity,
            };
            let neighbors = search_layer(txn, current, &ctx)?;
            if let Some(&best) = neighbors.first() {
                current = best;
            }
        }

        let ctx = SearchCtx {
            label,
            property,
            query,
            ef: meta.ef_search,
            level: 0,
            similarity: meta.similarity,
        };
        let candidates = search_layer(txn, current, &ctx)?;

        let mut results = Vec::new();
        for &candidate_id in &candidates {
            let vec = load_vector(txn, label, property, candidate_id)?;
            let score = meta.similarity.compute(&vec, query)?;
            results.push((candidate_id, score));
        }

        results.sort_by(|a, b| {
            let ord = if meta.similarity.is_higher_better() {
                b.1.partial_cmp(&a.1)
            } else {
                a.1.partial_cmp(&b.1)
            };
            ord.unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        results.truncate(k);
        Ok(results)
    }
}

struct SearchCtx<'a> {
    label: &'a str,
    property: &'a str,
    query: &'a [f32],
    ef: usize,
    level: u8,
    similarity: SimilarityKind,
}

fn search_layer(txn: &dyn ReadTxn, entry: NodeId, ctx: &SearchCtx<'_>) -> Result<Vec<NodeId>> {
    let mut visited = HashSet::new();
    let mut candidates = BinaryHeap::new();
    let mut results = BinaryHeap::new();

    visited.insert(entry);
    let entry_vec = load_vector(txn, ctx.label, ctx.property, entry)?;
    let score = ctx.similarity.compute(&entry_vec, ctx.query)?;
    let candidate = Candidate {
        node_id: entry,
        score,
        similarity: ctx.similarity,
    };
    candidates.push(candidate.clone());
    results.push(std::cmp::Reverse(candidate));

    while let Some(candidate) = candidates.pop() {
        if results.len() > ctx.ef
            && let Some(std::cmp::Reverse(worst)) = results.peek()
            && !candidate.is_better_than(worst)
        {
            break;
        }

        let neighbors = load_neighbors(txn, ctx.label, ctx.property, ctx.level, candidate.node_id)?;
        for neighbor_id in neighbors {
            if !visited.contains(&neighbor_id) {
                visited.insert(neighbor_id);
                let neighbor_vec = load_vector(txn, ctx.label, ctx.property, neighbor_id)?;
                let score = ctx.similarity.compute(&neighbor_vec, ctx.query)?;
                let c = Candidate {
                    node_id: neighbor_id,
                    score,
                    similarity: ctx.similarity,
                };
                candidates.push(c.clone());
                results.push(std::cmp::Reverse(c));
                if results.len() > ctx.ef {
                    results.pop();
                }
            }
        }
    }

    let mut out: Vec<_> = results
        .into_iter()
        .map(|std::cmp::Reverse(c)| c.node_id)
        .collect();
    out.sort();
    Ok(out)
}

fn select_neighbors(
    txn: &dyn ReadTxn,
    label: &str,
    property: &str,
    query: &[f32],
    candidates: &[NodeId],
    m: usize,
    similarity: SimilarityKind,
) -> Result<Vec<NodeId>> {
    let mut scored = Vec::new();
    for &id in candidates {
        let vec = load_vector(txn, label, property, id)?;
        let score = similarity.compute(&vec, query)?;
        scored.push((id, score));
    }

    scored.sort_by(|a, b| {
        let ord = if similarity.is_higher_better() {
            b.1.partial_cmp(&a.1)
        } else {
            a.1.partial_cmp(&b.1)
        };
        ord.unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    scored.truncate(m);
    Ok(scored.into_iter().map(|(id, _)| id).collect())
}

fn add_link(
    txn: &mut dyn WriteTxn,
    label: &str,
    property: &str,
    level: u8,
    from: NodeId,
    to: NodeId,
    meta: &VectorIndexMeta,
) -> Result<()> {
    let key = lnk_key(label, property, level, from, to);
    txn.put(Db::VectorIndexes, &key, &[])?;

    let prefix = lnk_prefix(label, property, level, from);
    let scanned = txn.scan(Db::VectorIndexes, Scan::Prefix(&prefix), None)?;
    let entries: Vec<_> = scanned
        .iter()
        .map(|e| (e.key.to_vec(), e.value.to_vec()))
        .collect();

    if entries.len() > meta.m {
        prune_links(txn, label, property, from, &entries, meta)?;
    }

    Ok(())
}

fn prune_links(
    txn: &mut dyn WriteTxn,
    label: &str,
    property: &str,
    node_id: NodeId,
    entries: &[(Vec<u8>, Vec<u8>)],
    meta: &VectorIndexMeta,
) -> Result<()> {
    let mut neighbors = Vec::new();
    for (key, _) in entries {
        if let Some((_, _, neighbor_id)) = decode_link_from_lnk_key(key) {
            neighbors.push(neighbor_id);
        }
    }

    if neighbors.len() <= meta.m {
        return Ok(());
    }

    let node_vec = load_vector(txn, label, property, node_id)?;
    let mut scored = Vec::new();
    for neighbor_id in neighbors {
        let neighbor_vec = load_vector(txn, label, property, neighbor_id)?;
        let score = meta.similarity.compute(&node_vec, &neighbor_vec)?;
        scored.push((neighbor_id, score));
    }

    scored.sort_by(|a, b| {
        let ord = if meta.similarity.is_higher_better() {
            b.1.partial_cmp(&a.1)
        } else {
            a.1.partial_cmp(&b.1)
        };
        ord.unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    scored.truncate(meta.m);

    let keep: HashSet<NodeId> = scored.into_iter().map(|(id, _)| id).collect();

    for (key, _) in entries {
        if let Some((_, _, neighbor_id)) = decode_link_from_lnk_key(key)
            && !keep.contains(&neighbor_id)
        {
            txn.delete(Db::VectorIndexes, key)?;
        }
    }

    Ok(())
}

fn load_neighbors(
    txn: &dyn ReadTxn,
    label: &str,
    property: &str,
    level: u8,
    node_id: NodeId,
) -> Result<Vec<NodeId>> {
    let prefix = lnk_prefix(label, property, level, node_id);
    let mut neighbors = Vec::new();
    for entry in txn.scan(Db::VectorIndexes, Scan::Prefix(&prefix), None)? {
        if let Some((_, _, neighbor_id)) = decode_link_from_lnk_key(entry.key) {
            neighbors.push(neighbor_id);
        }
    }

    Ok(neighbors)
}

fn load_vector(
    txn: &dyn ReadTxn,
    label: &str,
    property: &str,
    node_id: NodeId,
) -> Result<Vec<f32>> {
    let bytes = txn
        .get(Db::VectorIndexes, &vec_key(label, property, node_id))?
        .ok_or_else(|| crate::error::HelixiteError::Storage("vector not found".into()))?;
    deserialize_vector(&bytes)
}

fn random_level(node_id: NodeId, m: usize) -> u8 {
    let m_inv = 1.0 / m as f64;
    let mut level = 0;
    let mut seed = node_id.wrapping_mul(6364136223846793005).wrapping_add(1);
    loop {
        let r = (seed as f64) / (u64::MAX as f64);
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        if r < m_inv {
            level += 1;
        } else {
            break;
        }
    }
    level
}

#[derive(Debug, Clone)]
struct Candidate {
    node_id: NodeId,
    score: f32,
    similarity: SimilarityKind,
}

impl Candidate {
    fn is_better_than(&self, other: &Self) -> bool {
        if self.similarity.is_higher_better() {
            self.score > other.score
        } else {
            self.score < other.score
        }
    }
}

impl Eq for Candidate {}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.score.to_bits() == other.score.to_bits() && self.node_id == other.node_id
    }
}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let ord = if self.similarity.is_higher_better() {
            self.score.partial_cmp(&other.score)
        } else {
            other.score.partial_cmp(&self.score)
        };
        ord.unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| self.node_id.cmp(&other.node_id))
    }
}

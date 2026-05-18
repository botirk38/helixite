use std::collections::{BinaryHeap, HashSet};

use crate::id::NodeId;
use crate::storage::StorageEngine;
use crate::storage::engine::Db;

use super::VectorIndex;

const M: usize = 16;
const EF_CONSTRUCTION: usize = 200;
const EF_SEARCH: usize = 50;

pub(crate) fn search(
    storage: &impl StorageEngine,
    label: &str,
    property: &str,
    query: &[f32],
    k: usize,
) -> crate::error::Result<Vec<(NodeId, f32)>> {
    let prefix = VectorIndex::prefix(super::VectorIndexKind::Hnsw, label, property);
    let entries = storage.scan_prefix(Db::VectorIndexes, &prefix)?;

    let mut index = HnswIndex::new(M, EF_CONSTRUCTION);
    for (key, value) in entries {
        let node_id = VectorIndex::decode_node_id(&key).ok_or_else(|| {
            crate::error::HelixiteError::Storage("corrupt vector index key".into())
        })?;
        let vector = deserialize_vector(&value)?;
        index.insert(node_id, vector);
    }

    Ok(index.search(query, k, EF_SEARCH))
}

fn deserialize_vector(bytes: &[u8]) -> crate::error::Result<Vec<f32>> {
    if !bytes.len().is_multiple_of(4) {
        return Err(crate::error::HelixiteError::Storage(
            "corrupt vector data: length not multiple of 4".into(),
        ));
    }
    let mut vector = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        let f = f32::from_le_bytes(chunk.try_into().unwrap());
        vector.push(f);
    }
    Ok(vector)
}

struct HnswNode {
    node_id: NodeId,
    vector: Vec<f32>,
    layers: Vec<Vec<NodeId>>,
}

struct HnswIndex {
    nodes: Vec<HnswNode>,
    entry_point: Option<NodeId>,
    max_level: u8,
    m: usize,
    ef_construction: usize,
}

impl HnswIndex {
    fn new(m: usize, ef_construction: usize) -> Self {
        Self {
            nodes: Vec::new(),
            entry_point: None,
            max_level: 0,
            m,
            ef_construction,
        }
    }

    fn insert(&mut self, node_id: NodeId, vector: Vec<f32>) {
        let level = Self::random_level(self.m);
        let new_node = HnswNode {
            node_id,
            vector,
            layers: vec![Vec::new(); (level + 1) as usize],
        };

        if self.entry_point.is_none() {
            self.entry_point = Some(node_id);
            self.max_level = level;
            self.nodes.push(new_node);
            return;
        }

        let entry = self.entry_point.unwrap();
        let Some(entry_node) = self.find_node(entry) else {
            self.nodes.push(new_node);
            return;
        };
        let entry_level = entry_node.layers.len() as u8 - 1;

        let mut current = entry;
        if entry_level > level {
            for l in ((level + 1)..=entry_level).rev() {
                let neighbors = self.search_layer(current, &new_node.vector, 1, l as usize);
                if let Some(&best) = neighbors.first() {
                    current = best;
                }
            }
        }

        for l in (0..=level.min(entry_level)).rev() {
            let neighbors =
                self.search_layer(current, &new_node.vector, self.ef_construction, l as usize);
            let selected = self.select_neighbors(&new_node.vector, &neighbors);

            self.update_node_layers(node_id, l as usize, selected.clone());

            for &neighbor_id in &selected {
                self.add_bidirectional_link(neighbor_id, node_id, l as usize);
            }

            if let Some(&first) = selected.first() {
                current = first;
            }
        }

        if level > self.max_level {
            self.max_level = level;
        }

        self.nodes.push(new_node);
    }

    fn search(&self, query: &[f32], k: usize, ef: usize) -> Vec<(NodeId, f32)> {
        if self.nodes.is_empty() {
            return Vec::new();
        }

        let entry = self.entry_point.unwrap();
        let Some(entry_node) = self.find_node(entry) else {
            return Vec::new();
        };
        let entry_level = entry_node.layers.len() as u8 - 1;

        let mut current = entry;
        for l in (0..=entry_level).rev() {
            let neighbors = self.search_layer(current, query, 1, l as usize);
            if let Some(&best) = neighbors.first() {
                current = best;
            }
        }

        let candidates = self.search_layer(current, query, ef, 0);
        let mut results: Vec<_> = candidates
            .into_iter()
            .filter_map(|id| {
                self.find_node(id)
                    .map(|n| (n.node_id, cosine_similarity(&n.vector, query)))
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(k);
        results
    }

    fn search_layer(&self, entry: NodeId, query: &[f32], ef: usize, level: usize) -> Vec<NodeId> {
        let mut visited = HashSet::new();
        let mut candidates = BinaryHeap::new();
        let mut results = BinaryHeap::new();

        visited.insert(entry);
        if let Some(node) = self.find_node(entry) {
            let sim = cosine_similarity(&node.vector, query);
            let candidate = Candidate {
                node_id: entry,
                similarity: sim,
            };
            candidates.push(candidate.clone());
            results.push(candidate);
        }

        while let Some(candidate) = candidates.pop() {
            if let Some(worst) = results.peek()
                && candidate.similarity < worst.similarity
            {
                break;
            }

            let Some(node) = self.find_node(candidate.node_id) else {
                continue;
            };
            if level >= node.layers.len() {
                continue;
            }

            for &neighbor in &node.layers[level] {
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    if let Some(n) = self.find_node(neighbor) {
                        let sim = cosine_similarity(&n.vector, query);
                        let c = Candidate {
                            node_id: neighbor,
                            similarity: sim,
                        };
                        candidates.push(c.clone());
                        results.push(c);
                        if results.len() > ef {
                            results.pop();
                        }
                    }
                }
            }
        }

        results.into_iter().map(|c| c.node_id).collect()
    }

    fn select_neighbors(&self, query: &[f32], candidates: &[NodeId]) -> Vec<NodeId> {
        let mut scored: Vec<_> = candidates
            .iter()
            .filter_map(|&id| {
                self.find_node(id)
                    .map(|n| (id, cosine_similarity(&n.vector, query)))
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(self.m);
        scored.into_iter().map(|(id, _)| id).collect()
    }

    fn update_node_layers(&mut self, node_id: NodeId, level: usize, neighbors: Vec<NodeId>) {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.node_id == node_id)
            && level < node.layers.len()
        {
            node.layers[level] = neighbors;
        }
    }

    fn add_bidirectional_link(&mut self, neighbor_id: NodeId, new_id: NodeId, level: usize) {
        let Some(idx) = self.nodes.iter().position(|n| n.node_id == neighbor_id) else {
            return;
        };
        if level >= self.nodes[idx].layers.len() {
            return;
        }
        self.nodes[idx].layers[level].push(new_id);
        if self.nodes[idx].layers[level].len() > self.m {
            let vector = self.nodes[idx].vector.clone();
            let current_neighbors = self.nodes[idx].layers[level].clone();
            let best = self.select_neighbors_by_id(&vector, &current_neighbors);
            self.nodes[idx].layers[level] = best;
        }
    }

    fn select_neighbors_by_id(&self, query: &[f32], candidates: &[NodeId]) -> Vec<NodeId> {
        let mut scored: Vec<_> = candidates
            .iter()
            .filter_map(|&id| {
                self.find_node(id)
                    .map(|n| (id, cosine_similarity(&n.vector, query)))
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(self.m);
        scored.into_iter().map(|(id, _)| id).collect()
    }

    fn find_node(&self, node_id: NodeId) -> Option<&HnswNode> {
        self.nodes.iter().find(|n| n.node_id == node_id)
    }

    fn random_level(m: usize) -> u8 {
        let m_inv = 1.0 / m as f64;
        let mut level = 0;
        loop {
            let r: f64 = rand::random();
            if r < m_inv {
                level += 1;
            } else {
                break;
            }
        }
        level
    }
}

#[derive(Debug, Clone)]
struct Candidate {
    node_id: NodeId,
    similarity: f32,
}

impl Eq for Candidate {}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.similarity.to_bits() == other.similarity.to_bits() && self.node_id == other.node_id
    }
}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.similarity
            .partial_cmp(&other.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
            .reverse()
            .then_with(|| self.node_id.cmp(&other.node_id))
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom == 0.0 { 0.0 } else { dot / denom }
}

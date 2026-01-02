use crate::core::{DuplicationBlock, DuplicationLocation};
use dashmap::DashMap;
use rayon::prelude::*;
use std::path::PathBuf;
use xxhash_rust::xxh64::xxh64;

/// Detects code duplication across multiple files using parallel processing.
///
/// Uses rayon for parallel file processing and DashMap for concurrent hash aggregation.
/// This provides 2-4x speedup on multi-core systems compared to sequential processing.
pub fn detect_duplication(
    files: Vec<(PathBuf, String)>,
    min_lines: usize,
    _similarity_threshold: f64,
) -> Vec<DuplicationBlock> {
    // Thread-safe concurrent map for parallel aggregation
    let chunk_locations: DashMap<u64, Vec<DuplicationLocation>> = DashMap::new();

    // Parallel processing of files - extract chunks and compute hashes concurrently
    files.par_iter().for_each(|(path, content)| {
        for (start_line, chunk) in extract_chunks(content, min_lines) {
            let hash = calculate_hash(&chunk);
            let location = DuplicationLocation {
                file: path.clone(),
                start_line,
                end_line: start_line + min_lines - 1,
            };

            // Thread-safe insertion into DashMap
            chunk_locations.entry(hash).or_default().push(location);
        }
    });

    // Convert to result - sequential but small compared to hashing
    chunk_locations
        .into_iter()
        .filter_map(|(hash, locations)| {
            (locations.len() > 1).then_some(DuplicationBlock {
                hash,
                lines: min_lines,
                locations,
            })
        })
        .collect()
}

fn extract_chunks(content: &str, chunk_size: usize) -> Vec<(usize, String)> {
    let lines: Vec<&str> = content.lines().collect();

    if lines.len() < chunk_size {
        return Vec::new();
    }

    (0..=lines.len() - chunk_size)
        .map(|i| {
            let chunk = lines[i..i + chunk_size].join("\n");
            (i + 1, normalize_chunk(&chunk))
        })
        .collect()
}

fn normalize_chunk(chunk: &str) -> String {
    chunk
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with("//") && !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Calculates a fast non-cryptographic hash using xxHash64.
///
/// Returns a u64 hash value suitable for duplication detection.
/// xxHash64 provides excellent distribution and is 10-20x faster than SHA256.
fn calculate_hash(content: &str) -> u64 {
    xxh64(content.as_bytes(), 0)
}

pub fn calculate_similarity(chunk1: &str, chunk2: &str) -> f64 {
    let tokens1 = tokenize(chunk1);
    let tokens2 = tokenize(chunk2);
    let tokens2_set: std::collections::HashSet<_> = tokens2.iter().collect();

    let intersection_count = tokens1.iter().filter(|t| tokens2_set.contains(t)).count();
    let union_count = tokens1.len() + tokens2.len() - intersection_count;

    match union_count {
        0 => 0.0,
        n => intersection_count as f64 / n as f64,
    }
}

fn tokenize(content: &str) -> Vec<String> {
    content
        .split_whitespace()
        .map(str::to_lowercase)
        .filter(|s| s.len() > 2)
        .collect()
}

pub fn merge_adjacent_duplications(mut blocks: Vec<DuplicationBlock>) -> Vec<DuplicationBlock> {
    blocks.sort_by_key(|b| (b.locations[0].file.clone(), b.locations[0].start_line));

    blocks.into_iter().fold(Vec::new(), |mut merged, block| {
        match merged.last_mut() {
            Some(last) if can_merge(last, &block) => {
                merge_blocks(last, block);
            }
            _ => merged.push(block),
        }
        merged
    })
}

fn can_merge(block1: &DuplicationBlock, block2: &DuplicationBlock) -> bool {
    block1.locations.iter().any(|loc1| {
        block2
            .locations
            .iter()
            .any(|loc2| loc1.file == loc2.file && loc1.end_line + 1 == loc2.start_line)
    })
}

fn merge_blocks(target: &mut DuplicationBlock, source: DuplicationBlock) {
    target.lines += source.lines;

    for loc in target.locations.iter_mut() {
        if let Some(src_loc) = source
            .locations
            .iter()
            .find(|src_loc| loc.file == src_loc.file && loc.end_line + 1 == src_loc.start_line)
        {
            loc.end_line = src_loc.end_line;
        }
    }
}

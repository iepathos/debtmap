use crate::core::{DuplicationBlock, DuplicationLocation};
use rayon::prelude::*;
use std::path::PathBuf;

mod similarity;
#[cfg(test)]
mod threshold_tests;

use similarity::{SimilarityChunk, group_similar_chunks};

/// Detects code duplication across multiple files using parallel processing.
///
/// Uses rayon for parallel chunk extraction and deterministic similarity grouping.
/// A threshold of `1.0` requires normalized text equality. Lower thresholds use
/// exact set-Jaccard token similarity and emit only directly verified pairs.
pub fn detect_duplication(
    files: Vec<(PathBuf, String)>,
    min_lines: usize,
    similarity_threshold: f64,
) -> Vec<DuplicationBlock> {
    if min_lines == 0
        || !similarity_threshold.is_finite()
        || !(0.0 < similarity_threshold && similarity_threshold <= 1.0)
    {
        return Vec::new();
    }

    let chunks = extract_project_chunks(&files, min_lines);
    group_similar_chunks(chunks, min_lines, similarity_threshold)
}

fn extract_project_chunks(files: &[(PathBuf, String)], chunk_size: usize) -> Vec<SimilarityChunk> {
    // Chunks come out in line order per file, so ordering files orders all chunks.
    let mut files: Vec<_> = files.iter().collect();
    files.sort_by(|left, right| left.0.cmp(&right.0));
    files
        .par_iter()
        .flat_map_iter(|(path, content)| {
            extract_chunks(content, chunk_size)
                .into_iter()
                .map(|(start_line, normalized)| {
                    SimilarityChunk::new(
                        normalized,
                        DuplicationLocation {
                            file: path.clone(),
                            start_line,
                            end_line: start_line + chunk_size - 1,
                        },
                    )
                })
        })
        .collect()
}

fn extract_chunks(content: &str, chunk_size: usize) -> Vec<(usize, String)> {
    let lines: Vec<&str> = content.lines().collect();

    if lines.len() < chunk_size {
        return Vec::new();
    }

    let normalized: Vec<_> = lines.iter().map(|line| normalize_line(line)).collect();
    normalized
        .windows(chunk_size)
        .enumerate()
        .map(|(i, window)| {
            let kept: Vec<_> = window.iter().flatten().copied().collect();
            (i + 1, kept.join("\n"))
        })
        .collect()
}

fn normalize_line(line: &str) -> Option<&str> {
    let line = line.trim();
    (!line.is_empty() && !line.starts_with("//") && !line.starts_with('#')).then_some(line)
}

pub fn calculate_similarity(chunk1: &str, chunk2: &str) -> f64 {
    similarity::calculate_similarity(chunk1, chunk2)
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

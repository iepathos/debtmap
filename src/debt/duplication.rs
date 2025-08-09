use crate::core::{DuplicationBlock, DuplicationLocation};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;

pub fn detect_duplication(
    files: Vec<(PathBuf, String)>,
    min_lines: usize,
    _similarity_threshold: f64,
) -> Vec<DuplicationBlock> {
    let mut hash_map: HashMap<String, Vec<DuplicationLocation>> = HashMap::new();

    for (path, content) in files {
        let chunks = extract_chunks(&content, min_lines);

        for (start_line, chunk) in chunks {
            let hash = calculate_hash(&chunk);

            hash_map
                .entry(hash.clone())
                .or_default()
                .push(DuplicationLocation {
                    file: path.clone(),
                    start_line,
                    end_line: start_line + min_lines - 1,
                });
        }
    }

    hash_map
        .into_iter()
        .filter(|(_, locations)| locations.len() > 1)
        .map(|(hash, locations)| DuplicationBlock {
            hash,
            lines: min_lines,
            locations,
        })
        .collect()
}

fn extract_chunks(content: &str, chunk_size: usize) -> Vec<(usize, String)> {
    let lines: Vec<&str> = content.lines().collect();
    let mut chunks = Vec::new();

    if lines.len() < chunk_size {
        return chunks;
    }

    for i in 0..=lines.len() - chunk_size {
        let chunk = lines[i..i + chunk_size].join("\n");
        chunks.push((i + 1, normalize_chunk(&chunk)));
    }

    chunks
}

fn normalize_chunk(chunk: &str) -> String {
    chunk
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with("//") && !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

fn calculate_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn calculate_similarity(chunk1: &str, chunk2: &str) -> f64 {
    let tokens1 = tokenize(chunk1);
    let tokens2 = tokenize(chunk2);

    let intersection = tokens1.iter().filter(|t| tokens2.contains(t)).count();
    let union = tokens1.len() + tokens2.len() - intersection;

    if union == 0 {
        return 0.0;
    }

    intersection as f64 / union as f64
}

fn tokenize(content: &str) -> Vec<String> {
    content
        .split_whitespace()
        .map(|s| s.to_lowercase())
        .filter(|s| s.len() > 2)
        .collect()
}

pub fn merge_adjacent_duplications(blocks: Vec<DuplicationBlock>) -> Vec<DuplicationBlock> {
    let mut merged = Vec::new();
    let mut sorted = blocks;
    sorted.sort_by_key(|b| (b.locations[0].file.clone(), b.locations[0].start_line));

    for block in sorted {
        if let Some(last) = merged.last_mut() {
            if can_merge(last, &block) {
                merge_blocks(last, block);
            } else {
                merged.push(block);
            }
        } else {
            merged.push(block);
        }
    }

    merged
}

fn can_merge(block1: &DuplicationBlock, block2: &DuplicationBlock) -> bool {
    for loc1 in &block1.locations {
        for loc2 in &block2.locations {
            if loc1.file == loc2.file && loc1.end_line + 1 == loc2.start_line {
                return true;
            }
        }
    }
    false
}

fn merge_blocks(target: &mut DuplicationBlock, source: DuplicationBlock) {
    target.lines += source.lines;
    for loc in &mut target.locations {
        for src_loc in &source.locations {
            if loc.file == src_loc.file && loc.end_line + 1 == src_loc.start_line {
                loc.end_line = src_loc.end_line;
            }
        }
    }
}

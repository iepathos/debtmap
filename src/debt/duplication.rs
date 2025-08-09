use crate::core::{DuplicationBlock, DuplicationLocation};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;

pub fn detect_duplication(
    files: Vec<(PathBuf, String)>,
    min_lines: usize,
    _similarity_threshold: f64,
) -> Vec<DuplicationBlock> {
    let chunk_locations = files
        .into_iter()
        .flat_map(|(path, content)| {
            extract_chunks(&content, min_lines)
                .into_iter()
                .map(move |(start_line, chunk)| {
                    let hash = calculate_hash(&chunk);
                    let location = DuplicationLocation {
                        file: path.clone(),
                        start_line,
                        end_line: start_line + min_lines - 1,
                    };
                    (hash, location)
                })
        })
        .fold(
            HashMap::<String, Vec<DuplicationLocation>>::new(),
            |mut acc, (hash, location)| {
                acc.entry(hash).or_default().push(location);
                acc
            },
        );

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

fn calculate_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
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

    target.locations.iter_mut().for_each(|loc| {
        if let Some(src_loc) = source
            .locations
            .iter()
            .find(|src_loc| loc.file == src_loc.file && loc.end_line + 1 == src_loc.start_line)
        {
            loc.end_line = src_loc.end_line;
        }
    });
}

use crate::core::{DuplicationBlock, DuplicationLocation};
use foldhash::HashSet;
use rayon::slice::ParallelSliceMut;
use std::collections::{BTreeMap, BTreeSet};

mod blocks;
#[cfg(test)]
mod tests;

use blocks::{exact_blocks_not_in_pairs, fuzzy_block, sort_blocks};

pub(super) struct SimilarityChunk {
    normalized: String,
    hash: u64,
    pub location: DuplicationLocation,
}

impl SimilarityChunk {
    pub fn new(normalized: String, location: DuplicationLocation) -> Self {
        Self {
            hash: xxhash_rust::xxh64::xxh64(normalized.as_bytes(), 0),
            normalized,
            location,
        }
    }
}

pub(super) struct ContentGroup {
    pub normalized: String,
    pub locations: Vec<DuplicationLocation>,
    tokens: HashSet<String>,
}

pub(super) fn group_similar_chunks(
    chunks: Vec<SimilarityChunk>,
    min_lines: usize,
    threshold: f64,
) -> Vec<DuplicationBlock> {
    if threshold == 1.0 {
        return exact_hash_blocks(chunks, min_lines);
    }

    let groups = group_exact_content(chunks);
    let pairs = matching_pairs(&groups, threshold);
    let fuzzy_groups: BTreeSet<_> = pairs.iter().flat_map(|pair| [pair.0, pair.1]).collect();
    let mut blocks = exact_blocks_not_in_pairs(&groups, &fuzzy_groups, min_lines);
    blocks.extend(
        pairs
            .into_iter()
            .map(|pair| fuzzy_block(&groups, pair, min_lines)),
    );
    sort_blocks(&mut blocks);
    blocks
}

fn exact_hash_blocks(mut chunks: Vec<SimilarityChunk>, min_lines: usize) -> Vec<DuplicationBlock> {
    // Stable, so chunks sharing a hash keep their file and line order.
    chunks.par_sort_by_key(|chunk| chunk.hash);
    let mut blocks: Vec<_> = chunks
        .chunk_by(|left, right| left.hash == right.hash)
        .filter(|bucket| bucket.len() > 1)
        .flat_map(|bucket| verified_exact_blocks(bucket[0].hash, bucket, min_lines))
        .collect();
    sort_blocks(&mut blocks);
    blocks
}

fn verified_exact_blocks(
    hash: u64,
    chunks: &[SimilarityChunk],
    min_lines: usize,
) -> Vec<DuplicationBlock> {
    chunks
        .iter()
        .fold(
            BTreeMap::<&str, Vec<DuplicationLocation>>::new(),
            |mut contents, chunk| {
                contents
                    .entry(&chunk.normalized)
                    .or_default()
                    .push(chunk.location.clone());
                contents
            },
        )
        .into_values()
        .filter(|locations| locations.len() > 1)
        .map(|locations| DuplicationBlock {
            hash,
            lines: min_lines,
            locations,
        })
        .collect()
}

pub(super) fn calculate_similarity(left: &str, right: &str) -> f64 {
    token_similarity(&tokenize(left), &tokenize(right))
}

fn group_exact_content(chunks: Vec<SimilarityChunk>) -> Vec<ContentGroup> {
    chunks
        .into_iter()
        .fold(
            BTreeMap::<String, Vec<DuplicationLocation>>::new(),
            |mut map, chunk| {
                map.entry(chunk.normalized)
                    .or_default()
                    .push(chunk.location);
                map
            },
        )
        .into_iter()
        .map(|(normalized, locations)| ContentGroup {
            tokens: tokenize(&normalized),
            normalized,
            locations,
        })
        .collect()
}

fn matching_pairs(groups: &[ContentGroup], threshold: f64) -> Vec<(usize, usize)> {
    token_postings(groups)
        .into_values()
        .fold(
            BTreeMap::<(usize, usize), usize>::new(),
            |mut overlaps, group_ids| {
                add_posting_overlaps(groups, threshold, &group_ids, &mut overlaps);
                overlaps
            },
        )
        .into_iter()
        .filter(|((left, right), intersection)| {
            jaccard_from_intersection(groups, *left, *right, *intersection) >= threshold
        })
        .map(|(pair, _)| pair)
        .collect()
}

fn token_postings(groups: &[ContentGroup]) -> BTreeMap<&str, Vec<usize>> {
    groups
        .iter()
        .enumerate()
        .flat_map(|(group_id, group)| {
            group
                .tokens
                .iter()
                .map(move |token| (token.as_str(), group_id))
        })
        .fold(BTreeMap::new(), |mut postings, (token, group_id)| {
            postings.entry(token).or_default().push(group_id);
            postings
        })
}

fn add_posting_overlaps(
    groups: &[ContentGroup],
    threshold: f64,
    group_ids: &[usize],
    overlaps: &mut BTreeMap<(usize, usize), usize>,
) {
    for (offset, left) in group_ids.iter().enumerate() {
        for right in group_ids[offset + 1..]
            .iter()
            .filter(|right| size_can_match(&groups[*left], &groups[**right], threshold))
        {
            *overlaps.entry((*left, *right)).or_default() += 1;
        }
    }
}

fn size_can_match(left: &ContentGroup, right: &ContentGroup, threshold: f64) -> bool {
    let smaller = left.tokens.len().min(right.tokens.len()) as f64;
    let larger = left.tokens.len().max(right.tokens.len()) as f64;
    larger > 0.0 && smaller / larger >= threshold
}

fn jaccard_from_intersection(
    groups: &[ContentGroup],
    left: usize,
    right: usize,
    intersection: usize,
) -> f64 {
    let union = groups[left].tokens.len() + groups[right].tokens.len() - intersection;
    intersection as f64 / union as f64
}

fn tokenize(content: &str) -> HashSet<String> {
    content
        .split_whitespace()
        .map(str::to_lowercase)
        .filter(|token| token.len() > 2)
        .collect()
}

fn token_similarity(left: &HashSet<String>, right: &HashSet<String>) -> f64 {
    let union = left.union(right).count();
    if union == 0 {
        return 0.0;
    }
    left.intersection(right).count() as f64 / union as f64
}

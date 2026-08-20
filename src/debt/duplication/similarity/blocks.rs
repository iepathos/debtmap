use super::ContentGroup;
use crate::core::{DuplicationBlock, DuplicationLocation};
use std::collections::BTreeSet;
use xxhash_rust::xxh64::xxh64;

pub(super) fn exact_blocks(groups: &[ContentGroup], min_lines: usize) -> Vec<DuplicationBlock> {
    let mut blocks: Vec<_> = groups
        .iter()
        .filter(|group| group.locations.len() > 1)
        .map(|group| exact_block(group, min_lines))
        .collect();
    sort_blocks(&mut blocks);
    blocks
}

pub(super) fn exact_blocks_not_in_pairs(
    groups: &[ContentGroup],
    fuzzy_groups: &BTreeSet<usize>,
    min_lines: usize,
) -> Vec<DuplicationBlock> {
    groups
        .iter()
        .enumerate()
        .filter(|(index, group)| group.locations.len() > 1 && !fuzzy_groups.contains(index))
        .map(|(_, group)| exact_block(group, min_lines))
        .collect()
}

fn exact_block(group: &ContentGroup, min_lines: usize) -> DuplicationBlock {
    DuplicationBlock {
        hash: xxh64(group.normalized.as_bytes(), 0),
        lines: min_lines,
        locations: group.locations.clone(),
    }
}

pub(super) fn fuzzy_block(
    groups: &[ContentGroup],
    (left, right): (usize, usize),
    min_lines: usize,
) -> DuplicationBlock {
    let mut locations = groups[left].locations.clone();
    locations.extend(groups[right].locations.clone());
    locations.sort_by(location_order);
    DuplicationBlock {
        hash: fuzzy_hash(&groups[left].normalized, &groups[right].normalized),
        lines: min_lines,
        locations,
    }
}

fn fuzzy_hash(left: &str, right: &str) -> u64 {
    let mut evidence = b"debtmap:fuzzy-duplication:v1".to_vec();
    evidence.extend_from_slice(&(left.len() as u64).to_le_bytes());
    evidence.extend_from_slice(left.as_bytes());
    evidence.extend_from_slice(&(right.len() as u64).to_le_bytes());
    evidence.extend_from_slice(right.as_bytes());
    xxh64(&evidence, 0)
}

pub(super) fn sort_blocks(blocks: &mut [DuplicationBlock]) {
    blocks.sort_by(|left, right| {
        left.locations
            .iter()
            .map(location_key)
            .cmp(right.locations.iter().map(location_key))
            .then(left.hash.cmp(&right.hash))
    });
}

fn location_order(left: &DuplicationLocation, right: &DuplicationLocation) -> std::cmp::Ordering {
    location_key(left).cmp(&location_key(right))
}

fn location_key(location: &DuplicationLocation) -> (&std::path::Path, usize, usize) {
    (&location.file, location.start_line, location.end_line)
}

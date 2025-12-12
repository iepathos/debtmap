//! Stage 6: Grouping - Pure grouping functions for view items.
//!
//! This module provides pure functions for grouping items by location
//! and computing combined scores.

use crate::priority::view::{LocationGroup, SortCriteria, ViewItem};
use std::collections::HashMap;
use std::path::PathBuf;

/// Computes location groups from items.
///
/// Groups items by (file, function, line) and calculates combined scores.
pub fn compute_groups(items: &[ViewItem], sort_by: SortCriteria) -> Vec<LocationGroup> {
    let mut groups_map: HashMap<(PathBuf, String, usize), Vec<ViewItem>> = HashMap::new();

    for item in items {
        let loc = item.location();
        let key = loc.group_key();
        groups_map.entry(key).or_default().push(item.clone());
    }

    let mut groups: Vec<LocationGroup> = groups_map
        .into_values()
        .map(|group_items| {
            let location = group_items[0].location();
            LocationGroup::new(location, group_items)
        })
        .collect();

    // Sort groups by same criteria as items
    sort_groups(&mut groups, sort_by);

    groups
}

/// Sorts groups by criteria.
fn sort_groups(groups: &mut [LocationGroup], criteria: SortCriteria) {
    match criteria {
        SortCriteria::Score => {
            groups.sort_by(|a, b| {
                b.combined_score
                    .partial_cmp(&a.combined_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        SortCriteria::FilePath => {
            groups.sort_by(|a, b| a.location.file.cmp(&b.location.file));
        }
        SortCriteria::FunctionName => {
            groups.sort_by(|a, b| {
                let name_a = a.location.function.as_deref().unwrap_or("");
                let name_b = b.location.function.as_deref().unwrap_or("");
                name_a.cmp(name_b)
            });
        }
        // For coverage/complexity, use combined score as fallback
        _ => {
            groups.sort_by(|a, b| {
                b.combined_score
                    .partial_cmp(&a.combined_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    }
}

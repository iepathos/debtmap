use super::*;
use std::path::PathBuf;

fn chunk(name: &str, text: &str) -> SimilarityChunk {
    SimilarityChunk::new(
        text.into(),
        DuplicationLocation {
            file: PathBuf::from(name),
            start_line: 1,
            end_line: 3,
        },
    )
}

#[test]
fn similarity_is_symmetric_and_bounded_with_repeated_tokens() {
    let forward = calculate_similarity("foo foo foo", "foo");
    let reverse = calculate_similarity("foo", "foo foo foo");

    assert_eq!(forward, reverse);
    assert_eq!(forward, 1.0);
}

#[test]
fn does_not_merge_similarity_transitively() {
    let blocks = group_similar_chunks(
        vec![
            chunk("a.rs", "alpha bravo charlie"),
            chunk("b.rs", "alpha bravo delta"),
            chunk("c.rs", "alpha delta echo"),
        ],
        3,
        0.5,
    );

    assert_eq!(blocks.len(), 2);
    assert!(blocks.iter().all(|block| block.locations.len() == 2));
}

#[test]
fn exact_empty_token_chunks_survive_fuzzy_mode() {
    let blocks = group_similar_chunks(vec![chunk("a.rs", "x y"), chunk("b.rs", "x y")], 3, 0.8);

    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].locations.len(), 2);
}

#[test]
fn indexed_pairs_match_brute_force_for_small_token_sets() {
    let chunks: Vec<_> = (1_u8..16)
        .map(|mask| {
            let text = ["alpha", "bravo", "charlie", "delta"]
                .into_iter()
                .enumerate()
                .filter(|(index, _)| mask & (1 << index) != 0)
                .map(|(_, token)| token)
                .collect::<Vec<_>>()
                .join(" ");
            chunk(&format!("{mask}.rs"), &text)
        })
        .collect();
    let groups = group_exact_content(chunks);

    for threshold in [0.1, 0.5, 0.8, 0.99] {
        let actual: BTreeSet<_> = matching_pairs(&groups, threshold).into_iter().collect();
        let expected: BTreeSet<_> = (0..groups.len())
            .flat_map(|left| (left + 1..groups.len()).map(move |right| (left, right)))
            .filter(|(left, right)| {
                token_similarity(&groups[*left].tokens, &groups[*right].tokens) >= threshold
            })
            .collect();
        assert_eq!(actual, expected, "threshold {threshold}");
    }
}

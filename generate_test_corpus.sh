#!/bin/bash

# Script to generate test corpus for spec 111 validation

POSITIVE_DIR="tests/fixtures/functional_patterns/positive"
NEGATIVE_DIR="tests/fixtures/functional_patterns/negative"
EDGE_DIR="tests/fixtures/functional_patterns/edge_cases"

# Positive examples - clear functional patterns

# Iterator chains
cat > "$POSITIVE_DIR/p02_fold.rs" << 'EOF'
fn sum_values(items: Vec<i32>) -> i32 {
    items.iter().fold(0, |acc, x| acc + x)
}
EOF

cat > "$POSITIVE_DIR/p03_chain_take.rs" << 'EOF'
fn first_positive(items: Vec<i32>) -> Vec<i32> {
    items.into_iter()
        .filter(|&x| x > 0)
        .take(10)
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p04_flat_map.rs" << 'EOF'
fn flatten_and_double(nested: Vec<Vec<i32>>) -> Vec<i32> {
    nested.into_iter()
        .flat_map(|v| v)
        .map(|x| x * 2)
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p05_filter_map.rs" << 'EOF'
fn parse_numbers(items: Vec<String>) -> Vec<i32> {
    items.iter()
        .filter_map(|s| s.parse::<i32>().ok())
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p06_partition.rs" << 'EOF'
fn split_even_odd(items: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    items.into_iter()
        .partition(|&x| x % 2 == 0)
}
EOF

cat > "$POSITIVE_DIR/p07_zip.rs" << 'EOF'
fn combine_lists(a: Vec<i32>, b: Vec<i32>) -> Vec<(i32, i32)> {
    a.into_iter()
        .zip(b.into_iter())
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p08_scan.rs" << 'EOF'
fn running_sum(items: Vec<i32>) -> Vec<i32> {
    items.iter()
        .scan(0, |state, x| {
            *state += x;
            Some(*state)
        })
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p09_enumerate.rs" << 'EOF'
fn with_indices(items: Vec<String>) -> Vec<(usize, String)> {
    items.into_iter()
        .enumerate()
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p10_chain.rs" << 'EOF'
fn merge_lists(a: Vec<i32>, b: Vec<i32>) -> Vec<i32> {
    a.into_iter()
        .chain(b.into_iter())
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p11_skip_take.rs" << 'EOF'
fn get_page(items: Vec<i32>, page: usize, size: usize) -> Vec<i32> {
    items.into_iter()
        .skip(page * size)
        .take(size)
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p12_any_all.rs" << 'EOF'
fn check_conditions(items: Vec<i32>) -> (bool, bool) {
    let has_positive = items.iter().any(|&x| x > 0);
    let all_positive = items.iter().all(|&x| x > 0);
    (has_positive, all_positive)
}
EOF

cat > "$POSITIVE_DIR/p13_find.rs" << 'EOF'
fn find_first_even(items: Vec<i32>) -> Option<i32> {
    items.into_iter()
        .find(|&x| x % 2 == 0)
}
EOF

cat > "$POSITIVE_DIR/p14_position.rs" << 'EOF'
fn find_position(items: Vec<i32>, target: i32) -> Option<usize> {
    items.iter()
        .position(|&x| x == target)
}
EOF

cat > "$POSITIVE_DIR/p15_max_min.rs" << 'EOF'
fn find_extremes(items: Vec<i32>) -> (Option<i32>, Option<i32>) {
    let max = items.iter().max().copied();
    let min = items.iter().min().copied();
    (max, min)
}
EOF

cat > "$POSITIVE_DIR/p16_sum.rs" << 'EOF'
fn calculate_total(items: Vec<f64>) -> f64 {
    items.iter().sum()
}
EOF

cat > "$POSITIVE_DIR/p17_product.rs" << 'EOF'
fn calculate_product(items: Vec<i32>) -> i32 {
    items.iter().product()
}
EOF

cat > "$POSITIVE_DIR/p18_cloned.rs" << 'EOF'
fn clone_and_process(items: &[i32]) -> Vec<i32> {
    items.iter()
        .cloned()
        .filter(|&x| x > 0)
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p19_copied.rs" << 'EOF'
fn copy_and_transform(items: &[i32]) -> Vec<i32> {
    items.iter()
        .copied()
        .map(|x| x * 2)
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p20_inspect.rs" << 'EOF'
fn process_with_logging(items: Vec<i32>) -> Vec<i32> {
    items.into_iter()
        .inspect(|x| println!("Processing: {}", x))
        .filter(|&x| x > 0)
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p21_rev.rs" << 'EOF'
fn reverse_and_filter(items: Vec<i32>) -> Vec<i32> {
    items.into_iter()
        .rev()
        .filter(|&x| x > 0)
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p22_step_by.rs" << 'EOF'
fn every_nth(items: Vec<i32>, n: usize) -> Vec<i32> {
    items.into_iter()
        .step_by(n)
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p23_collect_result.rs" << 'EOF'
fn parse_all(items: Vec<String>) -> Result<Vec<i32>, std::num::ParseIntError> {
    items.iter()
        .map(|s| s.parse::<i32>())
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p24_collect_option.rs" << 'EOF'
fn try_parse_all(items: Vec<String>) -> Option<Vec<i32>> {
    items.iter()
        .map(|s| s.parse::<i32>().ok())
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p25_try_fold.rs" << 'EOF'
fn safe_sum(items: Vec<Result<i32, String>>) -> Result<i32, String> {
    items.into_iter()
        .try_fold(0, |acc, r| r.map(|v| acc + v))
}
EOF

cat > "$POSITIVE_DIR/p26_unzip.rs" << 'EOF'
fn separate_pairs(items: Vec<(i32, i32)>) -> (Vec<i32>, Vec<i32>) {
    items.into_iter().unzip()
}
EOF

cat > "$POSITIVE_DIR/p27_cycle.rs" << 'EOF'
fn repeat_pattern(items: Vec<i32>, count: usize) -> Vec<i32> {
    items.iter()
        .cycle()
        .take(count)
        .copied()
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p28_peekable.rs" << 'EOF'
fn group_consecutive(items: Vec<i32>) -> Vec<Vec<i32>> {
    let mut result = Vec::new();
    let mut iter = items.into_iter().peekable();

    while iter.peek().is_some() {
        let current = iter.next().unwrap();
        let group: Vec<i32> = std::iter::once(current)
            .chain(iter.peeking_take_while(|&x| x == current))
            .collect();
        result.push(group);
    }
    result
}
EOF

cat > "$POSITIVE_DIR/p29_windows.rs" << 'EOF'
fn pairwise_sums(items: &[i32]) -> Vec<i32> {
    items.windows(2)
        .map(|w| w[0] + w[1])
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p30_chunks.rs" << 'EOF'
fn batch_process(items: &[i32], batch_size: usize) -> Vec<i32> {
    items.chunks(batch_size)
        .map(|chunk| chunk.iter().sum())
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p31_split.rs" << 'EOF'
fn split_by_zero(items: &[i32]) -> Vec<Vec<i32>> {
    items.split(|&x| x == 0)
        .map(|slice| slice.to_vec())
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p32_filter_chain.rs" << 'EOF'
fn complex_filter(items: Vec<i32>) -> Vec<i32> {
    items.into_iter()
        .filter(|&x| x > 0)
        .filter(|&x| x < 100)
        .filter(|&x| x % 2 == 0)
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p33_map_chain.rs" << 'EOF'
fn transform_chain(items: Vec<i32>) -> Vec<i32> {
    items.into_iter()
        .map(|x| x * 2)
        .map(|x| x + 1)
        .map(|x| x / 2)
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p34_mixed_chain.rs" << 'EOF'
fn process_pipeline(items: Vec<i32>) -> i32 {
    items.into_iter()
        .filter(|&x| x > 0)
        .map(|x| x * 2)
        .filter(|&x| x < 100)
        .fold(0, |acc, x| acc + x)
}
EOF

cat > "$POSITIVE_DIR/p35_nested_map.rs" << 'EOF'
fn process_nested(items: Vec<Vec<i32>>) -> Vec<Vec<i32>> {
    items.into_iter()
        .map(|v| v.into_iter()
            .filter(|&x| x > 0)
            .collect())
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p36_option_chain.rs" << 'EOF'
fn process_optional(opt: Option<i32>) -> Option<i32> {
    opt.map(|x| x * 2)
       .filter(|&x| x > 0)
       .map(|x| x + 1)
}
EOF

cat > "$POSITIVE_DIR/p37_result_chain.rs" << 'EOF'
fn process_fallible(res: Result<i32, String>) -> Result<i32, String> {
    res.map(|x| x * 2)
       .and_then(|x| if x > 0 { Ok(x) } else { Err("Negative".to_string()) })
}
EOF

cat > "$POSITIVE_DIR/p38_hashmap_iter.rs" << 'EOF'
use std::collections::HashMap;

fn process_map(map: HashMap<String, i32>) -> Vec<i32> {
    map.into_iter()
        .map(|(_, v)| v)
        .filter(|&v| v > 0)
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p39_btreemap_iter.rs" << 'EOF'
use std::collections::BTreeMap;

fn sorted_values(map: BTreeMap<String, i32>) -> Vec<i32> {
    map.into_values()
        .filter(|&v| v > 0)
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p40_set_operations.rs" << 'EOF'
use std::collections::HashSet;

fn unique_positive(items: Vec<i32>) -> HashSet<i32> {
    items.into_iter()
        .filter(|&x| x > 0)
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p41_string_chars.rs" << 'EOF'
fn process_chars(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p42_string_lines.rs" << 'EOF'
fn non_empty_lines(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| !line.is_empty())
        .map(String::from)
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p43_bytes_iter.rs" << 'EOF'
fn filter_bytes(data: &[u8]) -> Vec<u8> {
    data.iter()
        .copied()
        .filter(|&b| b > 32)
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p44_range_iter.rs" << 'EOF'
fn range_sum(start: i32, end: i32) -> i32 {
    (start..end)
        .filter(|&x| x % 2 == 0)
        .sum()
}
EOF

cat > "$POSITIVE_DIR/p45_once_chain.rs" << 'EOF'
fn prepend_value(items: Vec<i32>, val: i32) -> Vec<i32> {
    std::iter::once(val)
        .chain(items.into_iter())
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p46_repeat_with.rs" << 'EOF'
fn generate_sequence(count: usize) -> Vec<i32> {
    std::iter::repeat_with(|| rand::random::<i32>())
        .take(count)
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p47_from_fn.rs" << 'EOF'
fn fibonacci(n: usize) -> Vec<i32> {
    let mut a = 0;
    let mut b = 1;
    std::iter::from_fn(move || {
        let result = a;
        let next = a + b;
        a = b;
        b = next;
        Some(result)
    })
    .take(n)
    .collect()
}
EOF

cat > "$POSITIVE_DIR/p48_successors.rs" << 'EOF'
fn powers_of_two(n: usize) -> Vec<i32> {
    std::iter::successors(Some(1), |&x| Some(x * 2))
        .take(n)
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p49_by_ref.rs" << 'EOF'
fn consume_partially(items: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    let mut iter = items.into_iter();
    let first_five: Vec<i32> = iter.by_ref().take(5).collect();
    let rest: Vec<i32> = iter.collect();
    (first_five, rest)
}
EOF

cat > "$POSITIVE_DIR/p50_fuse.rs" << 'EOF'
fn safe_iteration(items: Vec<Option<i32>>) -> Vec<i32> {
    items.into_iter()
        .fuse()
        .filter_map(|x| x)
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p51_dedup.rs" << 'EOF'
fn remove_duplicates(mut items: Vec<i32>) -> Vec<i32> {
    items.sort();
    items.into_iter()
        .collect::<Vec<_>>()
        .into_iter()
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p52_group_by.rs" << 'EOF'
fn group_by_parity(items: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    items.into_iter()
        .fold((Vec::new(), Vec::new()), |(mut evens, mut odds), x| {
            if x % 2 == 0 {
                evens.push(x);
            } else {
                odds.push(x);
            }
            (evens, odds)
        })
}
EOF

cat > "$POSITIVE_DIR/p53_parallel_map.rs" << 'EOF'
use rayon::prelude::*;

fn parallel_process(items: Vec<i32>) -> Vec<i32> {
    items.par_iter()
        .map(|x| x * 2)
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p54_parallel_filter.rs" << 'EOF'
use rayon::prelude::*;

fn parallel_filter(items: Vec<i32>) -> Vec<i32> {
    items.into_par_iter()
        .filter(|&x| x > 0)
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p55_reduce.rs" << 'EOF'
fn find_max(items: Vec<i32>) -> Option<i32> {
    items.into_iter()
        .reduce(|a, b| if a > b { a } else { b })
}
EOF

cat > "$POSITIVE_DIR/p56_flat_map_option.rs" << 'EOF'
fn extract_values(items: Vec<Option<Vec<i32>>>) -> Vec<i32> {
    items.into_iter()
        .flat_map(|opt| opt.into_iter().flatten())
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p57_take_while.rs" << 'EOF'
fn take_until_negative(items: Vec<i32>) -> Vec<i32> {
    items.into_iter()
        .take_while(|&x| x >= 0)
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p58_skip_while.rs" << 'EOF'
fn skip_leading_zeros(items: Vec<i32>) -> Vec<i32> {
    items.into_iter()
        .skip_while(|&x| x == 0)
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p59_map_while.rs" << 'EOF'
fn parse_until_error(items: Vec<String>) -> Vec<i32> {
    items.into_iter()
        .map_while(|s| s.parse::<i32>().ok())
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p60_array_chunks.rs" << 'EOF'
fn process_triplets(items: &[i32]) -> Vec<i32> {
    items.chunks(3)
        .map(|chunk| chunk.iter().sum())
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p61_intersperse.rs" << 'EOF'
fn join_with_separator(items: Vec<i32>, sep: i32) -> Vec<i32> {
    items.into_iter()
        .flat_map(|x| vec![x, sep])
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p62_cartesian_product.rs" << 'EOF'
fn cross_product(a: Vec<i32>, b: Vec<i32>) -> Vec<(i32, i32)> {
    a.iter()
        .flat_map(|&x| b.iter().map(move |&y| (x, y)))
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p63_transpose.rs" << 'EOF'
fn transpose_options(items: Vec<Option<i32>>) -> Option<Vec<i32>> {
    items.into_iter().collect()
}
EOF

cat > "$POSITIVE_DIR/p64_collect_hashmap.rs" << 'EOF'
use std::collections::HashMap;

fn build_map(items: Vec<(String, i32)>) -> HashMap<String, i32> {
    items.into_iter()
        .filter(|(_, v)| *v > 0)
        .collect()
}
EOF

cat > "$POSITIVE_DIR/p65_extend.rs" << 'EOF'
fn merge_with_filter(mut base: Vec<i32>, new: Vec<i32>) -> Vec<i32> {
    base.extend(new.into_iter().filter(|&x| x > 0));
    base
}
EOF

echo "Generated positive examples"

# Negative examples - imperative code without functional patterns

cat > "$NEGATIVE_DIR/n01_for_loop.rs" << 'EOF'
fn imperative_sum(items: Vec<i32>) -> i32 {
    let mut sum = 0;
    for item in items {
        sum += item;
    }
    sum
}
EOF

cat > "$NEGATIVE_DIR/n02_while_loop.rs" << 'EOF'
fn while_loop_process(mut items: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    while !items.is_empty() {
        if let Some(item) = items.pop() {
            result.push(item * 2);
        }
    }
    result
}
EOF

cat > "$NEGATIVE_DIR/n03_index_loop.rs" << 'EOF'
fn index_based_process(items: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    for i in 0..items.len() {
        if items[i] > 0 {
            result.push(items[i] * 2);
        }
    }
    result
}
EOF

cat > "$NEGATIVE_DIR/n04_mutable_vec.rs" << 'EOF'
fn mutate_in_place(items: &mut Vec<i32>) {
    for i in 0..items.len() {
        items[i] *= 2;
    }
}
EOF

cat > "$NEGATIVE_DIR/n05_nested_loops.rs" << 'EOF'
fn nested_iteration(matrix: Vec<Vec<i32>>) -> i32 {
    let mut sum = 0;
    for row in matrix {
        for val in row {
            sum += val;
        }
    }
    sum
}
EOF

cat > "$NEGATIVE_DIR/n06_counter.rs" << 'EOF'
fn count_positives(items: Vec<i32>) -> usize {
    let mut count = 0;
    for item in items {
        if item > 0 {
            count += 1;
        }
    }
    count
}
EOF

cat > "$NEGATIVE_DIR/n07_accumulator.rs" << 'EOF'
fn accumulate_values(items: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    let mut acc = 0;
    for item in items {
        acc += item;
        result.push(acc);
    }
    result
}
EOF

cat > "$NEGATIVE_DIR/n08_conditional_push.rs" << 'EOF'
fn filter_imperative(items: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    for item in items {
        if item > 0 && item < 100 {
            result.push(item);
        }
    }
    result
}
EOF

cat > "$NEGATIVE_DIR/n09_early_return.rs" << 'EOF'
fn find_first_imperative(items: Vec<i32>) -> Option<i32> {
    for item in items {
        if item > 10 {
            return Some(item);
        }
    }
    None
}
EOF

cat > "$NEGATIVE_DIR/n10_break_loop.rs" << 'EOF'
fn process_until_condition(items: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    for item in items {
        if item < 0 {
            break;
        }
        result.push(item * 2);
    }
    result
}
EOF

cat > "$NEGATIVE_DIR/n11_continue_loop.rs" << 'EOF'
fn skip_zeros(items: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    for item in items {
        if item == 0 {
            continue;
        }
        result.push(item);
    }
    result
}
EOF

cat > "$NEGATIVE_DIR/n12_swap_elements.rs" << 'EOF'
fn bubble_sort(mut items: Vec<i32>) -> Vec<i32> {
    for i in 0..items.len() {
        for j in 0..items.len() - 1 {
            if items[j] > items[j + 1] {
                items.swap(j, j + 1);
            }
        }
    }
    items
}
EOF

cat > "$NEGATIVE_DIR/n13_state_machine.rs" << 'EOF'
fn state_based_process(items: Vec<i32>) -> Vec<i32> {
    let mut state = 0;
    let mut result = Vec::new();
    for item in items {
        state += item;
        if state > 100 {
            result.push(state);
            state = 0;
        }
    }
    result
}
EOF

cat > "$NEGATIVE_DIR/n14_multiple_mutations.rs" << 'EOF'
fn complex_mutation(mut a: Vec<i32>, mut b: Vec<i32>) -> Vec<i32> {
    for item in b.drain(..) {
        a.push(item * 2);
    }
    a.sort();
    a
}
EOF

cat > "$NEGATIVE_DIR/n15_direct_index.rs" << 'EOF'
fn reverse_imperative(mut items: Vec<i32>) -> Vec<i32> {
    let len = items.len();
    for i in 0..len/2 {
        let temp = items[i];
        items[i] = items[len - 1 - i];
        items[len - 1 - i] = temp;
    }
    items
}
EOF

cat > "$NEGATIVE_DIR/n16_getter_setter.rs" << 'EOF'
struct Counter {
    value: i32,
}

impl Counter {
    fn increment(&mut self) {
        self.value += 1;
    }

    fn get(&self) -> i32 {
        self.value
    }
}

fn use_counter() -> i32 {
    let mut c = Counter { value: 0 };
    for _ in 0..10 {
        c.increment();
    }
    c.get()
}
EOF

cat > "$NEGATIVE_DIR/n17_hashmap_insert.rs" << 'EOF'
use std::collections::HashMap;

fn build_map_imperative(items: Vec<(String, i32)>) -> HashMap<String, i32> {
    let mut map = HashMap::new();
    for (k, v) in items {
        map.insert(k, v);
    }
    map
}
EOF

cat > "$NEGATIVE_DIR/n18_hashset_insert.rs" << 'EOF'
use std::collections::HashSet;

fn deduplicate_imperative(items: Vec<i32>) -> Vec<i32> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for item in items {
        if !seen.contains(&item) {
            seen.insert(item);
            result.push(item);
        }
    }
    result
}
EOF

cat > "$NEGATIVE_DIR/n19_string_builder.rs" << 'EOF'
fn build_string_imperative(items: Vec<&str>) -> String {
    let mut result = String::new();
    for item in items {
        result.push_str(item);
        result.push(',');
    }
    result
}
EOF

cat > "$NEGATIVE_DIR/n20_vec_extend.rs" << 'EOF'
fn flatten_imperative(nested: Vec<Vec<i32>>) -> Vec<i32> {
    let mut result = Vec::new();
    for vec in nested {
        for item in vec {
            result.push(item);
        }
    }
    result
}
EOF

cat > "$NEGATIVE_DIR/n21_match_mutation.rs" << 'EOF'
fn categorize_imperative(items: Vec<i32>) -> (Vec<i32>, Vec<i32>, Vec<i32>) {
    let mut neg = Vec::new();
    let mut zero = Vec::new();
    let mut pos = Vec::new();

    for item in items {
        match item {
            x if x < 0 => neg.push(x),
            0 => zero.push(0),
            x => pos.push(x),
        }
    }
    (neg, zero, pos)
}
EOF

cat > "$NEGATIVE_DIR/n22_option_unwrap.rs" << 'EOF'
fn extract_all_imperative(items: Vec<Option<i32>>) -> Vec<i32> {
    let mut result = Vec::new();
    for opt in items {
        if let Some(val) = opt {
            result.push(val);
        }
    }
    result
}
EOF

cat > "$NEGATIVE_DIR/n23_result_unwrap.rs" << 'EOF'
fn parse_all_imperative(items: Vec<String>) -> Vec<i32> {
    let mut result = Vec::new();
    for s in items {
        if let Ok(val) = s.parse::<i32>() {
            result.push(val);
        }
    }
    result
}
EOF

cat > "$NEGATIVE_DIR/n24_simple_return.rs" << 'EOF'
fn add_one(x: i32) -> i32 {
    x + 1
}
EOF

cat > "$NEGATIVE_DIR/n25_basic_arithmetic.rs" << 'EOF'
fn calculate(a: i32, b: i32) -> i32 {
    a * 2 + b / 3
}
EOF

cat > "$NEGATIVE_DIR/n26_if_else.rs" << 'EOF'
fn classify(x: i32) -> &'static str {
    if x < 0 {
        "negative"
    } else if x == 0 {
        "zero"
    } else {
        "positive"
    }
}
EOF

cat > "$NEGATIVE_DIR/n27_match_simple.rs" << 'EOF'
fn describe(x: i32) -> String {
    match x {
        0 => "zero".to_string(),
        1 => "one".to_string(),
        _ => "other".to_string(),
    }
}
EOF

cat > "$NEGATIVE_DIR/n28_struct_access.rs" << 'EOF'
struct Point { x: i32, y: i32 }

fn distance_squared(p: Point) -> i32 {
    p.x * p.x + p.y * p.y
}
EOF

cat > "$NEGATIVE_DIR/n29_tuple_access.rs" << 'EOF'
fn first_element(tuple: (i32, i32, i32)) -> i32 {
    tuple.0
}
EOF

cat > "$NEGATIVE_DIR/n30_array_index.rs" << 'EOF'
fn get_middle(arr: [i32; 5]) -> i32 {
    arr[2]
}
EOF

cat > "$NEGATIVE_DIR/n31_slice_len.rs" << 'EOF'
fn is_empty(slice: &[i32]) -> bool {
    slice.len() == 0
}
EOF

cat > "$NEGATIVE_DIR/n32_string_len.rs" << 'EOF'
fn string_length(s: &str) -> usize {
    s.len()
}
EOF

cat > "$NEGATIVE_DIR/n33_vec_push_single.rs" << 'EOF'
fn add_item(mut vec: Vec<i32>, item: i32) -> Vec<i32> {
    vec.push(item);
    vec
}
EOF

cat > "$NEGATIVE_DIR/n34_option_is_some.rs" << 'EOF'
fn has_value(opt: Option<i32>) -> bool {
    opt.is_some()
}
EOF

cat > "$NEGATIVE_DIR/n35_result_is_ok.rs" << 'EOF'
fn is_success(res: Result<i32, String>) -> bool {
    res.is_ok()
}
EOF

cat > "$NEGATIVE_DIR/n36_clone_simple.rs" << 'EOF'
fn duplicate(x: i32) -> (i32, i32) {
    (x, x)
}
EOF

cat > "$NEGATIVE_DIR/n37_format_string.rs" << 'EOF'
fn create_message(name: &str) -> String {
    format!("Hello, {}", name)
}
EOF

cat > "$NEGATIVE_DIR/n38_print.rs" << 'EOF'
fn log_message(msg: &str) {
    println!("{}", msg);
}
EOF

cat > "$NEGATIVE_DIR/n39_read_file.rs" << 'EOF'
use std::fs;

fn read_content(path: &str) -> Result<String, std::io::Error> {
    fs::read_to_string(path)
}
EOF

cat > "$NEGATIVE_DIR/n40_write_file.rs" << 'EOF'
use std::fs;

fn save_content(path: &str, content: &str) -> Result<(), std::io::Error> {
    fs::write(path, content)
}
EOF

cat > "$NEGATIVE_DIR/n41_thread_spawn.rs" << 'EOF'
use std::thread;

fn spawn_worker() {
    thread::spawn(|| {
        println!("Working");
    });
}
EOF

cat > "$NEGATIVE_DIR/n42_mutex_lock.rs" << 'EOF'
use std::sync::Mutex;

fn increment_shared(counter: &Mutex<i32>) {
    let mut value = counter.lock().unwrap();
    *value += 1;
}
EOF

cat > "$NEGATIVE_DIR/n43_arc_clone.rs" << 'EOF'
use std::sync::Arc;

fn share_data(data: Arc<i32>) -> Arc<i32> {
    Arc::clone(&data)
}
EOF

cat > "$NEGATIVE_DIR/n44_rc_clone.rs" << 'EOF'
use std::rc::Rc;

fn duplicate_rc(data: Rc<i32>) -> Rc<i32> {
    Rc::clone(&data)
}
EOF

cat > "$NEGATIVE_DIR/n45_box_deref.rs" << 'EOF'
fn unbox(b: Box<i32>) -> i32 {
    *b
}
EOF

echo "Generated negative examples"

# Edge cases

cat > "$EDGE_DIR/e01_empty_iterator.rs" << 'EOF'
fn process_empty() -> Vec<i32> {
    vec![].into_iter()
        .filter(|&x: &i32| x > 0)
        .collect()
}
EOF

cat > "$EDGE_DIR/e02_single_element.rs" << 'EOF'
fn single_map(x: i32) -> Vec<i32> {
    vec![x].into_iter()
        .map(|n| n * 2)
        .collect()
}
EOF

cat > "$EDGE_DIR/e03_single_method.rs" << 'EOF'
fn just_collect(items: Vec<i32>) -> Vec<i32> {
    items.into_iter().collect()
}
EOF

cat > "$EDGE_DIR/e04_two_methods.rs" << 'EOF'
fn minimal_chain(items: Vec<i32>) -> Vec<i32> {
    items.into_iter()
        .map(|x| x * 2)
        .collect()
}
EOF

cat > "$EDGE_DIR/e05_nested_closure.rs" << 'EOF'
fn nested_map(items: Vec<i32>) -> Vec<i32> {
    items.into_iter()
        .map(|x| {
            let y = x * 2;
            let z = y + 1;
            z
        })
        .collect()
}
EOF

cat > "$EDGE_DIR/e06_complex_closure.rs" << 'EOF'
fn complex_filter(items: Vec<i32>) -> Vec<i32> {
    items.into_iter()
        .filter(|&x| {
            let doubled = x * 2;
            doubled > 0 && doubled < 100
        })
        .collect()
}
EOF

cat > "$EDGE_DIR/e07_mixed_imperative.rs" << 'EOF'
fn mixed_style(items: Vec<i32>) -> Vec<i32> {
    let mut result: Vec<i32> = items.into_iter()
        .filter(|&x| x > 0)
        .collect();
    result.sort();
    result
}
EOF

cat > "$EDGE_DIR/e08_macro_in_chain.rs" << 'EOF'
fn with_macro(items: Vec<i32>) -> Vec<i32> {
    items.into_iter()
        .inspect(|x| println!("Value: {}", x))
        .collect()
}
EOF

cat > "$EDGE_DIR/e09_type_conversion.rs" << 'EOF'
fn convert_types(items: Vec<i32>) -> Vec<String> {
    items.into_iter()
        .map(|x| x.to_string())
        .collect()
}
EOF

cat > "$EDGE_DIR/e10_extremely_long_chain.rs" << 'EOF'
fn very_long_pipeline(items: Vec<i32>) -> i32 {
    items.into_iter()
        .filter(|&x| x > 0)
        .map(|x| x * 2)
        .filter(|&x| x < 100)
        .map(|x| x + 1)
        .filter(|&x| x % 2 == 0)
        .map(|x| x / 2)
        .filter(|&x| x > 5)
        .map(|x| x - 1)
        .sum()
}
EOF

echo "Generated edge case examples"
echo "Test corpus generation complete!"
echo "  Positive: 65 files"
echo "  Negative: 45 files"
echo "  Edge cases: 10 files"

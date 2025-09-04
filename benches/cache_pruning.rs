use criterion::{criterion_group, criterion_main, Criterion};
use debtmap::cache::auto_pruner::{AutoPruner, PruneStrategy};
use debtmap::cache::shared_cache::SharedCache;
use std::hint::black_box;
use tempfile::TempDir;

fn create_cache_with_entries(num_entries: usize) -> (SharedCache, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");

    let cache = SharedCache::new_with_cache_dir(None, cache_dir).unwrap();

    // Populate cache with entries
    for i in 0..num_entries {
        let key = format!("key_{}", i);
        let data = vec![0u8; 1024]; // 1KB per entry
        cache.put(&key, "test_component", &data).unwrap();
    }

    (cache, temp_dir)
}

fn bench_lru_pruning(c: &mut Criterion) {
    c.bench_function("prune_lru_100_entries", |b| {
        let (_cache, _dir) = create_cache_with_entries(100);
        let pruner = AutoPruner {
            max_size_bytes: 50 * 1024, // 50KB limit (will trigger pruning)
            strategy: PruneStrategy::Lru,
            ..Default::default()
        };

        b.iter(|| {
            let cache = SharedCache::with_auto_pruning(None, pruner.clone()).unwrap();
            black_box(cache.trigger_pruning().unwrap())
        });
    });

    c.bench_function("prune_lru_1000_entries", |b| {
        let (_cache, _dir) = create_cache_with_entries(1000);
        let pruner = AutoPruner {
            max_size_bytes: 500 * 1024, // 500KB limit
            strategy: PruneStrategy::Lru,
            ..Default::default()
        };

        b.iter(|| {
            let cache = SharedCache::with_auto_pruning(None, pruner.clone()).unwrap();
            black_box(cache.trigger_pruning().unwrap())
        });
    });

    c.bench_function("prune_lru_10000_entries", |b| {
        let (_cache, _dir) = create_cache_with_entries(10000);
        let pruner = AutoPruner {
            max_size_bytes: 5 * 1024 * 1024, // 5MB limit
            strategy: PruneStrategy::Lru,
            ..Default::default()
        };

        b.iter(|| {
            let cache = SharedCache::with_auto_pruning(None, pruner.clone()).unwrap();
            black_box(cache.trigger_pruning().unwrap())
        });
    });
}

fn bench_lfu_pruning(c: &mut Criterion) {
    c.bench_function("prune_lfu_1000_entries", |b| {
        let (_cache, _dir) = create_cache_with_entries(1000);
        let pruner = AutoPruner {
            max_size_bytes: 500 * 1024,
            strategy: PruneStrategy::Lfu,
            ..Default::default()
        };

        b.iter(|| {
            let cache = SharedCache::with_auto_pruning(None, pruner.clone()).unwrap();
            black_box(cache.trigger_pruning().unwrap())
        });
    });
}

fn bench_fifo_pruning(c: &mut Criterion) {
    c.bench_function("prune_fifo_1000_entries", |b| {
        let (_cache, _dir) = create_cache_with_entries(1000);
        let pruner = AutoPruner {
            max_size_bytes: 500 * 1024,
            strategy: PruneStrategy::Fifo,
            ..Default::default()
        };

        b.iter(|| {
            let cache = SharedCache::with_auto_pruning(None, pruner.clone()).unwrap();
            black_box(cache.trigger_pruning().unwrap())
        });
    });
}

fn bench_age_based_pruning(c: &mut Criterion) {
    c.bench_function("prune_age_based_1000_entries", |b| {
        let (_cache, _dir) = create_cache_with_entries(1000);
        let pruner = AutoPruner {
            max_age_days: 1,
            strategy: PruneStrategy::AgeBasedOnly,
            ..Default::default()
        };

        b.iter(|| {
            let cache = SharedCache::with_auto_pruning(None, pruner.clone()).unwrap();
            black_box(cache.trigger_pruning().unwrap())
        });
    });
}

fn bench_orphan_cleanup(c: &mut Criterion) {
    c.bench_function("clean_orphaned_entries_1000", |b| {
        let (cache, _dir) = create_cache_with_entries(1000);

        b.iter(|| black_box(cache.clean_orphaned_entries().unwrap()));
    });
}

criterion_group!(
    benches,
    bench_lru_pruning,
    bench_lfu_pruning,
    bench_fifo_pruning,
    bench_age_based_pruning,
    bench_orphan_cleanup
);
criterion_main!(benches);

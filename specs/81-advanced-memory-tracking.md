---
number: 81
title: Advanced Memory Tracking for Multi-Pass Analysis
category: optimization
priority: medium
status: draft
dependencies: [80]
created: 2025-09-02
---

# Specification 81: Advanced Memory Tracking for Multi-Pass Analysis

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: [80] Multi-Pass Analysis with Attribution

## Context

The current multi-pass analysis implementation (spec 80) uses a simplified memory tracking approach that provides rough estimates rather than accurate measurements. For production-quality analysis, especially when processing large codebases, accurate memory tracking is essential for:
- Validating the 50% memory overhead constraint
- Identifying memory-intensive operations
- Optimizing memory usage patterns
- Providing accurate performance diagnostics
- Enabling memory-constrained analysis modes

Real memory tracking requires platform-specific implementations and careful measurement of heap allocations, which goes beyond the simplified approach currently in place.

## Objective

Implement sophisticated memory tracking that accurately measures heap allocations, tracks memory usage throughout the multi-pass analysis lifecycle, and provides detailed memory attribution to help developers understand and optimize memory consumption patterns.

## Requirements

### Functional Requirements

- **Accurate Heap Measurement**: Track actual heap allocations using platform-specific APIs
- **Lifecycle Tracking**: Monitor memory usage at each phase of analysis
- **Memory Attribution**: Break down memory usage by component (AST, normalization, attribution)
- **Peak Memory Detection**: Identify and report peak memory usage points
- **Memory Profiling**: Generate detailed memory profiles for optimization
- **Incremental Tracking**: Support incremental memory updates during analysis
- **Platform Support**: Work across Linux, macOS, and Windows platforms

### Non-Functional Requirements

- **Low Overhead**: Memory tracking overhead must not exceed 5% of total analysis time
- **Accuracy**: Memory measurements accurate within 5% of actual usage
- **Thread Safety**: Support concurrent memory tracking in parallel analysis
- **Minimal Dependencies**: Use lightweight platform APIs without heavy external dependencies

## Acceptance Criteria

- [ ] Accurate heap memory measurement using platform-specific APIs implemented
- [ ] Memory tracking integrated throughout multi-pass analysis lifecycle
- [ ] Memory attribution shows breakdown by analysis phase and component
- [ ] Peak memory usage detection and reporting functional
- [ ] Memory profiling generates actionable optimization insights
- [ ] Cross-platform support validated on Linux, macOS, and Windows
- [ ] Memory tracking overhead verified to be under 5%
- [ ] Thread-safe memory tracking for parallel analysis confirmed
- [ ] Memory usage stays within 150% of single-pass analysis (50% overhead limit)
- [ ] Integration tests validate memory tracking accuracy

## Technical Details

### Implementation Approach

**Phase 1: Platform-Specific Memory APIs**
```rust
// New module: src/analysis/memory/tracker.rs
pub trait MemoryTracker: Send + Sync {
    fn get_current_usage(&self) -> MemoryUsage;
    fn get_peak_usage(&self) -> MemoryUsage;
    fn reset_peak(&mut self);
    fn start_phase(&mut self, phase: AnalysisPhase);
    fn end_phase(&mut self, phase: AnalysisPhase) -> PhaseMemoryStats;
}

#[cfg(target_os = "linux")]
pub struct LinuxMemoryTracker {
    proc_status: std::fs::File,
    phase_stack: Vec<PhaseTracking>,
}

#[cfg(target_os = "macos")]
pub struct MacOSMemoryTracker {
    task_info: mach::TaskInfo,
    phase_stack: Vec<PhaseTracking>,
}

#[cfg(target_os = "windows")]
pub struct WindowsMemoryTracker {
    process_handle: winapi::HANDLE,
    phase_stack: Vec<PhaseTracking>,
}
```

**Phase 2: Memory Attribution System**
```rust
// Memory attribution for different components
pub struct MemoryAttribution {
    pub ast_memory: MemoryUsage,
    pub normalization_memory: MemoryUsage,
    pub attribution_memory: MemoryUsage,
    pub diagnostic_memory: MemoryUsage,
    pub cache_memory: MemoryUsage,
    pub working_set: MemoryUsage,
}

impl MemoryAttribution {
    pub fn calculate_from_phases(phases: &[PhaseMemoryStats]) -> Self {
        // Attribute memory to specific components based on phase data
    }
    
    pub fn generate_insights(&self) -> Vec<MemoryInsight> {
        // Identify optimization opportunities
    }
}
```

**Phase 3: Memory Profiling Integration**
```rust
pub struct MemoryProfiler {
    tracker: Box<dyn MemoryTracker>,
    samples: Vec<MemorySample>,
    attribution: MemoryAttribution,
}

impl MemoryProfiler {
    pub fn profile_analysis<F, R>(&mut self, f: F) -> (R, MemoryProfile)
    where
        F: FnOnce() -> R,
    {
        self.start_profiling();
        let result = f();
        let profile = self.finish_profiling();
        (result, profile)
    }
    
    pub fn generate_report(&self) -> MemoryReport {
        MemoryReport {
            peak_usage: self.tracker.get_peak_usage(),
            average_usage: self.calculate_average(),
            attribution: self.attribution.clone(),
            timeline: self.generate_timeline(),
            insights: self.attribution.generate_insights(),
        }
    }
}
```

### Architecture Changes

**New Components:**
```
src/analysis/memory/
├── mod.rs                    # Memory tracking coordination
├── tracker.rs                # Platform-specific memory trackers
├── attribution.rs            # Memory attribution system
├── profiler.rs              # Memory profiling engine
├── insights.rs              # Memory usage insights generation
└── platform/
    ├── mod.rs               # Platform detection and selection
    ├── linux.rs             # Linux-specific implementation
    ├── macos.rs             # macOS-specific implementation
    └── windows.rs           # Windows-specific implementation
```

**Modified Components:**
- `src/analysis/multi_pass.rs`: Integrate memory profiler
- `src/analysis/diagnostics/reporter.rs`: Include memory profiling in reports
- `src/builders/unified_analysis.rs`: Add memory tracking options
- `src/cli.rs`: Add memory profiling CLI flags

### Data Structures

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    pub heap_bytes: u64,
    pub stack_bytes: u64,
    pub total_bytes: u64,
    pub resident_set_size: u64,
    pub virtual_memory_size: u64,
}

#[derive(Debug, Clone)]
pub struct PhaseMemoryStats {
    pub phase: AnalysisPhase,
    pub start_memory: MemoryUsage,
    pub end_memory: MemoryUsage,
    pub peak_memory: MemoryUsage,
    pub allocated_bytes: u64,
    pub freed_bytes: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone)]
pub enum AnalysisPhase {
    Parsing,
    RawAnalysis,
    Normalization,
    NormalizedAnalysis,
    Attribution,
    InsightGeneration,
    ReportGeneration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryProfile {
    pub total_allocated: u64,
    pub peak_usage: MemoryUsage,
    pub phases: Vec<PhaseMemoryStats>,
    pub attribution: MemoryAttribution,
    pub overhead_percentage: f32,
}
```

### Platform-Specific Implementation Details

**Linux Implementation:**
- Parse `/proc/self/status` for VmRSS and VmSize
- Use `mallinfo2()` for heap statistics
- Track page faults for memory pressure detection

**macOS Implementation:**
- Use `task_info()` with TASK_VM_INFO for memory statistics
- Query `malloc_zone_statistics()` for heap details
- Use `rusage` for additional metrics

**Windows Implementation:**
- Use `GetProcessMemoryInfo()` for working set and page file usage
- Query `HeapWalk()` for detailed heap statistics
- Use performance counters for comprehensive metrics

## Dependencies

- **Prerequisites**:
  - [80] Multi-Pass Analysis with Attribution (required for integration)
- **Platform Dependencies**:
  - Linux: libc for proc filesystem access
  - macOS: mach crate for task_info access
  - Windows: winapi crate for process memory APIs
- **Affected Components**:
  - Multi-pass analyzer
  - Diagnostic reporter
  - Performance metrics

## Testing Strategy

### Unit Tests
- **Platform Detection**: Verify correct platform-specific implementation selection
- **Memory Measurement**: Test accuracy of memory measurements against known allocations
- **Attribution Logic**: Validate memory attribution calculations
- **Phase Tracking**: Test phase-based memory tracking
- **Thread Safety**: Verify concurrent memory tracking works correctly

### Integration Tests
- **End-to-End Tracking**: Complete memory tracking through full analysis
- **Cross-Platform**: Test on Linux, macOS, and Windows CI environments
- **Large File Handling**: Verify memory tracking with large source files
- **Memory Limits**: Test behavior when approaching memory constraints
- **Profile Generation**: Validate memory profile report generation

### Performance Tests
- **Tracking Overhead**: Measure performance impact of memory tracking
- **Memory Accuracy**: Compare tracked vs actual memory usage
- **Scalability**: Test with varying codebase sizes
- **Parallel Analysis**: Verify memory tracking in parallel execution

## Documentation Requirements

### Code Documentation
- **Platform APIs**: Document platform-specific memory measurement approaches
- **Attribution Algorithm**: Explain how memory is attributed to components
- **Profiling Strategy**: Document memory profiling methodology
- **Optimization Guide**: Provide memory optimization recommendations

### User Documentation
- **Memory Profiling Guide**: How to use memory profiling features
- **Platform Support**: Document platform-specific limitations
- **Optimization Tips**: Best practices for memory-efficient analysis
- **Troubleshooting**: Common memory-related issues and solutions

### Architecture Updates
- **ARCHITECTURE.md**: Document memory tracking subsystem
- **Performance Guide**: Include memory profiling in performance documentation
- **Platform Support Matrix**: Document memory tracking capabilities per platform

## Implementation Notes

### Memory Measurement Challenges

- **Allocation Tracking**: Not all allocators provide detailed statistics
- **Shared Memory**: Distinguishing between shared and unique memory
- **Platform Variations**: Different platforms report memory differently
- **Timing Accuracy**: Sampling frequency affects measurement precision

### Optimization Opportunities

- **Lazy Allocation**: Defer memory allocation until needed
- **Memory Pooling**: Reuse memory across analysis phases
- **Streaming Processing**: Process data in chunks to reduce peak memory
- **Cache Management**: Implement intelligent cache eviction policies

### Future Enhancements

- **Memory Prediction**: Estimate memory requirements before analysis
- **Adaptive Limits**: Dynamically adjust analysis based on available memory
- **Memory Visualization**: Generate memory usage flame graphs
- **Leak Detection**: Identify potential memory leaks during analysis
---
number: 11
title: Add Context-Aware Risk Analysis
category: optimization
priority: high
status: draft
dependencies: [07, 08]
created: 2025-01-10
---

# Specification 11: Add Context-Aware Risk Analysis

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [07 - Recalibrate Risk Formula, 08 - Fix Testing Prioritization]

## Context

Current risk analysis treats all code paths equally, missing critical context about how code is actually used and its impact on the system. This leads to misaligned priorities where rarely-used complex code receives the same risk score as critical path functions.

Key missing context includes:
- **Critical Path Analysis**: Which functions are on user-facing critical paths
- **Dependency Chain Risk**: How risk propagates through the dependency graph
- **Historical Bug Correlation**: Which areas have historically had issues
- **Change Frequency**: How often code changes (churn rate)
- **Business Impact**: Which features are mission-critical

Without this context, risk scores fail to reflect real-world impact, leading to suboptimal testing and refactoring priorities.

## Objective

Implement context-aware risk analysis that considers code usage patterns, dependency relationships, historical data, and business impact to provide risk scores that accurately reflect the real-world consequences of failures.

## Requirements

### Functional Requirements

1. **Critical Path Identification**
   - Trace execution paths from entry points
   - Identify user-facing code paths
   - Mark API endpoints and handlers
   - Weight by usage frequency (if available)
   - Distinguish hot paths from cold paths

2. **Dependency Risk Propagation**
   - Calculate transitive risk through dependency graph
   - Identify high-risk dependency chains
   - Detect single points of failure
   - Measure blast radius of changes
   - Account for interface stability

3. **Historical Analysis Integration**
   - Integrate git history for change frequency
   - Correlate with bug fix commits
   - Track refactoring patterns
   - Identify unstable modules
   - Calculate code age and maturity

4. **Business Context Mapping**
   - Allow marking of critical features
   - Support priority annotations
   - Revenue impact assessment
   - User impact scoring
   - SLA-critical path identification

5. **Runtime Context (Optional)**
   - Import production metrics if available
   - Use profiling data for hot path detection
   - Integrate error rates and crash reports
   - Consider performance metrics
   - Support APM tool integration

### Non-Functional Requirements

1. **Performance**: Context analysis <2s for 10K functions
2. **Incrementality**: Support incremental context updates
3. **Configurability**: Pluggable context providers
4. **Privacy**: No sensitive data in analysis
5. **Portability**: Work without runtime data

## Acceptance Criteria

- [ ] Critical paths identified from all entry points
- [ ] Dependency risk properly propagates
- [ ] Git history influences risk scores
- [ ] Business context can be configured
- [ ] Risk scores show clear differentiation
- [ ] Context breakdown available on request
- [ ] Performance meets <2s requirement
- [ ] Incremental updates work correctly
- [ ] Configuration supports all context types
- [ ] Privacy requirements met
- [ ] Unit tests cover all context types
- [ ] Integration tests validate accuracy

## Technical Details

### Implementation Approach

1. **Context Provider Architecture**
```rust
pub trait ContextProvider {
    fn name(&self) -> &str;
    fn gather(&self, target: &AnalysisTarget) -> Result<Context>;
    fn weight(&self) -> f64;
}

pub struct ContextAggregator {
    providers: Vec<Box<dyn ContextProvider>>,
}

impl ContextAggregator {
    pub fn analyze(&self, codebase: &Codebase) -> ContextMap {
        let mut context_map = ContextMap::new();
        
        for provider in &self.providers {
            match provider.gather(&codebase) {
                Ok(context) => context_map.add(provider.name(), context),
                Err(e) => log::warn!("Provider {} failed: {}", provider.name(), e),
            }
        }
        
        context_map
    }
}
```

2. **Critical Path Analyzer**
```rust
pub struct CriticalPathAnalyzer {
    entry_points: Vec<EntryPoint>,
    call_graph: CallGraph,
}

impl CriticalPathAnalyzer {
    pub fn analyze(&self) -> CriticalPaths {
        let mut paths = CriticalPaths::new();
        
        for entry in &self.entry_points {
            let traversal = self.trace_from_entry(entry);
            paths.add_path(CriticalPath {
                entry: entry.clone(),
                functions: traversal.functions,
                weight: self.calculate_path_weight(&traversal),
                user_facing: entry.is_user_facing(),
            });
        }
        
        paths
    }
    
    fn trace_from_entry(&self, entry: &EntryPoint) -> Traversal {
        let mut visited = HashSet::new();
        let mut traversal = Traversal::new();
        
        self.dfs_trace(entry.function_id, &mut visited, &mut traversal);
        traversal
    }
}
```

3. **Dependency Risk Calculator**
```rust
pub struct DependencyRiskCalculator {
    dependency_graph: DependencyGraph,
    risk_scores: HashMap<ModuleId, f64>,
}

impl DependencyRiskCalculator {
    pub fn propagate_risk(&mut self) {
        // Iteratively propagate risk through dependency graph
        let mut changed = true;
        let mut iterations = 0;
        
        while changed && iterations < 10 {
            changed = false;
            
            for node in self.dependency_graph.nodes() {
                let old_risk = self.risk_scores[&node.id];
                let propagated_risk = self.calculate_propagated_risk(node);
                
                if (propagated_risk - old_risk).abs() > 0.01 {
                    self.risk_scores.insert(node.id, propagated_risk);
                    changed = true;
                }
            }
            
            iterations += 1;
        }
    }
    
    fn calculate_propagated_risk(&self, node: &Node) -> f64 {
        let base_risk = node.intrinsic_risk;
        let mut dependency_risk = 0.0;
        
        for dep in node.dependencies() {
            let dep_risk = self.risk_scores[&dep.id];
            let coupling_strength = dep.coupling_score();
            dependency_risk += dep_risk * coupling_strength * 0.3;
        }
        
        (base_risk + dependency_risk).min(10.0)
    }
}
```

4. **Historical Context Provider**
```rust
pub struct GitHistoryProvider {
    repo: Repository,
    cache: HistoryCache,
}

impl GitHistoryProvider {
    pub fn analyze_file(&self, path: &Path) -> FileHistory {
        if let Some(cached) = self.cache.get(path) {
            return cached;
        }
        
        let history = FileHistory {
            change_frequency: self.calculate_churn_rate(path),
            bug_fix_count: self.count_bug_fixes(path),
            last_modified: self.get_last_modified(path),
            author_count: self.count_unique_authors(path),
            stability_score: self.calculate_stability(path),
        };
        
        self.cache.set(path, history.clone());
        history
    }
    
    fn calculate_churn_rate(&self, path: &Path) -> f64 {
        let commits = self.repo.log().path(path).count();
        let age_days = self.get_file_age_days(path);
        
        if age_days > 0 {
            (commits as f64) / (age_days as f64) * 30.0  // Monthly rate
        } else {
            0.0
        }
    }
}
```

### Architecture Changes

1. **Risk Module Enhancement**
   - Add context provider system
   - Implement context aggregation
   - Create weighted risk calculation
   - Add context caching layer

2. **Integration Points**
   - Git integration for history
   - Call graph construction
   - Dependency graph enhancement
   - Optional APM tool connectors

### Data Structures

```rust
pub struct ContextualRisk {
    pub base_risk: f64,
    pub contextual_risk: f64,
    pub contexts: Vec<RiskContext>,
    pub explanation: String,
}

pub struct RiskContext {
    pub provider: String,
    pub weight: f64,
    pub contribution: f64,
    pub details: ContextDetails,
}

pub enum ContextDetails {
    CriticalPath {
        entry_points: Vec<String>,
        path_weight: f64,
    },
    DependencyChain {
        depth: usize,
        propagated_risk: f64,
        dependents: Vec<String>,
    },
    Historical {
        change_frequency: f64,
        bug_density: f64,
        age_days: u32,
    },
    Business {
        priority: Priority,
        impact: Impact,
        annotations: Vec<String>,
    },
}

pub struct CriticalPath {
    pub entry: EntryPoint,
    pub functions: Vec<FunctionId>,
    pub weight: f64,
    pub user_facing: bool,
}

pub struct EntryPoint {
    pub function_id: FunctionId,
    pub entry_type: EntryType,
    pub is_user_facing: bool,
}

pub enum EntryType {
    Main,
    ApiEndpoint,
    EventHandler,
    CliCommand,
    TestEntry,
}
```

### APIs and Interfaces

```rust
pub trait ContextProvider {
    fn gather(&self, target: &AnalysisTarget) -> Result<Context>;
    fn weight(&self) -> f64;
    fn explain(&self, context: &Context) -> String;
}

pub struct ContextConfig {
    pub providers: Vec<ProviderConfig>,
    pub weights: ContextWeights,
    pub cache_ttl: Duration,
}

pub struct ProviderConfig {
    pub name: String,
    pub enabled: bool,
    pub options: HashMap<String, Value>,
}
```

## Dependencies

- **Prerequisites**:
  - Spec 07 (Recalibrate Risk Formula) for base risk calculation
  - Spec 08 (Fix Testing Prioritization) for priority integration
- **Affected Components**:
  - `src/risk/mod.rs` - Add context system
  - `src/risk/context/` - New module for providers
  - `src/core/graph.rs` - Enhance dependency graph
  - `src/analyzers/` - Add call graph construction
- **External Dependencies**:
  - `git2` crate for Git history
  - Optional: APM client libraries

## Testing Strategy

- **Unit Tests**:
  - Test each context provider independently
  - Validate risk propagation algorithm
  - Test critical path detection
  - Verify context aggregation

- **Integration Tests**:
  - Test with real Git repositories
  - Validate critical path accuracy
  - Test dependency risk propagation
  - Verify context caching

- **Performance Tests**:
  - Benchmark large repository analysis
  - Test incremental update performance
  - Measure memory usage for graphs
  - Validate cache effectiveness

- **User Acceptance**:
  - Risk scores align with developer intuition
  - Critical paths match expected flows
  - Historical context is accurate
  - Business priorities properly reflected

## Documentation Requirements

- **Code Documentation**:
  - Document each context provider
  - Explain risk propagation algorithm
  - Describe critical path detection
  - Provide weight tuning guide

- **User Documentation**:
  - Add context configuration guide
  - Document provider options
  - Provide interpretation examples
  - Include troubleshooting guide

- **Architecture Updates**:
  - Update ARCHITECTURE.md with context system
  - Document provider architecture
  - Add data flow diagrams

## Implementation Notes

1. **Phased Implementation**
   - Phase 1: Critical path analysis
   - Phase 2: Dependency risk propagation
   - Phase 3: Git history integration
   - Phase 4: Business context support
   - Phase 5: Runtime metrics (optional)

2. **Default Providers**
   - Always: Critical path, dependency
   - If available: Git history
   - Optional: Business context, runtime

3. **Performance Optimization**
   - Cache all context calculations
   - Incremental graph updates
   - Parallel provider execution
   - Lazy context evaluation

## Migration and Compatibility

- **Breaking Changes**:
  - Risk scores will change significantly
  - New configuration format
  - Additional dependencies required

- **Migration Path**:
  1. Run without context (baseline)
  2. Enable providers incrementally
  3. Tune weights based on feedback
  4. Full context-aware by default

- **Fallback Behavior**:
  - Work without Git repository
  - Skip unavailable providers
  - Use base risk if context fails
  - Provide clear error messages
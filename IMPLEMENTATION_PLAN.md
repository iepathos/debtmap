## Stage 1: Restore Analyzer Signal
**Goal**: Ensure Python analysis preserves extracted dependencies, classes, and debt signals instead of returning empty placeholders.
**Success Criteria**: Python `FileMetrics` includes extracted dependencies/classes/debt items; extraction failures surface as explicit analysis debt instead of silent empty output.
**Tests**: Adapter unit tests for Python conversion; analyzer tests covering preserved metadata and failure signaling.
**Status**: Complete

## Stage 2: Remove Dead Toggles
**Goal**: Make Rust enhanced detection and TypeScript functional analysis configuration flags materially affect analysis.
**Success Criteria**: Rust enhanced detection can be disabled through the analyzer path; TypeScript functional analysis populates additional metrics only when enabled.
**Tests**: Unit tests for Rust analyzer flag behavior and TypeScript functional-analysis behavior.
**Status**: Complete

## Stage 3: Replace Placeholder Module Analysis
**Goal**: Replace JS/TS and Python module-structure text scanning with parser-backed analysis.
**Success Criteria**: Module structure reports real function/class names and separates class methods from module-level functions for Python and JS/TS.
**Tests**: Module-structure tests for Python and JS/TS files with classes, methods, and exported/top-level functions.
**Status**: Complete

## Stage 4: Verify End-to-End
**Goal**: Run targeted tests for the touched analyzer paths and update plan status.
**Success Criteria**: Relevant cargo tests pass and the plan reflects completed stages.
**Tests**: Targeted `cargo test` filters for Python analyzer/adapters, TypeScript analyzer/module structure, and Rust analyzer behavior.
**Status**: Complete

## Stage 5: Complete JS/TS Function Classification
**Goal**: Remove placeholder behavior from TypeScript function extraction where supported kinds are defined but never emitted.
**Success Criteria**: Class constructors are classified as `FunctionKind::Constructor` and covered by focused visitor tests.
**Tests**: TypeScript function-analysis tests for constructor extraction and kind classification.
**Status**: Complete

## Stage 6: Restore JS/TS Structural Signals
**Goal**: Stop discarding parsed JS/TS dependency and facade information in module-structure analysis.
**Success Criteria**: JS/TS module structure includes a populated dependency graph for imports/re-exports and computes facade metadata from barrel-style exports.
**Tests**: Module-structure tests covering imports, re-exports, dependency edges, and facade detection.
**Status**: Complete

## Stage 7: Honor TS Threshold Configuration
**Goal**: Make TypeScript AST analysis reflect configured complexity thresholds instead of ignoring them.
**Success Criteria**: Converted `FunctionMetrics` include threshold-derived analysis metadata that changes with strict vs lenient threshold configurations.
**Tests**: Visitor tests comparing threshold-sensitive analysis output across different presets.
**Status**: Complete

## Stage 8: Honor Python Analyzer Configuration
**Goal**: Remove dead Python analyzer knobs by making thresholds and functional-analysis settings affect output.
**Success Criteria**: Python analysis emits threshold-derived metadata and complexity debt items, and functional-analysis settings change detected patterns.
**Tests**: Python analyzer tests for threshold-sensitive debt output and functional-pattern detection.
**Status**: Complete

## Stage 9: Restore Python Structural Signals
**Goal**: Stop returning placeholder dependency and facade data from Python module-structure analysis.
**Success Criteria**: Python module structure includes dependency graph edges from extracted imports and computes package-facade metadata for `__init__.py`-style modules.
**Tests**: Python module-structure tests covering imports, dependency edges, and facade detection.
**Status**: Complete

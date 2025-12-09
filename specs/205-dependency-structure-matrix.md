---
number: 205
title: Dependency Structure Matrix (DSM) View
category: foundation
priority: low
status: draft
dependencies: [201, 203]
created: 2025-12-09
---

# Specification 205: Dependency Structure Matrix (DSM) View

**Category**: foundation
**Priority**: low
**Status**: draft
**Dependencies**: [201 - File-Level Dependency Metrics, 203 - TUI Coupling Visualization]

## Context

Directed graphs become unreadable at scale. A **Dependency Structure Matrix (DSM)** provides an alternative visualization that:

1. **Scales linearly**: 100x100 matrix is still usable; 100-node graph is chaos
2. **Reveals cycles immediately**: Cells above the diagonal indicate backward dependencies
3. **Shows layered architecture**: Triangular matrices indicate clean layering
4. **Enables pattern recognition**: Clusters, fan-out, fan-in visible at a glance

**DSM Concept**:
```
         A  B  C  D  E
      A  .  .  .  .  .     . = no dependency
      B  X  .  .  .  .     X = B depends on A (below diagonal = good)
      C  X  X  .  .  .     O = cycle (above diagonal = problem!)
      D  .  X  X  .  O
      E  .  .  X  X  .
```

Reading: Row depends on Column. Lower-left triangle = dependencies flow "down" (healthy).
Upper-right = dependencies flow "up" (cycles, problematic).

**Research backing**: DSM is used by NDepend, IntelliJ IDEA, Lattix, and academic research for software architecture analysis.

## Objective

Implement a Dependency Structure Matrix view that:
1. Shows module-to-module dependencies in matrix form
2. Highlights cycles in the upper triangle
3. Supports TUI interactive exploration
4. Provides text/markdown export for documentation

## Requirements

### Functional Requirements

1. **Matrix Generation**
   - Build square matrix from module dependencies
   - Row = dependent module, Column = dependency
   - Cell value: dependency count or boolean
   - Order modules to minimize upper-triangle cells (optional optimization)

2. **Cycle Detection and Highlighting**
   - Cells in upper-right triangle indicate cycles
   - Color these cells red/warning
   - Provide cycle path information on hover/select

3. **TUI Matrix View**
   - New view mode in TUI: `m` key to toggle DSM view
   - Scrollable for large matrices
   - Cell selection with arrow keys
   - Show dependency details on selection

4. **Text/Markdown Export**
   - `--format dsm` for text matrix output
   - ASCII art matrix with symbols
   - Legend explaining symbols

5. **Filtering and Grouping**
   - Filter to top-level modules only (reduce noise)
   - Group by directory/crate
   - Option to show file-level or module-level

6. **Matrix Metrics**
   - **Propagation cost**: How far changes propagate
   - **Cycle count**: Number of cells above diagonal
   - **Layering score**: How triangular is the matrix?

### Non-Functional Requirements

1. **Scalability**: Handle 50+ modules without performance issues
2. **Readability**: Matrix should be interpretable by developers
3. **Interactivity**: TUI navigation should be responsive

## Acceptance Criteria

- [ ] DSM matrix generates correctly from dependency data
- [ ] Upper-triangle cells highlighted as potential cycles
- [ ] TUI displays matrix with keyboard navigation
- [ ] Text export produces readable ASCII matrix
- [ ] Cell selection shows dependency details
- [ ] Matrix ordering minimizes above-diagonal cells
- [ ] Cycle paths can be displayed for problematic cells
- [ ] Works for codebases with 50+ modules

## Technical Details

### Implementation Approach

**Matrix Data Structure**:

```rust
// src/analysis/dsm.rs

pub struct DependencyMatrix {
    /// Module names in row/column order
    pub modules: Vec<String>,
    /// Adjacency matrix: matrix[row][col] = row depends on col
    pub matrix: Vec<Vec<DsmCell>>,
    /// Cycle information
    pub cycles: Vec<CycleInfo>,
}

#[derive(Clone, Default)]
pub struct DsmCell {
    pub has_dependency: bool,
    pub dependency_count: usize,
    pub is_cycle: bool,  // True if in upper triangle and has dependency
}

pub struct CycleInfo {
    pub modules: Vec<String>,
    pub severity: CycleSeverity,
}

impl DependencyMatrix {
    /// Build matrix from file dependencies
    pub fn from_file_dependencies(files: &[FileDebtItemOutput]) -> Self {
        let modules = Self::extract_modules(files);
        let mut matrix = vec![vec![DsmCell::default(); modules.len()]; modules.len()];

        // Fill matrix
        for file in files {
            let row_idx = Self::module_index(&modules, &file.location.file);
            if let Some(deps) = &file.dependencies {
                for dep in &deps.top_dependencies {
                    if let Some(col_idx) = Self::module_index(&modules, dep) {
                        matrix[row_idx][col_idx].has_dependency = true;
                        matrix[row_idx][col_idx].dependency_count += 1;

                        // Mark cycles (above diagonal)
                        if row_idx < col_idx {
                            matrix[row_idx][col_idx].is_cycle = true;
                        }
                    }
                }
            }
        }

        let cycles = Self::detect_cycles(&matrix, &modules);

        DependencyMatrix { modules, matrix, cycles }
    }

    /// Reorder modules to minimize upper-triangle dependencies
    /// Uses a simple topological sort with feedback arc set minimization
    pub fn optimize_ordering(&mut self) {
        // Implementation: Tarjan's algorithm for SCCs, then topological sort
    }

    /// Calculate matrix metrics
    pub fn metrics(&self) -> DsmMetrics {
        let total_cells = self.modules.len() * self.modules.len();
        let dependency_count = self.count_dependencies();
        let cycle_count = self.count_cycles();

        DsmMetrics {
            module_count: self.modules.len(),
            dependency_count,
            cycle_count,
            density: dependency_count as f64 / total_cells as f64,
            layering_score: self.calculate_layering_score(),
        }
    }
}

pub struct DsmMetrics {
    pub module_count: usize,
    pub dependency_count: usize,
    pub cycle_count: usize,
    pub density: f64,
    pub layering_score: f64,  // 0.0 = all cycles, 1.0 = perfect layers
}
```

**TUI Matrix View**:

```rust
// src/tui/views/dsm_view.rs

pub struct DsmView {
    matrix: DependencyMatrix,
    scroll_x: usize,
    scroll_y: usize,
    selected: Option<(usize, usize)>,
}

impl DsmView {
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Calculate visible cells based on area size
        let cell_width = 3;  // "X" or "." or " "
        let header_width = 20;  // Module name column
        let visible_cols = (area.width as usize - header_width) / cell_width;
        let visible_rows = area.height as usize - 2;  // Header row + border

        // Render column headers (rotated module names)
        self.render_column_headers(frame, area, visible_cols);

        // Render matrix cells
        for row in 0..visible_rows {
            let matrix_row = row + self.scroll_y;
            if matrix_row >= self.matrix.modules.len() {
                break;
            }

            // Row label (module name)
            let row_label = &self.matrix.modules[matrix_row];

            for col in 0..visible_cols {
                let matrix_col = col + self.scroll_x;
                if matrix_col >= self.matrix.modules.len() {
                    break;
                }

                let cell = &self.matrix.matrix[matrix_row][matrix_col];
                let symbol = self.cell_symbol(cell, matrix_row, matrix_col);
                let style = self.cell_style(cell, matrix_row, matrix_col, theme);

                // Render cell
            }
        }
    }

    fn cell_symbol(&self, cell: &DsmCell, row: usize, col: usize) -> &str {
        if row == col {
            "■"  // Diagonal (self)
        } else if cell.is_cycle {
            "●"  // Cycle (problem)
        } else if cell.has_dependency {
            "×"  // Normal dependency
        } else {
            "·"  // No dependency
        }
    }

    fn cell_style(&self, cell: &DsmCell, row: usize, col: usize, theme: &Theme) -> Style {
        if row == col {
            Style::default().fg(Color::DarkGray)
        } else if cell.is_cycle {
            Style::default().fg(Color::Red).bold()
        } else if cell.has_dependency {
            if row > col {
                Style::default().fg(Color::Green)  // Lower triangle = good
            } else {
                Style::default().fg(Color::Yellow)  // Upper triangle = warning
            }
        } else {
            Style::default().fg(Color::DarkGray)
        }
    }
}
```

**Text Export**:

```rust
// src/io/writers/dsm.rs

pub fn write_dsm_text(matrix: &DependencyMatrix, out: &mut impl Write) -> io::Result<()> {
    let metrics = matrix.metrics();

    writeln!(out, "DEPENDENCY STRUCTURE MATRIX")?;
    writeln!(out, "===========================")?;
    writeln!(out)?;
    writeln!(out, "Modules: {}", metrics.module_count)?;
    writeln!(out, "Dependencies: {}", metrics.dependency_count)?;
    writeln!(out, "Cycles: {} (cells above diagonal)", metrics.cycle_count)?;
    writeln!(out, "Layering Score: {:.0}%", metrics.layering_score * 100.0)?;
    writeln!(out)?;

    // Print matrix
    // Column headers (abbreviated)
    write!(out, "{:>20} ", "")?;
    for (i, _) in matrix.modules.iter().enumerate() {
        write!(out, "{:>2} ", i)?;
    }
    writeln!(out)?;

    // Rows
    for (row_idx, module) in matrix.modules.iter().enumerate() {
        let short_name: String = module.chars().take(18).collect();
        write!(out, "{:>20} ", short_name)?;

        for col_idx in 0..matrix.modules.len() {
            let cell = &matrix.matrix[row_idx][col_idx];
            let symbol = if row_idx == col_idx {
                "■"
            } else if cell.is_cycle {
                "●"
            } else if cell.has_dependency {
                "×"
            } else {
                "·"
            };
            write!(out, "{:>2} ", symbol)?;
        }
        writeln!(out)?;
    }

    writeln!(out)?;
    writeln!(out, "Legend: × = dependency, ● = cycle, ■ = self, · = none")?;
    writeln!(out, "        Lower triangle (good) | Upper triangle (cycles)")?;

    Ok(())
}
```

### Output Example

```
DEPENDENCY STRUCTURE MATRIX
===========================

Modules: 8
Dependencies: 15
Cycles: 2 (cells above diagonal)
Layering Score: 87%

                      0  1  2  3  4  5  6  7
              main.rs ■  ·  ·  ·  ·  ·  ·  ·
               lib.rs ×  ■  ·  ·  ·  ·  ·  ·
          commands.rs ×  ×  ■  ·  ●  ·  ·  ·  <- cycle!
          analysis.rs ×  ×  ×  ■  ·  ·  ·  ·
           priority.rs ·  ×  ×  ×  ■  ·  ·  ·
            output.rs ·  ·  ×  ·  ●  ■  ·  ·  <- cycle!
               tui.rs ·  ·  ·  ×  ×  ×  ■  ·
             utils.rs ·  ·  ·  ·  ·  ·  ·  ■

Legend: × = dependency, ● = cycle, ■ = self, · = none
        Lower triangle (good) | Upper triangle (cycles)
```

### Affected Components

- `src/analysis/dsm.rs` - New module for matrix logic
- `src/tui/views/dsm_view.rs` - New TUI view
- `src/io/writers/dsm.rs` - Text export
- `src/commands/analyze.rs` - Add `--format dsm`

## Dependencies

- **Prerequisites**: Spec 201 (dependency data)
- **Affected Components**: Analysis, TUI, output
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Matrix construction, cycle detection, ordering
- **Property Tests**: Matrix symmetry properties, cycle detection accuracy
- **Visual Tests**: Manual inspection of TUI rendering
- **Integration Tests**: End-to-end matrix generation from real codebases

## Documentation Requirements

- **User Documentation**: Explain DSM concept and how to interpret
- **Tutorial**: Walkthrough of using DSM to identify architecture issues

## Implementation Notes

1. Start with simple text output, then add TUI
2. Module ordering is optional optimization (can ship without)
3. Consider sparse matrix representation for very large codebases
4. Cache matrix computation (expensive for large graphs)

## Migration and Compatibility

- New feature, no breaking changes
- Optional view mode in TUI
- New output format option

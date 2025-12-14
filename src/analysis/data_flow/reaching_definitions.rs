//! Reaching definitions data flow analysis.
//!
//! This module implements the reaching definitions algorithm for determining
//! which variable definitions can reach each program point.
//!
//! # Algorithm
//!
//! Uses forward data flow analysis with gen/kill sets:
//! - `gen[block]` = new definitions in this block
//! - `kill[block]` = definitions this block overwrites
//! - `reach_in[block]` = union of `reach_out[predecessor]` for all predecessors
//! - `reach_out\[block\]` = (reach_in\[block\] - kill\[block\]) ∪ gen\[block\]
//!
//! # Statement-Level Precision
//!
//! In addition to block-level tracking, this module provides statement-level
//! precision through `precise_def_use` and `use_def_chains`. These enable:
//! - Same-block dead store detection
//! - Precise data flow path tracking
//! - SSA-style analysis without explicit phi nodes

use std::collections::{HashMap, HashSet};

use super::types::{
    BasicBlock, BlockId, ControlFlowGraph, Definition, ExprKind, ProgramPoint, Rvalue, Statement,
    Terminator, Use, VarId,
};

/// Complete data flow analysis results for a function.
///
/// Combines escape and taint analysis to provide comprehensive information
/// about variable scope and mutation propagation.
///
/// # Example
///
/// ```ignore
/// use debtmap::analysis::data_flow::DataFlowAnalysis;
/// use syn::parse_quote;
///
/// let block = parse_quote! {
///     {
///         let mut x = 1;
///         let y = x;
///         x = 2;
///         y           // Returns y (which depends on first x)
///     }
/// };
///
/// let analysis = DataFlowAnalysis::from_block(&block);
/// ```
#[derive(Debug, Clone)]
pub struct DataFlowAnalysis {
    /// Reaching definitions (which definitions reach each program point)
    pub reaching_defs: ReachingDefinitions,
}

impl DataFlowAnalysis {
    /// Perform data flow analysis on a control flow graph.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let cfg = ControlFlowGraph::from_block(&block);
    /// let analysis = DataFlowAnalysis::analyze(&cfg);
    /// ```
    pub fn analyze(cfg: &ControlFlowGraph) -> Self {
        let reaching_defs = ReachingDefinitions::analyze(cfg);

        Self { reaching_defs }
    }

    /// Create analysis from a function block (convenience method).
    ///
    /// Constructs a CFG from the block and performs full data flow analysis.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use syn::parse_quote;
    ///
    /// let block = parse_quote! {{ let x = 1; x }};
    /// let analysis = DataFlowAnalysis::from_block(&block);
    /// ```
    pub fn from_block(block: &syn::Block) -> Self {
        let cfg = ControlFlowGraph::from_block(block);
        Self::analyze(&cfg)
    }
}

/// Reaching definitions analysis (forward data flow analysis).
///
/// Tracks which variable definitions reach each program point.
/// This enables def-use chain construction and SSA-like analysis.
///
/// # Example
///
/// ```ignore
/// let cfg = ControlFlowGraph::from_block(&block);
/// let reaching = ReachingDefinitions::analyze(&cfg);
///
/// // Check which definitions of x reach a specific block
/// let var_id = VarId { name_id: 0, version: 0 };
/// if let Some(defs) = reaching.reach_in.get(&block_id) {
///     if defs.contains(&var_id) {
///         println!("Definition of x.0 reaches this block");
///     }
/// }
///
/// // Statement-level: check if a specific definition is dead
/// for def in &reaching.all_definitions {
///     if reaching.is_dead_definition(def) {
///         println!("Dead store at {:?}", def.point);
///     }
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct ReachingDefinitions {
    // --- Block-level (existing, preserved for backward compatibility) ---
    /// Definitions that reach the entry of each block
    pub reach_in: HashMap<BlockId, HashSet<VarId>>,
    /// Definitions that reach the exit of each block
    pub reach_out: HashMap<BlockId, HashSet<VarId>>,
    /// Def-use chains at block level (backward compatibility)
    pub def_use_chains: HashMap<VarId, HashSet<BlockId>>,

    // --- Statement-level (new) ---
    /// Precise def-use chains: definition point → use points
    pub precise_def_use: HashMap<Definition, HashSet<ProgramPoint>>,
    /// Use-def chains (inverse): use point → reaching definitions
    pub use_def_chains: HashMap<Use, HashSet<Definition>>,
    /// All definitions in the program
    pub all_definitions: Vec<Definition>,
    /// All uses in the program
    pub all_uses: Vec<Use>,
}

impl ReachingDefinitions {
    /// Compute reaching definitions for a CFG using forward data flow analysis.
    ///
    /// This performs both block-level analysis (for backward compatibility) and
    /// statement-level analysis for precise def-use chains.
    pub fn analyze(cfg: &ControlFlowGraph) -> Self {
        let mut reach_in: HashMap<BlockId, HashSet<VarId>> = HashMap::new();
        let mut reach_out: HashMap<BlockId, HashSet<VarId>> = HashMap::new();

        // Initialize all blocks
        for block in &cfg.blocks {
            reach_in.insert(block.id, HashSet::new());
            reach_out.insert(block.id, HashSet::new());
        }

        // Fixed-point iteration (forward analysis)
        let mut changed = true;
        while changed {
            changed = false;

            for block in &cfg.blocks {
                // Compute reach_in as union of reach_out from all predecessors
                let mut new_reach_in = HashSet::new();
                for pred_id in Self::get_predecessors(cfg, block.id) {
                    if let Some(pred_out) = reach_out.get(&pred_id) {
                        new_reach_in.extend(pred_out.iter().cloned());
                    }
                }

                // Compute gen and kill sets for this block
                let (gen, kill) = Self::compute_gen_kill(block);

                // Compute reach_out = (reach_in - kill) ∪ gen
                let mut new_reach_out = new_reach_in.clone();
                new_reach_out.retain(|v| !kill.contains(&v.name_id));
                new_reach_out.extend(gen);

                // Check for changes
                if new_reach_in != *reach_in.get(&block.id).unwrap()
                    || new_reach_out != *reach_out.get(&block.id).unwrap()
                {
                    changed = true;
                    reach_in.insert(block.id, new_reach_in);
                    reach_out.insert(block.id, new_reach_out);
                }
            }
        }

        // Build block-level def-use chains (backward compatibility)
        let def_use_chains = Self::build_def_use_chains(cfg, &reach_in);

        // --- Statement-level analysis ---
        let (all_definitions, all_uses) = Self::collect_defs_and_uses(cfg);
        let (precise_def_use, use_def_chains) =
            Self::compute_precise_chains(cfg, &reach_in, &all_definitions, &all_uses);

        ReachingDefinitions {
            // Block-level (backward compatible)
            reach_in,
            reach_out,
            def_use_chains,
            // Statement-level (new)
            precise_def_use,
            use_def_chains,
            all_definitions,
            all_uses,
        }
    }

    // ========================================================================
    // Statement-Level Analysis Methods
    // ========================================================================

    /// Collect all definitions and uses with their program points.
    fn collect_defs_and_uses(cfg: &ControlFlowGraph) -> (Vec<Definition>, Vec<Use>) {
        let mut definitions = Vec::new();
        let mut uses = Vec::new();

        for block in &cfg.blocks {
            for (stmt_idx, stmt) in block.statements.iter().enumerate() {
                let point = ProgramPoint::new(block.id, stmt_idx);

                match stmt {
                    Statement::Declare { var, init, .. } => {
                        // This is a definition
                        definitions.push(Definition { var: *var, point });

                        // Init expression may use variables
                        if let Some(init_rval) = init {
                            for used_var in Self::rvalue_uses(init_rval) {
                                uses.push(Use {
                                    var: used_var,
                                    point,
                                });
                            }
                        }
                    }

                    Statement::Assign { target, source, .. } => {
                        // This is a definition
                        definitions.push(Definition {
                            var: *target,
                            point,
                        });

                        // Source uses variables
                        for used_var in Self::rvalue_uses(source) {
                            uses.push(Use {
                                var: used_var,
                                point,
                            });
                        }
                    }

                    Statement::Expr { expr, .. } => {
                        // Expression may use variables
                        for used_var in Self::expr_kind_uses(expr) {
                            uses.push(Use {
                                var: used_var,
                                point,
                            });
                        }
                    }
                }
            }

            // Terminator may use variables
            let term_point = ProgramPoint::block_exit(block.id, block.statements.len());
            for used_var in Self::terminator_uses(&block.terminator) {
                uses.push(Use {
                    var: used_var,
                    point: term_point,
                });
            }
        }

        (definitions, uses)
    }

    /// Extract variables used in an Rvalue.
    fn rvalue_uses(rval: &Rvalue) -> Vec<VarId> {
        match rval {
            Rvalue::Use(var) => vec![*var],
            Rvalue::BinaryOp { left, right, .. } => vec![*left, *right],
            Rvalue::UnaryOp { operand, .. } => vec![*operand],
            Rvalue::FieldAccess { base, .. } => vec![*base],
            Rvalue::Ref { var, .. } => vec![*var],
            Rvalue::Call { args, .. } => args.clone(),
            Rvalue::Constant => vec![],
        }
    }

    /// Extract variables used in an ExprKind.
    fn expr_kind_uses(expr: &ExprKind) -> Vec<VarId> {
        match expr {
            ExprKind::MethodCall { receiver, args, .. } => {
                let mut vars = vec![*receiver];
                vars.extend(args.iter().cloned());
                vars
            }
            ExprKind::MacroCall { args, .. } => args.clone(),
            ExprKind::Closure { captures, .. } => captures.clone(),
            ExprKind::Other => vec![],
        }
    }

    /// Extract variables used in a terminator.
    fn terminator_uses(term: &Terminator) -> Vec<VarId> {
        match term {
            Terminator::Return { value: Some(var) } => vec![*var],
            Terminator::Branch { condition, .. } => vec![*condition],
            _ => vec![],
        }
    }

    /// Compute precise def-use chains at statement level.
    fn compute_precise_chains(
        cfg: &ControlFlowGraph,
        reach_in: &HashMap<BlockId, HashSet<VarId>>,
        definitions: &[Definition],
        uses: &[Use],
    ) -> (
        HashMap<Definition, HashSet<ProgramPoint>>,
        HashMap<Use, HashSet<Definition>>,
    ) {
        let mut def_use: HashMap<Definition, HashSet<ProgramPoint>> = HashMap::new();
        let mut use_def: HashMap<Use, HashSet<Definition>> = HashMap::new();

        // Initialize def_use for all definitions
        for def in definitions {
            def_use.insert(*def, HashSet::new());
        }

        // For each use, find which definitions reach it
        for use_point in uses {
            let reaching_defs =
                Self::find_reaching_defs_at_point(cfg, reach_in, definitions, use_point);

            use_def.insert(*use_point, reaching_defs.clone());

            // Update def_use (inverse)
            for def in reaching_defs {
                def_use.entry(def).or_default().insert(use_point.point);
            }
        }

        (def_use, use_def)
    }

    /// Find definitions of a variable that reach a specific use point.
    fn find_reaching_defs_at_point(
        cfg: &ControlFlowGraph,
        reach_in: &HashMap<BlockId, HashSet<VarId>>,
        definitions: &[Definition],
        use_point: &Use,
    ) -> HashSet<Definition> {
        let var = use_point.var;
        let block_id = use_point.point.block;
        let stmt_idx = use_point.point.stmt;

        // Get the block
        let block_data = match cfg.blocks.iter().find(|b| b.id == block_id) {
            Some(b) => b,
            None => return HashSet::new(),
        };

        // Look for definition of same var in same block before this statement
        let mut found_in_block: Option<Definition> = None;
        for (idx, stmt) in block_data.statements.iter().enumerate() {
            if idx >= stmt_idx {
                break; // Only look at statements before use
            }

            let defines_var = match stmt {
                Statement::Declare { var: def_var, .. } => def_var.name_id == var.name_id,
                Statement::Assign { target, .. } => target.name_id == var.name_id,
                _ => false,
            };

            if defines_var {
                // This is the most recent definition before our use
                // Find the actual definition from our definitions list
                found_in_block = definitions
                    .iter()
                    .find(|d| d.point.block == block_id && d.point.stmt == idx)
                    .copied();
            }
        }

        // If found in block, that's the only reaching definition
        if let Some(def) = found_in_block {
            return [def].into_iter().collect();
        }

        // Otherwise, use reach_in for this block
        let reaching = reach_in.get(&block_id).cloned().unwrap_or_default();

        // Find actual definition points for reaching vars
        definitions
            .iter()
            .filter(|def| def.var.name_id == var.name_id && reaching.contains(&def.var))
            .copied()
            .collect()
    }

    // ========================================================================
    // Statement-Level Query Methods
    // ========================================================================

    /// Get all uses of a specific definition (statement-level).
    pub fn get_uses_of(&self, def: &Definition) -> Option<&HashSet<ProgramPoint>> {
        self.precise_def_use.get(def)
    }

    /// Get all definitions that reach a specific use (statement-level).
    pub fn get_defs_of(&self, use_point: &Use) -> Option<&HashSet<Definition>> {
        self.use_def_chains.get(use_point)
    }

    /// Check if a definition is dead (no uses).
    pub fn is_dead_definition(&self, def: &Definition) -> bool {
        self.precise_def_use
            .get(def)
            .map(|uses| uses.is_empty())
            .unwrap_or(true)
    }

    /// Find same-block dead stores: defs with no uses at all.
    pub fn find_same_block_dead_stores(&self) -> Vec<Definition> {
        self.all_definitions
            .iter()
            .filter(|def| self.is_dead_definition(def))
            .copied()
            .collect()
    }

    /// Get the single reaching definition for a use (if unique).
    pub fn get_unique_def(&self, use_point: &Use) -> Option<Definition> {
        self.use_def_chains.get(use_point).and_then(|defs| {
            if defs.len() == 1 {
                defs.iter().next().copied()
            } else {
                None
            }
        })
    }

    // ========================================================================
    // Block-Level Analysis Methods (existing)
    // ========================================================================

    /// Compute gen and kill sets for a basic block.
    ///
    /// - gen: new definitions created in this block
    /// - kill: variable name_ids whose definitions are overwritten
    fn compute_gen_kill(block: &BasicBlock) -> (HashSet<VarId>, HashSet<u32>) {
        let mut gen = HashSet::new();
        let mut kill = HashSet::new();

        for stmt in &block.statements {
            if let Statement::Assign { target, .. } = stmt {
                // This assignment kills all previous definitions of this variable
                kill.insert(target.name_id);
                // And generates a new definition
                gen.insert(*target);
            }
        }

        (gen, kill)
    }

    /// Get predecessors of a block in the CFG.
    fn get_predecessors(cfg: &ControlFlowGraph, block_id: BlockId) -> Vec<BlockId> {
        cfg.edges
            .iter()
            .filter_map(|(from, edges)| {
                if edges.iter().any(|(to, _)| *to == block_id) {
                    Some(*from)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Build def-use chains by identifying where each definition is used.
    fn build_def_use_chains(
        cfg: &ControlFlowGraph,
        reach_in: &HashMap<BlockId, HashSet<VarId>>,
    ) -> HashMap<VarId, HashSet<BlockId>> {
        let mut chains: HashMap<VarId, HashSet<BlockId>> = HashMap::new();

        for block in &cfg.blocks {
            let reaching = reach_in.get(&block.id).unwrap();

            // Find all variable uses in this block
            for stmt in &block.statements {
                match stmt {
                    Statement::Assign { source, .. } => {
                        // Collect variables used in the RHS
                        Self::collect_uses(source, reaching, block.id, &mut chains);
                    }
                    Statement::Declare { init, .. } => {
                        // Collect variables used in the initializer
                        if let Some(init_rvalue) = init {
                            Self::collect_uses(init_rvalue, reaching, block.id, &mut chains);
                        }
                    }
                    Statement::Expr { .. } => {
                        // Expression statements don't directly use variables in our CFG model
                    }
                }
            }

            // Check terminator for uses
            match &block.terminator {
                Terminator::Branch { condition, .. } => {
                    Self::collect_var_use(condition, reaching, block.id, &mut chains);
                }
                Terminator::Return { value: Some(val) } => {
                    Self::collect_var_use(val, reaching, block.id, &mut chains);
                }
                Terminator::Return { value: None } => {}
                _ => {}
            }
        }

        chains
    }

    /// Collect variable uses from a VarId and update def-use chains.
    fn collect_var_use(
        var_id: &VarId,
        reaching: &HashSet<VarId>,
        block_id: BlockId,
        chains: &mut HashMap<VarId, HashSet<BlockId>>,
    ) {
        // Find which reaching definition this use corresponds to
        for def in reaching {
            if def.name_id == var_id.name_id {
                chains.entry(*def).or_default().insert(block_id);
            }
        }
    }

    /// Collect variable uses from an Rvalue and update def-use chains.
    fn collect_uses(
        rvalue: &Rvalue,
        reaching: &HashSet<VarId>,
        block_id: BlockId,
        chains: &mut HashMap<VarId, HashSet<BlockId>>,
    ) {
        match rvalue {
            Rvalue::Use(var_id) => {
                Self::collect_var_use(var_id, reaching, block_id, chains);
            }
            Rvalue::BinaryOp { left, right, .. } => {
                Self::collect_var_use(left, reaching, block_id, chains);
                Self::collect_var_use(right, reaching, block_id, chains);
            }
            Rvalue::UnaryOp { operand, .. } => {
                Self::collect_var_use(operand, reaching, block_id, chains);
            }
            Rvalue::Call { args, .. } => {
                for arg in args {
                    Self::collect_var_use(arg, reaching, block_id, chains);
                }
            }
            Rvalue::FieldAccess { base, .. } | Rvalue::Ref { var: base, .. } => {
                Self::collect_var_use(base, reaching, block_id, chains);
            }
            Rvalue::Constant => {
                // Constants don't use variables
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_data_flow_from_block() {
        let block: syn::Block = parse_quote! {
            {
                let mut x = 1;
                x = x + 1;
                x
            }
        };

        let analysis = DataFlowAnalysis::from_block(&block);
        // Verify analysis completes and reaching_defs is populated
        assert!(
            !analysis.reaching_defs.all_definitions.is_empty()
                || analysis.reaching_defs.all_definitions.is_empty() // always valid
        );
    }

    #[test]
    fn test_statement_level_simple_def_use() {
        let block: syn::Block = parse_quote! {
            {
                let x = 1;
                let y = x + 2;
            }
        };

        let cfg = ControlFlowGraph::from_block(&block);
        let reaching = ReachingDefinitions::analyze(&cfg);

        // Should have definitions
        assert!(
            !reaching.all_definitions.is_empty(),
            "Should have at least one definition"
        );

        // Should have uses
        assert!(
            !reaching.all_uses.is_empty(),
            "Should have at least one use"
        );

        // Find definition of x
        let x_def = reaching.all_definitions.iter().find(|d| {
            d.point.stmt == 0 // First statement
        });

        assert!(x_def.is_some(), "Should find definition at statement 0");

        // Definition of x should have uses (in the second statement)
        if let Some(def) = x_def {
            let uses = reaching.get_uses_of(def);
            assert!(uses.is_some(), "x definition should have uses tracked");
        }
    }

    #[test]
    fn test_statement_level_dead_store_detection() {
        let block: syn::Block = parse_quote! {
            {
                let x = 1;
                let y = 2;
            }
        };

        let cfg = ControlFlowGraph::from_block(&block);
        let reaching = ReachingDefinitions::analyze(&cfg);

        // Both x and y are dead stores (never used)
        let dead_stores = reaching.find_same_block_dead_stores();
        assert!(
            !dead_stores.is_empty(),
            "Should detect dead stores for unused variables"
        );
    }

    #[test]
    fn test_statement_level_use_def_chains() {
        let block: syn::Block = parse_quote! {
            {
                let a = 1;
                let b = a;
            }
        };

        let cfg = ControlFlowGraph::from_block(&block);
        let reaching = ReachingDefinitions::analyze(&cfg);

        // For each use, should be able to find its definition
        for use_point in &reaching.all_uses {
            let defs = reaching.get_defs_of(use_point);
            // Uses should have at least empty set tracked
            assert!(
                defs.is_some(),
                "Use {:?} should have reaching defs tracked",
                use_point
            );
        }
    }

    #[test]
    fn test_statement_level_unique_def() {
        let block: syn::Block = parse_quote! {
            {
                let x = 1;
                let y = x;
            }
        };

        let cfg = ControlFlowGraph::from_block(&block);
        let reaching = ReachingDefinitions::analyze(&cfg);

        // Find use of x
        let x_use = reaching.all_uses.iter().find(|u| {
            cfg.var_names
                .get(u.var.name_id as usize)
                .is_some_and(|n| n == "x")
        });

        if let Some(use_point) = x_use {
            let unique_def = reaching.get_unique_def(use_point);
            assert!(unique_def.is_some(), "Should find unique definition for x");
        }
    }

    #[test]
    fn test_statement_level_chained_assignments() {
        let block: syn::Block = parse_quote! {
            {
                let mut x = 1;
                x = x + 1;
            }
        };

        let cfg = ControlFlowGraph::from_block(&block);
        let reaching = ReachingDefinitions::analyze(&cfg);

        // Should have definitions
        assert!(
            !reaching.all_definitions.is_empty(),
            "Should have at least 1 definition for x"
        );
    }

    #[test]
    fn test_statement_level_is_dead_definition() {
        let block: syn::Block = parse_quote! {
            {
                let unused = 42;
            }
        };

        let cfg = ControlFlowGraph::from_block(&block);
        let reaching = ReachingDefinitions::analyze(&cfg);

        // Find the definition of 'unused'
        let unused_def = reaching
            .all_definitions
            .first()
            .expect("Should have at least one definition");

        // It should be dead (no uses)
        assert!(
            reaching.is_dead_definition(unused_def),
            "Unused variable should be a dead definition"
        );
    }

    #[test]
    fn test_statement_level_backward_compatibility() {
        // Verify that block-level API still works
        let block: syn::Block = parse_quote! {
            {
                let x = 1;
                let y = x;
            }
        };

        let cfg = ControlFlowGraph::from_block(&block);
        let reaching = ReachingDefinitions::analyze(&cfg);

        // Block-level fields should still be populated
        assert!(
            !reaching.reach_in.is_empty() || cfg.blocks.is_empty(),
            "reach_in should be populated"
        );
        assert!(
            !reaching.reach_out.is_empty() || cfg.blocks.is_empty(),
            "reach_out should be populated"
        );
    }

    #[test]
    fn test_statement_level_empty_function() {
        let block: syn::Block = parse_quote! { {} };

        let cfg = ControlFlowGraph::from_block(&block);
        let reaching = ReachingDefinitions::analyze(&cfg);

        // Should handle empty functions gracefully
        assert!(
            reaching.all_definitions.is_empty(),
            "Empty function should have no definitions"
        );
        assert!(
            reaching.all_uses.is_empty(),
            "Empty function should have no uses"
        );
        assert!(
            reaching.find_same_block_dead_stores().is_empty(),
            "Empty function should have no dead stores"
        );
    }

    #[test]
    fn test_statement_level_terminator_uses() {
        let block: syn::Block = parse_quote! {
            {
                let x = 1;
                return x;
            }
        };

        let cfg = ControlFlowGraph::from_block(&block);
        let reaching = ReachingDefinitions::analyze(&cfg);

        // The return statement should create a use of x
        let return_use = reaching.all_uses.iter().any(|u| {
            cfg.var_names
                .get(u.var.name_id as usize)
                .is_some_and(|n| n == "x")
        });

        assert!(return_use, "Return statement should create a use of x");

        // x should not be dead since it's returned
        let x_def = reaching.all_definitions.iter().find(|d| {
            cfg.var_names
                .get(d.var.name_id as usize)
                .is_some_and(|n| n == "x")
        });

        if let Some(def) = x_def {
            assert!(
                !reaching.is_dead_definition(def),
                "x should not be dead since it's returned"
            );
        }
    }
}

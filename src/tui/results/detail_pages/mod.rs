//! Multi-page detail view for debt items.
//!
//! Provides eight pages of contextual information:
//! - Page 1: Overview (score, metrics)
//! - Page 2: Score Breakdown (detailed scoring analysis)
//! - Page 3: Context (AI context suggestions)
//! - Page 4: Dependencies (callers, callees, blast radius)
//! - Page 5: Git Context (history, risk, dampening)
//! - Page 6: Patterns (purity, frameworks, language features)
//! - Page 7: Data Flow (mutations, I/O operations, escape analysis)
//! - Page 8: Responsibilities (role and responsibility analysis)
//!
//! Navigation:
//! - ←→/hl: Switch pages
//! - 1-8: Jump to page
//! - ↑↓/jk: Navigate items (preserves page)

pub mod components;
pub mod context;
pub mod data_flow;
pub mod dependencies;
pub mod git_context;
pub mod overview;
pub mod patterns;
pub mod responsibilities;
pub mod score_breakdown;

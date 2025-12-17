//! Multi-page detail view for debt items.
//!
//! Provides seven pages of contextual information:
//! - Page 1: Overview (score, metrics, recommendation)
//! - Page 2: Score Breakdown (detailed scoring analysis)
//! - Page 3: Dependencies (callers, callees, blast radius)
//! - Page 4: Git Context (history, risk, dampening)
//! - Page 5: Patterns (purity, frameworks, language features)
//! - Page 6: Data Flow (mutations, I/O operations, escape analysis)
//! - Page 7: Responsibilities (role and responsibility analysis)
//!
//! Navigation:
//! - ←→/hl: Switch pages
//! - 1-7: Jump to page
//! - ↑↓/jk: Navigate items (preserves page)

pub mod components;
pub mod data_flow;
pub mod dependencies;
pub mod git_context;
pub mod overview;
pub mod patterns;
pub mod responsibilities;
pub mod score_breakdown;

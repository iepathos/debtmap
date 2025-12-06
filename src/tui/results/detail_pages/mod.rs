//! Multi-page detail view for debt items.
//!
//! Provides four pages of contextual information:
//! - Page 1: Overview (score, metrics, recommendation)
//! - Page 2: Dependencies (callers, callees, blast radius)
//! - Page 3: Git Context (history, risk, dampening)
//! - Page 4: Patterns (purity, frameworks, language features)
//!
//! Navigation:
//! - Tab/←→: Switch pages
//! - 1-4: Jump to page
//! - n/p: Navigate items (preserves page)

pub mod components;
pub mod dependencies;
pub mod git_context;
pub mod overview;
pub mod patterns;

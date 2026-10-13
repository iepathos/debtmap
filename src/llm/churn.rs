//! Recent LLM churn: signs that LLMs struggle (or succeed) editing a function.
//!
//! Only commits inside a recent window count as rework. Within it:
//! - repeated LLM-co-authored edits (beyond the first),
//! - fixes landing shortly after an LLM edit,
//! - reverts of LLM edits (matched by hash, or landing shortly after one),
//! - bursts of conventional `fix(scope):` commits for scopes this function's
//!   recent commits used.
//!
//! Older LLM edits that were never reverted and that no fix or revert
//! followed within the window are evidence the function is LLM-editable and
//! give a small, capped score bonus.

use crate::risk::context::git_history::activity::{CommitActivity, CommitFact};
use chrono::{DateTime, Duration, Utc};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Case-insensitive globs matched against `Co-authored-by` values and
/// "Generated with/by" footers. Patterns avoid bare first names (e.g. "devin")
/// that would match human co-authors.
pub const BUILTIN_AGENTS: &[&str] = &[
    "*noreply@anthropic.com*",
    "*claude code*",
    "*cursoragent@cursor.com*",
    "*cursor agent*",
    "*copilot*",
    "*codex*",
    "*chatgpt*",
    "*gemini*",
    "*google-labs-jules*",
    "*noreply@aider.chat*",
    "aider (*",
    "*devin-ai-integration*",
    "*ampcode.com*",
    "*windsurf*",
    "*codeium*",
    "*opencode*",
    "*factory-droid*",
    "*qodo*",
    "*tabnine*",
    "*sweep-ai*",
];

/// `[llm_editability.churn]` settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ChurnConfig {
    /// Only commits this recent count as rework.
    pub window_days: u32,
    /// Extra agent globs, matched case-insensitively.
    pub agents: Vec<String>,
    /// Include [`BUILTIN_AGENTS`].
    pub builtin_agents: bool,
    /// Maximum logit bonus from proven (never-fixed) LLM edits.
    pub proven_edit_bonus: f64,
    /// Proven edits at which half the bonus applies.
    pub proven_edit_tolerance: f64,
    pub topics: TopicConfig,
}

impl Default for ChurnConfig {
    fn default() -> Self {
        Self {
            window_days: 14,
            agents: Vec::new(),
            builtin_agents: true,
            proven_edit_bonus: 0.5,
            proven_edit_tolerance: 3.0,
            topics: TopicConfig::default(),
        }
    }
}

/// `[llm_editability.churn.topics]`: group recent fixes by commit-message
/// topic. Compares every recent fix with each function's recent commits, so
/// it is off by default.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TopicConfig {
    pub enabled: bool,
    /// Weighted share of terms two messages must have in common (0..=1].
    pub min_similarity: f64,
}

impl Default for TopicConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            min_similarity: 0.3,
        }
    }
}

impl ChurnConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.window_days == 0 {
            return Err("llm_editability.churn.window_days must be > 0".into());
        }
        if self.proven_edit_bonus < 0.0 || self.proven_edit_tolerance <= 0.0 {
            return Err("llm_editability.churn: proven_edit_bonus must be >= 0 and \
                 proven_edit_tolerance > 0"
                .into());
        }
        if !(self.topics.min_similarity > 0.0 && self.topics.min_similarity <= 1.0) {
            return Err("llm_editability.churn.topics.min_similarity must be in (0, 1]".into());
        }
        AgentMatcher::new(self).map(|_| ())
    }

    /// Logit bonus for `proven` never-fixed LLM edits, saturating at the maximum.
    pub fn proven_bonus(&self, proven: f64) -> f64 {
        let proven = proven.max(0.0);
        self.proven_edit_bonus * proven / (proven + self.proven_edit_tolerance)
    }
}

/// Decides whether a commit was co-authored by an AI agent.
#[derive(Debug, Clone)]
pub struct AgentMatcher(GlobSet);

impl AgentMatcher {
    pub fn new(config: &ChurnConfig) -> Result<Self, String> {
        let builtins = BUILTIN_AGENTS
            .iter()
            .copied()
            .filter(|_| config.builtin_agents);
        builtins
            .chain(config.agents.iter().map(String::as_str))
            .try_fold(GlobSetBuilder::new(), |mut set, pattern| {
                let glob = GlobBuilder::new(pattern)
                    .case_insensitive(true)
                    .build()
                    .map_err(|e| format!("llm_editability.churn.agents: {e}"))?;
                set.add(glob);
                Ok::<_, String>(set)
            })?
            .build()
            .map(Self)
            .map_err(|e| format!("llm_editability.churn.agents: {e}"))
    }

    pub fn is_llm(&self, commit: &CommitFact) -> bool {
        commit.agent_marks.iter().any(|mark| self.0.is_match(mark))
    }
}

/// Recent churn for one function, as emitted in JSON.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChurnSummary {
    pub window_days: u32,
    /// LLM-co-authored commits touching the function inside the window.
    pub llm_edits: u32,
    /// Fixes inside the window that landed within `window_days` of an LLM edit.
    pub fixes_after_llm_edit: u32,
    /// Reverts inside the window that undo an LLM edit: the reverted commit
    /// when its hash is known, otherwise one landing within `window_days` of
    /// an LLM edit.
    #[serde(default)]
    pub reverts_of_llm_edits: u32,
    /// Fixes inside the window in this function's busiest fix group.
    pub fix_burst: u32,
    /// `None` without recent fixes in any group.
    pub fix_burst_group: Option<FixGroup>,
    /// On-topic fixes in a scope this function's recent commits used: both
    /// signals agree, so each adds [`CORROBORATION_BONUS`] to rework.
    #[serde(default)]
    pub corroborated_fixes: u32,
    /// On-topic fixes filed under scopes this function's recent commits did
    /// not use, by scope: LLM edits here may break code elsewhere.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub cross_scope_fixes: BTreeMap<String, u32>,
    /// LLM edits older than the window, never reverted, that no fix or revert
    /// followed within the window.
    pub proven_llm_edits: u32,
}

/// How a function's related fixes were grouped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixGroup {
    /// Conventional `fix(scope):` commits for a scope its recent commits used.
    Scope(String),
    /// Scope-less conventional `fix:` commits touching its file.
    File,
    /// Fix commits whose messages share a topic with its recent commits.
    Topic,
}

/// Extra rework per fix that matches both the function's scope and topic.
pub const CORROBORATION_BONUS: f64 = 0.5;

impl ChurnSummary {
    /// Rework count: a single recent LLM edit or grouped fix is normal; every
    /// follow-up fix and revert of an LLM edit counts, corroborated fixes
    /// count extra, and on-topic fixes under other scopes count too.
    pub fn rework(&self) -> f64 {
        let excess = |n: u32| n.saturating_sub(1) as f64;
        excess(self.llm_edits)
            + self.fixes_after_llm_edit as f64
            + self.reverts_of_llm_edits as f64
            + excess(self.fix_burst)
            + CORROBORATION_BONUS * self.corroborated_fixes as f64
            + self.cross_scope_fixes.values().sum::<u32>() as f64
    }
}

/// Recent on-topic fixes, split against the function's recent scopes.
#[derive(Default)]
struct TopicSplit {
    /// Scope-less or same-scope fixes: the topic fix group.
    grouped: u32,
    corroborated: u32,
    cross_scope: BTreeMap<String, u32>,
}

fn split_topic_fixes(
    activity: &CommitActivity,
    scopes: &BTreeSet<&str>,
    since: DateTime<Utc>,
) -> TopicSplit {
    activity
        .topic_fixes
        .iter()
        .filter(|f| f.date >= since)
        .fold(TopicSplit::default(), |mut split, fix| {
            match fix.scope.as_deref() {
                Some(s) if scopes.contains(s) => split.corroborated += 1,
                Some(s) if !scopes.is_empty() => {
                    *split.cross_scope.entry(s.to_string()).or_insert(0) += 1;
                    return split;
                }
                _ => {}
            }
            split.grouped += 1;
            split
        })
}

/// A function's commits split by authorship, within one churn window.
struct History<'a> {
    commits: &'a [CommitFact],
    llm: Vec<&'a CommitFact>,
    window: Duration,
}

impl History<'_> {
    fn follows_llm(&self, c: &CommitFact) -> bool {
        self.llm
            .iter()
            .any(|e| e.date < c.date && c.date - e.date <= self.window)
    }

    /// The reverted commit when its hash is in this history, else a recent LLM edit.
    fn undoes_llm(&self, revert: &CommitFact) -> bool {
        match self.commits.iter().find(|c| revert.reverts(c)) {
            Some(target) => self.llm.iter().any(|e| e.hash == target.hash),
            None => self.follows_llm(revert),
        }
    }

    fn proven(&self, edit: &CommitFact, now: DateTime<Utc>) -> bool {
        let soon_after = |c: &CommitFact| c.date > edit.date && c.date - edit.date <= self.window;
        let undone = self
            .commits
            .iter()
            .any(|c| c.reverts(edit) || ((c.fix || c.revert) && soon_after(c)));
        !edit.fix && !edit.revert && edit.date + self.window <= now && !undone
    }
}

pub fn summarize(
    activity: &CommitActivity,
    matcher: &AgentMatcher,
    now: DateTime<Utc>,
    window_days: u32,
) -> ChurnSummary {
    let window = Duration::days(window_days.into());
    let history = History {
        commits: &activity.commits,
        llm: activity
            .commits
            .iter()
            .filter(|c| matcher.is_llm(c))
            .collect(),
        window,
    };
    let recent: Vec<&CommitFact> = activity
        .commits
        .iter()
        .filter(|c| c.date >= now - window)
        .collect();
    let count =
        |pred: &dyn Fn(&CommitFact) -> bool| recent.iter().filter(|c| pred(c)).count() as u32;
    let since = now - window;
    let scopes: BTreeSet<&str> = recent.iter().filter_map(|c| c.scope.as_deref()).collect();
    let topics = split_topic_fixes(activity, &scopes, since);
    let (fix_burst_group, fix_burst) = busiest_fix_group(activity, &scopes, topics.grouped, since);
    ChurnSummary {
        window_days,
        llm_edits: count(&|c| matcher.is_llm(c)),
        fixes_after_llm_edit: count(&|c| c.fix && !c.revert && history.follows_llm(c)),
        reverts_of_llm_edits: count(&|c| c.revert && history.undoes_llm(c)),
        fix_burst,
        fix_burst_group,
        corroborated_fixes: topics.corroborated,
        cross_scope_fixes: topics.cross_scope,
        proven_llm_edits: history
            .llm
            .iter()
            .filter(|e| history.proven(e, now))
            .count() as u32,
    }
}

/// Busiest recent fix group. Ties go to scopes (alphabetically), then the
/// file, then the topic.
fn busiest_fix_group(
    activity: &CommitActivity,
    scopes: &BTreeSet<&str>,
    topic_fixes: u32,
    since: DateTime<Utc>,
) -> (Option<FixGroup>, u32) {
    let recent = |dates: &[DateTime<Utc>]| dates.iter().filter(|d| **d >= since).count() as u32;
    let groups: Vec<(FixGroup, u32)> = scopes
        .iter()
        .filter_map(|&s| {
            Some((
                FixGroup::Scope(s.into()),
                recent(activity.scope_fixes.get(s)?),
            ))
        })
        .chain([
            (FixGroup::File, recent(&activity.file_fixes)),
            (FixGroup::Topic, topic_fixes),
        ])
        .filter(|(_, n)| *n > 0)
        .collect();
    groups
        .into_iter()
        .rev()
        .max_by_key(|(_, n)| *n)
        .map(|(group, n)| (Some(group), n))
        .unwrap_or((None, 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::risk::context::git_history::activity::{TopicFix, commit_fact};
    use std::collections::BTreeMap;

    const CLAUDE: &str = "\n\nCo-authored-by: Claude <noreply@anthropic.com>";

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-10-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn days_ago(n: i64, message: &str) -> CommitFact {
        commit_fact(&format!("{n:07x}cafe"), now() - Duration::days(n), message)
    }

    fn matcher() -> AgentMatcher {
        AgentMatcher::new(&ChurnConfig::default()).unwrap()
    }

    fn summary(commits: Vec<CommitFact>) -> ChurnSummary {
        let activity = CommitActivity {
            commits,
            ..Default::default()
        };
        summarize(&activity, &matcher(), now(), 14)
    }

    fn reverting(n: i64) -> String {
        format!("Revert \"feat: a\"\n\nThis reverts commit {n:07x}cafe.")
    }

    #[test]
    fn builtin_patterns_match_agents_but_not_humans() {
        let m = matcher();
        let llm = |msg: &str| m.is_llm(&days_ago(0, msg));
        assert!(llm(&format!("feat: x{CLAUDE}")));
        assert!(llm(
            "feat: x\n\nCo-authored-by: Cursor Agent <cursoragent@cursor.com>"
        ));
        assert!(llm(
            "feat: x\n\n🤖 Generated with [Claude Code](https://claude.com)"
        ));
        assert!(!llm(
            "feat: x\n\nCo-authored-by: Devin Smith <devin@example.com>"
        ));
        assert!(!llm(
            "feat: x\n\nCo-authored-by: Claude Dupont <claude@example.fr>"
        ));
        assert!(!llm("feat: x"));
    }

    #[test]
    fn custom_agents_extend_or_replace_builtins() {
        let config = ChurnConfig {
            agents: vec!["*our-bot*".into()],
            builtin_agents: false,
            ..Default::default()
        };
        let m = AgentMatcher::new(&config).unwrap();
        assert!(m.is_llm(&days_ago(0, "x\n\nCo-authored-by: Our-Bot <b@x.io>")));
        assert!(!m.is_llm(&days_ago(0, &format!("x{CLAUDE}"))));
    }

    #[test]
    fn rejects_invalid_config() {
        let bad_glob = ChurnConfig {
            agents: vec!["[".into()],
            ..Default::default()
        };
        assert!(bad_glob.validate().is_err());
        let zero_window = ChurnConfig {
            window_days: 0,
            ..Default::default()
        };
        assert!(zero_window.validate().is_err());
    }

    #[test]
    fn repeated_recent_llm_edits_and_follow_up_fixes_are_rework() {
        let s = summary(vec![
            days_ago(6, &format!("feat: a{CLAUDE}")),
            days_ago(4, &format!("refactor: b{CLAUDE}")),
            days_ago(2, "fix: undo the breakage"),
        ]);
        assert_eq!(s.llm_edits, 2);
        assert_eq!(s.fixes_after_llm_edit, 1);
        assert_eq!(s.rework(), 2.0);
    }

    #[test]
    fn old_edits_are_not_rework_and_unfixed_ones_are_proven() {
        let s = summary(vec![
            days_ago(90, &format!("feat: a{CLAUDE}")),
            days_ago(60, &format!("feat: b{CLAUDE}")),
            days_ago(55, "fix: broke b"),
            days_ago(3, &format!("feat: too new to judge{CLAUDE}")),
        ]);
        assert_eq!(s.llm_edits, 1);
        assert_eq!(s.rework(), 0.0);
        assert_eq!(s.proven_llm_edits, 1);
    }

    #[test]
    fn reverts_of_llm_edits_are_rework() {
        let s = summary(vec![
            days_ago(6, &format!("feat: a{CLAUDE}")),
            days_ago(5, &reverting(6)),
            days_ago(3, "refactor: human edit"),
            days_ago(1, &reverting(3)),
        ]);
        assert_eq!(
            s.reverts_of_llm_edits, 1,
            "the human edit's revert is not LLM rework"
        );
        assert_eq!(s.rework(), 1.0);
    }

    #[test]
    fn hashless_reverts_count_when_they_follow_an_llm_edit() {
        let s = summary(vec![
            days_ago(6, &format!("feat: a{CLAUDE}")),
            days_ago(5, "revert: back out a"),
        ]);
        assert_eq!(s.reverts_of_llm_edits, 1);
        assert_eq!(s.fixes_after_llm_edit, 0);
    }

    #[test]
    fn reverted_llm_edits_are_never_proven() {
        let s = summary(vec![
            days_ago(90, &format!("feat: a{CLAUDE}")),
            days_ago(60, &reverting(90)),
            days_ago(50, &format!("feat: b{CLAUDE}")),
            days_ago(45, "revert: back out b"),
        ]);
        assert_eq!(s.proven_llm_edits, 0);
        assert_eq!(s.rework(), 0.0, "old reverts are outside the window");
    }

    fn days(ns: &[i64]) -> Vec<DateTime<Utc>> {
        ns.iter().map(|n| now() - Duration::days(*n)).collect()
    }

    #[test]
    fn fix_bursts_in_a_recent_scope_count() {
        let activity = CommitActivity {
            commits: vec![days_ago(3, "feat(parser): tokens")],
            scope_fixes: BTreeMap::from([("parser".to_string(), days(&[40, 5, 3, 1]))]),
            file_fixes: days(&[2, 1]),
            ..Default::default()
        };
        let s = summarize(&activity, &matcher(), now(), 14);
        let parser = FixGroup::Scope("parser".into());
        assert_eq!((s.fix_burst_group.clone(), s.fix_burst), (Some(parser), 3));
        assert_eq!(s.rework(), 2.0);
    }

    #[test]
    fn scopeless_fix_bursts_count_per_file() {
        let activity = CommitActivity {
            commits: vec![days_ago(3, "feat: tokens")],
            file_fixes: days(&[40, 5, 3, 1]),
            ..Default::default()
        };
        let s = summarize(&activity, &matcher(), now(), 14);
        assert_eq!(
            (s.fix_burst_group.clone(), s.fix_burst),
            (Some(FixGroup::File), 3)
        );
        assert_eq!(s.rework(), 2.0);
    }

    #[test]
    fn topic_fix_bursts_count_and_lose_ties() {
        let activity = CommitActivity {
            commits: vec![days_ago(3, "feat: tokens")],
            file_fixes: days(&[5, 1]),
            topic_fixes: topic(&[(6, None), (4, None), (2, None)]),
            ..Default::default()
        };
        let s = summarize(&activity, &matcher(), now(), 14);
        assert_eq!((s.fix_burst_group, s.fix_burst), (Some(FixGroup::Topic), 3));
        let tie = CommitActivity {
            topic_fixes: topic(&[(4, None), (2, None)]),
            ..activity
        };
        let s = summarize(&tie, &matcher(), now(), 14);
        assert_eq!(s.fix_burst_group, Some(FixGroup::File));
    }

    fn topic(fixes: &[(i64, Option<&str>)]) -> Vec<TopicFix> {
        fixes
            .iter()
            .map(|(n, scope)| TopicFix {
                date: now() - Duration::days(*n),
                scope: scope.map(String::from),
            })
            .collect()
    }

    #[test]
    fn same_scope_topic_fixes_corroborate_and_count_extra() {
        let activity = CommitActivity {
            commits: vec![days_ago(5, "feat(parser): tabs")],
            scope_fixes: BTreeMap::from([("parser".to_string(), days(&[4, 3, 1]))]),
            topic_fixes: topic(&[
                (4, Some("parser")),
                (3, Some("parser")),
                (40, Some("parser")),
            ]),
            ..Default::default()
        };
        let s = summarize(&activity, &matcher(), now(), 14);
        assert_eq!(s.fix_burst, 3);
        assert_eq!(s.corroborated_fixes, 2);
        assert_eq!(s.rework(), 2.0 + 2.0 * CORROBORATION_BONUS);
    }

    #[test]
    fn other_scope_topic_fixes_are_reported_and_count() {
        let activity = CommitActivity {
            commits: vec![days_ago(5, "feat(parser): tabs")],
            topic_fixes: topic(&[(4, Some("lexer")), (3, Some("lexer")), (2, None)]),
            ..Default::default()
        };
        let s = summarize(&activity, &matcher(), now(), 14);
        assert_eq!(
            s.cross_scope_fixes,
            BTreeMap::from([("lexer".to_string(), 2)])
        );
        assert_eq!(
            (s.fix_burst_group.clone(), s.fix_burst),
            (Some(FixGroup::Topic), 1)
        );
        assert_eq!(s.rework(), 2.0);
    }

    #[test]
    fn scoped_topic_fixes_join_the_topic_group_for_unscoped_functions() {
        let activity = CommitActivity {
            commits: vec![days_ago(5, "feat: tabs")],
            topic_fixes: topic(&[(4, Some("lexer")), (3, None)]),
            ..Default::default()
        };
        let s = summarize(&activity, &matcher(), now(), 14);
        assert!(s.cross_scope_fixes.is_empty());
        assert_eq!((s.fix_burst, s.corroborated_fixes), (2, 0));
    }

    #[test]
    fn no_recent_fixes_means_no_group() {
        let s = summary(vec![days_ago(3, "feat: tokens")]);
        assert_eq!((s.fix_burst_group, s.fix_burst), (None, 0));
    }

    #[test]
    fn rejects_out_of_range_topic_similarity() {
        let config = |min_similarity| ChurnConfig {
            topics: TopicConfig {
                enabled: true,
                min_similarity,
            },
            ..Default::default()
        };
        assert!(config(0.0).validate().is_err());
        assert!(config(1.5).validate().is_err());
        assert!(config(1.0).validate().is_ok());
    }

    #[test]
    fn proven_bonus_saturates() {
        let config = ChurnConfig::default();
        assert_eq!(config.proven_bonus(0.0), 0.0);
        assert!((config.proven_bonus(3.0) - 0.25).abs() < 1e-12);
        assert!(config.proven_bonus(1000.0) < config.proven_edit_bonus);
    }
}

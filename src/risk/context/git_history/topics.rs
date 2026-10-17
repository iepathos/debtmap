//! Opt-in topic matching between commit messages.
//!
//! Messages are reduced to stemmed, stopword-free terms, without the
//! conventional-commit type, trailers, or "Generated with" footers. Two
//! messages are related when the terms they share carry at least
//! `min_similarity` of the weight of all their terms (weighted Jaccard), with
//! each term weighted by its inverse document frequency across recent commits,
//! so rare words like "tokenizer" count more than common ones like "update".

use super::activity::{TopicFix, conventional_fix_scope};
use super::git2_provider::{FileCommitScan, is_bug_fix_message};
use bm25::{DefaultTokenizer, Language, Tokenizer};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};

type Terms = HashSet<String>;

/// Recent fix commits, indexed for topic matching.
pub struct TopicIndex {
    tokenizer: DefaultTokenizer,
    since: DateTime<Utc>,
    min_similarity: f64,
    fixes: Vec<(TopicFix, String, Terms)>,
    doc_freq: HashMap<String, usize>,
    docs: usize,
}

impl TopicIndex {
    /// Indexes the fix commits in `scans` made at or after `since`; term
    /// weights come from all commits made since then.
    pub fn new(scans: &[FileCommitScan], since: DateTime<Utc>, min_similarity: f64) -> Self {
        let tokenizer = DefaultTokenizer::new(Language::English);
        let recent: Vec<(&FileCommitScan, Terms)> = scans
            .iter()
            .filter(|s| s.date >= since)
            .map(|s| (s, terms(&tokenizer, &s.message)))
            .collect();
        let doc_freq = recent
            .iter()
            .flat_map(|(_, t)| t)
            .fold(HashMap::new(), |mut acc, t| {
                *acc.entry(t.clone()).or_insert(0) += 1;
                acc
            });
        let fixes = recent
            .iter()
            .filter(|(s, _)| is_bug_fix_message(&s.message))
            .map(|(s, t)| {
                let scope = conventional_fix_scope(&s.message).flatten();
                (
                    TopicFix {
                        date: s.date,
                        scope,
                    },
                    s.message.clone(),
                    t.clone(),
                )
            })
            .collect();
        Self {
            tokenizer,
            since,
            min_similarity,
            fixes,
            doc_freq,
            docs: recent.len(),
        }
    }

    /// Indexed fixes related to any of the recent `commits` (date, message).
    pub fn related_fixes<'a>(
        &self,
        commits: impl IntoIterator<Item = (DateTime<Utc>, &'a str)>,
    ) -> Vec<TopicFix> {
        let own = own_terms(self, commits);
        self.fixes
            .iter()
            .filter(|(fix, message, terms)| related_to(self, &own, fix, message, terms))
            .map(|(fix, _, _)| fix.clone())
            .collect()
    }

    fn similarity(&self, a: &Terms, b: &Terms) -> f64 {
        let union: f64 = a.union(b).map(|t| self.idf(t)).sum();
        let shared: f64 = a.intersection(b).map(|t| self.idf(t)).sum();
        if union > 0.0 { shared / union } else { 0.0 }
    }

    /// Smoothed IDF, always >= 1.
    fn idf(&self, term: &str) -> f64 {
        let df = self.doc_freq.get(term).copied().unwrap_or(0);
        ((1 + self.docs) as f64 / (1 + df) as f64).ln() + 1.0
    }
}

fn own_terms<'a>(
    index: &TopicIndex,
    commits: impl IntoIterator<Item = (DateTime<Utc>, &'a str)>,
) -> Vec<(DateTime<Utc>, &'a str, Terms)> {
    commits
        .into_iter()
        .filter(|(date, _)| *date >= index.since)
        .filter_map(|(date, message)| {
            let parsed = terms(&index.tokenizer, message);
            (!parsed.is_empty()).then_some((date, message, parsed))
        })
        .collect()
}

fn related_to(
    index: &TopicIndex,
    own: &[(DateTime<Utc>, &str, Terms)],
    fix: &TopicFix,
    message: &str,
    fix_terms: &Terms,
) -> bool {
    own.iter().any(|(date, own_message, own_terms)| {
        (*date != fix.date || *own_message != message)
            && index.similarity(own_terms, fix_terms) >= index.min_similarity
    })
}

fn terms(tokenizer: &DefaultTokenizer, message: &str) -> Terms {
    tokenizer
        .tokenize(&topic_text(message))
        .into_iter()
        .filter(|t| t.len() > 1 && t.chars().all(char::is_alphabetic))
        .collect()
}

/// Scope, description and body for conventional commits; otherwise the
/// message without trailers. "Generated with/by" footers are dropped.
fn topic_text(message: &str) -> String {
    let text = match git_conventional::Commit::parse(message.trim()) {
        Ok(c) => [
            c.scope().map(|s| s.as_str()),
            Some(c.description()),
            c.body(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n"),
        Err(_) => without_trailers(message),
    };
    text.lines()
        .filter(|line| {
            let line = line.to_lowercase();
            !line.contains("generated with") && !line.contains("generated by")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn without_trailers(message: &str) -> String {
    let trailers: Vec<String> = git2::message_trailers_strs(message)
        .map(|t| t.iter().map(|(k, v)| format!("{k}: {v}")).collect())
        .unwrap_or_default();
    message
        .lines()
        .filter(|line| !trailers.iter().any(|t| line.trim() == t))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-10-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn scan(days_ago: i64, message: &str) -> FileCommitScan {
        FileCommitScan {
            date: now() - Duration::days(days_ago),
            message: message.to_string(),
            author_email: String::new(),
            file_churn: HashMap::new(),
        }
    }

    fn index(scans: &[FileCommitScan]) -> TopicIndex {
        TopicIndex::new(scans, now() - Duration::days(14), 0.3)
    }

    #[test]
    fn strips_conventional_type_trailers_and_footers() {
        let tokenizer = DefaultTokenizer::new(Language::English);
        let t = terms(
            &tokenizer,
            "fix(parser): Handle tabs\n\nTabs were dropped.\n\n🤖 Generated with Claude Code\n\n\
             Co-authored-by: Claude <noreply@anthropic.com>",
        );
        assert_eq!(
            t,
            Terms::from(["parser", "handl", "tab", "drop"].map(String::from))
        );
    }

    #[test]
    fn related_fixes_share_rare_terms() {
        let scans = [
            scan(9, "feat: tokenizer handles tab characters"),
            scan(5, "fix: tokenizer drops tab characters at line start"),
            scan(4, "fix: update readme typo"),
            scan(3, "chore: update dependencies"),
            scan(2, "chore: update ci"),
            scan(1, "fix(Lexer): tokenizer crashes on tab characters"),
            scan(40, "fix: tokenizer tab characters again (too old)"),
        ];
        let index = index(&scans);
        let own = [(scans[0].date, scans[0].message.as_str())];
        let fix = |i: usize, scope: Option<&str>| TopicFix {
            date: scans[i].date,
            scope: scope.map(String::from),
        };
        assert_eq!(
            index.related_fixes(own),
            vec![fix(1, None), fix(5, Some("lexer"))]
        );
    }

    #[test]
    fn own_fix_is_not_reported_as_related() {
        let scans = [scan(2, "fix: tokenizer drops tab characters")];
        let own = [(scans[0].date, scans[0].message.as_str())];
        assert!(index(&scans).related_fixes(own).is_empty());
    }

    #[test]
    fn old_own_commits_do_not_define_the_topic() {
        let scans = [scan(5, "fix: tokenizer drops tab characters")];
        let old = now() - Duration::days(30);
        let own = [(old, "feat: tokenizer handles tab characters")];
        assert!(index(&scans).related_fixes(own).is_empty());
    }

    #[test]
    fn non_conventional_messages_drop_trailers() {
        let text = without_trailers("Fix tab handling\n\nCo-authored-by: Bot <b@x.io>");
        assert_eq!(text.trim(), "Fix tab handling");
    }
}

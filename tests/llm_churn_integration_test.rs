//! Recent LLM churn from a real git history with agent trailers.

mod common;

use common::git_fixture::{CLAUDE_TRAILER, CURSOR_TRAILER, GitFixture};
use debtmap::core::FunctionMetrics;
use debtmap::llm::{AgentMatcher, ChurnConfig, ChurnSummary, FixGroup, summarize};
use debtmap::risk::context::git_history::GitHistoryProvider;
use debtmap::risk::context::{AnalysisTarget, ContextDetails, ContextProvider};
use std::process::Command;

const FILE: &str = "src/lib.rs";

fn stable_helper() -> &'static str {
    "pub fn stable_helper(x: i32) -> i32 {\n    x + 1\n}\n"
}

fn human_fn(version: u32) -> String {
    format!(
        "pub fn human_fn(x: f64) -> f64 {{\n    let label = \"human_fn v{version}\";\n    \
         let _ = label;\n    x.round()\n}}\n"
    )
}

fn other_parser(version: u32) -> String {
    format!(
        "pub fn other_parser(s: &str) -> usize {{\n    let label = \"other_parser v{version}\";\n    \
         let _ = label;\n    s.len()\n}}\n"
    )
}

/// Nested branches so the analyzer reports it as a debt item.
fn hot_parser(version: u32) -> String {
    format!(
        r#"pub fn hot_parser(input: &str, strict: bool, depth: u32) -> Option<usize> {{
    let label = "hot_parser v{version}";
    let _ = label;
    if input.is_empty() {{
        return None;
    }}
    let mut total = 0;
    for (i, c) in input.chars().enumerate() {{
        if c == '\t' {{
            if strict {{
                if depth > 3 {{
                    return None;
                }} else if i % 2 == 0 {{
                    total += 2;
                }} else {{
                    total += 3;
                }}
            }} else {{
                total += 1;
            }}
        }} else if c.is_whitespace() {{
            if strict && depth > 1 {{
                total += 4;
            }}
        }} else {{
            total += 5;
        }}
    }}
    Some(total)
}}
"#
    )
}

fn source(parts: &[String]) -> String {
    parts.concat()
}

/// Timeline (days ago):
/// - 100: Claude adds `stable_helper` (never fixed: proven LLM edit)
/// - 90: a human adds `human_fn`
/// - 10: Claude adds `hot_parser` and `other_parser` (`feat(parser)`)
/// - 8: Cursor edits `hot_parser`
/// - 6: a human fixes `hot_parser` (`fix(parser)`)
/// - 4: Claude edits `hot_parser` again
/// - 3: a human fixes `other_parser` (`fix(parser)`)
/// - 2: a human fixes `human_fn` (scope-less `fix:`, no LLM involved)
/// - 1: Claude edits `human_fn`
/// - 0: a human reverts that edit (`git revert` message with the hash)
fn agent_repo() -> GitFixture {
    let helper = stable_helper().to_string();
    let repo = GitFixture::new();
    repo.commit(
        FILE,
        &source(std::slice::from_ref(&helper)),
        "feat: add stable_helper",
        Some(CLAUDE_TRAILER),
        100,
    )
    .commit(
        FILE,
        &source(&[helper.clone(), human_fn(1)]),
        "feat: add human_fn",
        None,
        90,
    );
    let version = |hot: u32, other: u32, human: u32| {
        source(&[
            helper.clone(),
            human_fn(human),
            other_parser(other),
            hot_parser(hot),
        ])
    };
    repo.commit(
        FILE,
        &version(1, 1, 1),
        "feat(parser): add parsers",
        Some(CLAUDE_TRAILER),
        10,
    )
    .commit(
        FILE,
        &version(2, 1, 1),
        "refactor(parser): simplify hot_parser",
        Some(CURSOR_TRAILER),
        8,
    )
    .commit(
        FILE,
        &version(3, 1, 1),
        "fix(parser): handle empty input",
        None,
        6,
    )
    .commit(
        FILE,
        &version(4, 1, 1),
        "feat(parser): support tabs",
        Some(CLAUDE_TRAILER),
        4,
    )
    .commit(
        FILE,
        &version(4, 2, 1),
        "fix(parser): other_parser edge case",
        None,
        3,
    )
    .commit(FILE, &version(4, 2, 2), "fix: human_fn rounding", None, 2)
    .commit(
        FILE,
        &version(4, 2, 3),
        "refactor: tidy human_fn",
        Some(CLAUDE_TRAILER),
        1,
    );
    let reverted = repo.head();
    repo.commit(
        FILE,
        &version(4, 2, 2),
        &format!("Revert \"refactor: tidy human_fn\"\n\nThis reverts commit {reverted}."),
        None,
        0,
    );
    repo
}

fn churn_for(repo: &GitFixture, provider: &GitHistoryProvider, function: &str) -> ChurnSummary {
    let target = AnalysisTarget {
        root_path: repo.path().to_path_buf(),
        file_path: repo.path().join(FILE),
        function_name: function.to_string(),
        line_range: (1, 40),
        reference_time: chrono::Utc::now(),
    };
    let context = provider.gather(&target).expect("git context");
    let ContextDetails::Historical { activity, .. } = context.details else {
        panic!("expected historical context for {function}");
    };
    let matcher = AgentMatcher::new(&ChurnConfig::default()).expect("builtin agents");
    summarize(&activity, &matcher, chrono::Utc::now(), 14)
}

#[test]
fn git_history_yields_llm_churn_per_function() {
    let repo = agent_repo();
    let mut provider = GitHistoryProvider::new(repo.path().to_path_buf()).expect("git repo");
    let metrics: Vec<FunctionMetrics> = ["stable_helper", "human_fn", "other_parser", "hot_parser"]
        .into_iter()
        .map(|name| FunctionMetrics::new(name.into(), repo.path().join(FILE), 1))
        .collect();
    provider
        .preload_function_histories(&metrics)
        .expect("preload histories");

    let hot = churn_for(&repo, &provider, "hot_parser");
    assert_eq!(hot.llm_edits, 3, "{hot:?}");
    assert_eq!(hot.fixes_after_llm_edit, 1, "{hot:?}");
    assert_eq!(
        (hot.fix_burst_group.clone(), hot.fix_burst),
        (Some(FixGroup::Scope("parser".into())), 2),
        "{hot:?}"
    );
    assert_eq!(hot.reverts_of_llm_edits, 0, "{hot:?}");
    assert_eq!(hot.rework(), 4.0);
    assert_eq!(hot.proven_llm_edits, 0);

    let stable = churn_for(&repo, &provider, "stable_helper");
    assert_eq!((stable.llm_edits, stable.rework()), (0, 0.0), "{stable:?}");
    assert_eq!(stable.proven_llm_edits, 1, "{stable:?}");

    let reverted = churn_for(&repo, &provider, "human_fn");
    assert_eq!(reverted.llm_edits, 1, "{reverted:?}");
    assert_eq!(reverted.fixes_after_llm_edit, 0, "{reverted:?}");
    assert_eq!(reverted.reverts_of_llm_edits, 1, "{reverted:?}");
    assert_eq!(
        (reverted.fix_burst_group.clone(), reverted.fix_burst),
        (Some(FixGroup::File), 1),
        "the scope-less fix is grouped under the file: {reverted:?}"
    );
    assert_eq!(reverted.rework(), 1.0, "{reverted:?}");
}

/// `llm_editability` of `hot_parser` from `debtmap analyze --format json`.
fn analyze_hot_parser(repo: &GitFixture, config_toml: &str) -> serde_json::Value {
    let config = repo.path().join("llm.toml");
    std::fs::write(&config, config_toml).expect("write config");
    let output = Command::new(env!("CARGO_BIN_EXE_debtmap"))
        .args([
            "analyze",
            ".",
            "--context",
            "--min-score",
            "0",
            "--format",
            "json",
        ])
        .env("DEBTMAP_CONFIG", &config)
        .current_dir(repo.path())
        .output()
        .expect("run debtmap");
    assert!(
        output.status.success(),
        "debtmap failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("analyze emits JSON");
    let items = json["items"].as_array().expect("items array");
    let hot = items
        .iter()
        .find(|i| i["location"]["function"] == "hot_parser")
        .unwrap_or_else(|| panic!("hot_parser not reported: {json:#}"));
    hot["llm_editability"].clone()
}

#[test]
fn analyze_json_reports_churn_and_rework_action() {
    let llm = analyze_hot_parser(&agent_repo(), "[llm_editability]\nenabled = true\n");
    assert_eq!(llm["churn"]["llm_edits"], 3, "{llm:#}");
    assert_eq!(
        llm["churn"]["fix_burst_group"]["scope"], "parser",
        "{llm:#}"
    );
    assert_eq!(llm["features"]["rework"], 4.0, "{llm:#}");
    let actions = llm["actions"].as_array().expect("actions array");
    assert!(
        actions.iter().any(|a| a["feature"] == "rework"
            && a["text"]
                .as_str()
                .is_some_and(|t| t.contains("3 LLM edits"))),
        "{llm:#}"
    );
}

/// An LLM edit to `hot_parser` under `parser`, then human fixes on the same
/// topic in another file (one under `parser`, one under `lexer`, one
/// scope-less), and an unrelated fix.
fn topic_repo() -> GitFixture {
    let repo = GitFixture::new();
    let tabs = "src/tabs.rs";
    repo.commit(
        FILE,
        &hot_parser(1),
        "feat(parser): parse tab characters in parser input",
        Some(CLAUDE_TRAILER),
        6,
    )
    .commit(
        tabs,
        "// v1\n",
        "fix(parser): tab characters miscounted in parser input",
        None,
        4,
    )
    .commit(
        tabs,
        "// v2\n",
        "fix(lexer): parser input with tab characters crashes",
        None,
        3,
    )
    .commit(
        tabs,
        "// v3\n",
        "fix: tab characters in parser input lose their column",
        None,
        2,
    )
    .commit("README.md", "typo fixed\n", "fix: readme typo", None, 1);
    repo
}

#[test]
fn topic_matching_corroborates_scopes_and_reports_cross_scope_fixes() {
    let repo = topic_repo();
    let off = analyze_hot_parser(&repo, "[llm_editability]\nenabled = true\n");
    assert_eq!(
        off["churn"]["fix_burst_group"]["scope"], "parser",
        "{off:#}"
    );
    assert_eq!(off["churn"]["corroborated_fixes"], 0, "{off:#}");
    assert_eq!(off["features"]["rework"], 0.0, "{off:#}");

    let on = analyze_hot_parser(
        &repo,
        "[llm_editability]\nenabled = true\n[llm_editability.churn.topics]\nenabled = true\n",
    );
    let churn = &on["churn"];
    assert_eq!(churn["fix_burst_group"], "topic", "{on:#}");
    assert_eq!(churn["fix_burst"], 2, "same-scope and scope-less: {on:#}");
    assert_eq!(churn["corroborated_fixes"], 1, "{on:#}");
    assert_eq!(churn["cross_scope_fixes"]["lexer"], 1, "{on:#}");
    assert_eq!(on["features"]["rework"], 2.5, "1 + 0.5 + 1: {on:#}");
    let rework = on["actions"]
        .as_array()
        .and_then(|a| a.iter().find(|a| a["feature"] == "rework"))
        .unwrap_or_else(|| panic!("no rework action: {on:#}"));
    assert!(
        rework["text"]
            .as_str()
            .is_some_and(|t| t.contains("under other scopes (lexer: 1)")),
        "{on:#}"
    );
}

/// Claude adds and then edits `hot_parser`; a human silently restores the
/// previous version without saying "revert".
fn silent_revert_repo() -> GitFixture {
    let repo = GitFixture::new();
    repo.commit(
        FILE,
        &hot_parser(1),
        "feat: add hot_parser",
        Some(CLAUDE_TRAILER),
        6,
    )
    .commit(
        FILE,
        &hot_parser(2),
        "perf: faster tab handling",
        Some(CLAUDE_TRAILER),
        4,
    )
    .commit(
        FILE,
        &hot_parser(1),
        "Restore previous tab handling",
        None,
        2,
    );
    repo
}

#[test]
fn content_reverts_detect_silent_reverts_when_enabled() {
    let repo = silent_revert_repo();
    let off = analyze_hot_parser(&repo, "[llm_editability]\nenabled = true\n");
    assert_eq!(off["churn"]["reverts_of_llm_edits"], 0, "{off:#}");
    assert_eq!(off["features"]["rework"], 1.0, "{off:#}");

    let on = analyze_hot_parser(
        &repo,
        "[llm_editability]\nenabled = true\n[llm_editability.churn]\ncontent_reverts = true\n",
    );
    assert_eq!(on["churn"]["reverts_of_llm_edits"], 1, "{on:#}");
    assert_eq!(on["features"]["rework"], 2.0, "{on:#}");
}

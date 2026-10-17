//! Temporary git repositories with backdated, optionally agent-co-authored commits.

use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

pub const CLAUDE_TRAILER: &str = "Co-authored-by: Claude <noreply@anthropic.com>";
pub const CURSOR_TRAILER: &str = "Co-authored-by: Cursor <cursoragent@cursor.com>";

pub struct GitFixture {
    dir: TempDir,
}

impl GitFixture {
    pub fn new() -> Self {
        let fixture = Self {
            dir: TempDir::new().expect("create temp dir"),
        };
        fixture.git(&["init", "-q", "-b", "main"], None);
        fixture.git(&["config", "user.email", "dev@example.com"], None);
        fixture.git(&["config", "user.name", "Dev Human"], None);
        let no_hooks = fixture.path().join(".git").join("no-hooks");
        std::fs::create_dir(&no_hooks).expect("create empty hooks dir");
        let no_hooks = no_hooks.to_str().expect("temp path is UTF-8");
        fixture.git(&["config", "core.hooksPath", no_hooks], None);
        fixture
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Write `content` to `file` and commit it `days_ago` days in the past.
    /// `trailer` (e.g. [`CLAUDE_TRAILER`]) is appended as a git trailer.
    pub fn commit(
        &self,
        file: &str,
        content: &str,
        message: &str,
        trailer: Option<&str>,
        days_ago: i64,
    ) -> &Self {
        let path = self.path().join(file);
        std::fs::create_dir_all(path.parent().expect("file has a parent")).expect("mkdir");
        std::fs::write(&path, content).expect("write fixture file");
        self.git(&["add", file], None);
        let message = trailer
            .map(|t| format!("{message}\n\n{t}"))
            .unwrap_or_else(|| message.to_string());
        let date = chrono::Utc::now() - chrono::Duration::days(days_ago);
        let date = format!("@{} +0000", date.timestamp());
        self.git(
            &[
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-q",
                "--no-verify",
                "-m",
                &message,
            ],
            Some(&date),
        );
        self
    }

    /// Full hash of the latest commit.
    pub fn head(&self) -> String {
        self.git(&["rev-parse", "HEAD"], None).trim().to_string()
    }

    fn git(&self, args: &[&str], date: Option<&str>) -> String {
        let mut cmd = Command::new("git");
        cmd.args(args).current_dir(self.path());
        if let Some(date) = date {
            cmd.env("GIT_AUTHOR_DATE", date)
                .env("GIT_COMMITTER_DATE", date);
        }
        let output = cmd.output().expect("run git");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }
}

use anyhow::Result;
use std::io::Write;

#[derive(Debug, Clone)]
pub struct TocEntry {
    pub level: usize,
    pub title: String,
    pub anchor: String,
}

pub struct TocBuilder {
    entries: Vec<TocEntry>,
}

impl TocBuilder {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add_entry(&mut self, level: usize, title: &str) {
        let anchor = title
            .to_lowercase()
            .replace(' ', "-")
            .replace(['(', ')', '[', ']', '{', '}', '/', '\\'], "");

        self.entries.push(TocEntry {
            level,
            title: title.to_string(),
            anchor,
        });
    }

    pub fn write_toc<W: Write>(&self, writer: &mut W, max_depth: usize) -> Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }

        writeln!(writer, "## Table of Contents\n")?;

        for entry in &self.entries {
            if entry.level <= max_depth {
                let indent = "  ".repeat(entry.level.saturating_sub(1));
                writeln!(writer, "{}- [{}](#{})", indent, entry.title, entry.anchor)?;
            }
        }

        writeln!(writer)?;
        Ok(())
    }

    pub fn entries(&self) -> &[TocEntry] {
        &self.entries
    }
}

impl Default for TocBuilder {
    fn default() -> Self {
        Self::new()
    }
}

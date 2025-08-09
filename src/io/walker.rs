use crate::core::Language;
use anyhow::Result;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

pub struct FileWalker {
    root: PathBuf,
    languages: Vec<Language>,
    ignore_patterns: Vec<String>,
}

impl FileWalker {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            languages: vec![Language::Rust, Language::Python],
            ignore_patterns: vec![],
        }
    }

    pub fn with_languages(mut self, languages: Vec<Language>) -> Self {
        self.languages = languages;
        self
    }

    pub fn with_ignore_patterns(mut self, patterns: Vec<String>) -> Self {
        self.ignore_patterns = patterns;
        self
    }

    pub fn walk(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let walker = WalkBuilder::new(&self.root)
            .hidden(false)
            .git_ignore(true)
            .build();

        for entry in walker {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && self.should_process(path) {
                files.push(path.to_path_buf());
            }
        }

        Ok(files)
    }

    fn should_process(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy();
            let lang = Language::from_extension(&ext_str);

            if !self.languages.contains(&lang) {
                return false;
            }

            let path_str = path.to_string_lossy();
            for pattern in &self.ignore_patterns {
                if glob::Pattern::new(pattern)
                    .map(|p| p.matches(&path_str))
                    .unwrap_or(false)
                {
                    return false;
                }
            }

            true
        } else {
            false
        }
    }
}

pub fn find_project_files(root: &Path, languages: Vec<Language>) -> Result<Vec<PathBuf>> {
    FileWalker::new(root.to_path_buf())
        .with_languages(languages)
        .walk()
}

pub fn count_lines(path: &Path) -> Result<usize> {
    let content = std::fs::read_to_string(path)?;
    Ok(content.lines().count())
}

pub fn get_file_size(path: &Path) -> Result<u64> {
    let metadata = std::fs::metadata(path)?;
    Ok(metadata.len())
}

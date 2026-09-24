//! Independently locate expected definitions and calls in fixture source text.
use super::Expectation;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Location {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
}

pub struct Oracle {
    pub definitions: BTreeMap<String, Location>,
    pub sites: Vec<Location>,
}

impl Oracle {
    pub fn new(files: &[(PathBuf, String)], expectations: &[Expectation]) -> Self {
        let mut definitions = BTreeMap::new();
        for (file, source) in files {
            assert!(source.is_ascii(), "Matrix locations use ASCII fixtures");
            for (offset, _) in source.match_indices("/*@") {
                let rest = &source[offset + 3..];
                let end = rest.find("*/").expect("closed definition marker");
                let label = &rest[..end];
                let tail = offset + 3 + end + 2;
                let ident = tail + source[tail..].find("fn ").expect("marked fn") + 3;
                assert!(
                    definitions
                        .insert(label.into(), locate(file, source, ident))
                        .is_none(),
                    "duplicate label {label}"
                );
            }
        }
        let sites = expectations
            .iter()
            .map(|expected| site(files, expected))
            .collect();
        Self { definitions, sites }
    }
}

fn site(files: &[(PathBuf, String)], expected: &Expectation) -> Location {
    let marker = format!("/*#{}*/", expected.site);
    let matches: Vec<_> = files
        .iter()
        .flat_map(|(file, source)| {
            source
                .match_indices(&marker)
                .map(move |(offset, _)| (file, source, offset))
        })
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "site marker {} must be unique",
        expected.site
    );
    let (file, source, offset) = matches[0];
    let tail = offset + marker.len();
    let token = tail + source[tail..].find(expected.token).expect("site token");
    locate(file, source, token)
}

fn locate(file: &std::path::Path, source: &str, offset: usize) -> Location {
    let prefix = &source[..offset];
    Location {
        file: file.to_path_buf(),
        line: prefix.bytes().filter(|byte| *byte == b'\n').count() + 1,
        column: prefix
            .rfind('\n')
            .map(|line| offset - line - 1)
            .unwrap_or(offset),
    }
}

/// Configuration for enhanced markdown output
#[derive(Debug, Clone)]
pub struct MarkdownConfig {
    pub include_toc: bool,
    pub toc_depth: usize,
    pub include_visualizations: bool,
    pub include_code_snippets: bool,
    pub snippet_context_lines: usize,
    pub repository_type: RepositoryType,
    pub base_url: Option<String>,
    pub detail_level: DetailLevel,
    pub include_statistics: bool,
    pub collapsible_sections: bool,
}

impl Default for MarkdownConfig {
    fn default() -> Self {
        Self {
            include_toc: true,
            toc_depth: 3,
            include_visualizations: true,
            include_code_snippets: false, // Disabled by default for now
            snippet_context_lines: 3,
            repository_type: RepositoryType::Git,
            base_url: None,
            detail_level: DetailLevel::Standard,
            include_statistics: true,
            collapsible_sections: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum DetailLevel {
    Summary,  // Executive summary only
    Standard, // Default level with key sections
    Detailed, // All sections with expanded information
    Complete, // Everything including raw data
}

#[derive(Debug, Clone, PartialEq)]
pub enum RepositoryType {
    Git,
    GitHub,
    GitLab,
    Bitbucket,
    Custom(String),
}

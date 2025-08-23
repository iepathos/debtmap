
fn count_lines(block: &syn::Block) -> usize {
    // Simple approximation based on statement count
    block.stmts.len().max(1)
}

pub fn extract_call_graph(file: &syn::File) -> CallGraph {
    CallGraph::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_lines() {
        let result = count_lines(&block);
        assert_eq!(result, 1);
    }
}

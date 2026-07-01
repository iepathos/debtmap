//! Type-aware Solidity cross-contract call graph resolution.

use crate::analyzers::solidity::calls::{SolidityCallKind, SolidityCallShape, extract_calls};
use crate::analyzers::solidity::parser::{node_text, parse_source};
use crate::core::Language;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tree_sitter::Node;

#[derive(Debug, Clone)]
pub struct SolidityBatchSnapshot {
    pub path: PathBuf,
    pub language: Language,
    pub functions: Vec<String>,
}

pub fn compute_call_edges(
    snapshots: &[SolidityBatchSnapshot],
) -> HashMap<(PathBuf, String), Vec<String>> {
    let index = SolidityCallIndex::from_snapshots(snapshots);
    let mut edges = HashMap::new();

    for snapshot in snapshots
        .iter()
        .filter(|snapshot| snapshot.language == Language::Solidity)
    {
        let Some(caller_contract) = index.contract_for_file(&snapshot.path) else {
            continue;
        };

        for function in &snapshot.functions {
            let downstream: Vec<_> = index
                .calls_for_function(&snapshot.path, function)
                .iter()
                .filter_map(|call| {
                    index.resolve_call(call, &caller_contract, &snapshot.path, function)
                })
                .collect();

            if !downstream.is_empty() {
                edges.insert((snapshot.path.clone(), function.clone()), downstream);
            }
        }
    }

    edges
}

#[derive(Debug, Default)]
struct SolidityCallIndex {
    contracts: HashMap<String, ContractRecord>,
    functions_by_contract_method: HashMap<(String, String), Vec<String>>,
    import_symbols: HashMap<PathBuf, HashMap<String, String>>,
    file_contracts: HashMap<PathBuf, String>,
    function_calls: HashMap<(PathBuf, String), Vec<SolidityCallShape>>,
}

#[derive(Debug, Clone)]
struct ContractRecord {
    name: String,
    bases: Vec<String>,
    state_variables: HashMap<String, String>,
}

impl SolidityCallIndex {
    fn from_snapshots(snapshots: &[SolidityBatchSnapshot]) -> Self {
        let mut index = Self::default();
        index.seed_functions(snapshots);

        for snapshot in snapshots
            .iter()
            .filter(|snapshot| snapshot.language == Language::Solidity)
        {
            let Ok(content) = std::fs::read_to_string(&snapshot.path) else {
                continue;
            };
            let Ok(ast) = parse_source(&content, &snapshot.path) else {
                continue;
            };
            index.ingest_ast(&ast);
        }

        index
    }

    fn seed_functions(&mut self, snapshots: &[SolidityBatchSnapshot]) {
        for snapshot in snapshots
            .iter()
            .filter(|snapshot| snapshot.language == Language::Solidity)
        {
            for function in &snapshot.functions {
                let (contract, method) = split_qualified_name(function);
                push_function(
                    &mut self.functions_by_contract_method,
                    contract,
                    method,
                    function.clone(),
                );
            }
        }
    }

    fn ingest_ast(&mut self, ast: &crate::core::ast::SolidityAst) {
        let imports = import_symbols(ast);
        self.import_symbols.insert(ast.path.clone(), imports);
        ingest_contracts(ast.tree.root_node(), ast, self);
        ingest_function_calls(ast.tree.root_node(), ast, None, self);
    }

    fn contract_for_file(&self, file: &Path) -> Option<String> {
        self.file_contracts.get(file).cloned()
    }

    fn calls_for_function(&self, file: &Path, qualified_name: &str) -> Vec<SolidityCallShape> {
        self.function_calls
            .get(&(file.to_path_buf(), qualified_name.to_string()))
            .cloned()
            .unwrap_or_default()
    }

    fn resolve_call(
        &self,
        call: &SolidityCallShape,
        caller_contract: &str,
        caller_file: &Path,
        caller_name: &str,
    ) -> Option<String> {
        match call.kind {
            SolidityCallKind::Bare => {
                self.resolve_bare(&call.method_name, caller_contract, caller_file)
            }
            SolidityCallKind::Selector => self.resolve_selector(call, caller_contract, caller_file),
            SolidityCallKind::TypeCast => self.resolve_cast(call, caller_file),
        }
        .filter(|target| target != caller_name)
    }

    fn resolve_bare(
        &self,
        method: &str,
        caller_contract: &str,
        caller_file: &Path,
    ) -> Option<String> {
        self.resolve_in_contract_or_bases(caller_contract, method, caller_file)
            .or_else(|| self.unambiguous_bare(method))
    }

    fn resolve_selector(
        &self,
        call: &SolidityCallShape,
        caller_contract: &str,
        caller_file: &Path,
    ) -> Option<String> {
        let receiver = call.receiver.as_deref()?;
        let receiver_type = self.receiver_type(receiver, caller_contract, caller_file)?;
        self.resolve_in_contract_or_bases(&receiver_type, &call.method_name, caller_file)
    }

    fn resolve_cast(&self, call: &SolidityCallShape, caller_file: &Path) -> Option<String> {
        let cast_type = call.cast_type.as_deref()?;
        let contract = self.resolve_type_name(cast_type, caller_file);
        self.resolve_in_contract_or_bases(&contract, &call.method_name, caller_file)
    }

    fn receiver_type(
        &self,
        receiver: &str,
        caller_contract: &str,
        caller_file: &Path,
    ) -> Option<String> {
        if receiver == "this" {
            return Some(caller_contract.to_string());
        }

        self.contracts
            .get(caller_contract)
            .and_then(|contract| contract.state_variables.get(receiver))
            .map(|type_name| self.resolve_type_name(type_name, caller_file))
    }

    fn resolve_type_name(&self, type_name: &str, file: &Path) -> String {
        self.import_symbols
            .get(file)
            .and_then(|symbols| symbols.get(type_name).cloned())
            .or_else(|| {
                self.contracts
                    .get(type_name)
                    .map(|contract| contract.name.clone())
            })
            .unwrap_or_else(|| type_name.to_string())
    }

    fn resolve_in_contract_or_bases(
        &self,
        contract: &str,
        method: &str,
        prefer_file: &Path,
    ) -> Option<String> {
        self.pick_function(contract, method, prefer_file)
            .or_else(|| {
                self.contracts.get(contract).and_then(|record| {
                    record.bases.iter().find_map(|base| {
                        self.resolve_in_contract_or_bases(base, method, prefer_file)
                    })
                })
            })
    }

    fn pick_function(&self, contract: &str, method: &str, prefer_file: &Path) -> Option<String> {
        let matches = self
            .functions_by_contract_method
            .get(&(contract.to_string(), method.to_string()))?;

        prefer_same_file_contract(matches, contract, prefer_file).or_else(|| unique_match(matches))
    }

    fn unambiguous_bare(&self, method: &str) -> Option<String> {
        let matches = self
            .functions_by_contract_method
            .iter()
            .filter(|((_, name), _)| name == method)
            .flat_map(|(_, qualified)| qualified.clone())
            .collect::<Vec<_>>();
        unique_match(&matches)
    }
}

fn prefer_same_file_contract(matches: &[String], contract: &str, file: &Path) -> Option<String> {
    let contract_file = matches
        .iter()
        .find(|qualified| qualified.starts_with(&format!("{contract}.")))
        .cloned();
    contract_file.filter(|_| {
        file.to_string_lossy()
            .to_ascii_lowercase()
            .contains(&contract.to_ascii_lowercase())
    })
}

fn split_qualified_name(name: &str) -> (String, String) {
    name.rsplit_once('.')
        .map(|(contract, method)| (contract.to_string(), method.to_string()))
        .unwrap_or_else(|| (String::new(), name.to_string()))
}

fn push_function(
    index: &mut HashMap<(String, String), Vec<String>>,
    contract: String,
    method: String,
    qualified: String,
) {
    let entry = index.entry((contract, method)).or_default();
    if !entry.contains(&qualified) {
        entry.push(qualified);
    }
}

fn unique_match(matches: &[String]) -> Option<String> {
    let mut unique = matches.to_vec();
    unique.sort();
    unique.dedup();
    match unique.len() {
        1 => Some(unique[0].clone()),
        _ => None,
    }
}

fn ingest_contracts(
    node: Node,
    ast: &crate::core::ast::SolidityAst,
    index: &mut SolidityCallIndex,
) {
    if let Some(record) = contract_record(node, ast) {
        index
            .file_contracts
            .entry(ast.path.clone())
            .or_insert_with(|| record.name.clone());
        index.contracts.insert(record.name.clone(), record);
    }

    walk_children(node, |child| ingest_contracts(child, ast, index));
}

fn contract_record(node: Node, ast: &crate::core::ast::SolidityAst) -> Option<ContractRecord> {
    let kind = node.kind();
    if !matches!(
        kind,
        "contract_declaration" | "interface_declaration" | "library_declaration"
    ) {
        return None;
    }

    let name = node_text(&node.child_by_field_name("name")?, &ast.source).to_string();
    Some(ContractRecord {
        name: name.clone(),
        bases: inheritance_names(node, ast),
        state_variables: state_variables(node, ast),
    })
}

fn ingest_function_calls(
    node: Node,
    ast: &crate::core::ast::SolidityAst,
    contract_name: Option<String>,
    index: &mut SolidityCallIndex,
) {
    let current_contract = contract_name.or_else(|| contract_name_from_node(node, ast));

    if node.kind() == "function_definition" {
        if let Some(contract) = current_contract.clone() {
            let method = node
                .child_by_field_name("name")
                .map(|name| node_text(&name, &ast.source).to_string())
                .unwrap_or_else(|| "function".to_string());
            let qualified = format!("{contract}.{method}");
            push_function(
                &mut index.functions_by_contract_method,
                contract.clone(),
                method.clone(),
                qualified.clone(),
            );

            if let Some(body) = node.child_by_field_name("body") {
                index
                    .function_calls
                    .insert((ast.path.clone(), qualified), extract_calls(body, ast));
            }
        }
    }

    let next_contract = match node.kind() {
        "contract_declaration" | "interface_declaration" | "library_declaration" => node
            .child_by_field_name("name")
            .map(|name| node_text(&name, &ast.source).to_string())
            .or(current_contract),
        _ => current_contract,
    };

    walk_children(node, |child| {
        ingest_function_calls(child, ast, next_contract.clone(), index)
    });
}

fn import_symbols(ast: &crate::core::ast::SolidityAst) -> HashMap<String, String> {
    let mut symbols = HashMap::new();
    collect_import_symbols(ast.tree.root_node(), ast, &mut symbols);
    symbols
}

fn collect_import_symbols(
    node: Node,
    ast: &crate::core::ast::SolidityAst,
    symbols: &mut HashMap<String, String>,
) {
    if node.kind() == "import_directive" {
        symbols.extend(import_directive_symbols(node, ast));
    }
    walk_children(node, |child| collect_import_symbols(child, ast, symbols));
}

fn import_directive_symbols(
    node: Node,
    ast: &crate::core::ast::SolidityAst,
) -> HashMap<String, String> {
    let mut symbols = HashMap::new();
    let names = import_names(node, ast);
    let aliases = import_aliases(node, ast);

    for (index, name) in names.iter().enumerate() {
        let alias = aliases.get(index).cloned().unwrap_or_else(|| name.clone());
        symbols.insert(alias, name.clone());
    }

    if symbols.is_empty() {
        if let Some(source) = import_source_contract(node, ast) {
            symbols.insert(source.clone(), source);
        }
    }

    symbols
}

fn import_names(node: Node, ast: &crate::core::ast::SolidityAst) -> Vec<String> {
    import_field_identifiers(node, ast, "import_name")
}

fn import_aliases(node: Node, ast: &crate::core::ast::SolidityAst) -> Vec<String> {
    import_field_identifiers(node, ast, "alias")
}

fn import_field_identifiers(
    node: Node,
    ast: &crate::core::ast::SolidityAst,
    field: &str,
) -> Vec<String> {
    node.child_by_field_name(field)
        .map(|field_node| {
            let mut cursor = field_node.walk();
            field_node
                .children(&mut cursor)
                .filter(|child| child.kind() == "identifier")
                .map(|child| node_text(&child, &ast.source).to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn import_source_contract(node: Node, ast: &crate::core::ast::SolidityAst) -> Option<String> {
    let source = node.child_by_field_name("source")?;
    let path = node_text(&source, &ast.source).trim_matches('"');
    Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::to_string)
}

fn state_variables(node: Node, ast: &crate::core::ast::SolidityAst) -> HashMap<String, String> {
    let mut variables = HashMap::new();
    collect_state_variables(node, ast, &mut variables);
    variables
}

fn collect_state_variables(
    node: Node,
    ast: &crate::core::ast::SolidityAst,
    variables: &mut HashMap<String, String>,
) {
    if node.kind() == "state_variable_declaration" {
        if let (Some(name), Some(type_node)) = (
            node.child_by_field_name("name"),
            node.child_by_field_name("type"),
        ) {
            variables.insert(
                node_text(&name, &ast.source).to_string(),
                node_text(&type_node, &ast.source).trim().to_string(),
            );
        }
    }

    walk_children(node, |child| collect_state_variables(child, ast, variables));
}

fn inheritance_names(node: Node, ast: &crate::core::ast::SolidityAst) -> Vec<String> {
    let mut names = Vec::new();
    walk_children(node, |child| {
        if child.kind() == "inheritance_specifier" {
            if let Some(name) = child
                .child_by_field_name("ancestor")
                .or_else(|| child.child_by_field_name("name"))
            {
                names.push(node_text(&name, &ast.source).trim().to_string());
            }
        }
    });
    names
}

fn contract_name_from_node(node: Node, ast: &crate::core::ast::SolidityAst) -> Option<String> {
    let mut current = node.parent();
    while let Some(parent) = current {
        if matches!(
            parent.kind(),
            "contract_declaration" | "interface_declaration" | "library_declaration"
        ) {
            return parent
                .child_by_field_name("name")
                .map(|name| node_text(&name, &ast.source).to_string());
        }
        current = parent.parent();
    }
    None
}

fn walk_children(node: Node, mut visit: impl FnMut(Node)) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::solidity::parser::parse_source;
    use crate::config::SolidityLanguageConfig;
    use std::fs;
    use tempfile::TempDir;

    fn snapshot_fixture(files: &[(&str, &str)]) -> (TempDir, Vec<SolidityBatchSnapshot>) {
        let temp = TempDir::new().unwrap();
        let snapshots = files
            .iter()
            .map(|(path, content)| {
                let file_path = temp.path().join(path);
                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent).unwrap();
                }
                fs::write(&file_path, content).unwrap();
                let ast = parse_source(content, &file_path).unwrap();
                let metrics = crate::analyzers::solidity::orchestration::analyze_solidity_file(
                    &ast,
                    10,
                    &SolidityLanguageConfig::default(),
                );
                SolidityBatchSnapshot {
                    path: file_path,
                    language: Language::Solidity,
                    functions: metrics
                        .complexity
                        .functions
                        .iter()
                        .map(|function| function.name.clone())
                        .collect(),
                }
            })
            .collect();
        (temp, snapshots)
    }

    #[test]
    fn test_same_contract_internal_call_resolves() {
        let (_temp, snapshots) = snapshot_fixture(&[(
            "Vault.sol",
            r#"pragma solidity 0.8.20;
contract Vault {
    function withdraw() public { _settle(); }
    function _settle() internal {}
}"#,
        )]);

        let edges = compute_call_edges(&snapshots);
        assert_eq!(
            edges.get(&(snapshots[0].path.clone(), "Vault.withdraw".to_string())),
            Some(&vec!["Vault._settle".to_string()])
        );
    }

    #[test]
    fn test_inherited_call_resolves_to_base_contract() {
        let (_temp, snapshots) = snapshot_fixture(&[
            (
                "Settlement.sol",
                r#"pragma solidity 0.8.20;
contract Settlement { function settle() public {} }"#,
            ),
            (
                "Vault.sol",
                r#"pragma solidity 0.8.20;
import "./Settlement.sol";
contract Vault is Settlement {
    function withdraw() public { settle(); }
}"#,
            ),
        ]);

        let vault = &snapshots[1];
        let edges = compute_call_edges(&snapshots);
        assert_eq!(
            edges.get(&(vault.path.clone(), "Vault.withdraw".to_string())),
            Some(&vec!["Settlement.settle".to_string()])
        );
    }

    #[test]
    fn test_selector_call_uses_state_variable_type() {
        let (_temp, snapshots) = snapshot_fixture(&[
            (
                "Settlement.sol",
                r#"pragma solidity 0.8.20;
contract Settlement { function settle() public {} }"#,
            ),
            (
                "Vault.sol",
                r#"pragma solidity 0.8.20;
import "./Settlement.sol";
contract Vault {
    Settlement settlement;
    function withdraw() public { settlement.settle(); }
}"#,
            ),
        ]);

        let vault = &snapshots[1];
        let edges = compute_call_edges(&snapshots);
        assert_eq!(
            edges.get(&(vault.path.clone(), "Vault.withdraw".to_string())),
            Some(&vec!["Settlement.settle".to_string()])
        );
    }

    #[test]
    fn test_interface_cast_call_resolves() {
        let (_temp, snapshots) = snapshot_fixture(&[
            (
                "IERC20.sol",
                r#"pragma solidity 0.8.20;
interface IERC20 { function transfer(address to, uint256 amount) external returns (bool); }"#,
            ),
            (
                "Vault.sol",
                r#"pragma solidity 0.8.20;
import "./IERC20.sol";
contract Vault {
    function payout(address token, address to, uint256 amount) public {
        IERC20(token).transfer(to, amount);
    }
}"#,
            ),
        ]);

        let vault = &snapshots[1];
        let edges = compute_call_edges(&snapshots);
        assert_eq!(
            edges.get(&(vault.path.clone(), "Vault.payout".to_string())),
            Some(&vec!["IERC20.transfer".to_string()]),
            "snapshots: {:?}, edges: {:?}",
            snapshots,
            edges
        );
    }

    #[test]
    fn test_ambiguous_bare_call_stays_unresolved() {
        let (_temp, snapshots) = snapshot_fixture(&[
            (
                "Alpha.sol",
                r#"pragma solidity 0.8.20;
contract Alpha { function run() public {} }"#,
            ),
            (
                "Beta.sol",
                r#"pragma solidity 0.8.20;
contract Beta { function run() public {} }"#,
            ),
            (
                "Caller.sol",
                r#"pragma solidity 0.8.20;
contract Caller {
    function dispatch() public { run(); }
}"#,
            ),
        ]);

        let caller = &snapshots[2];
        let edges = compute_call_edges(&snapshots);
        assert!(
            edges
                .get(&(caller.path.clone(), "Caller.dispatch".to_string()))
                .is_none()
        );
    }
}

use rustpython_parser::ast;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedSymbol {
    pub module: String,
    pub name: String,
    pub alias: Option<String>,
    pub is_wildcard: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportedSymbol {
    pub name: String,
    pub qualified_name: String,
    pub is_class: bool,
    pub is_function: bool,
}

#[derive(Debug, Default, Clone)]
pub struct ImportTracker {
    imports: Vec<ImportedSymbol>,
    #[allow(dead_code)]
    module_path: PathBuf,
}

impl ImportTracker {
    pub fn new(module_path: PathBuf) -> Self {
        Self {
            imports: Vec::new(),
            module_path,
        }
    }

    pub fn track_import(&mut self, stmt: &ast::StmtImport) {
        for alias in &stmt.names {
            let module = alias.name.as_str().to_string();
            let alias_name = alias.asname.as_ref().map(|n| n.as_str().to_string());

            self.imports.push(ImportedSymbol {
                module: module.clone(),
                name: module,
                alias: alias_name,
                is_wildcard: false,
            });
        }
    }

    pub fn track_import_from(&mut self, stmt: &ast::StmtImportFrom) {
        let module = stmt
            .module
            .as_ref()
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| String::from("."));

        for alias in &stmt.names {
            let name = alias.name.as_str();
            let is_wildcard = name == "*";
            let alias_name = alias.asname.as_ref().map(|n| n.as_str().to_string());

            self.imports.push(ImportedSymbol {
                module: module.clone(),
                name: name.to_string(),
                alias: alias_name,
                is_wildcard,
            });
        }
    }

    pub fn get_imports(&self) -> &[ImportedSymbol] {
        &self.imports
    }

    pub fn resolve_name(&self, name: &str) -> Option<String> {
        for import in &self.imports {
            if let Some(alias) = &import.alias {
                if alias == name {
                    return Some(format!("{}.{}", import.module, import.name));
                }
            } else if import.name == name {
                if import.module == "." || import.module.is_empty() {
                    return Some(import.name.clone());
                }
                return Some(format!("{}.{}", import.module, import.name));
            } else if import.module == name && !import.is_wildcard {
                return Some(import.module.clone());
            }
        }
        None
    }

    pub fn has_wildcard_import_from(&self, module: &str) -> bool {
        self.imports
            .iter()
            .any(|i| i.module == module && i.is_wildcard)
    }
}

pub fn extract_exports(module: &ast::Mod) -> Vec<ExportedSymbol> {
    let mut exports = Vec::new();

    if let ast::Mod::Module(module) = module {
        for stmt in &module.body {
            match stmt {
                ast::Stmt::FunctionDef(f) => {
                    exports.push(ExportedSymbol {
                        name: f.name.as_str().to_string(),
                        qualified_name: f.name.as_str().to_string(),
                        is_class: false,
                        is_function: true,
                    });
                }
                ast::Stmt::AsyncFunctionDef(f) => {
                    exports.push(ExportedSymbol {
                        name: f.name.as_str().to_string(),
                        qualified_name: f.name.as_str().to_string(),
                        is_class: false,
                        is_function: true,
                    });
                }
                ast::Stmt::ClassDef(c) => {
                    let class_name = c.name.as_str();
                    exports.push(ExportedSymbol {
                        name: class_name.to_string(),
                        qualified_name: class_name.to_string(),
                        is_class: true,
                        is_function: false,
                    });

                    for item in &c.body {
                        if let ast::Stmt::FunctionDef(method) = item {
                            let method_name = method.name.as_str();
                            exports.push(ExportedSymbol {
                                name: format!("{}.{}", class_name, method_name),
                                qualified_name: format!("{}.{}", class_name, method_name),
                                is_class: false,
                                is_function: true,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }

    exports
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_tracking() {
        let mut tracker = ImportTracker::new(PathBuf::from("test.py"));

        let ast_import = ast::StmtImport {
            names: vec![ast::Alias {
                name: ast::Identifier::new("os"),
                asname: None,
                range: Default::default(),
            }],
            range: Default::default(),
        };

        tracker.track_import(&ast_import);
        assert_eq!(tracker.get_imports().len(), 1);
        assert_eq!(tracker.get_imports()[0].module, "os");
    }

    #[test]
    fn test_import_from_tracking() {
        let mut tracker = ImportTracker::new(PathBuf::from("test.py"));

        let ast_import_from = ast::StmtImportFrom {
            module: Some(ast::Identifier::new("typing")),
            names: vec![ast::Alias {
                name: ast::Identifier::new("List"),
                asname: None,
                range: Default::default(),
            }],
            level: None,
            range: Default::default(),
        };

        tracker.track_import_from(&ast_import_from);
        assert_eq!(tracker.get_imports().len(), 1);
        assert_eq!(tracker.get_imports()[0].module, "typing");
        assert_eq!(tracker.get_imports()[0].name, "List");
    }

    #[test]
    fn test_name_resolution() {
        let mut tracker = ImportTracker::new(PathBuf::from("test.py"));

        let ast_import_from = ast::StmtImportFrom {
            module: Some(ast::Identifier::new("collections")),
            names: vec![ast::Alias {
                name: ast::Identifier::new("defaultdict"),
                asname: None,
                range: Default::default(),
            }],
            level: None,
            range: Default::default(),
        };

        tracker.track_import_from(&ast_import_from);

        let resolved = tracker.resolve_name("defaultdict");
        assert_eq!(resolved, Some("collections.defaultdict".to_string()));
    }
}

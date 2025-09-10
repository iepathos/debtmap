use super::TestFramework;
use rustpython_parser::ast::{self, Stmt};

pub struct FrameworkDetector {
    has_unittest_import: bool,
    has_pytest_import: bool,
    has_nose_import: bool,
    has_doctest_import: bool,
    unittest_class_count: usize,
    pytest_fixture_count: usize,
}

impl FrameworkDetector {
    pub fn new() -> Self {
        Self {
            has_unittest_import: false,
            has_pytest_import: false,
            has_nose_import: false,
            has_doctest_import: false,
            unittest_class_count: 0,
            pytest_fixture_count: 0,
        }
    }

    pub fn analyze_module(&mut self, module: &ast::Mod) {
        if let ast::Mod::Module(ast::ModModule { body, .. }) = module {
            for stmt in body {
                self.analyze_statement(stmt);
            }
        }
    }

    fn analyze_statement(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Import(import) => {
                for alias in &import.names {
                    let name = alias.name.as_str();
                    if name == "unittest" || name.starts_with("unittest.") {
                        self.has_unittest_import = true;
                    } else if name == "pytest" || name.starts_with("pytest.") {
                        self.has_pytest_import = true;
                    } else if name == "nose" || name.starts_with("nose.") {
                        self.has_nose_import = true;
                    } else if name == "doctest" {
                        self.has_doctest_import = true;
                    }
                }
            }
            Stmt::ImportFrom(import_from) => {
                if let Some(module) = &import_from.module {
                    let module_str = module.as_str();
                    if module_str == "unittest" || module_str.starts_with("unittest.") {
                        self.has_unittest_import = true;
                    } else if module_str == "pytest" || module_str.starts_with("pytest.") {
                        self.has_pytest_import = true;
                    } else if module_str == "nose" || module_str.starts_with("nose.") {
                        self.has_nose_import = true;
                    } else if module_str == "doctest" {
                        self.has_doctest_import = true;
                    }
                }
            }
            Stmt::ClassDef(class_def) => {
                // Check if class inherits from unittest.TestCase
                for base in &class_def.bases {
                    if let ast::Expr::Name(name) = base {
                        if name.id.as_str() == "TestCase" {
                            self.unittest_class_count += 1;
                        }
                    } else if let ast::Expr::Attribute(attr) = base {
                        if attr.attr.as_str() == "TestCase" {
                            self.unittest_class_count += 1;
                        }
                    }
                }
            }
            Stmt::FunctionDef(func_def) => {
                // Check for pytest fixtures
                for decorator in &func_def.decorator_list {
                    if let ast::Expr::Name(name) = decorator {
                        if name.id.as_str() == "fixture" {
                            self.pytest_fixture_count += 1;
                        }
                    } else if let ast::Expr::Attribute(attr) = decorator {
                        if attr.attr.as_str() == "fixture" {
                            self.pytest_fixture_count += 1;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn detect_framework(&self) -> TestFramework {
        // Prioritize based on specific patterns
        if self.unittest_class_count > 0 {
            TestFramework::Unittest
        } else if self.pytest_fixture_count > 0 || self.has_pytest_import {
            TestFramework::Pytest
        } else if self.has_unittest_import {
            TestFramework::Unittest
        } else if self.has_nose_import {
            TestFramework::Nose
        } else if self.has_doctest_import {
            TestFramework::Doctest
        } else {
            // Default to pytest for files with test_ functions
            TestFramework::Pytest
        }
    }
}

pub fn detect_test_framework(module: &ast::Mod) -> TestFramework {
    let mut detector = FrameworkDetector::new();
    detector.analyze_module(module);
    detector.detect_framework()
}

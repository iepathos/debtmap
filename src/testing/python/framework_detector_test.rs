#[cfg(test)]
mod tests {
    use super::super::framework_detector::{detect_test_framework, FrameworkDetector};
    use super::super::TestFramework;
    use rustpython_parser::ast;

    fn parse_module(code: &str) -> ast::Mod {
        rustpython_parser::parse(code, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse")
            .into()
    }

    #[test]
    fn test_new_detector() {
        let detector = FrameworkDetector::new();
        assert_eq!(detector.detect_framework(), TestFramework::Pytest); // Default
    }

    #[test]
    fn test_detect_unittest_by_import() {
        let code = r#"
import unittest

def test_something():
    pass
"#;
        let module = parse_module(code);
        let framework = detect_test_framework(&module);
        assert_eq!(framework, TestFramework::Unittest);
    }

    #[test]
    fn test_detect_unittest_by_from_import() {
        let code = r#"
from unittest import TestCase

def test_something():
    pass
"#;
        let module = parse_module(code);
        let framework = detect_test_framework(&module);
        assert_eq!(framework, TestFramework::Unittest);
    }

    #[test]
    fn test_detect_unittest_by_class_inheritance() {
        let code = r#"
import unittest

class TestExample(unittest.TestCase):
    def test_method(self):
        self.assertEqual(1, 1)
"#;
        let module = parse_module(code);
        let framework = detect_test_framework(&module);
        assert_eq!(framework, TestFramework::Unittest);
    }

    #[test]
    fn test_detect_unittest_by_testcase_base() {
        let code = r#"
from unittest import TestCase

class TestExample(TestCase):
    def test_method(self):
        self.assertEqual(1, 1)
"#;
        let module = parse_module(code);
        let framework = detect_test_framework(&module);
        assert_eq!(framework, TestFramework::Unittest);
    }

    #[test]
    fn test_detect_pytest_by_import() {
        let code = r#"
import pytest

def test_something():
    assert True
"#;
        let module = parse_module(code);
        let framework = detect_test_framework(&module);
        assert_eq!(framework, TestFramework::Pytest);
    }

    #[test]
    fn test_detect_pytest_by_fixture() {
        let code = r#"
@pytest.fixture
def setup_data():
    return {"key": "value"}

def test_with_fixture(setup_data):
    assert setup_data["key"] == "value"
"#;
        let module = parse_module(code);
        let framework = detect_test_framework(&module);
        assert_eq!(framework, TestFramework::Pytest);
    }

    #[test]
    fn test_detect_pytest_by_fixture_decorator() {
        let code = r#"
import pytest

@fixture
def my_fixture():
    return 42

def test_something(my_fixture):
    assert my_fixture == 42
"#;
        let module = parse_module(code);
        let framework = detect_test_framework(&module);
        assert_eq!(framework, TestFramework::Pytest);
    }

    #[test]
    fn test_detect_nose_by_import() {
        let code = r#"
import nose

def test_something():
    assert True
"#;
        let module = parse_module(code);
        let framework = detect_test_framework(&module);
        assert_eq!(framework, TestFramework::Nose);
    }

    #[test]
    fn test_detect_nose_by_tools_import() {
        let code = r#"
from nose.tools import assert_equal

def test_something():
    assert_equal(1, 1)
"#;
        let module = parse_module(code);
        let framework = detect_test_framework(&module);
        assert_eq!(framework, TestFramework::Nose);
    }

    #[test]
    fn test_detect_doctest() {
        let code = r#"
import doctest

def add(a, b):
    """
    >>> add(2, 3)
    5
    """
    return a + b

if __name__ == "__main__":
    doctest.testmod()
"#;
        let module = parse_module(code);
        let framework = detect_test_framework(&module);
        assert_eq!(framework, TestFramework::Doctest);
    }

    #[test]
    fn test_prioritize_unittest_class_over_import() {
        let code = r#"
import pytest
import unittest

class TestExample(unittest.TestCase):
    def test_method(self):
        self.assertEqual(1, 1)
"#;
        let module = parse_module(code);
        let framework = detect_test_framework(&module);
        // Should prioritize unittest because of TestCase inheritance
        assert_eq!(framework, TestFramework::Unittest);
    }

    #[test]
    fn test_prioritize_pytest_fixture_over_imports() {
        let code = r#"
import nose

@pytest.fixture
def setup():
    return {}

def test_something(setup):
    assert True
"#;
        let module = parse_module(code);
        let framework = detect_test_framework(&module);
        // Should prioritize pytest because of fixture
        assert_eq!(framework, TestFramework::Pytest);
    }

    #[test]
    fn test_multiple_imports_priority() {
        let code = r#"
import unittest
import pytest
import nose

def test_something():
    assert True
"#;
        let module = parse_module(code);
        let framework = detect_test_framework(&module);
        // Should prioritize pytest when only imports are present
        assert_eq!(framework, TestFramework::Pytest);
    }

    #[test]
    fn test_default_to_pytest_for_simple_tests() {
        let code = r#"
def test_something():
    assert True

def test_another():
    assert False == False
"#;
        let module = parse_module(code);
        let framework = detect_test_framework(&module);
        // Should default to pytest for simple test functions
        assert_eq!(framework, TestFramework::Pytest);
    }

    #[test]
    fn test_unittest_submodule_import() {
        let code = r#"
from unittest.mock import Mock, patch

def test_with_mock():
    m = Mock()
    assert m is not None
"#;
        let module = parse_module(code);
        let framework = detect_test_framework(&module);
        assert_eq!(framework, TestFramework::Unittest);
    }

    #[test]
    fn test_pytest_submodule_import() {
        let code = r#"
from pytest.mark import parametrize

@parametrize("input,expected", [(1, 2), (2, 3)])
def test_increment(input, expected):
    assert input + 1 == expected
"#;
        let module = parse_module(code);
        let framework = detect_test_framework(&module);
        assert_eq!(framework, TestFramework::Pytest);
    }

    #[test]
    fn test_empty_module() {
        let code = "";
        let module = parse_module(code);
        let framework = detect_test_framework(&module);
        // Should default to pytest
        assert_eq!(framework, TestFramework::Pytest);
    }

    #[test]
    fn test_module_with_no_test_indicators() {
        let code = r#"
import os
import sys

def helper_function():
    return 42

class DataProcessor:
    def process(self):
        pass
"#;
        let module = parse_module(code);
        let framework = detect_test_framework(&module);
        // Should default to pytest even without test indicators
        assert_eq!(framework, TestFramework::Pytest);
    }

    #[test]
    fn test_multiple_unittest_classes() {
        let code = r#"
import unittest

class TestFirst(unittest.TestCase):
    def test_one(self):
        pass

class TestSecond(unittest.TestCase):
    def test_two(self):
        pass
"#;
        let module = parse_module(code);
        let mut detector = FrameworkDetector::new();
        detector.analyze_module(&module);
        
        // Should detect multiple unittest classes
        assert_eq!(detector.detect_framework(), TestFramework::Unittest);
    }

    #[test]
    fn test_multiple_pytest_fixtures() {
        let code = r#"
import pytest

@pytest.fixture
def first_fixture():
    return 1

@fixture
def second_fixture():
    return 2

def test_with_fixtures(first_fixture, second_fixture):
    assert first_fixture + second_fixture == 3
"#;
        let module = parse_module(code);
        let framework = detect_test_framework(&module);
        assert_eq!(framework, TestFramework::Pytest);
    }

    #[test]
    fn test_mixed_frameworks_real_world() {
        // Real-world scenario: project migrating from unittest to pytest
        let code = r#"
import unittest
import pytest

# Old unittest tests
class LegacyTests(unittest.TestCase):
    def test_old_style(self):
        self.assertEqual(1, 1)

# New pytest tests
@pytest.fixture
def modern_fixture():
    return {"data": "test"}

def test_new_style(modern_fixture):
    assert modern_fixture["data"] == "test"
"#;
        let module = parse_module(code);
        let framework = detect_test_framework(&module);
        // Should prioritize unittest due to TestCase class
        assert_eq!(framework, TestFramework::Unittest);
    }

    #[test]
    fn test_import_alias() {
        let code = r#"
import unittest as ut

class TestExample(ut.TestCase):
    def test_method(self):
        pass
"#;
        let module = parse_module(code);
        let framework = detect_test_framework(&module);
        // Should still detect unittest
        assert_eq!(framework, TestFramework::Unittest);
    }
}
use debtmap::analyzers::{analyze_file, get_analyzer};
use debtmap::core::{DebtType, Language};
use std::path::PathBuf;

#[test]
fn test_javascript_file_analysis() {
    let content = r#"
// TODO: Refactor this function
function calculateTotal(items) {
    let total = 0;
    for (let i = 0; i < items.length; i++) {
        if (items[i].price > 0) {
            total += items[i].price * items[i].quantity;
        }
    }
    return total;
}

// FIXME: Handle edge cases
const processOrder = (order) => {
    if (!order) return null;
    
    const items = order.items || [];
    const total = calculateTotal(items);
    
    return {
        ...order,
        total,
        processed: true
    };
};

export { calculateTotal, processOrder };
"#;

    let analyzer = get_analyzer(Language::JavaScript);
    let result = analyze_file(content.to_string(), PathBuf::from("test.js"), &*analyzer);

    assert!(result.is_ok());
    let metrics = result.unwrap();

    // Check language detection
    assert_eq!(metrics.language, Language::JavaScript);

    // Check function detection
    assert_eq!(metrics.complexity.functions.len(), 2);

    // Check TODO/FIXME detection
    let todo_items: Vec<_> = metrics
        .debt_items
        .iter()
        .filter(|item| item.debt_type == DebtType::Todo)
        .collect();
    assert_eq!(todo_items.len(), 1);

    let fixme_items: Vec<_> = metrics
        .debt_items
        .iter()
        .filter(|item| item.debt_type == DebtType::Fixme)
        .collect();
    assert_eq!(fixme_items.len(), 1);
}

#[test]
fn test_typescript_file_analysis() {
    let content = r#"
interface User {
    id: number;
    name: string;
    email: string;
}

// TODO: Add validation
async function fetchUser(id: number): Promise<User | null> {
    try {
        const response = await fetch(`/api/users/${id}`);
        if (response.ok) {
            return await response.json() as User;
        } else if (response.status === 404) {
            return null;
        } else {
            throw new Error(`Failed to fetch user: ${response.status}`);
        }
    } catch (error) {
        console.error('Error fetching user:', error);
        return null;
    }
}

class UserService {
    private cache: Map<number, User> = new Map();

    // HACK: This is a temporary workaround
    async getUser(id: number): Promise<User | null> {
        if (this.cache.has(id)) {
            return this.cache.get(id) || null;
        }
        
        const user = await fetchUser(id);
        if (user) {
            this.cache.set(id, user);
        }
        return user;
    }
}

export { User, UserService, fetchUser };
"#;

    let analyzer = get_analyzer(Language::TypeScript);
    let result = analyze_file(content.to_string(), PathBuf::from("test.ts"), &*analyzer);

    assert!(result.is_ok());
    let metrics = result.unwrap();

    // Check language detection
    assert_eq!(metrics.language, Language::TypeScript);

    // Check function detection (should find fetchUser and getUser)
    assert!(metrics.complexity.functions.len() >= 2);
}

#[test]
fn test_javascript_complexity_metrics() {
    let content = r#"
function complexFunction(data) {
    if (data.type === 'A') {
        if (data.value > 100) {
            for (let i = 0; i < data.items.length; i++) {
                if (data.items[i].active) {
                    switch (data.items[i].category) {
                        case 'X':
                            return 'X-High';
                        case 'Y':
                            return 'Y-High';
                        default:
                            return 'Other-High';
                    }
                }
            }
        } else if (data.value > 50) {
            return 'Medium';
        }
    } else if (data.type === 'B') {
        try {
            return processTypeB(data);
        } catch (error) {
            console.error(error);
            return 'Error';
        }
    }
    
    return 'Default';
}
"#;

    let analyzer = get_analyzer(Language::JavaScript);
    let result = analyze_file(content.to_string(), PathBuf::from("complex.js"), &*analyzer);

    assert!(result.is_ok());
    let metrics = result.unwrap();

    assert_eq!(metrics.complexity.functions.len(), 1);
    let func = &metrics.complexity.functions[0];

    // Check that complexity is calculated
    assert!(func.cyclomatic > 5); // High complexity due to nested conditions
    assert!(func.cognitive > 5); // High cognitive complexity due to nesting
}

#[test]
fn test_jsx_file_detection() {
    let content = r#"
import React, { useState } from 'react';

function TodoList({ items }) {
    const [filter, setFilter] = useState('all');
    
    // TODO: Implement filtering logic
    const filteredItems = items.filter(item => {
        if (filter === 'all') return true;
        if (filter === 'active') return !item.completed;
        if (filter === 'completed') return item.completed;
        return true;
    });
    
    return (
        <div className="todo-list">
            <select onChange={(e) => setFilter(e.target.value)}>
                <option value="all">All</option>
                <option value="active">Active</option>
                <option value="completed">Completed</option>
            </select>
            <ul>
                {filteredItems.map(item => (
                    <li key={item.id}>{item.text}</li>
                ))}
            </ul>
        </div>
    );
}

export default TodoList;
"#;

    let analyzer = get_analyzer(Language::JavaScript);
    let result = analyze_file(
        content.to_string(),
        PathBuf::from("component.jsx"),
        &*analyzer,
    );

    assert!(result.is_ok());
    let metrics = result.unwrap();

    // Check import dependencies
    assert!(metrics.dependencies.iter().any(|d| d.name == "react"));

    // Check TODO detection
    assert!(metrics
        .debt_items
        .iter()
        .any(|item| item.debt_type == DebtType::Todo));
}

#[test]
fn test_import_export_dependencies() {
    let content = r#"
import fs from 'fs';
import { readFile, writeFile } from 'fs/promises';
import * as path from 'path';
const util = require('util');
const { exec } = require('child_process');

// Dynamic import
async function loadModule() {
    const module = await import('./dynamic-module');
    return module;
}

export { loadModule };
export default loadModule;
"#;

    let analyzer = get_analyzer(Language::JavaScript);
    let result = analyze_file(content.to_string(), PathBuf::from("imports.js"), &*analyzer);

    assert!(result.is_ok());
    let metrics = result.unwrap();

    // Check various import styles are detected
    assert!(metrics.dependencies.iter().any(|d| d.name == "fs"));
    assert!(metrics.dependencies.iter().any(|d| d.name == "fs/promises"));
    assert!(metrics.dependencies.iter().any(|d| d.name == "path"));
    assert!(metrics.dependencies.iter().any(|d| d.name == "util"));
    assert!(metrics
        .dependencies
        .iter()
        .any(|d| d.name == "child_process"));
    assert!(metrics
        .dependencies
        .iter()
        .any(|d| d.name == "./dynamic-module"));
}

#[test]
fn test_arrow_function_detection() {
    let content = r#"
const add = (a, b) => a + b;

const multiply = (a, b) => {
    return a * b;
};

const complexArrow = (data) => {
    if (data.type === 'special') {
        return data.value * 2;
    }
    return data.value;
};

class Calculator {
    compute = (operation, a, b) => {
        switch (operation) {
            case 'add': return a + b;
            case 'subtract': return a - b;
            case 'multiply': return a * b;
            case 'divide': return b !== 0 ? a / b : 0;
            default: return 0;
        }
    }
}
"#;

    let analyzer = get_analyzer(Language::JavaScript);
    let result = analyze_file(content.to_string(), PathBuf::from("arrows.js"), &*analyzer);

    assert!(result.is_ok());
    let metrics = result.unwrap();

    // Should detect all arrow functions
    assert!(metrics.complexity.functions.len() >= 4);
}

#[test]
fn test_async_await_analysis() {
    let content = r#"
async function fetchData(url) {
    try {
        const response = await fetch(url);
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        const data = await response.json();
        return data;
    } catch (error) {
        console.error('Fetch error:', error);
        throw error;
    }
}

const processData = async (urls) => {
    const results = [];
    
    for (const url of urls) {
        try {
            const data = await fetchData(url);
            results.push(data);
        } catch (error) {
            // FIXME: Better error handling needed
            results.push(null);
        }
    }
    
    return results;
};
"#;

    let analyzer = get_analyzer(Language::JavaScript);
    let result = analyze_file(content.to_string(), PathBuf::from("async.js"), &*analyzer);

    assert!(result.is_ok());
    let metrics = result.unwrap();

    // Should detect async functions
    assert_eq!(metrics.complexity.functions.len(), 2);

    // Check for FIXME comment
    assert!(metrics
        .debt_items
        .iter()
        .any(|item| item.debt_type == DebtType::Fixme));
}

#[test]
fn test_typescript_generics_and_types() {
    let content = r#"
type Result<T, E> = { ok: true; value: T } | { ok: false; error: E };

function wrapResult<T, E>(fn: () => T): Result<T, E> {
    try {
        const value = fn();
        return { ok: true, value };
    } catch (error) {
        return { ok: false, error: error as E };
    }
}

interface Repository<T> {
    findById(id: string): Promise<T | null>;
    save(entity: T): Promise<void>;
    delete(id: string): Promise<boolean>;
}

class UserRepository implements Repository<User> {
    async findById(id: string): Promise<User | null> {
        // TODO: Implement database query
        return null;
    }
    
    async save(user: User): Promise<void> {
        // TODO: Implement save logic
    }
    
    async delete(id: string): Promise<boolean> {
        // TODO: Implement delete logic
        return false;
    }
}
"#;

    let analyzer = get_analyzer(Language::TypeScript);
    let result = analyze_file(
        content.to_string(),
        PathBuf::from("generics.ts"),
        &*analyzer,
    );

    assert!(result.is_ok());
    let metrics = result.unwrap();

    // Should detect class methods
    assert!(metrics.complexity.functions.len() >= 4);

    // Should detect TODO comments
    let todo_items: Vec<_> = metrics
        .debt_items
        .iter()
        .filter(|item| item.debt_type == DebtType::Todo)
        .collect();
    assert!(todo_items.len() >= 3);
}

#[test]
fn test_suppression_comments_javascript() {
    let content = r#"
// debtmap:ignore-next-line [todo]
// TODO: This should be ignored

function test() {
    // debtmap:ignore-start [*]
    // TODO: Also ignored
    // FIXME: This too
    // debtmap:ignore-end
    
    // TODO: This should be detected
}
"#;

    let analyzer = get_analyzer(Language::JavaScript);
    let result = analyze_file(
        content.to_string(),
        PathBuf::from("suppressed.js"),
        &*analyzer,
    );

    assert!(result.is_ok());
    let metrics = result.unwrap();

    // Should only detect one TODO (the last one)
    let todo_items: Vec<_> = metrics
        .debt_items
        .iter()
        .filter(|item| item.debt_type == DebtType::Todo)
        .collect();
    assert_eq!(todo_items.len(), 1);
}

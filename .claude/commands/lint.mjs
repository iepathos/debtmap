#!/usr/bin/env node

import { execSync } from 'child_process';
import { readFileSync, writeFileSync } from 'fs';
import { join } from 'path';

/**
 * /lint command - Fix formatting and linting issues from just fmt-check && just lint output
 * 
 * Usage: /lint <shell_output>
 * 
 * This command parses the output from failed fmt-check and lint commands and automatically
 * fixes the issues.
 */

// Get the shell output from command line arguments
const args = process.argv.slice(2);
if (args.length === 0) {
    console.error('Error: No shell output provided');
    console.error('Usage: /lint <shell_output>');
    process.exit(1);
}

const shellOutput = args.join(' ');

// Parse the output to identify issues
const lines = shellOutput.split('\n');
let hasFormattingIssues = false;
let hasLintIssues = false;
const formattingFiles = new Set();
const lintIssues = [];

for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    
    // Check for formatting issues (cargo fmt --check output)
    if (line.includes('Diff in') || line.includes('would be reformatted')) {
        hasFormattingIssues = true;
        // Extract file path from lines like "Diff in /path/to/file.rs"
        const diffMatch = line.match(/Diff in (.+?)(?:\s+at line|$)/);
        if (diffMatch) {
            formattingFiles.add(diffMatch[1]);
        }
        // Also catch files from summary lines
        const fileMatch = line.match(/^(.+\.rs) would be reformatted$/);
        if (fileMatch) {
            formattingFiles.add(fileMatch[1]);
        }
    }
    
    // Check for clippy warnings/errors
    if (line.includes('warning:') || line.includes('error:')) {
        hasLintIssues = true;
        // Parse clippy output format: "warning: message --> src/file.rs:line:col"
        const nextLine = i + 1 < lines.length ? lines[i + 1] : '';
        if (nextLine.includes('-->')) {
            const locationMatch = nextLine.match(/-->\s+(.+?):(\d+):(\d+)/);
            if (locationMatch) {
                lintIssues.push({
                    file: locationMatch[1],
                    line: parseInt(locationMatch[2]),
                    column: parseInt(locationMatch[3]),
                    message: line.trim(),
                    raw: line + '\n' + nextLine
                });
            }
        }
    }
}

console.log('=== Lint Analysis Results ===\n');
console.log(`Shell output received: ${shellOutput.length} characters`);
console.log(`Formatting issues detected: ${hasFormattingIssues}`);
console.log(`Lint issues detected: ${hasLintIssues}`);

if (hasFormattingIssues) {
    console.log(`\nFiles needing formatting: ${formattingFiles.size}`);
    for (const file of formattingFiles) {
        console.log(`  - ${file}`);
    }
}

if (hasLintIssues) {
    console.log(`\nClippy issues found: ${lintIssues.length}`);
    for (const issue of lintIssues.slice(0, 5)) { // Show first 5 issues
        console.log(`  - ${issue.file}:${issue.line} - ${issue.message.substring(0, 80)}...`);
    }
    if (lintIssues.length > 5) {
        console.log(`  ... and ${lintIssues.length - 5} more issues`);
    }
}

// Fix the issues
console.log('\n=== Applying Fixes ===\n');

// Fix formatting issues first
if (hasFormattingIssues) {
    console.log('Running cargo fmt to fix formatting issues...');
    try {
        execSync('cargo fmt', { stdio: 'inherit' });
        console.log('✅ Formatting fixed successfully');
    } catch (error) {
        console.error('❌ Failed to run cargo fmt:', error.message);
    }
}

// Fix clippy issues if there are auto-fixable ones
if (hasLintIssues) {
    console.log('\nAttempting to fix clippy issues...');
    try {
        // Try to apply clippy fixes
        execSync('cargo clippy --fix --allow-dirty --allow-staged', { stdio: 'inherit' });
        console.log('✅ Applied available clippy fixes');
    } catch (error) {
        console.error('⚠️  Some clippy issues may require manual fixes:', error.message);
    }
}

// Verify the fixes
console.log('\n=== Verification ===\n');
console.log('Running checks to verify fixes...');

let allFixed = true;

// Check formatting
try {
    execSync('cargo fmt --check', { stdio: 'pipe' });
    console.log('✅ Formatting check passed');
} catch (error) {
    console.log('⚠️  Formatting still has issues');
    allFixed = false;
}

// Check clippy
try {
    execSync('cargo clippy -- -D warnings', { stdio: 'pipe' });
    console.log('✅ Clippy check passed');
} catch (error) {
    console.log('⚠️  Clippy still has warnings - manual fixes may be required');
    allFixed = false;
}

if (allFixed) {
    console.log('\n✨ All issues have been fixed successfully!');
} else {
    console.log('\n⚠️  Some issues remain and may require manual intervention.');
    console.log('Run "just fmt-check && just lint" to see remaining issues.');
}

// Provide summary
console.log('\n=== Summary ===\n');
console.log('Actions taken:');
if (hasFormattingIssues) {
    console.log('  • Ran cargo fmt to fix formatting');
}
if (hasLintIssues) {
    console.log('  • Ran cargo clippy --fix for auto-fixable lints');
}
if (!hasFormattingIssues && !hasLintIssues) {
    console.log('  • No issues detected in the provided output');
}

console.log('\nNext steps:');
console.log('  1. Review the changes with "git diff"');
console.log('  2. Run "just fmt-check && just lint" to verify all issues are fixed');
console.log('  3. If issues remain, they may require manual fixes');
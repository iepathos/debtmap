---
name: product-analysis
description: Analyze debtmap's own output to identify product improvements
---

# Product Analysis of Debtmap

Run debtmap on itself and evaluate the tool's effectiveness and areas for improvement.

## Step 1: Generate Analysis

```bash
debtmap analyze . --lcov target/coverage/lcov.info --top 100
```

## Step 2: Evaluate Output Quality

Review the generated debt items and assess:

### Validity Assessment
- **Are the debt items genuine technical debt?**
  - Do they represent real maintenance burdens?
  - Are they actionable by developers?
  - Do they align with common tech debt patterns?

### Ranking Quality
- **Is the prioritization meaningful?**
  - Do high-priority items truly have more impact?
  - Is the scoring algorithm capturing the right signals?
  - Are dependencies and critical paths weighted appropriately?

### Actionability
- **Do items provide clear guidance?**
  - Is the "why" behind each debt item clear?
  - Are the suggested fixes practical?
  - Is there enough context to take action?

## Step 3: Identify Product Improvements

Based on the analysis, consider improvements to:

### Detection Algorithms
- Better pattern recognition for debt types
- More nuanced complexity scoring
- Improved dependency impact analysis

### Output Format
- Clearer explanations of why items are flagged
- More specific refactoring guidance
- Better grouping of related debt items

### Scoring System
- Refine weights for different debt factors
- Consider code coverage impact more precisely
- Better balance between size and complexity

### User Experience
- More filtering and sorting options
- Better visualization of debt clusters
- Clearer next-step recommendations

## Step 4: Priority Improvements

Focus on changes that would:
1. **Reduce false positives** - Avoid flagging non-issues
2. **Increase actionability** - Make recommendations more specific
3. **Improve prioritization** - Surface the most impactful items first
4. **Enhance explanations** - Help users understand the "why"

## Goal

The objective is NOT to fix the technical debt found in debtmap itself, but rather to use debtmap's own output as a test case to improve the tool's analysis capabilities, scoring algorithms, and user-facing recommendations.
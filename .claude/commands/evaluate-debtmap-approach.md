---
name: evaluate-debtmap-approach
description: Evaluate whether debtmap's approach (static analysis + heuristics) can deliver accurate debt assessment
---

# Debtmap Approach Evaluation

Evaluate the fundamental viability of debtmap's approach: Can static analysis and heuristics accurately identify technical debt and provide actionable fix recommendations?

## Step 1: Understand Debtmap's Scope and Ambitions

Examine the codebase to understand what debtmap is trying to accomplish:

### Read Core Documentation
- Review README.md for stated goals
- Examine architecture documentation
- Review specs/ directory for feature requirements
- Study src/debt/ directory to understand debt detection patterns

### Identify Key Claims
Document what debtmap claims to do:
- What types of technical debt can it detect?
- What accuracy does it aim for?
- How actionable are the recommendations supposed to be?
- What languages and patterns does it support?

## Step 2: Analyze Detection Mechanisms

Examine how debtmap actually identifies technical debt:

### Static Analysis Capabilities
Review the analyzers to understand what can be detected:
- **Code complexity** - Cyclomatic complexity, cognitive complexity, nesting depth
- **Structural patterns** - God objects, long functions, deep inheritance
- **Test coverage gaps** - Untested complex code
- **Dependency issues** - Tight coupling, circular dependencies
- **Code smells** - Duplicated code, dead code, magic numbers

### Heuristic Quality
Evaluate the heuristics used:
- Are thresholds evidence-based or arbitrary?
- Do they account for language idioms and best practices?
- Are they context-aware (test code vs production code)?
- Do they adapt to codebase size and domain?

### Pattern Recognition Limits
Identify what static analysis *cannot* detect:
- **Business logic debt** - Incorrect implementations that compile
- **Architectural mismatches** - Design that doesn't fit requirements
- **Performance issues** - Code that's slow but not complex
- **Security vulnerabilities** - Subtle security anti-patterns
- **Team knowledge gaps** - Code only one person understands

## Step 3: Evaluate Theoretical Soundness

Assess whether the approach is theoretically viable:

### Can Static Analysis Identify Real Debt?

**Strong correlation cases** (where static analysis works well):
- High complexity is usually harder to maintain
- Low test coverage in complex code is risky
- Very long functions are typically problematic
- Deep nesting indicates cognitive overload
- High coupling increases change impact

**Weak correlation cases** (where static analysis struggles):
- Simple code can still be "wrong" for the domain
- Well-tested code might test the wrong behavior
- Short functions can still be poorly designed
- Flat structure doesn't guarantee good architecture

**Question to answer**: What percentage of real technical debt can be detected through static analysis alone?

### Are Heuristics Accurate Enough?

**Precision vs Recall tradeoff**:
- **High precision** - Few false positives, but misses real debt
- **High recall** - Catches most debt, but many false positives

Evaluate debtmap's current position on this spectrum:
- What's the false positive rate?
- What's the false negative rate?
- Is the tradeoff appropriate for the use case?

### Can Recommendations Be Actionable?

**What makes a recommendation actionable?**
- Specific enough to guide refactoring
- Explains *why* something is debt, not just *what* is flagged
- Provides context about impact and urgency
- Suggests concrete refactoring patterns

Evaluate debtmap's recommendations:
- Do they meet these criteria?
- Can they realistically be automated?
- Do they require human judgment?

## Step 4: Run Empirical Validation

Test debtmap's effectiveness on real codebases:

### Self-Analysis
```bash
just analyze-self
```

Review the output and ask:
- Are the top items genuine technical debt?
- Would fixing them actually improve maintainability?
- Are the recommendations clear and actionable?
- Any obvious false positives or false negatives?

### Known Debt Validation

Identify 5-10 pieces of code in debtmap that you *know* are technical debt:
- Does debtmap flag them?
- Are they prioritized appropriately?
- Are the fix recommendations helpful?

Identify 5-10 pieces of code that are *intentionally* complex or large:
- Does debtmap incorrectly flag them?
- Can you suppress false positives easily?

## Step 5: Assess Realistic Capabilities

Based on the analysis, determine what debtmap can realistically accomplish:

### What It Can Do Well
Identify areas where static analysis + heuristics excel:
- Complexity hotspots that need attention
- Coverage gaps in critical code
- Structural anti-patterns (god objects, long methods)
- Maintainability trends over time
- Cross-team consistency checking

### What It Struggles With
Identify inherent limitations:
- Context-dependent debt (what's right for this domain?)
- Subtle architectural issues
- Code that's technically fine but doesn't meet requirements
- Performance and security issues
- Human factors (understandability, team knowledge)

### What It Should Never Claim
Identify impossible promises:
- "Automatically fix all technical debt"
- "Perfect debt detection with no false positives"
- "Replace human code review"
- "Understand business requirements"

## Step 6: Evaluate Fix Recommendations

Assess the quality and feasibility of automated fix suggestions:

### Current State of Recommendations
Review what debtmap currently suggests:
- Are fixes specific or generic?
- Do they preserve functionality?
- Are they safe to apply automatically?
- Do they actually reduce debt?

### Theoretical Limits of Automation
Consider what can be automated:
- **Safe automated fixes**:
  - Extract method refactoring for long functions
  - Reduce nesting through guard clauses
  - Extract constants for magic numbers
  - Split god objects by interface segregation

- **Risky automated fixes**:
  - Changing algorithms to reduce complexity
  - Restructuring architecture
  - Removing "dead" code that might be needed
  - Simplifying business logic

### Recommendation Quality Framework
Evaluate recommendations on:
1. **Safety** - Can be applied without breaking functionality?
2. **Specificity** - Concrete steps vs vague suggestions?
3. **Completeness** - Addresses root cause vs symptoms?
4. **Maintainability** - Actually improves code quality?

## Step 7: Identify Gaps and Opportunities

### Current Gaps
List areas where debtmap falls short of its ambitions:
- Detection gaps (what debt is missed?)
- False positive sources (what's incorrectly flagged?)
- Recommendation quality (are fixes truly actionable?)
- Context awareness (does it understand the domain?)

### Potential Improvements
Suggest enhancements to close gaps:
- **Better heuristics** - More context-aware thresholds
- **Pattern libraries** - Recognize common design patterns
- **ML enhancement** - Learn from user feedback on debt items
- **Domain adaptation** - Configure for different project types
- **Impact analysis** - Better prioritization through dependency analysis

### Realistic Roadmap
Propose a path forward:
1. **Short-term** (already feasible):
   - Improve existing heuristics based on false positive analysis
   - Add more language-specific patterns
   - Enhance explanation quality

2. **Medium-term** (requires research):
   - Context-aware analysis (test vs production, framework patterns)
   - Automated fix generation for safe refactorings
   - Integration with CI/CD for trend analysis

3. **Long-term** (fundamental research):
   - ML-based debt detection
   - Semantic understanding of business logic
   - Collaborative filtering (learn from similar projects)

## Step 8: Synthesize Findings

Create a comprehensive assessment answering the core question:

### Can Debtmap Deliver on Its Promise?

**Yes, with caveats:**
- Static analysis can identify *some* technical debt accurately
- Heuristics need continuous refinement to minimize false positives
- Recommendations can be actionable for *structural* debt
- Human judgment is still required for *semantic* debt

**Success Criteria:**
- Focus on high-confidence, high-impact debt
- Accept that some false positives are inevitable
- Provide context and explanations, not just flags
- Enable human decision-making rather than replace it

### What Makes Debtmap Valuable?

Not as a replacement for human judgment, but as:
1. **A screening tool** - Surface hotspots for human review
2. **A consistency checker** - Enforce team standards
3. **A trend analyzer** - Track debt over time
4. **A teaching tool** - Educate developers about code quality

### Honest Assessment

Document both strengths and limitations:

**Strengths:**
- Can detect structural complexity objectively
- Provides measurable, repeatable analysis
- Scales to large codebases
- Helps prioritize refactoring efforts

**Limitations:**
- Cannot understand business requirements
- Struggles with context-dependent decisions
- May miss subtle architectural issues
- Requires tuning for each project

**Positioning:**
Debtmap is most effective when positioned as a *decision support tool* that augments human expertise, not an autonomous debt-fixing oracle.

## Step 9: Recommendations for Messaging

Based on the evaluation, recommend how debtmap should position itself:

### What to Emphasize
- Objective complexity measurement
- Trend analysis and tracking
- Team consensus building
- Prioritization assistance

### What to Avoid Claiming
- Perfect accuracy in debt detection
- Fully automated debt remediation
- Understanding of business logic
- Replacement for code review

### Honest Value Proposition
"Debtmap uses static analysis to identify complexity hotspots, structural anti-patterns, and test coverage gaps—giving you an objective foundation for refactoring decisions. It won't understand your business logic or catch every type of debt, but it will surface the most measurable, high-impact issues for your team to address."

## Output Format

Produce a detailed report with:

```markdown
# Debtmap Approach Viability Assessment

## Executive Summary
[Can debtmap deliver accurate debt assessment? Under what conditions?]

## Detection Accuracy Analysis
### What It Detects Well
- [List with evidence]

### What It Misses
- [List with evidence]

### False Positive Rate
- [Analysis from self-analysis and known cases]

## Recommendation Quality
### Actionability Score
- [Rate 1-10 with justification]

### Safety Analysis
- [Which fixes are safe to automate?]

## Theoretical Soundness
### Static Analysis Capabilities
- [What's possible vs impossible]

### Heuristic Limitations
- [Where do simple rules break down?]

## Realistic Scope
### Achievable Goals
- [What debtmap can realistically do]

### Unachievable Goals
- [What it should not claim]

## Recommendations
### Product Direction
- [How to position debtmap]

### Technical Improvements
- [What to build next]

### Messaging Strategy
- [How to communicate capabilities honestly]

## Conclusion
[Final verdict: Is the approach sound? What needs to change?]
```

## Success Criteria

This evaluation is successful if it provides:
1. An honest assessment of debtmap's capabilities and limitations
2. Clear understanding of where static analysis works and where it doesn't
3. Actionable recommendations for improving accuracy
4. Realistic positioning for the product
5. Evidence-based claims about what debtmap can deliver

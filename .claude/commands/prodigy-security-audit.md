# Security Audit Command

Performs comprehensive security analysis and generates remediation specs.

## Usage

```
/prodigy-security-audit [focus]
```

Examples:
- `/prodigy-security-audit` - Full security audit
- `/prodigy-security-audit input-validation` - Focus on input sanitization
- `/prodigy-security-audit dependencies` - Audit third-party dependencies
- `/prodigy-security-audit secrets` - Check for exposed secrets

## What This Command Does

1. **Security Analysis**
   - Scans for common vulnerabilities
   - Identifies insecure patterns
   - Checks dependency vulnerabilities
   - Finds exposed secrets or keys

2. **Risk Assessment**
   - Categorizes issues by severity
   - Prioritizes critical vulnerabilities
   - Provides CVSS-style scoring
   - Maps to OWASP guidelines

3. **Remediation Spec**
   - Detailed fix instructions
   - Security best practices
   - Testing requirements
   - Commits audit results

## Security Checks

- **Input Validation**: SQL injection, XSS, command injection
- **Authentication**: Weak auth, missing MFA, session issues
- **Authorization**: Privilege escalation, IDOR
- **Cryptography**: Weak algorithms, key management
- **Dependencies**: Known CVEs, outdated packages
- **Secrets**: Hardcoded credentials, API keys
- **Configuration**: Insecure defaults, debug modes

## Severity Levels

- **Critical**: Immediate exploitation risk
- **High**: Significant security impact
- **Medium**: Moderate risk
- **Low**: Minor issues
- **Info**: Best practice recommendations

## Output Format

Generates and commits:

```
security: audit spec for {focus} security-{timestamp}
```

## Compliance Standards

- OWASP Top 10
- CWE/SANS Top 25
- NIST guidelines
- Industry-specific requirements
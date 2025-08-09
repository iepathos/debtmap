# Security Policy

## Supported Versions

We release patches for security vulnerabilities. Which versions are eligible receiving such patches depends on the CVSS v3.0 Rating:

| CVSS v3.0 | Supported Versions                        |
| --------- | ----------------------------------------- |
| 9.0-10.0  | Releases within the last three months    |
| 4.0-8.9   | Most recent release                       |

## Reporting a Vulnerability

Please report (suspected) security vulnerabilities to **[iepathos@gmail.com](mailto:iepathos@gmail.com)**. You will receive a response from us within 48 hours. If the issue is confirmed, we will release a patch as soon as possible depending on complexity but historically within a few days.

## Security Measures

### Development Security
- All dependencies are regularly audited using `cargo audit`
- Automated security scanning runs weekly and on every pull request
- License compliance is enforced through `cargo deny`
- Supply chain security is monitored through dependency verification

### Build Security
- All releases are built in isolated GitHub Actions environments
- Release binaries include SHA256 checksums for integrity verification
- Cross-compilation targets are verified for consistency
- Build artifacts are created reproducibly where possible

### Code Security
- Rust's memory safety features prevent common security vulnerabilities
- All code goes through review before being merged
- Automated linting and formatting enforce consistent code quality
- Security-focused clippy lints are enabled and enforced

### Dependency Security
- Dependencies are automatically updated by Dependabot
- Major version updates require manual review
- Only well-maintained crates from trusted sources are used
- Transitive dependencies are regularly audited

## Security Features

### ClaudeForge Security
- Templates are validated before processing
- File system operations are sandboxed to project directories
- Git operations use safe, validated paths
- Configuration files are validated for correctness
- Network requests are made only to verified endpoints

### Template Security
- Generated project templates include security best practices
- Default configurations prioritize security over convenience
- Security-focused dependencies are included by default
- Generated code includes appropriate input validation

## Response Process

1. **Receipt**: Security reports are acknowledged within 48 hours
2. **Assessment**: Vulnerability is assessed and classified within 5 days
3. **Development**: Patch is developed and tested
4. **Coordination**: If needed, we coordinate with other affected parties
5. **Release**: Security patch is released with appropriate advisories
6. **Disclosure**: Public disclosure occurs after patch is available

## Security Contacts

- **Primary**: Glen Baker (iepathos@gmail.com)
- **Repository**: https://github.com/iepathos/claudeforge
- **Security Advisory**: GitHub Security Advisories

## Hall of Fame

We appreciate security researchers who responsibly disclose vulnerabilities. Contributors will be acknowledged here (with permission).

---

*This security policy is subject to change. Please check back regularly for updates.*
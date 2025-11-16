# Security Policy

## Supported Versions

We release patches for security vulnerabilities in the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

The Forklift team takes security bugs seriously. We appreciate your efforts to responsibly disclose your findings, and will make every effort to acknowledge your contributions.

### How to Report a Security Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please report security vulnerabilities by emailing the maintainer or using GitHub's private vulnerability reporting feature:

1. **GitHub Security Advisory (Preferred)**: Go to the [Security tab](https://github.com/onlydole/forklift/security/advisories) and click "Report a vulnerability"
2. **Email**: Contact the repository owner directly through GitHub

Please include the following information in your report:

- Type of vulnerability
- Full paths of source file(s) related to the vulnerability
- The location of the affected source code (tag/branch/commit or direct URL)
- Any special configuration required to reproduce the issue
- Step-by-step instructions to reproduce the issue
- Proof-of-concept or exploit code (if possible)
- Impact of the issue, including how an attacker might exploit it

### What to Expect

- **Acknowledgment**: We will acknowledge receipt of your vulnerability report within 48 hours
- **Updates**: We will send you regular updates about our progress (at least every 5 business days)
- **Timeline**: We aim to disclose vulnerabilities within 90 days of receiving the report
- **Credit**: If you would like, we will credit you in the security advisory and release notes

### Security Best Practices for Users

When using Forklift, please follow these security best practices:

1. **Token Management**:
   - Never commit your GitHub token to version control
   - Use environment variables or `.env` files (ensure `.env` is in `.gitignore`)
   - Rotate tokens regularly
   - Use tokens with minimal required permissions (public repository read access is sufficient)

2. **Keep Updated**:
   - Regularly update to the latest version to receive security patches
   - Monitor the [Security Advisories](https://github.com/onlydole/forklift/security/advisories) page

3. **Dependency Security**:
   - This project uses automated security audits through GitHub Actions
   - Review the [security workflow results](https://github.com/onlydole/forklift/actions) regularly

4. **Safe Execution**:
   - Only run Forklift from trusted sources
   - Verify the integrity of downloaded binaries
   - Be cautious when using tokens with elevated permissions

## Security Update Process

When a security vulnerability is confirmed:

1. A security advisory will be created
2. A fix will be developed in a private repository fork
3. The fix will be tested thoroughly
4. A new version will be released with the security patch
5. The security advisory will be published with details

## Scope

The following are **in scope** for security reports:

- Authentication bypass
- Unauthorized access to data
- Command injection
- Code injection
- Information disclosure
- API token leakage
- Dependency vulnerabilities with exploitable impact

The following are **out of scope**:

- Denial of Service attacks requiring excessive resources
- Issues in dependencies without proof of exploitability in Forklift
- Social engineering attacks
- Issues requiring physical access to a user's machine

## Contact

For any questions about this security policy, please open an issue in the repository or contact the maintainers.

## Attribution

We would like to thank all security researchers who have responsibly disclosed vulnerabilities to us.

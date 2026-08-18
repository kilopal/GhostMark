# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.13.x  | ✅ Yes             |
| < 0.13  | ❌ No              |

## Reporting a Vulnerability

If you discover a security vulnerability in GhostMark, please report it responsibly.

**Do NOT open a public GitHub issue for security vulnerabilities.**

Instead, please use [GitHub's private vulnerability reporting](https://github.com/kilopal/GhostMark/security/advisories/new) to submit your report. We will acknowledge receipt within 48 hours and work with you to understand and address the issue.

### What to include

- A description of the vulnerability
- Steps to reproduce the issue
- The potential impact
- Any suggested fixes (if applicable)

## Scope

GhostMark processes user-provided files and text locally. Security concerns include:

- **File parsing vulnerabilities** in the Rust core (buffer overflows, panics on malformed input)
- **WASM sandbox escapes** in the browser extension or playground
- **Supply chain attacks** on dependencies

## Out of Scope

- The effectiveness of watermark removal (this is a feature question, not a security issue)
- Social engineering attacks
- Denial of service on the CLI tool (it runs locally)

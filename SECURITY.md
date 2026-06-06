# Security Policy

## Supported Versions

| Version | Supported |
|---|---|
| 0.x.x | ✅ |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

If you discover a security vulnerability, please send an email to:
**security@hypaper.dev** (or open a private GitHub Security Advisory)

Please include:
- A description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

You will receive a response within 48 hours.
We will work with you to understand and resolve the issue before any public disclosure.

## Scope

Security issues in scope:
- `hypaper-daemon` IPC socket
- `.hyscene` scene parser (malformed input, path traversal)
- Rhai script sandboxing bypass
- `hypaper-server` API (when released)

Out of scope:
- Issues in third-party dependencies (report to upstream)
- Issues requiring physical access to the machine

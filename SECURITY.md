# Security Policy

## Supported versions

Security fixes are made on the `main` branch and included in the next release.

| Version | Supported |
|---|---|
| `main` | Yes |
| Older releases | No |

## Reporting a vulnerability

**Please don't report security issues in public issues, discussions or pull requests.**

Report them privately through GitHub:

1. Go to the repository's [Security tab](https://github.com/AbnormallyGrim/uma-stable-match/security).
2. Select **Report a vulnerability**.
3. Describe the issue, how to reproduce it, and its possible impact.

You can expect an acknowledgement within 7 days. Confirmed issues will be fixed as quickly as possible, and you'll be credited in the advisory unless you'd prefer not to be.

## Scope

Examples of issues worth reporting:

- API keys or other secrets exposed in responses, logs, error messages or the web interface
- Ways to make the server send requests on a user's behalf, or to reach hosts other than uma.moe
- Bypassing rate limits in a way that could overload the uma.moe API
- Cross-site scripting or injection in the web interface
- Vulnerable dependencies that affect this project

Out of scope:

- Vulnerabilities in [uma.moe](https://uma.moe) itself. Please report those to the uma.moe maintainers.
- Issues in the game itself. Please report those to Cygames, Inc.

## Keeping your own instance secure

If you run Uma Stable Match yourself:

- Keep your uma.moe API key in `.env` or an environment variable, never in code or committed files.
- Rate limiting is always on. Behind a reverse proxy, set `TRUST_FORWARDED_FOR=true` so each visitor gets their own limit; otherwise everyone shares the proxy's.
- Only set `TRUST_FORWARDED_FOR=true` behind a proxy that sets `X-Forwarded-For`, or visitors could pick their own address.
- If your key leaks, revoke it at [uma.moe/settings](https://uma.moe/settings) and create a new one.

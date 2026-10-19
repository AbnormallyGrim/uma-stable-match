# Contributing to Uma Stable Match

Thanks for your interest in contributing! Bug reports, ideas and pull requests are all welcome.

By taking part, you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Ways to contribute

- **Report a bug:** [open a bug report](https://github.com/AbnormallyGrim/uma-stable-match/issues/new?template=bug_report.yml).
- **Suggest a feature:** [open a feature request](https://github.com/AbnormallyGrim/uma-stable-match/issues/new?template=feature_request.yml).
- **Ask a question or share an idea:** start a thread in [Discussions](https://github.com/AbnormallyGrim/uma-stable-match/discussions).
- **Report a security issue:** follow the [Security Policy](SECURITY.md). Don't open a public issue.

## Workflow

Every change starts with an issue.

1. **Open or pick an issue.** For anything larger than a small fix, discuss the approach in the issue first.
2. **Create a branch from `main`**, named after the issue number and a short description:

   ```
   <issue-number>-<short-description>
   ```

   For example: `12-balance-score-weighting`.
3. **Make your changes**, with tests where it makes sense.
4. **Open a pull request** into `main`. Fill in the template, and include `Closes #<issue-number>` so the issue closes when the PR is merged.
5. **Pass the checks.** CI must be green before merging.

Pull requests are squash-merged, so each one becomes a single commit on `main`.

## Commit messages

Use the format:

```
<type>: <summary> (#<issue-number>)
```

| Type | Use for |
|---|---|
| `feat` | New features |
| `fix` | Bug fixes |
| `docs` | Documentation only |
| `test` | Adding or updating tests |
| `refactor` | Code changes that don't change behaviour |
| `build` | Build system, workspace or dependency changes |
| `ci` | CI configuration |
| `chore` | Other maintenance |

Example: `feat: add balance weight option (#12)`

## Checks before you push

### Rust

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### Web frontend

Once the `frontend/` app exists:

```bash
cd frontend
npm run check
npm test
```

## Keep personal information out

Everything committed to this repository is public, including commit history. Before committing, make sure your changes contain:

- **No API keys, tokens or passwords.** Use `.env`, which is git-ignored.
- **No personal details:** trainer IDs, real names, email addresses, file paths containing your username, or screenshots showing any of these.
- **No real player or club data** in tests or examples. Use made-up names and IDs.

To keep your email address out of commits, turn on **Keep my email addresses private** in your [GitHub email settings](https://github.com/settings/emails) and commit with your `@users.noreply.github.com` address.

## Code guidelines

- Keep the `scoring` crate free of network and storage code. Data goes in, scores come out.
- `unsafe` code isn't allowed.
- Prefer small, focused pull requests.

### Documentation

- Give every public item a short comment in the code. Missing documentation is flagged by the linter.
- Keep code comments brief. Add an inline comment only where the code isn't obvious.
- Put longer explanations, such as how something works, formulas and examples, in the crate's or app's `README.md`. A crate's README is also its `cargo doc` front page, and its code examples run as tests.

### Tests

- **Unit tests** go in a `tests.rs` file next to the module they test, declared at the bottom of the module:

  ```rust
  #[cfg(test)]
  mod tests;
  ```

  For example, tests for `src/scoring.rs` live in `src/scoring/tests.rs` and can test private functions.
- **Tests of the public API** go in the crate's `tests/` folder.
- Use made-up data in tests, never real player or club data.

## License

By contributing, you agree that your contributions are licensed under the project's [MIT License](LICENSE).

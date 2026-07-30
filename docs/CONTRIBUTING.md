# Contributing

## Branch protection (Closes #795)

`main` should be configured with the following GitHub branch protection
rules (Settings → Branches → Add rule for `main`):

- **Require a pull request before merging** — no direct pushes to `main`.
- **Require review from Code Owners** — changes under `contracts/bridge`,
  `contracts/lending`, and `contracts/oracle` must be approved by the matching
  owners defined in `.github/CODEOWNERS`.
- **Require signed commits** — every commit on `main` must be GPG/SSH signed.
- **Disallow force pushes** to `main`.
- **Disallow branch deletion** for `main`.
- **Require status checks to pass before merging** (CI must be green).

These are repo *settings*, not code, so they can't be applied via this PR —
an org admin needs to toggle them in the repository settings UI or via the
GitHub API (`PUT /repos/{owner}/{repo}/branches/main/protection`). This doc
exists so the expected configuration is written down and reviewable, and so
CI/README can reference a single source of truth for the policy.

The repository now tracks these security-sensitive paths in
`.github/CODEOWNERS`:

- `contracts/bridge`
- `contracts/lending`
- `contracts/oracle`

## Signing your commits

```
git config commit.gpgsign true
git config user.signingkey <your-key-id>
```

See [GitHub's guide to signing commits](https://docs.github.com/en/authentication/managing-commit-signature-verification)
for generating and registering a key.

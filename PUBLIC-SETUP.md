# Public Repository Setup

This checklist turns the repository into the intended public project with CI, security scanning, dependency automation, and signed build provenance.

## 1. Back up the current repository

On the Linux host:

```bash
cd /opt/laliga-dns-evade
git status
git log --oneline -10
```

Do not continue with uncommitted changes that you do not understand.

## 2. Review the repository for secrets before making it public

Current tree:

```bash
git grep -nEi \
  'password|passwd|secret|token|api[_-]?key|authorization|bearer|private[_-]?key'
```

History:

```bash
git log -p --all
```

Also review `.gitignore` and confirm that keys, certificates with private material, `.env` files, and local credentials are excluded.

A `.gitignore` entry does not remove data already committed to history.

## 3. Copy the public-project files

Copy these files into the repository:

```text
README.md
SECURITY.md
CHANGELOG.md
PUBLICATION-CHECKLIST.md
docs/INSTALL.md
docs/ARCHITECTURE.md
docs/CONFIGURATION.md
docs/OPERATIONS.md
docs/TROUBLESHOOTING.md
docs/RELEASES.md
.github/workflows/ci.yml
.github/workflows/security.yml
.github/workflows/release.yml
.github/dependabot.yml
```

Preserve the existing:

```text
LICENSE
Cargo.toml
Cargo.lock
src/
scripts/
systemd/
```

## 4. Validate locally

```bash
cd /opt/laliga-dns-evade

cargo check --locked
cargo test --locked
cargo build --release --locked

git diff --check
git status --short
```

Review the complete diff:

```bash
git diff
```

## 5. Commit the public-project changes

```bash
git add \
  README.md \
  SECURITY.md \
  CHANGELOG.md \
  PUBLICATION-CHECKLIST.md \
  docs \
  .github

git diff --cached --check
git diff --cached --stat

git commit -m "Prepare public project documentation and automation"
git push
```

## 6. Make the repository public

On GitHub:

```text
Repository
-> Settings
-> General
-> Danger Zone
-> Change repository visibility
-> Public
```

Read GitHub's visibility-change warnings before confirming.

## 7. Configure repository metadata

On the repository main page, edit the About section.

Suggested description:

```text
DNS response rewriting proxy for mitigating collateral IP blocking on shared CDN infrastructure.
```

Suggested topics:

```text
dns
rust
cloudflare
dns-proxy
svcb
https-record
ipv6
systemd
networking
```

Set the website field only if you have a project page worth linking.

## 8. Enable security features

Go to:

```text
Settings
-> Advanced Security
```

Enable or confirm:

```text
Dependency graph
Dependabot alerts
Dependabot security updates
```

Dependency Review uses the dependency graph.

## 9. Enable CodeQL

For this small Rust repository, use GitHub CodeQL default setup.

Go to:

```text
Settings
-> Advanced Security
-> CodeQL analysis
-> Set up
-> Default
```

After enabling it, wait for the first analysis to finish and check:

```text
Security
-> Code scanning
```

Do not describe CodeQL as a formal security audit.

## 10. Enable Private Vulnerability Reporting

Go to:

```text
Settings
-> Advanced Security
-> Private vulnerability reporting
```

Enable it if the option is available.

That gives researchers a private channel instead of forcing security reports into public Issues.

## 11. Verify GitHub Actions

After the documentation/automation commit reaches `main`, open:

```text
Actions
```

Confirm:

```text
CI        -> passing
Security  -> passing
```

The Dependency Review job only runs for pull requests.

## 12. Configure branch protection or a ruleset

Recommended minimum for `main`:

```text
Settings
-> Rules
-> Rulesets
```

Create a branch ruleset for `main`.

Recommended controls:

```text
Require a pull request before merging
Require status checks to pass
Block force pushes
Block branch deletion
```

Once the workflows have run at least once, require the relevant checks:

```text
Rust checks
RustSec cargo-audit
Dependency review
```

For a one-person repository, requiring PRs is optional. Requiring passing checks before merge is still useful.

## 13. Verify README badges

After Actions and CodeQL have run, confirm the badges render correctly on the repository front page.

Expected badges include:

```text
CI
Security
Latest Release
Rust
License
Platforms
Security scanning
DNS
```

The `Security scanning` badge is informational. The actual security status is represented by the Security workflow and GitHub Code Scanning results.

## 14. Prepare v0.1.0

Confirm:

```bash
grep -A5 '^\[package\]' Cargo.toml
```

Expected:

```text
version = "0.1.0"
```

Run:

```bash
cargo check --locked
cargo test --locked
cargo build --release --locked
git diff --check
git status
```

Do not tag unless the working tree is clean.

## 15. Publish v0.1.0

```bash
git tag -a v0.1.0 -m "v0.1.0"
git push origin v0.1.0
```

Open:

```text
GitHub
-> Actions
-> Release
```

Wait until both architecture builds and the publish job succeed.

Then open:

```text
GitHub
-> Releases
-> v0.1.0
```

Expected assets:

```text
laliga-dns-evade-v0.1.0-linux-amd64.tar.gz
laliga-dns-evade-v0.1.0-linux-arm64.tar.gz
SHA256SUMS
```

## 16. Verify the release

Download the release assets and verify:

```bash
sha256sum -c SHA256SUMS
```

Verify provenance:

```bash
gh attestation verify \
  laliga-dns-evade-v0.1.0-linux-arm64.tar.gz \
  --repo vdias/laliga-dns-evade
```

The provenance should point back to:

```text
vdias/laliga-dns-evade
.github/workflows/release.yml
the v0.1.0 commit/tag
```

## 17. Final checks

Repository front page:

```text
README renders correctly
badges render
documentation links work
license detected
latest release badge shows v0.1.0
```

Security:

```text
CodeQL completed
cargo-audit workflow passing
Dependabot alerts enabled
Private Vulnerability Reporting enabled
```

Release:

```text
amd64 archive present
arm64 archive present
SHA256SUMS present
artifact attestations verify
```

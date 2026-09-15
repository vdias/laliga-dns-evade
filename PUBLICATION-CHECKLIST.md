# Publication Checklist

Use this checklist before changing the repository visibility to public.

## Repository content

- [ ] README reviewed.
- [ ] LICENSE present and intentional.
- [ ] SECURITY.md present.
- [ ] CHANGELOG.md present.
- [ ] Installation documentation reviewed.
- [ ] Architecture documentation reviewed.
- [ ] Runtime paths contain no private infrastructure information that should remain private.
- [ ] Example domains and IPs are safe to publish.

## Secrets

- [ ] Current tree searched for passwords, tokens, API keys, private keys, and credentials.
- [ ] Git history reviewed for secrets.
- [ ] `.gitignore` covers local secrets and private key material.
- [ ] No `.env`, private certificate, PFX, SSH key, or API credential is tracked.
- [ ] Any previously committed secret has been rotated before publication.

## Code quality

- [ ] `cargo check --locked` passes.
- [ ] `cargo test --locked` passes.
- [ ] `cargo build --release --locked` passes.
- [ ] `git diff --check` passes.
- [ ] Working tree is clean before tagging.

## GitHub security

- [ ] Dependency graph enabled.
- [ ] Dependabot alerts enabled.
- [ ] Dependabot security updates enabled.
- [ ] CodeQL default setup enabled.
- [ ] First CodeQL analysis completed successfully.
- [ ] Private Vulnerability Reporting enabled.
- [ ] Security workflow passing.

## GitHub Actions

- [ ] CI workflow passing.
- [ ] Security workflow passing.
- [ ] Dependency review tested on a pull request.
- [ ] Release workflow present.
- [ ] Actions permissions reviewed.

## Release

- [ ] `Cargo.toml` version matches intended tag.
- [ ] Semantic version tag created.
- [ ] amd64 archive published.
- [ ] ARM64 archive published.
- [ ] `SHA256SUMS` published.
- [ ] Artifact attestations generated.
- [ ] At least one release archive verified with `gh attestation verify`.

## Repository presentation

- [ ] Description configured.
- [ ] Topics configured.
- [ ] README badges render correctly.
- [ ] Documentation links resolve.
- [ ] Latest release badge resolves.
- [ ] Acknowledgements are visible.

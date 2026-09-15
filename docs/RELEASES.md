# Release Process

GitHub Releases are generated automatically from semantic version tags.

## Published targets

The release workflow builds natively on GitHub-hosted Linux runners:

| Asset | GitHub runner | Architecture |
| --- | --- | --- |
| `linux-amd64` | `ubuntu-24.04` | x86_64 |
| `linux-arm64` | `ubuntu-24.04-arm` | ARM64 |

The ARM64 build is not cross-compiled. It is built natively on GitHub's ARM64 hosted runner.

## Release contents

For a tag such as `v0.1.0`, the release contains:

```text
laliga-dns-evade-v0.1.0-linux-amd64.tar.gz
laliga-dns-evade-v0.1.0-linux-arm64.tar.gz
SHA256SUMS
```

Each archive contains:

```text
laliga-dns-evade-v0.1.0-linux-<arch>/
├── laliga-dns-evade
├── LICENSE
└── README.md
```

## Release preflight

Before creating a release tag:

```bash
cargo check --locked
cargo test --locked
cargo build --release --locked

git diff --check
git status
```

The working tree must be clean.

Confirm the version in `Cargo.toml` matches the intended tag.

For `v0.1.0`:

```toml
[package]
version = "0.1.0"
```

## Create the release

Create an annotated tag:

```bash
git tag -a v0.1.0 -m "v0.1.0"
git push origin v0.1.0
```

That push starts `.github/workflows/release.yml`.

The workflow:

1. validates that the tag matches `Cargo.toml`;
2. runs the unit tests on x86_64 and ARM64;
3. builds the release binary natively on each architecture;
4. packages each binary;
5. generates a GitHub artifact attestation for each archive;
6. generates `SHA256SUMS`;
7. creates the GitHub Release and uploads all assets.

Do not manually upload a locally compiled production binary to the release.

## Verify checksums

Download all assets into one directory and run:

```bash
sha256sum -c SHA256SUMS
```

## Verify build provenance

Install/authenticate GitHub CLI and run:

```bash
gh attestation verify \
  laliga-dns-evade-v0.1.0-linux-arm64.tar.gz \
  --repo vdias/laliga-dns-evade
```

Repeat for the amd64 archive if required.

## Failed release workflow

If a build, test, attestation, or publication step fails, do not reuse the same release as though it had succeeded.

Fix the cause in `main`, update the package version if necessary, and create a new semantic version tag.

For a pre-release project, a failed unpublished `v0.1.0` tag may be deleted and recreated only if no public release or consumer has relied on it. Once published, prefer a new version.

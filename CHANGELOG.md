# Changelog

All notable changes to this project are documented here.

## [Unreleased]

## [0.1.0] - 2026-09-15

### Added

- DNS proxy over UDP and TCP.
- IPv4 `A` record inspection and rewriting.
- IPv6 `AAAA` record inspection and rewriting.
- Cloudflare IPv4 and IPv6 prefix validation.
- In-place DNS address replacement.
- DNSSEC `AD` flag clearing after response modification.
- Blocked non-Cloudflare address detection and logging.
- HTTPS/SVCB parsing.
- HTTPS/SVCB `ipv4hint` and `ipv6hint` rewriting.
- Unit tests covering IPv4, IPv6, rewriting, DNSSEC flag handling, network matching, and HTTPS/SVCB hints.

### Documentation

- Redesigned README for public release.
- Added installation, architecture, configuration, operations, troubleshooting, release, and security documentation.
- Added acknowledgements for ¿Hay ahora fútbol? and `Oihalitz/xdp-dns-evadeproxy`.

### Automation

- Added GitHub Actions CI.
- Added RustSec dependency auditing.
- Added pull-request dependency review.
- Added Dependabot configuration.
- Added automated amd64 and ARM64 GitHub Releases.
- Added SHA-256 checksums for release archives.
- Added GitHub artifact attestations for release build provenance.

### Changed

- DNS caching moved out of the rewriting layer and into the upstream resolver/cache.
- The rewriting layer remains cacheless so returned responses are evaluated against current runtime policy.

### Validated

- `cargo check` successful.
- 19 unit tests passing.
- Release build successful.
- End-to-end HTTPS `ipv4hint` rewrite validated with a temporary blocked Cloudflare address.

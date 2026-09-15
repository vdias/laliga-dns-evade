# LaLiga DNS Evade

[![CI](https://github.com/vdias/laliga-dns-evade/actions/workflows/ci.yml/badge.svg)](https://github.com/vdias/laliga-dns-evade/actions/workflows/ci.yml)
[![Security](https://github.com/vdias/laliga-dns-evade/actions/workflows/security.yml/badge.svg)](https://github.com/vdias/laliga-dns-evade/actions/workflows/security.yml)
[![Latest Release](https://img.shields.io/github/v/release/vdias/laliga-dns-evade?display_name=tag)](https://github.com/vdias/laliga-dns-evade/releases/latest)
[![Rust](https://img.shields.io/badge/Rust-1.93%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/github/license/vdias/laliga-dns-evade)](LICENSE)
[![Platforms](https://img.shields.io/badge/Linux-amd64%20%7C%20arm64-blue?logo=linux)](https://github.com/vdias/laliga-dns-evade/releases)
[![Security scanning](https://img.shields.io/badge/security-CodeQL%20%2B%20RustSec-success?logo=github)](SECURITY.md)
[![DNS](https://img.shields.io/badge/DNS-A%20%7C%20AAAA%20%7C%20HTTPS%20%7C%20SVCB-informational)](docs/ARCHITECTURE.md)

A lightweight DNS response rewriting proxy for mitigating collateral IP blocking on shared Cloudflare infrastructure.

`laliga-dns-evade` sits between a DNS frontend and an upstream resolver. It inspects DNS responses and selectively rewrites blocked Cloudflare addresses to an alternative address inside the same Cloudflare network.

The proxy supports IPv4 `A`, IPv6 `AAAA`, and HTTPS/SVCB address hints (`ipv4hint` / `ipv6hint`).

## Why this exists

Some ISP blocking events target IP addresses instead of individual hostnames. On shared CDN infrastructure, a single IP can serve many unrelated websites, so blocking that IP can make legitimate services unreachable as collateral damage.

This project does not tunnel traffic, bypass application authentication, or act as a VPN. It only modifies DNS responses when a blocked address is both:

1. present in the configured blocklist; and
2. inside a configured Cloudflare prefix.

Blocked non-Cloudflare addresses are detected and logged, but are not rewritten.

## Features

- DNS proxy over UDP and TCP.
- IPv4 `A` record inspection and rewriting.
- IPv6 `AAAA` record inspection and rewriting.
- HTTPS/SVCB `ipv4hint` and `ipv6hint` inspection and rewriting.
- Cloudflare prefix validation before address replacement.
- IPv4 and IPv6 support.
- In-place DNS packet modification.
- DNSSEC AD flag cleared when a response is modified.
- No cache inside the rewriting layer.
- Small Rust codebase with unit tests.
- Designed for systemd deployments.
- Works with an external DNS frontend and an external resolver/cache.

## Architecture

Reference deployment:

```text
DNS clients
    |
    | DoH / DoT / DoQ
    v
+----------------------+
| AdGuard Home         |
| Filtering            |
| DNS cache disabled   |
+----------+-----------+
           |
           | UDP/TCP 127.0.0.1:5335
           v
+-------------------------------+
| laliga-dns-evade              |
|                               |
| blocked-any.txt               |
| cloudflare-v4.txt             |
| cloudflare-v6.txt             |
|                               |
| A / AAAA / HTTPS / SVCB       |
| selective response rewriting  |
+---------------+---------------+
                |
                | UDP/TCP 127.0.0.1:5336
                v
+-------------------------------+
| dnsproxy                      |
| cache + optimistic cache      |
| encrypted upstream resolvers  |
+---------------+---------------+
                |
                v
             Internet
```

The Rust proxy itself does not require AdGuard Home. Any DNS frontend capable of forwarding standard DNS over UDP/TCP to `127.0.0.1:5335` can be used.

Likewise, `dnsproxy` is part of the reference deployment rather than a hard dependency of the core proxy.

## How rewriting works

For each response, the proxy inspects address-bearing records:

- `A` (`TYPE 1`)
- `AAAA` (`TYPE 28`)
- `SVCB` (`TYPE 64`)
- `HTTPS` (`TYPE 65`)
- `ipv4hint` (`SvcParamKey 4`)
- `ipv6hint` (`SvcParamKey 6`)

When an address is both blocked and contained in a Cloudflare prefix, the proxy searches for an unblocked alternative address inside that same Cloudflare network and rewrites the DNS response in place.

Example:

```text
original HTTPS response:
ipv4hint=104.16.132.229,104.16.133.229

104.16.132.229 is blocked

rewritten HTTPS response:
ipv4hint=104.16.132.230,104.16.133.229
```

Addresses outside Cloudflare are deliberately left unchanged. An arbitrary neighboring address in another CDN or hosting network is not guaranteed to serve the same hostname.

## Releases

Pre-built Linux binaries are published through [GitHub Releases](https://github.com/vdias/laliga-dns-evade/releases):

- Linux x86_64 / amd64
- Linux ARM64 / aarch64

Release archives are built natively by GitHub Actions from the tagged commit and include SHA-256 checksums.

GitHub artifact attestations are generated for each release archive so build provenance can be verified.

Example:

```bash
sha256sum -c SHA256SUMS
```

With GitHub CLI:

```bash
gh attestation verify \
  laliga-dns-evade-v0.1.0-linux-arm64.tar.gz \
  --repo vdias/laliga-dns-evade
```

See [Release process](docs/RELEASES.md).

## DNSSEC considerations

A DNS response that has been modified can no longer be considered the cryptographically authenticated response returned by the upstream resolver.

Whenever `laliga-dns-evade` rewrites a response, it clears the DNS `AD` (Authenticated Data) flag before sending the response to the client.

The proxy does not perform DNSSEC validation itself. Validation belongs to the upstream resolver.

See [Security](SECURITY.md) for details.

## Quick start

Build:

```bash
cargo check --locked
cargo test --locked
cargo build --release --locked
```

The reference runtime expects:

```text
/var/lib/laliga-dns-evade/blocked-any.txt
/var/lib/laliga-dns-evade/cloudflare-v4.txt
/var/lib/laliga-dns-evade/cloudflare-v6.txt
```

Default proxy path:

```text
listen:   127.0.0.1:5335 UDP/TCP
upstream: 127.0.0.1:5336 UDP/TCP
```

For the complete installation procedure, see [docs/INSTALL.md](docs/INSTALL.md).

## Documentation

- [Installation](docs/INSTALL.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Configuration](docs/CONFIGURATION.md)
- [Operations](docs/OPERATIONS.md)
- [Troubleshooting](docs/TROUBLESHOOTING.md)
- [Release process](docs/RELEASES.md)
- [Security](SECURITY.md)
- [Changelog](CHANGELOG.md)

## Current scope and limitations

Implemented:

- IPv4 and IPv6 DNS address inspection.
- Cloudflare-only address rewriting.
- UDP and TCP DNS transport.
- HTTPS/SVCB `ipv4hint` and `ipv6hint` rewriting.
- Blocked non-Cloudflare address detection and logging.
- DNSSEC AD flag handling after modification.

Not implemented:

- Generic rewriting for arbitrary CDNs or hosting providers.
- Residential probing.
- Hostname-specific verified redirect pools.
- VPN, HTTP proxy, or traffic tunneling functionality.
- DNSSEC validation inside the proxy.
- Built-in DNS cache.

These limitations are intentional. Replacing an address outside Cloudflare without validating that the replacement serves the same hostname could break connectivity or direct clients to the wrong service.

## Security

Automated security controls include:

- GitHub CodeQL static analysis.
- RustSec `cargo-audit`.
- GitHub dependency review on pull requests.
- Dependabot dependency updates and alerts.
- GitHub artifact attestations for published release archives.

These controls are automated checks, not a formal independent security audit.

See [SECURITY.md](SECURITY.md).

## Acknowledgements

This project would not exist in its current form without the work and public resources provided by:

- **[¿Hay ahora fútbol?](https://hayahora.futbol/)**
  Provides public, automatically updated information about IP addresses affected by ISP blocking in Spain. The reference updater consumes:
  `https://hayahora.futbol/estado/blocked-any.txt`

- **[Oihalitz/xdp-dns-evadeproxy](https://github.com/Oihalitz/xdp-dns-evadeproxy)**
  The original project and main inspiration for the DNS response rewriting approach used here. Its work around Cloudflare anycast rewriting and HTTPS/SVCB address hints strongly influenced this implementation.

`laliga-dns-evade` is an independent implementation and is not a fork of `xdp-dns-evadeproxy`.

Thanks to both projects for making their work and data publicly available.

## Responsible use

This project is intended to preserve access to legitimate services affected by collateral IP blocking.

Users are responsible for complying with the laws, policies, and service terms applicable to their environment.

## License

See [LICENSE](LICENSE).

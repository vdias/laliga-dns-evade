# Installation

This document covers the core `laliga-dns-evade` systemd deployment.

## Requirements

- Linux with systemd.
- Rust/Cargo when building from source.
- `dig` for DNS verification.
- Local upstream DNS resolver reachable over UDP/TCP.
- Root privileges for installation.

The proxy has been production-tested on Linux ARM64. GitHub Releases also provide an amd64 build produced by CI.

## Install from GitHub Release

Download the archive for your architecture and `SHA256SUMS` from the selected GitHub Release.

Verify:

```bash
sha256sum -c SHA256SUMS
```

Optional provenance verification:

```bash
gh attestation verify \
  laliga-dns-evade-v0.1.0-linux-arm64.tar.gz \
  --repo vdias/laliga-dns-evade
```

Extract:

```bash
tar -xzf laliga-dns-evade-v0.1.0-linux-arm64.tar.gz
cd laliga-dns-evade-v0.1.0-linux-arm64
```

Install:

```bash
sudo install \
  -o root \
  -g root \
  -m 0755 \
  laliga-dns-evade \
  /usr/local/bin/laliga-dns-evade
```

## Build from source

```bash
git clone https://github.com/vdias/laliga-dns-evade.git
cd laliga-dns-evade

cargo check --locked
cargo test --locked
cargo build --release --locked
```

Install:

```bash
sudo install \
  -o root \
  -g root \
  -m 0755 \
  target/release/laliga-dns-evade \
  /usr/local/bin/laliga-dns-evade
```

## Runtime account

```bash
sudo useradd \
  --system \
  --no-create-home \
  --shell /usr/sbin/nologin \
  laliga-dns-evade
```

If the account already exists, do not recreate it.

## Runtime directory

```bash
sudo install \
  -d \
  -o root \
  -g laliga-dns-evade \
  -m 0750 \
  /var/lib/laliga-dns-evade
```

Required files:

```text
/var/lib/laliga-dns-evade/blocked-any.txt
/var/lib/laliga-dns-evade/cloudflare-v4.txt
/var/lib/laliga-dns-evade/cloudflare-v6.txt
```

Reference sources:

```text
https://hayahora.futbol/estado/blocked-any.txt
https://www.cloudflare.com/ips-v4
https://www.cloudflare.com/ips-v6
```

Install validated files with:

```text
owner: root
group: laliga-dns-evade
mode:  0640
```

## Upstream resolver

The reference build expects:

```text
127.0.0.1:5336
```

The upstream must answer normal DNS over UDP and TCP.

## systemd

Install the repository unit:

```bash
sudo install \
  -o root \
  -g root \
  -m 0644 \
  systemd/laliga-dns-evade.service \
  /etc/systemd/system/laliga-dns-evade.service

sudo systemctl daemon-reload
sudo systemctl enable --now laliga-dns-evade.service
```

Verify:

```bash
sudo systemctl status laliga-dns-evade.service --no-pager -l
```

## Functional validation

```bash
dig @127.0.0.1 -p 5335 cloudflare.com A +short
dig @127.0.0.1 -p 5335 cloudflare.com AAAA +short
dig @127.0.0.1 -p 5335 cloudflare.com HTTPS +short
```

Journal:

```bash
sudo journalctl \
  -u laliga-dns-evade.service \
  --since "5 minutes ago" \
  --no-pager
```

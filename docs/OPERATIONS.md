# Operations

## Health

```bash
sudo systemctl is-active laliga-dns-evade.service
```

Expected:

```text
active
```

## Logs

```bash
sudo journalctl \
  -u laliga-dns-evade.service \
  --since "10 minutes ago" \
  --no-pager
```

## DNS checks

```bash
dig @127.0.0.1 -p 5335 cloudflare.com A +short
dig @127.0.0.1 -p 5335 cloudflare.com AAAA +short
dig @127.0.0.1 -p 5335 cloudflare.com HTTPS +short
```

## Source validation

Before deployment:

```bash
cargo check --locked
cargo test --locked
git diff --check
```

Build:

```bash
cargo build --release --locked
```

## Binary deployment

Backup:

```bash
sudo cp -a \
  /usr/local/bin/laliga-dns-evade \
  /usr/local/bin/laliga-dns-evade.previous
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

Restart:

```bash
sudo systemctl restart laliga-dns-evade.service
sudo systemctl is-active laliga-dns-evade.service
```

Run DNS checks after every binary deployment.

## Cache behavior

Do not flush the upstream DNS cache merely because the blocklist changed.

Cached DNS responses still pass through the rewriting layer and are evaluated against the currently loaded policy.

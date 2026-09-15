# Troubleshooting

Use one check at a time and branch from observed evidence.

## Service not active

```bash
sudo systemctl status laliga-dns-evade.service --no-pager -l
```

If needed:

```bash
sudo journalctl \
  -u laliga-dns-evade.service \
  -b \
  --no-pager
```

## Listener missing

```bash
sudo ss -lntup | grep '127\.0\.0\.1:5335'
```

## Upstream unavailable

```bash
sudo ss -lntup | grep '127\.0\.0\.1:5336'
```

Then:

```bash
dig @127.0.0.1 -p 5336 cloudflare.com A +short
```

If this fails, fix the upstream resolver first.

## Compare upstream and rewrite layer

```bash
dig @127.0.0.1 -p 5336 cloudflare.com A +short
dig @127.0.0.1 -p 5335 cloudflare.com A +short
```

## Confirm a blocked IP

```bash
sudo grep -Fx '<IP>' \
  /var/lib/laliga-dns-evade/blocked-any.txt
```

## HTTPS/SVCB

```bash
dig @127.0.0.1 -p 5335 example.com HTTPS +short
```

Inspect `ipv4hint` and `ipv6hint`.

## Non-Cloudflare blocked address

A log message such as:

```text
BLOCKED NON-CLOUDFLARE address detected
```

is expected behavior.

The project intentionally does not perform generic non-Cloudflare address rewriting.

## cargo fmt unavailable

`cargo fmt` is optional and requires `rustfmt`.

It is not required to build or run this project.

Use:

```bash
cargo check --locked
cargo test --locked
cargo build --release --locked
```

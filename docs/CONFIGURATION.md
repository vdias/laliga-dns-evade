# Configuration

## Listener

```text
127.0.0.1:5335
```

Transports:

```text
UDP
TCP
```

## Upstream

```text
127.0.0.1:5336
```

The upstream must provide standard DNS over UDP and TCP.

## Runtime files

```text
/var/lib/laliga-dns-evade/blocked-any.txt
/var/lib/laliga-dns-evade/cloudflare-v4.txt
/var/lib/laliga-dns-evade/cloudflare-v6.txt
```

Recommended permissions:

```text
directory:
  root:laliga-dns-evade 0750

files:
  root:laliga-dns-evade 0640
```

## Reload behavior

Runtime files are loaded at process startup and reloaded on `SIGHUP`.

After replacing validated runtime data:

```bash
sudo systemctl reload laliga-dns-evade.service
sudo systemctl is-active laliga-dns-evade.service
```

All three files are read and validated before they replace the data in memory. If any file is invalid, the service keeps the previous data and logs `Reload failed, keeping previous data`. Queries in flight finish with the data they started with. The listeners stay open, so no query is dropped.

Inspect startup counts:

```bash
sudo journalctl \
  -u laliga-dns-evade.service \
  -n 20 \
  --no-pager
```

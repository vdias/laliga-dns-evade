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

Runtime files are loaded at process startup.

After replacing validated runtime data:

```bash
sudo systemctl restart laliga-dns-evade.service
sudo systemctl is-active laliga-dns-evade.service
```

Inspect startup counts:

```bash
sudo journalctl \
  -u laliga-dns-evade.service \
  -n 20 \
  --no-pager
```

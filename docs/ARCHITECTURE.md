# Architecture

`laliga-dns-evade` is a DNS response rewriting layer, not a recursive resolver, cache, VPN, HTTP proxy, or traffic tunnel.

## Reference path

```text
client
  |
DNS frontend
  |
  | UDP/TCP 127.0.0.1:5335
  v
laliga-dns-evade
  |
  | UDP/TCP 127.0.0.1:5336
  v
resolver/cache
  |
upstream DNS
```

On the response path, cached and fresh responses pass through the rewriting layer before reaching clients.

## Runtime inputs

```text
/var/lib/laliga-dns-evade/blocked-any.txt
/var/lib/laliga-dns-evade/cloudflare-v4.txt
/var/lib/laliga-dns-evade/cloudflare-v6.txt
```

## Parsed DNS data

| Record / parameter | Number | Address family |
| --- | ---: | --- |
| A | 1 | IPv4 |
| AAAA | 28 | IPv6 |
| SVCB | 64 | depends on SvcParam |
| HTTPS | 65 | depends on SvcParam |
| ipv4hint | SvcParamKey 4 | IPv4 |
| ipv6hint | SvcParamKey 6 | IPv6 |

## Rewrite policy

```text
address in blocklist?
  no -> unchanged
  yes
    |
address inside Cloudflare prefix?
  no -> log and unchanged
  yes
    |
find unblocked replacement inside same Cloudflare network
  none -> log and unchanged
  found -> rewrite in place and clear DNS AD flag
```

## Why Cloudflare-only

A neighboring address in an arbitrary CDN prefix is not guaranteed to serve the same hostname, TLS certificate, or application.

The project therefore does not perform generic neighbor rewrites for non-Cloudflare addresses.

## HTTPS/SVCB

Modern clients may use `ipv4hint` or `ipv6hint` from HTTPS/SVCB records instead of relying exclusively on A/AAAA responses.

The proxy applies the same blocklist and Cloudflare policy to those hints.

## DNSSEC

If the upstream resolver sets `AD`, modifying the response invalidates the meaning of that flag for the modified packet.

The proxy clears `AD` whenever it rewrites an address.

## Cache placement

Reference deployment:

```text
DNS frontend cache: disabled
laliga-dns-evade:   no cache
upstream dnsproxy:  cache enabled
```

That arrangement allows cached responses to be evaluated against the current blocklist on every return path.

## Inspiration

This is an independent implementation inspired by:

```text
https://github.com/Oihalitz/xdp-dns-evadeproxy
```

The original project includes broader functionality. This project deliberately keeps a narrower Cloudflare-focused scope.

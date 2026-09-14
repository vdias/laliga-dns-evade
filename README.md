# LaLiga DNS Evade

`laliga-dns-evade` is a lightweight DNS response rewriting proxy written in Rust.

Its purpose is to detect DNS A/AAAA responses containing IP addresses present in a dynamic block list and, when the blocked address belongs to a Cloudflare network, replace it with a nearby non-blocked address from the same Cloudflare prefix.

The project is designed to run between AdGuard Home and an encrypted upstream DNS proxy.

## Architecture

Production flow:

```text
DNS client
    |
    v
Nginx / DoH / DoT / DoQ
    |
    v
AdGuard Home
    |
    | cache hit
    +--------------------> response
    |
    | cache miss
    v
127.0.0.1:5335
laliga-dns-evade
    |
    v
127.0.0.1:5336
AdGuard dnsproxy
    |
    v
DNS-over-HTTP/3 upstreams
    |
    +--> Cloudflare
    +--> Quad9
```

The updater runs independently from the Rust daemon:

```text
systemd timer
    |
    v
update-laliga-dns-evade.sh
    |
    +--> blocked-any.txt
    +--> Cloudflare IPv4 prefixes
    +--> Cloudflare IPv6 prefixes
    |
    v
Validate downloaded data
    |
    v
Compare hashes
    |
    +--> no changes -> exit
    |
    v
Atomic data replacement
    |
    v
Restart laliga-dns-evade
    |
    v
Verify DNS
    |
    v
Flush AdGuard Home cache
```

## Current capabilities

Implemented:

- UDP DNS proxy.
- TCP DNS proxy with DNS-over-TCP framing.
- IPv4 A record parsing.
- IPv6 AAAA record parsing.
- Detection against a dynamic IP block list.
- Cloudflare IPv4/IPv6 network detection.
- Selection of a nearby non-blocked Cloudflare address.
- In-place DNS RDATA rewriting.
- UDP A rewriting.
- UDP AAAA rewriting.
- TCP A rewriting.
- TCP AAAA rewriting.
- DNSSEC AD flag clearing when a signed DNS response is modified.
- Dynamic block list updates.
- Dynamic Cloudflare prefix updates.
- AdGuard Home cache invalidation after list changes.
- systemd services and timer.
- Rollback of data files if the evade service fails after an update.

Not implemented yet:

- HTTPS/SVCB `ipv4hint` / `ipv6hint` rewriting.
- Non-Cloudflare CDN rewriting.
- Akamai/Fastly/GitHub-specific verified redirect pools.
- Runtime list reload without restarting the daemon.
- Prometheus metrics.
- Residential probing.

Blocked non-Cloudflare addresses are currently detected and logged but are not rewritten.

---

# Validated platform

This deployment has been validated on:

```text
OS:           Ubuntu 26.04.1 LTS
Architecture: aarch64 / arm64
Rust:         1.93.1
Cargo:        1.93.1
dnsproxy:     v0.84.1
AdGuard Home: v0.107.79
```

The Rust application itself is portable, but package names, paths and the `dnsproxy` binary must be adjusted if another distribution or CPU architecture is used.

The deployment documented below assumes:

```text
AdGuard Home directory:
  /opt/AdGuardHome

AdGuard Home configuration:
  /opt/AdGuardHome/AdGuardHome.yaml

AdGuard Home local HTTP API:
  http://127.0.0.1:3001

laliga-dns-evade source:
  /opt/laliga-dns-evade

laliga-dns-evade listener:
  127.0.0.1:5335

dnsproxy listener:
  127.0.0.1:5336
```

---

# Repository layout

```text
laliga-dns-evade/
├── Cargo.lock
├── Cargo.toml
├── README.md
├── .gitignore
│
├── config/
│   └── adguard.env.example
│
├── scripts/
│   └── update-laliga-dns-evade.sh
│
├── src/
│   ├── blocklist.rs
│   ├── dns.rs
│   ├── main.rs
│   └── networks.rs
│
└── systemd/
    ├── dnsproxy.service
    ├── laliga-dns-evade.service
    ├── laliga-dns-evade-update.service
    └── laliga-dns-evade-update.timer
```

---

# 1. Server prerequisites

The server must already have:

- Working Internet connectivity.
- Working DNS resolution independent from this service.
- AdGuard Home installed and operational.
- Git access to the private repository.
- Root or sudo access.
- TCP/UDP ports `5335` and `5336` free on loopback.

Check architecture:

```bash
uname -m
```

Expected on the validated host:

```text
aarch64
```

Check Ubuntu version:

```bash
cat /etc/os-release
```

Install required packages:

```bash
sudo apt update

sudo apt install -y \
  ca-certificates \
  curl \
  git \
  cargo \
  rustc \
  dnsutils \
  python3
```

Verify:

```bash
rustc --version
cargo --version
git --version
curl --version
dig -v
```

---

# 2. Configure GitHub access

The repository is private.

A dedicated SSH deploy key is recommended instead of reusing a personal SSH key.

Create the SSH directory:

```bash
install -d -m 0700 ~/.ssh
```

Generate a dedicated key:

```bash
ssh-keygen \
  -t ed25519 \
  -f ~/.ssh/id_ed25519_github_laliga \
  -C "laliga-dns-evade-deploy"
```

For a deployment server, the GitHub Deploy Key only needs read access unless the server will also push development changes.

Display the public key:

```bash
cat ~/.ssh/id_ed25519_github_laliga.pub
```

In GitHub:

```text
Repository
  -> Settings
  -> Deploy keys
  -> Add deploy key
```

Add the public key.

Create the SSH alias:

```bash
cat >> ~/.ssh/config <<'EOF_SSH'

Host github-laliga-dns-evade
    HostName github.com
    User git
    IdentityFile ~/.ssh/id_ed25519_github_laliga
    IdentitiesOnly yes
EOF_SSH

chmod 0600 ~/.ssh/config
```

Validate GitHub authentication:

```bash
ssh -T git@github-laliga-dns-evade
```

GitHub normally reports that authentication succeeded but shell access is not provided.

---

# 3. Clone the repository

Create the application directory:

```bash
sudo install -d -o "$USER" -g "$USER" -m 0755 /opt/laliga-dns-evade
```

Clone:

```bash
git clone \
  git@github-laliga-dns-evade:vdias/laliga-dns-evade.git \
  /opt/laliga-dns-evade
```

Enter the repository:

```bash
cd /opt/laliga-dns-evade
```

Check the branch and revision:

```bash
git status
git log -1 --oneline
```

---

# 4. Build laliga-dns-evade

Run the test suite first:

```bash
cd /opt/laliga-dns-evade

cargo check
cargo test
```

All tests must succeed before installing the binary.

Build the release binary:

```bash
cargo build --release
```

Install it:

```bash
sudo install \
  -o root \
  -g root \
  -m 0755 \
  target/release/laliga-dns-evade \
  /usr/local/bin/laliga-dns-evade
```

Verify:

```bash
ls -lh /usr/local/bin/laliga-dns-evade
```

---

# 5. Install AdGuard dnsproxy

`dnsproxy` provides the encrypted HTTP/3 upstream layer.

The production deployment currently uses:

```text
dnsproxy v0.84.1
```

Download the official binary matching the server architecture from the AdGuardTeam `dnsproxy` GitHub Releases page.

For Linux ARM64 the expected release asset naming format is:

```text
dnsproxy-linux-arm64-v0.84.1.tar.gz
```

Example installation workflow:

```bash
cd /tmp

curl -fLO \
  https://github.com/AdguardTeam/dnsproxy/releases/download/v0.84.1/dnsproxy-linux-arm64-v0.84.1.tar.gz

tar -xzf dnsproxy-linux-arm64-v0.84.1.tar.gz

sudo install \
  -o root \
  -g root \
  -m 0755 \
  linux-arm64/dnsproxy \
  /usr/local/bin/dnsproxy
```

Verify:

```bash
/usr/local/bin/dnsproxy --version
```

Expected:

```text
dnsproxy version v0.84.1
```

The project intentionally disables the dnsproxy cache because AdGuard Home is the caching layer.

---

# 6. Create the service account

Create a dedicated unprivileged account:

```bash
sudo useradd \
  --system \
  --no-create-home \
  --shell /usr/sbin/nologin \
  laliga-dns-evade
```

If the account already exists, do not recreate it.

Check:

```bash
getent passwd laliga-dns-evade
```

---

# 7. Create persistent directories

Create configuration and data directories:

```bash
sudo install -d \
  -o root \
  -g root \
  -m 0750 \
  /etc/laliga-dns-evade

sudo install -d \
  -o root \
  -g laliga-dns-evade \
  -m 0750 \
  /var/lib/laliga-dns-evade
```

Dynamic files are stored under:

```text
/var/lib/laliga-dns-evade/blocked-any.txt
/var/lib/laliga-dns-evade/cloudflare-v4.txt
/var/lib/laliga-dns-evade/cloudflare-v6.txt
```

---

# 8. Bootstrap the initial data

The daemon requires its data files before it can start.

Download the block list:

```bash
sudo curl \
  --fail \
  --location \
  --silent \
  --show-error \
  --output /var/lib/laliga-dns-evade/blocked-any.txt \
  https://hayahora.futbol/estado/blocked-any.txt
```

Download Cloudflare IPv4 ranges:

```bash
sudo curl \
  --fail \
  --location \
  --silent \
  --show-error \
  --output /var/lib/laliga-dns-evade/cloudflare-v4.txt \
  https://www.cloudflare.com/ips-v4
```

Download Cloudflare IPv6 ranges:

```bash
sudo curl \
  --fail \
  --location \
  --silent \
  --show-error \
  --output /var/lib/laliga-dns-evade/cloudflare-v6.txt \
  https://www.cloudflare.com/ips-v6
```

Set ownership and permissions:

```bash
sudo chown \
  root:laliga-dns-evade \
  /var/lib/laliga-dns-evade/*.txt

sudo chmod \
  0640 \
  /var/lib/laliga-dns-evade/*.txt
```

Validate that the files are not empty:

```bash
awk 'NF && $1 !~ /^#/' \
  /var/lib/laliga-dns-evade/blocked-any.txt |
  wc -l

awk 'NF && $1 !~ /^#/' \
  /var/lib/laliga-dns-evade/cloudflare-v4.txt |
  wc -l

awk 'NF && $1 !~ /^#/' \
  /var/lib/laliga-dns-evade/cloudflare-v6.txt |
  wc -l
```

Cloudflare currently publishes 15 IPv4 prefixes and 7 IPv6 prefixes.

The block list is dynamic and its count changes frequently.

---

# 9. Configure AdGuard Home API credentials

The updater clears the AdGuard Home DNS cache whenever the block list or Cloudflare networks change.

Copy the example:

```bash
sudo install \
  -o root \
  -g root \
  -m 0600 \
  config/adguard.env.example \
  /etc/laliga-dns-evade/adguard.env
```

Edit:

```bash
sudo nano /etc/laliga-dns-evade/adguard.env
```

Example:

```text
AGH_USER=vdias
AGH_PASS=replace-with-the-real-password
```

Permissions must remain:

```bash
sudo chmod 0600 /etc/laliga-dns-evade/adguard.env
sudo chown root:root /etc/laliga-dns-evade/adguard.env
```

Verify:

```bash
sudo stat -c '%U %G %a %n' \
  /etc/laliga-dns-evade/adguard.env
```

Expected:

```text
root root 600 /etc/laliga-dns-evade/adguard.env
```

Never commit this file to Git.

Test the AdGuard Home cache API:

```bash
sudo bash -c '
set -a
source /etc/laliga-dns-evade/adguard.env
set +a

curl \
  --fail \
  --silent \
  --show-error \
  --user "${AGH_USER}:${AGH_PASS}" \
  --request POST \
  http://127.0.0.1:3001/control/cache_clear

echo "AdGuard cache flush: OK"
'
```

Expected:

```text
AdGuard cache flush: OK
```

---

# 10. Install the updater

Install the updater script:

```bash
sudo install \
  -o root \
  -g root \
  -m 0755 \
  scripts/update-laliga-dns-evade.sh \
  /usr/local/sbin/update-laliga-dns-evade.sh
```

Validate Bash syntax:

```bash
sudo bash -n \
  /usr/local/sbin/update-laliga-dns-evade.sh
```

Do not run the updater yet if the services have not been installed because the script performs service preflight checks.

---

# 11. Install systemd units

Install the units from the repository:

```bash
sudo install \
  -o root \
  -g root \
  -m 0644 \
  systemd/dnsproxy.service \
  /etc/systemd/system/dnsproxy.service

sudo install \
  -o root \
  -g root \
  -m 0644 \
  systemd/laliga-dns-evade.service \
  /etc/systemd/system/laliga-dns-evade.service

sudo install \
  -o root \
  -g root \
  -m 0644 \
  systemd/laliga-dns-evade-update.service \
  /etc/systemd/system/laliga-dns-evade-update.service

sudo install \
  -o root \
  -g root \
  -m 0644 \
  systemd/laliga-dns-evade-update.timer \
  /etc/systemd/system/laliga-dns-evade-update.timer

sudo systemctl daemon-reload
```

---

# 12. Start dnsproxy

Enable and start:

```bash
sudo systemctl enable --now dnsproxy.service
```

Verify:

```bash
sudo systemctl status dnsproxy.service \
  --no-pager \
  -l
```

Check sockets:

```bash
sudo ss -lntup |
  grep ':5336\b'
```

Expected:

```text
UDP 127.0.0.1:5336
TCP 127.0.0.1:5336
```

Test resolution:

```bash
dig @127.0.0.1 \
  -p 5336 \
  cloudflare.com \
  A \
  +short
```

The query must return valid addresses.

---

# 13. Start laliga-dns-evade

Enable and start:

```bash
sudo systemctl enable --now \
  laliga-dns-evade.service
```

Verify:

```bash
sudo systemctl status \
  laliga-dns-evade.service \
  --no-pager \
  -l
```

Check sockets:

```bash
sudo ss -lntup |
  grep ':5335\b'
```

Expected:

```text
UDP 127.0.0.1:5335
TCP 127.0.0.1:5335
```

The journal should show:

```text
Loaded <N> blocked IP addresses
Loaded 15 Cloudflare IPv4 prefixes and 7 IPv6 prefixes
laliga-dns-evade v0.1.0
Listening on 127.0.0.1:5335 (UDP/TCP)
Upstream: 127.0.0.1:5336
```

Test UDP:

```bash
dig @127.0.0.1 \
  -p 5335 \
  cloudflare.com \
  A \
  +short
```

Test TCP:

```bash
dig @127.0.0.1 \
  -p 5335 \
  cloudflare.com \
  A \
  +tcp \
  +short
```

Test IPv6:

```bash
dig @127.0.0.1 \
  -p 5335 \
  cloudflare.com \
  AAAA \
  +short
```

---

# 14. Configure AdGuard Home upstream

Before modifying AdGuard Home, back up its configuration.

The validated deployment uses:

```text
/opt/AdGuardHome/AdGuardHome.yaml
```

Back up:

```bash
sudo cp -a \
  /opt/AdGuardHome/AdGuardHome.yaml \
  /opt/AdGuardHome/AdGuardHome.yaml.pre-laliga-dns-evade
```

Stop AdGuard Home before manually modifying YAML because AdGuard Home may rewrite its configuration while running:

```bash
sudo systemctl stop AdGuardHome.service
```

Replace the existing upstream configuration with:

```yaml
dns:
  upstream_dns:
    - 127.0.0.1:5335
```

Do not overwrite unrelated AdGuard Home configuration.

For example, the relevant section should look similar to:

```yaml
  upstream_dns:
    - 127.0.0.1:5335
  upstream_dns_file: ""
```

Start AdGuard Home:

```bash
sudo systemctl start AdGuardHome.service
```

Verify:

```bash
sudo systemctl status \
  AdGuardHome.service \
  --no-pager \
  -l
```

---

# 15. Validate the complete DNS chain

The resulting path should be:

```text
AdGuard Home
    ->
laliga-dns-evade :5335
    ->
dnsproxy :5336
    ->
Cloudflare / Quad9 over HTTP/3
```

Verify dnsproxy:

```bash
dig @127.0.0.1 \
  -p 5336 \
  cloudflare.com \
  A \
  +short
```

Verify laliga-dns-evade:

```bash
dig @127.0.0.1 \
  -p 5335 \
  cloudflare.com \
  A \
  +short
```

Then perform a query through the normal client path and verify it appears in the AdGuard Home Query Log.

---

# 16. Enable automatic list updates

Test the updater manually first:

```bash
sudo /usr/local/sbin/update-laliga-dns-evade.sh
```

A change should produce output similar to:

```text
Downloading source data
Validating downloaded data
Validated <N> blocked addresses
Validated 15 Cloudflare IPv4 prefixes
Validated 7 Cloudflare IPv6 prefixes
blocked-any.txt changed
Installing updated data
Restarting laliga-dns-evade.service
Flushing AdGuard Home DNS cache
Update completed successfully
```

If no data changed:

```text
No changes detected
```

In that case the script does not restart the daemon and does not clear the AdGuard cache.

Enable the timer:

```bash
sudo systemctl enable --now \
  laliga-dns-evade-update.timer
```

Verify:

```bash
systemctl list-timers \
  laliga-dns-evade-update.timer \
  --no-pager
```

Check timer status:

```bash
systemctl status \
  laliga-dns-evade-update.timer \
  --no-pager \
  -l
```

The current production cadence is approximately every five minutes with a small randomized delay.

---

# 17. Cache design

Only AdGuard Home should provide DNS caching.

Current design:

```text
AdGuard Home:
  cache enabled
  cache size: 64 MiB
  optimistic caching enabled

laliga-dns-evade:
  no cache

dnsproxy:
  cache disabled
```

The ordering matters:

```text
AdGuard cache HIT
    -> response directly

AdGuard cache MISS
    -> laliga-dns-evade
    -> dnsproxy
```

When any of these files change:

```text
blocked-any.txt
cloudflare-v4.txt
cloudflare-v6.txt
```

the updater:

1. Installs the new validated files.
2. Restarts `laliga-dns-evade`.
3. Verifies DNS resolution through port 5335.
4. Clears the AdGuard Home DNS cache.

The cache is cleared after the evade daemon has successfully loaded the new data.

This prevents AdGuard Home from continuing to serve an address that has just entered the block list.

---

# 18. DNSSEC behavior

Encrypted upstream resolvers may return validated DNSSEC responses containing:

```text
AD = Authenticated Data
RRSIG records
```

Example upstream response:

```text
flags: qr rd ra ad
```

If `laliga-dns-evade` changes an A or AAAA record, the original DNSSEC signature no longer authenticates the modified RRset.

For that reason, whenever an address is rewritten, `laliga-dns-evade` clears the AD flag.

Example:

```text
Original:
  qr rd ra ad

Rewritten:
  qr rd ra
```

The RRSIG record is currently preserved.

This is intentional.

A downstream DNSSEC-validating client can therefore still detect that the modified RRset no longer matches the original signature.

Responses that are not rewritten are left unchanged.

---

# 19. Rewrite logic

For each A or AAAA address found in DNS Answer, Authority or Additional sections:

```text
Is address in blocked-any.txt?
    |
    no
    -> leave unchanged

    yes
    |
    v
Is address inside a known Cloudflare prefix?
    |
    no
    -> log BLOCKED NON-CLOUDFLARE
    -> leave unchanged

    yes
    |
    v
Find nearby address
    |
    v
Must:
  - stay inside the same Cloudflare prefix
  - not be present in blocked-any.txt
```

IPv4 prefers candidates inside the same `/24`.

The algorithm tests nearby addresses before expanding farther inside the Cloudflare network.

IPv6 checks nearby addresses inside the containing Cloudflare prefix.

---

# 20. Logs

dnsproxy:

```bash
sudo journalctl \
  -u dnsproxy.service \
  -f
```

laliga-dns-evade:

```bash
sudo journalctl \
  -u laliga-dns-evade.service \
  -f
```

Updater:

```bash
sudo journalctl \
  -u laliga-dns-evade-update.service \
  -f
```

Recent evade activity:

```bash
sudo journalctl \
  -u laliga-dns-evade.service \
  --since "30 minutes ago" \
  --no-pager
```

A successful rewrite looks like:

```text
Rewritten blocked Cloudflare address:
104.16.132.229 -> 104.16.132.230
```

TCP rewrites are explicitly identified as TCP in the log.

---

# 21. Service verification

Check all components:

```bash
systemctl is-active dnsproxy.service
systemctl is-active laliga-dns-evade.service
systemctl is-active AdGuardHome.service
systemctl is-active laliga-dns-evade-update.timer
```

Check listeners:

```bash
sudo ss -lntup |
  grep -E ':(5335|5336)\b'
```

Expected:

```text
127.0.0.1:5335 UDP/TCP
127.0.0.1:5336 UDP/TCP
```

Neither service should listen publicly.

---

# 22. Manual end-to-end rewrite test

A controlled test can prove that requests are actually passing through `laliga-dns-evade`.

First identify a current Cloudflare A record:

```bash
dig @127.0.0.1 \
  -p 5336 \
  cloudflare.com \
  A \
  +short
```

Back up the current block list:

```bash
sudo cp -a \
  /var/lib/laliga-dns-evade/blocked-any.txt \
  /var/lib/laliga-dns-evade/blocked-any.txt.testbak
```

Temporarily append one returned Cloudflare IP:

```bash
echo '104.16.132.229' |
  sudo tee -a \
  /var/lib/laliga-dns-evade/blocked-any.txt \
  >/dev/null
```

Restart the evade service:

```bash
sudo systemctl restart \
  laliga-dns-evade.service
```

Clear AdGuard Home cache using the configured API credentials:

```bash
sudo bash -c '
set -a
source /etc/laliga-dns-evade/adguard.env
set +a

curl \
  --fail \
  --silent \
  --show-error \
  --user "${AGH_USER}:${AGH_PASS}" \
  --request POST \
  http://127.0.0.1:3001/control/cache_clear
'
```

Query through the normal DNS path.

The blocked address should be replaced by a nearby non-blocked Cloudflare address.

Verify the journal:

```bash
sudo journalctl \
  -u laliga-dns-evade.service \
  --since "5 minutes ago" \
  --no-pager
```

Restore immediately after testing:

```bash
sudo mv \
  /var/lib/laliga-dns-evade/blocked-any.txt.testbak \
  /var/lib/laliga-dns-evade/blocked-any.txt

sudo systemctl restart \
  laliga-dns-evade.service
```

Clear the AdGuard cache again after restoring the production list.

---

# 23. Data sources

Dynamic blocked address list:

```text
https://hayahora.futbol/estado/blocked-any.txt
```

Cloudflare IPv4 networks:

```text
https://www.cloudflare.com/ips-v4
```

Cloudflare IPv6 networks:

```text
https://www.cloudflare.com/ips-v6
```

Upstream DNS services currently used by `dnsproxy`:

```text
Cloudflare:
  h3://1.1.1.1/dns-query
  h3://1.0.0.1/dns-query

Quad9:
  h3://9.9.9.10/dns-query
  h3://149.112.112.10/dns-query
```

---

# 24. Security

The services are designed to bind only to loopback:

```text
127.0.0.1:5335
127.0.0.1:5336
```

They must not be exposed directly to the Internet.

`laliga-dns-evade` runs under the dedicated unprivileged account:

```text
laliga-dns-evade
```

`dnsproxy` uses a systemd dynamic user.

Systemd hardening includes, where applicable:

```text
NoNewPrivileges
PrivateTmp
PrivateDevices
ProtectSystem
ProtectHome
ProtectKernelTunables
ProtectKernelModules
ProtectKernelLogs
ProtectControlGroups
RestrictAddressFamilies
RestrictSUIDSGID
LockPersonality
MemoryDenyWriteExecute
```

The AdGuard API password is stored in:

```text
/etc/laliga-dns-evade/adguard.env
```

Required permissions:

```text
root:root
0600
```

It must never be stored in Git.

---

# 25. Updating application code

Pull the latest source:

```bash
cd /opt/laliga-dns-evade

git fetch origin
git status
git pull --ff-only
```

Run tests:

```bash
cargo check
cargo test
```

Build:

```bash
cargo build --release
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
sudo systemctl restart \
  laliga-dns-evade.service
```

Verify:

```bash
sudo systemctl status \
  laliga-dns-evade.service \
  --no-pager \
  -l

dig @127.0.0.1 \
  -p 5335 \
  cloudflare.com \
  A \
  +short
```

Do not consider an application upgrade successful only because compilation completed.

The service and DNS function must both be validated.

---

# 26. Updating deployment files

If systemd units change:

```bash
cd /opt/laliga-dns-evade

sudo install \
  -o root \
  -g root \
  -m 0644 \
  systemd/*.service \
  systemd/*.timer \
  /etc/systemd/system/
```

Then:

```bash
sudo systemctl daemon-reload
```

Restart only the units affected by the change.

If the updater changes:

```bash
sudo install \
  -o root \
  -g root \
  -m 0755 \
  scripts/update-laliga-dns-evade.sh \
  /usr/local/sbin/update-laliga-dns-evade.sh
```

Validate:

```bash
sudo bash -n \
  /usr/local/sbin/update-laliga-dns-evade.sh
```

---

# 27. Rollback

## Application binary

Before replacing a production binary, an optional backup can be made:

```bash
sudo cp -a \
  /usr/local/bin/laliga-dns-evade \
  /usr/local/bin/laliga-dns-evade.previous
```

Restore:

```bash
sudo install \
  -o root \
  -g root \
  -m 0755 \
  /usr/local/bin/laliga-dns-evade.previous \
  /usr/local/bin/laliga-dns-evade

sudo systemctl restart \
  laliga-dns-evade.service
```

## AdGuard Home upstream

Restore the configuration backup:

```bash
sudo systemctl stop \
  AdGuardHome.service

sudo cp -a \
  /opt/AdGuardHome/AdGuardHome.yaml.pre-laliga-dns-evade \
  /opt/AdGuardHome/AdGuardHome.yaml

sudo systemctl start \
  AdGuardHome.service
```

Verify DNS afterwards.

---

# 28. Troubleshooting

## dnsproxy does not resolve

Check:

```bash
sudo systemctl status \
  dnsproxy.service \
  --no-pager \
  -l
```

Then:

```bash
sudo journalctl \
  -u dnsproxy.service \
  -n 50 \
  --no-pager
```

Direct functional test:

```bash
dig @127.0.0.1 \
  -p 5336 \
  example.com \
  A \
  +short
```

Do not troubleshoot `laliga-dns-evade` until port 5336 resolves correctly.

## laliga-dns-evade does not start

Check:

```bash
sudo systemctl status \
  laliga-dns-evade.service \
  --no-pager \
  -l
```

Then:

```bash
sudo journalctl \
  -u laliga-dns-evade.service \
  -n 50 \
  --no-pager
```

Check data files:

```bash
sudo ls -lah \
  /var/lib/laliga-dns-evade
```

Check ownership:

```bash
sudo stat \
  /var/lib/laliga-dns-evade/*.txt
```

Test the upstream independently before changing anything else:

```bash
dig @127.0.0.1 \
  -p 5336 \
  example.com \
  A \
  +short
```

## AdGuard resolves but rewrites are not visible

Check whether AdGuard answered from cache.

Clear the client DNS cache first.

For Windows:

```powershell
Clear-DnsClientCache
```

Then clear the AdGuard cache using its API.

Check the evade journal:

```bash
sudo journalctl \
  -u laliga-dns-evade.service \
  --since "5 minutes ago" \
  --no-pager
```

## Updater fails

Check:

```bash
sudo systemctl status \
  laliga-dns-evade-update.service \
  --no-pager \
  -l
```

Then:

```bash
sudo journalctl \
  -u laliga-dns-evade-update.service \
  -n 100 \
  --no-pager
```

Manual execution:

```bash
sudo /usr/local/sbin/update-laliga-dns-evade.sh
```

The updater fails before replacing production files if downloaded data is empty or syntactically invalid.

---

# 29. Git workflow

Before committing:

```bash
cd /opt/laliga-dns-evade

cargo check
cargo test

git status
git diff
```

Commit:

```bash
git add .
git commit -m "Describe the change"
git push
```

Never commit:

```text
/etc/laliga-dns-evade/adguard.env
private SSH keys
runtime block lists
temporary test files
target/
```

---

# 30. Production service summary

Services:

```text
AdGuardHome.service
dnsproxy.service
laliga-dns-evade.service
laliga-dns-evade-update.service
laliga-dns-evade-update.timer
```

Runtime paths:

```text
/usr/local/bin/dnsproxy
/usr/local/bin/laliga-dns-evade
/usr/local/sbin/update-laliga-dns-evade.sh

/etc/laliga-dns-evade/adguard.env

/var/lib/laliga-dns-evade/blocked-any.txt
/var/lib/laliga-dns-evade/cloudflare-v4.txt
/var/lib/laliga-dns-evade/cloudflare-v6.txt
```

Ports:

```text
127.0.0.1:5335 UDP/TCP  laliga-dns-evade
127.0.0.1:5336 UDP/TCP  dnsproxy
```

Automatic update cadence:

```text
approximately every 5 minutes
```

Cache ownership:

```text
AdGuard Home:       enabled
laliga-dns-evade:   none
dnsproxy:            disabled
```

#!/usr/bin/env bash
#
# Purpose:
#   Update the dynamic data used by laliga-dns-evade.
#
# Parameters:
#   None.
#
# Examples:
#   sudo /usr/local/sbin/update-laliga-dns-evade.sh
#
# Author:
#   VDIAS
#
# Version:
#   1.0.0
#
# Date:
#   2026-09-14
#
# Requirements:
#   - bash
#   - curl
#   - sha256sum
#   - python3
#   - systemctl
#   - dig
#   - flock
#   - /etc/laliga-dns-evade/adguard.env
#   - laliga-dns-evade.service
#
# Changelog:
#   1.0.0 - Initial production updater.
#

set -Eeuo pipefail

readonly DATA_DIR="/var/lib/laliga-dns-evade"
readonly ENV_FILE="/etc/laliga-dns-evade/adguard.env"
readonly LOCK_FILE="/run/lock/laliga-dns-evade-update.lock"

readonly BLOCKLIST_FILE="${DATA_DIR}/blocked-any.txt"
readonly CF_V4_FILE="${DATA_DIR}/cloudflare-v4.txt"
readonly CF_V6_FILE="${DATA_DIR}/cloudflare-v6.txt"

readonly BLOCKLIST_URL="https://hayahora.futbol/estado/blocked-any.txt"
readonly CF_V4_URL="https://www.cloudflare.com/ips-v4"
readonly CF_V6_URL="https://www.cloudflare.com/ips-v6"

readonly SERVICE_NAME="laliga-dns-evade.service"
readonly DNS_TEST_SERVER="127.0.0.1"
readonly DNS_TEST_PORT="5335"
readonly DNS_TEST_NAME="cloudflare.com"

TEMP_DIR=""
BACKUP_DIR=""

log() {
    printf '%s %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$*"
}

fail() {
    log "ERROR: $*" >&2
    exit 1
}

cleanup() {
    if [[ -n "${TEMP_DIR}" && -d "${TEMP_DIR}" ]]; then
        rm -rf "${TEMP_DIR}"
    fi

    if [[ -n "${BACKUP_DIR}" && -d "${BACKUP_DIR}" ]]; then
        rm -rf "${BACKUP_DIR}"
    fi
}

trap cleanup EXIT
trap 'fail "Unexpected failure at line ${LINENO}"' ERR

require_command() {
    command -v "$1" >/dev/null 2>&1 ||
        fail "Required command not found: $1"
}

download_file() {
    local url="$1"
    local destination="$2"

    curl \
        --fail \
        --location \
        --silent \
        --show-error \
        --connect-timeout 10 \
        --max-time 60 \
        --retry 3 \
        --retry-delay 2 \
        --output "${destination}" \
        "${url}"
}

validate_ip_list() {
    local file="$1"

    python3 - "${file}" <<'PY'
import ipaddress
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
count = 0

for line_number, raw_line in enumerate(path.read_text().splitlines(), start=1):
    line = raw_line.strip()

    if not line or line.startswith("#"):
        continue

    try:
        ipaddress.ip_address(line)
    except ValueError as exc:
        raise SystemExit(
            f"{path}: invalid IP at line {line_number}: {line}: {exc}"
        )

    count += 1

if count == 0:
    raise SystemExit(f"{path}: no IP addresses found")

print(count)
PY
}

validate_network_list() {
    local file="$1"
    local family="$2"

    python3 - "${file}" "${family}" <<'PY'
import ipaddress
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
expected_version = int(sys.argv[2])
count = 0

for line_number, raw_line in enumerate(path.read_text().splitlines(), start=1):
    line = raw_line.strip()

    if not line or line.startswith("#"):
        continue

    try:
        network = ipaddress.ip_network(line, strict=True)
    except ValueError as exc:
        raise SystemExit(
            f"{path}: invalid network at line {line_number}: {line}: {exc}"
        )

    if network.version != expected_version:
        raise SystemExit(
            f"{path}: wrong address family at line {line_number}: {line}"
        )

    count += 1

if count == 0:
    raise SystemExit(f"{path}: no networks found")

print(count)
PY
}

file_changed() {
    local current="$1"
    local candidate="$2"

    [[ ! -f "${current}" ]] && return 0

    [[ "$(sha256sum "${current}" | awk '{print $1}')" != \
       "$(sha256sum "${candidate}" | awk '{print $1}')" ]]
}

restore_backup() {
    log "Restoring previous data files"

    install -o root -g laliga-dns-evade -m 0640 \
        "${BACKUP_DIR}/blocked-any.txt" \
        "${BLOCKLIST_FILE}"

    install -o root -g laliga-dns-evade -m 0640 \
        "${BACKUP_DIR}/cloudflare-v4.txt" \
        "${CF_V4_FILE}"

    install -o root -g laliga-dns-evade -m 0640 \
        "${BACKUP_DIR}/cloudflare-v6.txt" \
        "${CF_V6_FILE}"

    systemctl restart "${SERVICE_NAME}" || true
}

flush_adguard_cache() {
    # shellcheck disable=SC1090
    source "${ENV_FILE}"

    [[ -n "${AGH_USER:-}" ]] ||
        fail "AGH_USER is not defined in ${ENV_FILE}"

    [[ -n "${AGH_PASS:-}" ]] ||
        fail "AGH_PASS is not defined in ${ENV_FILE}"

    curl \
        --fail \
        --silent \
        --show-error \
        --user "${AGH_USER}:${AGH_PASS}" \
        --request POST \
        http://127.0.0.1:3001/control/cache_clear \
        >/dev/null
}

verify_dns() {
    local result

    result="$(
        dig \
            @"${DNS_TEST_SERVER}" \
            -p "${DNS_TEST_PORT}" \
            "${DNS_TEST_NAME}" \
            A \
            +short
    )"

    [[ -n "${result}" ]] ||
        return 1
}

main() {
    [[ "${EUID}" -eq 0 ]] ||
        fail "This script must run as root"

    require_command curl
    require_command sha256sum
    require_command python3
    require_command systemctl
    require_command dig
    require_command flock
    require_command install

    [[ -d "${DATA_DIR}" ]] ||
        fail "Data directory not found: ${DATA_DIR}"

    [[ -r "${ENV_FILE}" ]] ||
        fail "AdGuard credentials file not readable: ${ENV_FILE}"

    systemctl is-active --quiet dnsproxy.service ||
        fail "dnsproxy.service is not active"

    systemctl is-active --quiet "${SERVICE_NAME}" ||
        fail "${SERVICE_NAME} is not active"

    exec 9>"${LOCK_FILE}"
    flock -n 9 ||
        fail "Another update is already running"

    TEMP_DIR="$(mktemp -d)"
    BACKUP_DIR="$(mktemp -d)"

    log "Downloading source data"

    download_file \
        "${BLOCKLIST_URL}" \
        "${TEMP_DIR}/blocked-any.txt"

    download_file \
        "${CF_V4_URL}" \
        "${TEMP_DIR}/cloudflare-v4.txt"

    download_file \
        "${CF_V6_URL}" \
        "${TEMP_DIR}/cloudflare-v6.txt"

    log "Validating downloaded data"

    local blocked_count
    local cf_v4_count
    local cf_v6_count

    blocked_count="$(
        validate_ip_list "${TEMP_DIR}/blocked-any.txt"
    )"

    cf_v4_count="$(
        validate_network_list "${TEMP_DIR}/cloudflare-v4.txt" 4
    )"

    cf_v6_count="$(
        validate_network_list "${TEMP_DIR}/cloudflare-v6.txt" 6
    )"

    log "Validated ${blocked_count} blocked addresses"
    log "Validated ${cf_v4_count} Cloudflare IPv4 prefixes"
    log "Validated ${cf_v6_count} Cloudflare IPv6 prefixes"

    local changed=false

    if file_changed \
        "${BLOCKLIST_FILE}" \
        "${TEMP_DIR}/blocked-any.txt"
    then
        changed=true
        log "blocked-any.txt changed"
    fi

    if file_changed \
        "${CF_V4_FILE}" \
        "${TEMP_DIR}/cloudflare-v4.txt"
    then
        changed=true
        log "cloudflare-v4.txt changed"
    fi

    if file_changed \
        "${CF_V6_FILE}" \
        "${TEMP_DIR}/cloudflare-v6.txt"
    then
        changed=true
        log "cloudflare-v6.txt changed"
    fi

    if [[ "${changed}" == false ]]; then
        log "No changes detected"
        exit 0
    fi

    cp -a "${BLOCKLIST_FILE}" "${BACKUP_DIR}/blocked-any.txt"
    cp -a "${CF_V4_FILE}" "${BACKUP_DIR}/cloudflare-v4.txt"
    cp -a "${CF_V6_FILE}" "${BACKUP_DIR}/cloudflare-v6.txt"

    log "Installing updated data"

    install -o root -g laliga-dns-evade -m 0640 \
        "${TEMP_DIR}/blocked-any.txt" \
        "${BLOCKLIST_FILE}"

    install -o root -g laliga-dns-evade -m 0640 \
        "${TEMP_DIR}/cloudflare-v4.txt" \
        "${CF_V4_FILE}"

    install -o root -g laliga-dns-evade -m 0640 \
        "${TEMP_DIR}/cloudflare-v6.txt" \
        "${CF_V6_FILE}"

    log "Restarting ${SERVICE_NAME}"

    if ! systemctl restart "${SERVICE_NAME}"; then
        restore_backup
        fail "Service restart failed; previous data restored"
    fi

    if ! systemctl is-active --quiet "${SERVICE_NAME}"; then
        restore_backup
        fail "Service is not active; previous data restored"
    fi

    if ! verify_dns; then
        restore_backup
        fail "DNS verification through port ${DNS_TEST_PORT} failed; previous data restored"
    fi

    log "Flushing AdGuard Home DNS cache"

    if ! flush_adguard_cache; then
        fail "Data updated successfully, but AdGuard cache flush failed"
    fi

    log "Update completed successfully"
}

main "$@"

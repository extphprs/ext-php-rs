#!/usr/bin/env bash
set -Eeuo pipefail
shopt -s inherit_errexit

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
readonly SCRIPT_DIR
readonly RESTARTS=3
readonly URL=http://localhost:8080/worker.php

_tmpdir=""
_server=""

cleanup() {
  if [[ -n "${_server}" ]]; then
    kill "${_server}" 2>/dev/null || true
    wait "${_server}" 2>/dev/null || true
  fi
  rm -rf -- "${_tmpdir}"
}

main() {
  if (($# != 1)); then
    printf 'usage: %s <extension.so>\n' "${0##*/}" >&2
    return 2
  fi
  if [[ ! -f "$1" ]]; then
    printf 'extension not found: %s\n' "$1" >&2
    return 2
  fi

  local extension
  extension="$(realpath -- "$1")"

  trap 'printf "Error at %s:%d\n" "${BASH_SOURCE[0]}" "${LINENO}" >&2' ERR
  trap cleanup EXIT

  _tmpdir="$(mktemp -d)"
  printf 'extension=%s\n' "${extension}" >"${_tmpdir}/ext-php-rs.ini"
  export PHP_INI_SCAN_DIR=":${_tmpdir}"

  cd -- "${SCRIPT_DIR}"
  frankenphp run --config Caddyfile &
  _server=$!

  curl -fsS --max-time 10 --retry 30 --retry-connrefused --retry-delay 1 "${URL}"

  local restart
  for ((restart = 1; restart <= RESTARTS; restart++)); do
    printf 'restart %d\n' "${restart}"
    curl -fsS --max-time 60 -X POST http://localhost:2019/frankenphp/workers/restart
    curl -fsS --max-time 10 "${URL}"
  done
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  main "$@"
fi

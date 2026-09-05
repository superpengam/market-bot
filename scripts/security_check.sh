#!/usr/bin/env bash
# Fails when tracked files look like committed secrets or runtime .env files.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${root}"

violations=0

is_example_env() {
  local path="$1"
  local base
  base="$(basename "${path}")"
  [[ "${base}" == ".env.example" ]]
}

is_runtime_env() {
  local path="$1"
  local base
  base="$(basename "${path}")"
  case "${base}" in
    .env|.env.local|.env.production|.env.staging|.env.development)
      return 0
      ;;
  esac
  return 1
}

list_tracked_files() {
  if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    git ls-files
    return
  fi
  find . -type f \
    ! -path './.git/*' \
    ! -path './target/*' \
    ! -path './node_modules/*' \
    ! -path '*/node_modules/*' \
    ! -path '*/.next/*'
}

while IFS= read -r path; do
  [[ -n "${path}" ]] || continue
  [[ -f "${path}" ]] || continue

  if is_runtime_env "${path}" && ! is_example_env "${path}"; then
    echo "tracked runtime env file: ${path}"
    violations=$((violations + 1))
    continue
  fi

  if is_example_env "${path}"; then
    continue
  fi

  if grep -E -q -- '-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----' "${path}" 2>/dev/null; then
    echo "private key material: ${path}"
    violations=$((violations + 1))
  fi

  if grep -E -q -- 'sk_live_[0-9A-Za-z]{10,}' "${path}" 2>/dev/null; then
    echo "live payment secret pattern: ${path}"
    violations=$((violations + 1))
  fi

  if grep -E -q -- 'AKIA[0-9A-Z]{16}' "${path}" 2>/dev/null; then
    echo "AWS access key pattern: ${path}"
    violations=$((violations + 1))
  fi
done < <(list_tracked_files)

if [[ "${violations}" -gt 0 ]]; then
  echo "security_check failed with ${violations} finding(s)"
  exit 1
fi

echo "security_check clean"
exit 0

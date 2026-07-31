#!/usr/bin/env bash

set -euo pipefail

readonly script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly repo_root="$(cd "${script_dir}/.." && pwd)"
readonly resource_dir="${repo_root}/src-tauri/resources"
readonly resource_manifest="${resource_dir}/resources.lock"

mode="fetch"
if [[ ${1:-} == "--check" ]]; then
  mode="check"
elif [[ $# -gt 0 ]]; then
  echo "Usage: $0 [--check]" >&2
  exit 2
fi

checksum() {
  local path=$1
  local digest

  if command -v shasum >/dev/null 2>&1; then
    digest="$(shasum -a 256 "${path}")"
  elif command -v sha256sum >/dev/null 2>&1; then
    digest="$(sha256sum "${path}")"
  else
    echo "Neither shasum nor sha256sum is available" >&2
    return 1
  fi

  printf '%s\n' "${digest%% *}"
}

verify() {
  local path=$1
  local expected=$2

  [[ -f ${path} ]] && [[ $(checksum "${path}") == "${expected}" ]]
}

download() {
  local relative_path=$1
  local url=$2
  local expected=$3
  local compression=$4
  local destination="${resource_dir}/${relative_path}"
  local download_dir
  local archive
  local expanded

  if verify "${destination}" "${expected}"; then
    echo "verified ${relative_path}"
    return
  fi

  if [[ ${mode} == "check" ]]; then
    echo "missing or invalid resource: ${relative_path}" >&2
    return 1
  fi

  command -v curl >/dev/null 2>&1 || {
    echo "curl is required to download bundle resources" >&2
    return 1
  }

  download_dir="$(mktemp -d)"
  archive="${download_dir}/asset"
  expanded="${download_dir}/expanded"
  trap 'rm -rf -- "${download_dir}"' RETURN

  echo "downloading ${relative_path}"
  curl --fail --location --retry 3 --silent --show-error "${url}" --output "${archive}"

  case ${compression} in
    none)
      expanded=${archive}
      ;;
    xz)
      command -v xz >/dev/null 2>&1 || {
        echo "xz is required to unpack ${relative_path}" >&2
        return 1
      }
      xz --decompress --stdout "${archive}" >"${expanded}"
      ;;
    *)
      echo "unsupported compression '${compression}' for ${relative_path}" >&2
      return 1
      ;;
  esac

  if ! verify "${expanded}" "${expected}"; then
    echo "checksum mismatch for ${relative_path}" >&2
    return 1
  fi

  mkdir -p "$(dirname "${destination}")"
  install -m 0644 "${expanded}" "${destination}"
  echo "installed ${relative_path}"
}

while IFS='|' read -r relative_path url expected compression; do
  [[ -z ${relative_path} || ${relative_path} == \#* ]] && continue

  case ${relative_path} in
    /*|*..*)
      echo "unsafe destination in ${resource_manifest}: ${relative_path}" >&2
      exit 1
      ;;
  esac

  download "${relative_path}" "${url}" "${expected}" "${compression}"
done <"${resource_manifest}"

#!/usr/bin/env sh

set -eu

# Download the GitHub attestation bundle for one release asset and move the
# generated JSONL file to the requested final path.

artifact_path="${1:?missing artifact path}"
output_path="${2:?missing output path}"

# This helper is used from release.yml, so fail immediately if the GitHub token
# is missing instead of letting gh emit a less specific error.
if [ -z "${GH_TOKEN:-}" ]; then
  echo "GH_TOKEN is required" >&2
  exit 1
fi

artifact_path="$(realpath "${artifact_path}")"
# Download into a temp directory because gh attestation chooses its own filename.
tmpdir="$(mktemp -d)"
trap 'rm -rf "${tmpdir}"' EXIT INT TERM

(
  cd "${tmpdir}"
  gh attestation download "${artifact_path}" -R "${GITHUB_REPOSITORY}"
)

# gh names the JSONL bundle by digest, so locate the generated file and move it
# to the release filename expected by the workflow.
attestation_bundle="$(find "${tmpdir}" -maxdepth 1 -type f -name 'sha256*.jsonl' | head -1)"
test -n "${attestation_bundle}" || {
  echo "failed to download attestation bundle" >&2
  exit 1
}

mkdir -p "$(dirname "${output_path}")"
mv "${attestation_bundle}" "${output_path}"

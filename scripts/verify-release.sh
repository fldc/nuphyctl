#!/usr/bin/env bash
# Verify release files without executing the downloaded binary.
set -euo pipefail

if [[ $# -lt 3 || $# -gt 4 ]]; then
  echo "Usage: $0 TAG SOURCE_COMMIT {x86_64|aarch64} [DIRECTORY]" >&2
  exit 2
fi

tag=$1
commit=$2
arch=$3
directory=${4:-.}
if [[ ! "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ || ! "$commit" =~ ^[0-9a-f]{40}$ ]]; then
  echo "Expected a vMAJOR.MINOR.PATCH tag and full source commit SHA" >&2
  exit 2
fi
case "$arch" in
  x86_64|aarch64) ;;
  *) echo "Unsupported architecture: $arch" >&2; exit 2 ;;
esac

cd "$directory"
base="nuphyctl-$tag-linux-$arch"
policy=(
  --repo fldc/nuphyctl
  --signer-workflow fldc/nuphyctl/.github/workflows/release.yml
  --source-ref "refs/tags/$tag"
  --source-digest "$commit"
  --deny-self-hosted-runners
  --bundle "$base.provenance.json"
  --format json
)

# Verify the archive before reading any of its contents. A single signed bundle
# contains subjects for the archive, executable, checksums, and build inputs.
gh attestation verify "$base.tar.gz" "${policy[@]}" > "$base.archive.verification.json"
temporary=$(mktemp -d)
trap 'rm -rf -- "$temporary"' EXIT
tar -xzf "$base.tar.gz" -C "$temporary" -- nuphyctl
gh attestation verify "$temporary/nuphyctl" "${policy[@]}" > "$base.executable.verification.json"
for suffix in SHA256SUMS build-info.txt; do
  gh attestation verify "$base.$suffix" "${policy[@]}" > /dev/null
done

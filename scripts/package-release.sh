#!/usr/bin/env bash
# Package an already-built native release binary, without downloading anything.
set -euo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"
version=$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["version"])')
target=$(rustc -vV | sed -n 's/^host: //p')
name="astrarium-$version-$target"
binary="target/release/astrarium"

if [[ ! -x "$binary" ]] || [[ $("$binary" --version) != "astrarium $version" ]]; then
  echo "Build the current release first: cargo build --release --locked" >&2
  exit 1
fi

stage=$(mktemp -d "${TMPDIR:-/tmp}/astrarium-release.XXXXXX")
trap 'rm -rf -- "$stage"' EXIT
mkdir -p "$stage/$name/data" dist
install -m755 "$binary" "$stage/$name/astrarium"
install -m644 LICENSE README.md CHANGELOG.md CONTRIBUTING.md "$stage/$name/"
install -m644 data/LICENSE-* data/*.md "$stage/$name/data/"
cp -R assets docs "$stage/$name/"
tar -C "$stage" -czf "dist/$name.tar.gz" "$name"
(cd dist && sha256sum "$name.tar.gz" > SHA256SUMS)
echo "dist/$name.tar.gz"

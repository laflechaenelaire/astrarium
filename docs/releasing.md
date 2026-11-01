# Publishing and releasing

## Repository

The repository is prepared for `laflechaenelaire/astrarium` on GitHub. If the
owner or name changes, update `Cargo.toml`, README links and these notes.

Suggested description:

> An offline planetarium for your terminal. Explore stars, planets, the Sun and Moon with keyboard and mouse. Built in Rust.

Suggested topics: `rust`, `tui`, `terminal`, `astronomy`, `planetarium`, `linux`,
`omarchy`, `ratatui`, `offline`.

Once the source has been reviewed and committed, create/push the public repo:

```sh
gh repo create laflechaenelaire/astrarium --public --source=. --remote=origin --push \
  --description 'An offline planetarium for your terminal. Explore stars, planets, the Sun and Moon with keyboard and mouse. Built in Rust.'
gh repo edit --add-topic rust,tui,terminal,astronomy,planetarium,linux,omarchy,ratatui,offline
```

CI runs on pull requests and pushes to `main`. The showcase is in
`assets/preview.png`; upload it with the launch post in `docs/launch.md`.
Set the repository's social preview to `assets/social-preview.png` if desired.

## Versioned release

1. Update `Cargo.toml`, `Cargo.lock`, `Makefile` and `packaging/PKGBUILD` together.
2. Update `CHANGELOG.md`, add `docs/releases/v<version>.md` with release notes,
   and update the download version in `README.md`.
3. Run the checks in `CONTRIBUTING.md`, review the changes and commit them.
4. Push the matching version tag:

   ```sh
   git tag -a v0.4.0 -m 'Astrarium 0.4.0'
   git push origin main v0.4.0
   ```

The release workflow rejects tags that don't match Cargo's version. It builds
on Ubuntu 22.04, runs the tests and PTY smoke test, then publishes a GitHub
release with a native x86-64 GNU/Linux archive and SHA-256 checksums. That build
targets glibc 2.35 or later. The local Arch build can require a newer glibc.

Only push a version tag when you're ready to publish: the tag workflow publishes
the release automatically. Other architectures can build from source.

## Local release archives

```sh
cargo build --release --locked
bash scripts/package-release.sh
```

The script checks the binary version and packages the executable, documentation
and licences in `dist/`, together with `SHA256SUMS`. It builds for the current
host; it does not cross-compile.

For an Arch package, use `make dist` and then `makepkg -si` from `dist/`.
The generated PKGBUILD includes the source archive's checksum. Publishing to
AUR and official Omarchy inclusion are separate from this repository's release.

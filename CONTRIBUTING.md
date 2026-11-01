# Contributing to Astrarium

Thanks for helping make the sky easier to explore. Issues and pull requests
are welcome; for larger changes, open an issue first to discuss the interaction.

## Local setup

Use stable Rust, Python 3, a C compiler and Clang/libclang. The toolchain file
includes rustfmt and Clippy. On Arch, the native build tools are provided by
`base-devel` and `clang`. With mise, prefix Cargo commands with
`mise exec rust@stable --`.

```sh
cargo run --release -- --lat=0 --lon=0 --at=2026-01-01T00:00:00Z
```

## Before a pull request

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
python3 scripts/smoke.py target/release/astrarium
```

The PTY test uses an isolated configuration directory. Unit and integration
tests don't need network access after Cargo dependencies are available.

For UI changes, try a narrow terminal and both `--lang en` and `--lang es`.
Keep a keyboard alternative for mouse interactions, and preserve clean terminal
restoration on exit. Add meaningful regression tests when changing behavior.

## Code map

| Module | Responsibility |
|---|---|
| `src/main.rs` | CLI, event loop and terminal lifecycle |
| `src/app.rs` | Application state, input, search and observer configuration |
| `src/astro.rs` | Stellar coordinate transformations and camera projection |
| `src/solar.rs` | Safe, serialized access to Astronomy Engine |
| `src/labels.rs` | Zoom-sensitive label placement and collision handling |
| `src/i18n.rs` | Language selection, persistence and compass labels |
| `src/ui.rs` | Terminal rendering and English/Spanish interface text |

Keep geometric coordinates separate from drawing. When changing astronomy,
compare against independent data such as JPL Horizons; document the epoch,
observer, reference frame, refraction setting and tolerance. Reference examples
are in `data/ephemeris-validation.md`.

## Catalogue updates

`data/sky.json` is generated and embedded at compile time. To rebuild it:

```sh
python3 scripts/catalog.py
```

The generator pins its upstream revision. Updating that revision is an explicit
data update: review the resulting catalogue, update provenance, and retain the
upstream licence. Normal builds never download sky data.

## Style and scope

- Follow rustfmt and the surrounding code; keep dependencies purposeful.
- Add both English and Spanish strings for new UI text.
- Preserve star catalogue names and user-entered location names.
- Keep the quiet astronomical-chart aesthetic and terminal compatibility.
- Attribute external data, assets and code appropriately.

Contributions are licensed under the project's MIT licence. Third-party data
retains its own licence. Release instructions are in `docs/releasing.md`.

# Setup

## Toolchain

- Rust with `edition = 2024` and `rust-version = 1.85`. A stable toolchain at
  or above 1.85 is required (workspace will refuse to build otherwise).
- Cargo manages everything; there is no separate build script, Makefile, or
  `build.rs`.

Install via `rustup` if needed:

```
rustup install stable
rustup default stable
rustc --version    # must be >= 1.85.0
```

## Cloning

```
git clone <remote>/gargantua.git relavistic-renderer
cd relavistic-renderer
```

The on-disk folder is `relavistic-renderer` (sic). The crate name in the root
`Cargo.toml` is `relavistic-renderer`. The git remote is named `gargantua`.

## Build

```
cargo build                 # whole workspace, debug
cargo build --release       # release
cargo build -p gr-core      # just the physics core
```

Note: `crates/gr-core/src/lib.rs` re-exports `Kerr` and `SuperposedSchwarzschild`,
which are not defined in `metric.rs` yet. A clean `cargo build` of `gr-core`
may fail until those types are added or the re-exports are removed. See
`docs/notes.md`.

The root `Cargo.toml` also declares `render = { path = "crates/render" }`
under `[workspace.dependencies]`, but no `crates/render` directory exists.
This entry is unused (nothing depends on `render`) but is a foot-gun.

## Run

```
cargo run                   # runs src/main.rs (stub: prints "Hello, world!")
cargo run -p gr-tracer      # stub
cargo run -p gr-renderer    # stub
```

There is no scene-file format, no CLI argument parser, and no image output
yet. The renderer is effectively driven from unit tests for now.

## Test

```
cargo test                  # runs every crate's tests
cargo test -p gr-core       # the only crate with tests today
```

Tests in `gr-core` cover:

- `schwarzschild.rs` -- the metric approaches Minkowski at large `r`.
- `geodesic.rs` -- a circular orbit at `r = 6M` (the innermost stable
  circular orbit for Schwarzschild) stays approximately at `r = 6M` after
  one orbital period.

## Editor / LSP

`rust-analyzer` works out of the box. The workspace root is the top-level
`Cargo.toml`; opening any sub-crate alone will miss the workspace-level
dependency pins.

## Build artifacts

`target/` is gitignored at every level (`**/target` in `.gitignore`).
A few stale `target/` trees may exist inside individual crates from earlier
per-crate builds; they are harmless.

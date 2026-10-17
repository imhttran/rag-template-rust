# Rust foundation

This directory hosts the Rust foundation for the RAG template: a library crate
(`rag_foundation`) exposing domain types and configuration, plus thin binaries
that mirror the existing Go command surface. The Go codebase is unchanged and
its commands keep working; the Rust checks run alongside the Go checks.

## Toolchain

The toolchain is pinned in [`rust-toolchain.toml`](rust-toolchain.toml):

```toml
[toolchain]
channel = "1.83.0"
components = ["rustfmt", "clippy", "rust-src", "lldb-preview"]
profile = "minimal"
```

- **channel 1.83.0** — the stable release used to build and lock the crate.
- **rustfmt** — formats sources; drives Zed's format-on-save.
- **clippy** — linting; surfaces warnings as Zed diagnostics.
- **rust-src** — standard-library sources for go-to-definition into `std`.
- **lldb-preview** — the debugger used by the Zed debug smoke check.

With `rustup` installed, opening this directory (or any child) installs the
pinned toolchain and components automatically. To install manually:

```sh
rustup toolchain install 1.83.0 --component rustfmt clippy rust-src lldb-preview
```

### macOS

```sh
# Install rustup (official, from https://rustup.rs).
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Then, from the repository root:
cd rust && rustup show
```

### Linux

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cd rust && rustup show
```

## Dependencies

Every direct dependency is documented with an official source and a reason for
inclusion. Versions are locked in `Cargo.lock`.

| Crate | Version | Official documentation | Reason |
| --- | --- | --- | --- |
| `serde` | 1.0.219 | <https://serde.rs/>, <https://docs.rs/serde/latest/serde/> | Serialization/deserialization framework; `derive` gives `Serialize`/`Deserialize` for the domain types. Maintained by the serde maintainers and widely used. |
| `serde_json` | 1.0.140 | <https://docs.rs/serde_json/latest/serde_json/> | JSON (de)serialization used by unit tests and for round-tripping retrieved documents. The canonical serde JSON implementation. |
| `thiserror` | 2.0.12 | <https://docs.rs/thiserror/latest/thiserror/> | Ergonomic, zero-dependency error types for library errors such as configuration errors that must name the offending variable. Maintained by dtolnay. |
| `tempfile` (dev) | 3.19.1 | <https://docs.rs/tempfile/latest/tempfile/> | Isolated temporary directories and files for tests. Maintained by the rust-lang nursery. |

Selection criteria: maintained crates with current releases, mutually
compatible (all build under the pinned 1.83.0 toolchain and resolve together in
`Cargo.lock`), and aligned with the domain and configuration work.

## Building and checking

Run from the `rust/` directory:

```sh
cargo build          # library + declared binaries
cargo test           # domain-types and configuration unit tests
cargo fmt --check    # formatting
cargo clippy --all-targets -- -D warnings  # linting
```

The same checks are wired into the repository `Makefile` as `rust-build`,
`rust-test`, `rust-fmt`, and `rust-clippy`, and are reachable through
`make check` alongside the Go checks.

## Platform verification status

Each platform is recorded as **verified** or **unverified**. Success is never
inferred for a machine that was not directly exercised.

| Platform | Toolchain / cargo | rustfmt | clippy | Notes |
| --- | --- | --- | --- | --- |
| macOS | unverified | unverified | unverified | Not exercised in this environment; pending manual verification. |
| Linux | unverified | unverified | unverified | Not exercised in this environment; pending manual verification. |

Desktop/Zed GUI checks (completion, go-to-definition, hover, diagnostics,
format-on-save, running a unit test from Zed, debug smoke check) are recorded in
[`docs/plans/PLAN-RUST-MIGRATION.md`](../docs/plans/PLAN-RUST-MIGRATION.md) and
in the editor-verification report; where desktop access was unavailable they are
marked **pending manual verification**, not successful.

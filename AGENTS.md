# Working on a native Horizon mod

> **Build slots (v2, 2026-11-09):** every build/test/e2e goes through the
> campaign runner (`ports/tools/build-slot.sh` from the campaign root,
> `../tools/build-slot.sh` from this port). It **blocks in the kernel** until a
> slot is free — a freed slot is taken in ~0.07 s — so **never poll**: no
> `while ! slot; sleep` loop, no background build. It sets `CARGO_BUILD_JOBS` per
> step (build/test 3, stage 2, e2e 1), journals grants/releases, and a timeout
> exits **75**. Read `../_shared/build-slots-usage.md` before running anything;
> observe with `status` / `watch` / `events` / `stats`, never with `ps`+`sleep`.
> Never pass `--target-dir`, never run a bare `cargo` here.

Read README.md and `horizon-mod.toml` before editing. Keep this repository
separate from Horizon and horizon-mixins.

This port is an **SDK package** (mod API 0.2): one flat `.hmod` with
`horizon-mod.toml`, `Cargo.toml`, `src/lib.rs` (the one `impl mod_api::Hooks`),
`src/runtime.rs` (the pure logic and its tests) and `assets/` + `data/`. There is
no `compatibility.json` and no mixin plan: `[native]` in the manifest is the pin,
and the SDK is fetched into `.horizon/sdk/` by any `cargo horizon-mod` command.

`src/runtime.rs` is the port's rule file and is **byte-identical** to the retired
bundle's copy — never reformat it, never add `mod_api` types to it. The glue is
`src/basic_recycling_factory.rs`: read the game, call the runtime, write the
result back, declare nothing of its own beyond the container, the buffer and the
block entity type.

Validate with `cargo test`, `cargo clippy --all-targets -- -D warnings`,
`cargo fmt --check` (never `--all`: that formats the fetched SDK), then
`rm -f Cargo.lock && cargo horizon-mod build`, `cargo horizon-mod check
mekmm.hmod` and `cargo horizon-mod fits mekmm.hmod --build nightly`. The in-game
check is `GAME=<launcher-built client> tools/e2e.sh` (`SELFTEST=1` must fail); it
needs `horizon-launcher native-build . --trust-native-code`, which is not
installed in every environment. Keep validation claims and supported revisions
honest.

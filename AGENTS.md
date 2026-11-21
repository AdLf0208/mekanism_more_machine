# Working on a native Horizon mod

> **Build slots (v2, 2026-11-09):** every build/test/e2e goes through the
> campaign runner (`../tools/build-slot.sh` from this port). It **blocks in the
> kernel** until a slot is free — a freed slot is taken in ~0.07 s — so **never
> poll**: no `while ! slot; sleep` loop, no background build. It sets
> `CARGO_BUILD_JOBS` per step, journals grants/releases, and a timeout exits
> **75**. Read `../_shared/build-slots-usage.md` before running anything;
> observe with `status` / `watch` / `events` / `stats`, never with `ps`+`sleep`.
> Never pass `--target-dir`, never run a bare `cargo` here.

Read README.md and `horizon-mod.toml` before editing. Keep this repository
separate from Horizon and horizon-mixins.

This port is an **SDK package** (mod API 0.2): one flat `.hmod` with
`horizon-mod.toml`, `Cargo.toml`, `src/lib.rs` (the one `impl mod_api::Hooks`),
`src/runtime.rs` (the pure logic and its tests) and `assets/` + `data/`. There is
no `compatibility.json` and no mixin plan: `[native]` in the manifest is the pin,
and the SDK is fetched into `.horizon/sdk/` by any `cargo horizon-mod` command.
**After a repin, delete `.horizon/sdk` first** — a cached SDK is not refreshed.

`src/runtime.rs` is the port's rule file: no `mod_api` types, everything the
machine does is a pure function with a test. Do not name a local type after one
the SDK exports (`Item`, `Stack`, `Recipe`, `Block`, `Config`, `Attached`,
`Command`): the `fits --build <other>` analyser resolves it to the SDK's and
reports a false incompatibility. The glue is `src/recycling_factory.rs`: read the
game, call the runtime, write the result back, declare nothing of its own beyond
the per-tier container, buffer and block entity type.

Validate with `cargo test`, `cargo clippy --all-targets -- -D warnings`,
`cargo fmt --check` (never `--all`: that formats the fetched SDK), then
`rm -f Cargo.lock && cargo horizon-mod build`, `cargo horizon-mod check
mekmm.hmod`, `cargo horizon-mod fits mekmm.hmod --build 0a875da` and — the real
gate — `--build 0b14f8a`. The in-game check is
`ports/tools/launcher-game.sh build mekmm` then
`GAME=… tools/e2e.sh` (`SELFTEST=1` must fail); it launches `--offscreen`
because a menu assertion needs the client's screen tick. Keep validation claims
and supported revisions honest.

# Mekanism: MoreMachine for Horizon — the Basic Recycling Factory

A native Horizon mod package (`.hmod`, mod API **0.2**) carrying its own content, for
Mekanism: MoreMachine (mekmm) 1.4.2 (MIT, `lostmyself8/Mekanism-MoreMachine`, tag
`v1.4.2-1.21.1`), the exact ATM10 jar `mekmm-1.21.1-1.4.2.jar`, an add-on of Mekanism.

This is slice 1: **one factory at one tier**.

- `mekmm:basic_recycling_factory` — the basic tier of the recycling factory family: **3
  parallel processes**, **200 ticks** per operation, **50 FE/t per process**, one shared
  **20,000 FE** buffer (10,000 FE per operation), one input and one output slot per process,
  and `mekmm:scrap` ×1 on a **17 %** chance roll. No energy, no recipe, too few items, a full
  output slot and a missed roll all behave as upstream's `CachedRecipe` does.
- `mekmm:scrap` — the factory's output item, on the jar's model and texture.
- The vanilla 3×3 menu shows the nine container slots; `/mekmm info` prints the energy, the
  three progresses, the three cycle counters and the six working slots.

One flat package holds both halves: `assets/` and `data/` are the carried block, item, models,
textures, lang, loot table, tags and creative tab, and `src/` is the Rust — one
`impl mod_api::Hooks`, the block entity Part and the declarations beside the pure runtime.
`src/runtime.rs` is the port's logic (the factory tick, the roll, the recipe table) with its
tests; `src/basic_recycling_factory.rs` is the glue.

```sh
# the SDK ladder (inside a ports/tools/build-slot.sh lease; CARGO_INCREMENTAL=0)
cargo test                                        # 25 tests
cargo clippy --all-targets -- -D warnings
cargo fmt --check                                 # not --all: that formats the fetched SDK
rm -f Cargo.lock && cargo horizon-mod build        # -> target/horizon-mod/mekmm.hmod
cargo horizon-mod check mekmm.hmod
cargo horizon-mod fits mekmm.hmod --build nightly
GAME=<launcher-built client> tools/e2e.sh          # the in-game check
SELFTEST=1 GAME=… tools/e2e.sh                     # must exit non-zero
```

Read [notes.md](notes.md) for the slice boundary, the deviations, the gap list and the SDK
migration, [FINDINGS.md](FINDINGS.md) for the upstream analysis (where every number came from)
and [VALIDATION.md](VALIDATION.md) for the ladder evidence.

## Licence and status

MIT, read from the jar's `META-INF/neoforge.mods.toml`; the carried assets are under the same
declaration (no `ASSETS_LICENSE*` in the jar). **Local-only** until the maintainer approves
publishing (campaign ARCHITECTURE.md §8). See [LICENSE](LICENSE) and
[LICENSE-NOTICE.md](LICENSE-NOTICE.md).

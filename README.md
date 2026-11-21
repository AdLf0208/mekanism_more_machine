# Mekanism: MoreMachine for Horizon — the recycling factory's four tiers

A native Horizon mod package (`.hmod`, mod API **0.2**) carrying its own content, for
Mekanism: MoreMachine (mekmm) 1.4.2 (MIT, `lostmyself8/Mekanism-MoreMachine`, tag
`v1.4.2-1.21.1`), the exact ATM10 jar `mekmm-1.21.1-1.4.2.jar`, an add-on of Mekanism.

This slice is the **recycling factory family at every tier the mod registers without Evolved
Mekanism** — `basic`, `advanced`, `elite`, `ultimate`:

| block | processes | buffer | slots (grid) |
|---|---:|---:|---|
| `mekmm:basic_recycling_factory` | 3 | 30,000 FE | 9 (3×3) |
| `mekmm:advanced_recycling_factory` | 5 | 50,000 FE | 18 (9×2) |
| `mekmm:elite_recycling_factory` | 7 | 70,000 FE | 18 (9×2) |
| `mekmm:ultimate_recycling_factory` | 9 | 90,000 FE | 27 (9×3) |

Every process runs **200 ticks** at **50 FE/t**, so one operation costs **10,000 FE** however
many run in parallel; the buffer is upstream's
`max(storage × 0.5, usage) × processes` (`MoreMachineFactory.setMachineData`). Each process has
one input and one output slot and produces `mekmm:scrap` ×1 on the jar's **17 %** chance roll.
No energy, no recipe, too few items, a full output slot and a missed roll all behave as
upstream's `CachedRecipe` does. `/mekmm info` prints a factory's energy, its per-process
progresses and cycle counters and its working slots; the container opens the smallest vanilla
chest grid that holds its slots.

One flat package holds both halves: `assets/` and `data/` are the carried blocks, items, models,
textures, lang, loot tables, tags and creative tab, and `src/` is the Rust — one
`impl mod_api::Hooks`, one block entity Part per tier and the declarations beside the pure
runtime. `src/runtime.rs` is the port's logic (the tiers, the factory tick, the roll, the recipe
table) with its tests; `src/recycling_factory.rs` is the glue.

```sh
# the SDK ladder (inside a ports/tools/build-slot.sh lease; CARGO_INCREMENTAL=0)
cargo test                                        # 35 tests
cargo clippy --all-targets -- -D warnings
cargo fmt --check                                 # not --all: that formats the fetched SDK
rm -f Cargo.lock && cargo horizon-mod build        # -> target/horizon-mod/mekmm.hmod
cargo horizon-mod check mekmm.hmod
cargo horizon-mod fits mekmm.hmod --build 0b14f8a  # a different build: the real comparison
GAME=<launcher-built client> tools/e2e.sh          # the in-game check (--offscreen)
SELFTEST=1 GAME=… tools/e2e.sh                     # must exit non-zero
```

Read [notes.md](notes.md) for the slice boundary, the deviations, the coverage table and the gap
list, [FINDINGS.md](FINDINGS.md) for the upstream analysis (where every number came from) and
[VALIDATION.md](VALIDATION.md) for the ladder evidence.

## Licence and status

Mekanism: MoreMachine is **MIT** (read from the jar's `META-INF/neoforge.mods.toml`); this port
is a clean-room Rust rewrite of its behaviour, not a translation. Local-only until the
maintainer approves publishing.

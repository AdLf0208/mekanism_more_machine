# Mekanism:More Machine — a Horizon port

**Mekanism:More Machine** is a Minecraft mod. Adds more useful machines to Mekansim. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [lostmyself8/Mekanism-MoreMachine](https://github.com/lostmyself8/Mekanism-MoreMachine) — licence **MIT**.

| | |
|---|---|
| Upstream mod | Mekanism:More Machine ([source](https://github.com/lostmyself8/Mekanism-MoreMachine)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `mekmm.hmod` (install it with the Horizon launcher) |
| Licence | **MIT** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

- **The embedded-content control leg is gone.** 0.1.0 proved the block came from the bundle's
- **A non-factory block entity is "No block entity there." by construction** (trap 2), which is
- **Invalid command input** is answered by the server's dispatcher, not by four hand-written

## Not yet ported

- **The rest of the mod.** The other 15 factory families (144 factory
- **The chemical system** (Mekanism gases/slurries/pigments) is the parent's
- **The recycler itself** (`mekmm:recycler`, the single-machine form of this
- **`mekanism:substrate`** recipe (chance 0.43) is dropped: no such item here.
- **Redstone control, side config, upgrades, the ejector, the active state**
- **No in-game run at 0.2.0.** `horizon-launcher` is not installed in this environment, so a
- **The menu is the vanilla `minecraft:generic_3x3`** (unchanged from 0.1.0): a mod menu type
- **Invalid-input command text differs** (trap 5).

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `mekmm.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

Mekanism:More Machine was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **MIT**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/lostmyself8/Mekanism-MoreMachine

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run mekmm` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/mekmm/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).

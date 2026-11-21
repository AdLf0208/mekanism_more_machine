# mekmm (Mekanism: MoreMachine) → Horizon — the recycling factory's four tiers

Ported from **Mekanism: MoreMachine 1.4.2** (`lostmyself8/Mekanism-MoreMachine`,
**MIT**), the exact ATM10 jar `mekmm-1.21.1-1.4.2.jar`
(3,721,605 B, SHA-256 `f019b2d47d4dd6dfd72f066bb86b1572aa9089ac530221ef2b2ff136218f3c53`;
licence read from `META-INF/neoforge.mods.toml`). Upstream behaviour was read
from the tag matching the jar, **`v1.4.2-1.21.1`** (commit
`fd2108ffd998351dc37064ea240736f6c345ceed`); analysis only, no Java translated,
no jar decompiled. `FINDINGS.md` records the files read and every number.

**LOCAL-ONLY** until the maintainer approves publishing (ARCHITECTURE.md §8).
MIT is licence rank 0 (permissive); the approval is still required.

Status: **version 0.2.2 — the recycling family at its four real tiers** (2026-11-21, pin
`0a875da`): `mekmm:{basic,advanced,elite,ultimate}_recycling_factory` — 3/5/7/9 parallel
processes, 200 ticks per operation, 50 FE/t per process, one shared buffer of
30,000/50,000/70,000/90,000 FE, one input and one output slot per process, `mekmm:scrap` on a
17 % chance roll, plus the item, the per-tier container and persistence. The slice is written
up under [`## Slice 2`](#slice-2--the-factorys-four-real-tiers-version-022-2026-11-21) below;
everything else in the mod is a planned gap (the list follows). The in-game leg is measured at
this version (`ports/_shared/evidence/mekmm-e2e/`): PASS + `EXIT:0`, control `EXIT:1`,
`SELFTEST=1` `EXIT:1`.

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port

## Class + tier, approach, licence

- **Class `content`, tier `machines`** (`ports/ledger.csv`, from
  `ports/_shared/source-map/atm10-sources.csv`), licence **MIT → rank 0**,
  **add-on** of Mekanism (the jar's `neoforge.mods.toml` requires
  `mekanism [10.7.19,)`).
- **Approach from the tier** (`horizon/docs/modpacks.md`): a machines port is a
  block entity with its state in NBT, a container/menu, an energy capability and
  a recipe tick; **there is no cross-mod API**, so the Mekanism machinery this
  factory needs (a FE buffer, per-process progress, input/output slots, the
  `CachedRecipe` tick order) is re-implemented in this crate, using
  `ports/mekanism` only as the *pattern* — no crate of another port is imported
  or called.
- Queue position: MIT rank 0, mechanics, add-on with a `ported-partial` parent
  (`mekanism`), so it sorts after the core mods of its rank and is workable
  now. `precheck-mod.py mekmm` → exit 0, DISPATCHABLE.

## Why this one factory (the artefact, not the brief's example)

The brief offered an `*_smelting_factory` or the `*_liquifying_factory`. **This
jar has no `smelting_factory`** (smelting is Mekanism's own factory), and the
`liquifying` family needs Mekanism chemicals, which are outside this slice.
Of the jar's **16 factory families**, only the `recycling` family has recipes
that work over 26.3's own items: the other item→item families (`lathing`,
`rolling_mill`, `stamping`, `pressing`) ship **only** `neoforge:mod_loaded`-gated
`compat/` recipes (Immersive Engineering, Create Addition, AE2 families…), and
the register-based families (`centrifuging`, `crystallizing`, `dissolving`,
`oxidizing`, `painting`, `pigment_extracting`, `planting`,
`pressurised_reacting`, `replicating`, `washing`, `liquifying`) need Mekanism
chemicals (slurries, pigments, `mekmm:nutrient_solution`). The recycler is the
one machine whose behaviour is faithful without the parent's chemistry. The
artefact beat the brief's example; that is recorded here rather than silently
swapped.

## Behaviour (slice 1 — the basic tier as 0.1.0–0.2.1 shipped it)

*The 20,000 FE buffer row below is **superseded**: the same jar's factory builder gives the
basic tier 30,000 FE, and slice 2 corrected it (see
[`## Slice 2`](#slice-2--the-factorys-four-real-tiers-version-022-2026-11-21) → "The numbers,
and the defect this slice fixes"). The rest of the table still describes the port. Every
"20,000" in this historical table is the **single machine's** `StorageConfig.recycler`.*

| | Upstream | This port |
|---|---|---|
| machine | `TileEntityRecyclingFactory` on `TileEntityMoreMachineFactory` (Mekanism `FactoryTier.BASIC`) | the same shape in `runtime::factory_tick` |
| processes | **3** (`FactoryTier.BASIC.processes`) | `BASIC_PROCESSES = 3`, three independent progress counters |
| ticks per operation | **200** (`BASE_TICKS_REQUIRED = 10 * 20`) | `RECYCLER_TICKS = 200` |
| energy | **50 FE/t per process** (`UsageConfig.recycler`) | `RECYCLER_USAGE = 50`, spent per process per tick |
| buffer | **20,000 FE**, shared by the three processes (`StorageConfig.recycler`) | `RECYCLER_CAPACITY = 20_000` via `native_block_entity_energy` |
| energy per operation | 10,000 FE (200 × 50), pinned by the e2e | same |
| slots | one input + one output per process, plus the energy slot | inputs `0,1,2`, outputs `3,4,5`, the energy slot `6` declared and unused; `7,8` are the menu's spare cells (see deviations) |
| input use | consumed at the **end** of the operation (`OneInputCachedRecipe.finishProcessing`) | consumed at `progress == 200`, never earlier (unit-tested and shown in game: 199 ticks leave the stone) |
| output | `mekmm:scrap` ×1 on a **0.17** chance; a miss still consumes the input and the energy | `Recipe.chance_percent = 17`, `Outcome::Produced { output: None }` on a miss |
| output room | the max chance output must fit before energy is spent | `NoOutputSpace` holds the progress, spends nothing |
| error rules | no input / no recipe / too few items **reset** the progress; no energy holds it | `IdleReason::{NoInput, NoRecipe, NotEnoughInput}` reset; `{NoEnergy, NoOutputSpace}` hold (unit-tested) |
| recipes | `mekmm:recycler`: `c:stones` → scrap (0.17), `dirt`/`grass_block` → scrap (0.17), `mekanism:substrate` → scrap (0.43) | 11 rows: the 9 `c:stones` members and `dirt`/`grass_block` at 17 %; `substrate` dropped (not registered here) |
| block | `BlockMoreFactoryMachine`, hardness 3.5 / resistance 16, `requiresCorrectToolForDrops`, stone sounds, `facing` × `active` | barrel template (same numbers, same two properties via `facing` × `open`), the jar's five face textures as a plain cube |
| loot | self-drop with `copy_name` + 9 `mekanism:*` components | plain self-drop (26.3 singular `survives_explosion`); the component copy has no port |
| tags | `mineable/pickaxe`, not `needs_iron_tool` | same single entry |

### Hooks

*The 0.1.0 `.hnmod` wiring, kept as history. The 0.2.0 mapping is the
helper→hook table in
[`## Migration to the SDK format`](#migration-to-the-sdk-format-version-020-2026-11-18);
these injections no longer exist.*

| Hook (API) | Helper | What it does |
|---|---|---|
| `content_root` (HEAD, cancellable) | `embedded_content.rs` | Writes the embedded `.hmod` to `.horizon-cache/` and appends it to the dev list (AE2 pattern): one `.hnmod` ships the content and the code |
| `block_entity` (15/129) | `block_entity.rs` | Gives the factory a block entity |
| `block_entity_ticks` (15) | `ticks.rs` | The factory ticks |
| `block_entity_container` (16/17) | `container.rs` | Nine slots (3 in, 3 out, the energy slot and 2 spare) and the `minecraft:generic_3x3` menu |
| `block_entity_energy` (142) | `energy.rs` | The 20,000 FE buffer (`energy` in the saved compound) |
| `block_entity_tick` (17) | `tick.rs` | One factory tick: three processes, one buffer |
| `register_commands` (8) | `register.rs` | `/mekmm info\|energy\|chance\|progress` |
| `perform_command` (8) | `perform.rs` | The command bodies |

Harness additions (recorded, like `/mekanism energy`): `/mekmm info <pos>` prints a
factory's FE, the three progresses, the three cycle counters and the six slots;
`/mekmm energy <amount> <pos>` (op 2) sets its FE; `/mekmm chance <0|1|2> <pos>`
(op 2) pins the chance roll off/on or back to the real roll; `/mekmm progress`
prints the counters. They are the controls the in-game check uses; the machine
works without them.

## Deviations

1. **The chance roll is deterministic, not a world RNG.** Upstream rolls a
   `RandomSource` per operation. This port derives the roll from the block
   position (mixed with the dimension), the process index and the process's
   completed-operation counter — a pure SplitMix64 in `runtime.rs`, exact
   percentages, no shared RNG state. A factory therefore replays the same
   sequence every world, the rate is exactly 17 % over a run (unit-tested over
   10,000 seeds), and the in-game check can pin the roll on/off with the
   harness command instead of depending on a lucky roll. The `cycles` counter it
   needs is saved as an IntArray (`progress` is the other).
2. **Slots**: upstream interleaves input/output per process and adds an energy
   slot (7 slots). This port groups them (inputs `0,1,2`, outputs `3,4,5`) so
   the 3×3 vanilla menu reads left-to-right. The energy slot is declared at `6`
   but unused (no item energy in this slice), and `7,8` exist because a mod
   container's menu **must have exactly as many slots as the menu type**
   (`native_hooks.rs` filters any other size out) and the pin's only 9-slot
   vanilla grid is `minecraft:generic_3x3`.
3. **The GUI** is that vanilla 3×3 grid: upstream's own factory screen, its
   progress bars, energy gauge, side-config and sorting buttons are not drawn.
   The readout is `/mekmm info`.
4. **`active` block state**: the blockstate carries both variants (as
   `facing` × `open` on the barrel template) but the port never sets it, so the
   idle front always shows; upstream swaps the LED front.
5. **Not craftable**: the jar's own crafting recipe needs `mekmm:recycler`,
   Mekanism alloys and circuits, so it is not carried; the block is given by
   command in the check. Breaking the factory drops the block only (upstream
   copies energy and contents into the item via `copy_components`).
6. **No side configuration, no upgrades, no ejector, no redstone control**: the
   factory runs on its own tick with every face able to insert energy and reach
   the slots. Upstream's config component, speed/energy upgrades and ejector are
   later slices.
7. **`c:stones` is the port's definition** of the convention tag (26.3 ships
   none): stone, granite, diorite, andesite, deepslate, tuff, calcite,
   dripstone_block, smooth_stone — the same membership
   `ports/cookingforblockheads` carries.
8. **The `.hnmod` is stored, not deflated** (`tools/package.py`): the same
   sources produced **19,758 B / `715a4fde…`** on the second machine and
   **19,818 B / `cf80f974…`** here, with byte-identical entries inside — a
   zip's deflate stream depends on the zlib behind Python, and the registry
   pins a SHA-256. With `ZIP_STORED` the archive is a function of the sources
   alone and both machines now agree on the delivered file
   (**58,547 B / `31b43c0b…`**). The `.hmod` (built by the pin's
   `cargo-horizon-mod`, flate2) was already identical on both
   (`2035b553…`).

## Gaps

- **The rest of the mod.** The other 15 factory families (144 factory
  blockstates), the chemical tanks (`mid`/`max`), every single machine
  (`recycler`, `planting_station`, `presser`, `cnc_lathe`, `cnc_rolling_mill`,
  `cnc_stamper`, `replicator`, `fluid_replicator`, `chemical_replicator`), the
  `large_*` multiblocks, the silver ore/block/item set, the
  `scrap_box`/`uu_matter`/crystals and the compat recipes are **not ported**.
- **The five Evolved Mekanism tiers** (`overclocked`, `quantum`, `dense`,
  `multiversal`, `creative`) are `blocked(evolvedmekanism-not-ported)`:
  `MoreMachineUtils.getFactoryTier()` registers only Mekanism's four
  (`EnumUtils.FACTORY_TIERS`) and appends `MoreMachineEnumUtils.EM_TIERS` only when
  `evolvedmekanism` is loaded (`MoreMachineEnumUtils.java:50-56`). Their
  blockstates and models are in the jar; the tier type they extend is not.
- **The chemical system** (Mekanism gases/slurries/pigments) is the parent's
  deep tier; without it, the families that need it cannot be faithful.
- **The recycler itself** (`mekmm:recycler`, the single-machine form of this
  factory) is not registered — the factory is the slice.
- **`mekanism:substrate`** recipe (chance 0.43) is dropped: no such item here.
- **Redstone control, side config, upgrades, the ejector, the active state**
  and the machine's own GUI remain gaps (deviations 3–6).

## Upstream sources read

Listed file by file in `FINDINGS.md` (tag `v1.4.2-1.21.1`): the recycling
factory tile, the factory base, the recycler tile, `RecyclerRecipe`,
`MoreMachineOutputHelper`, `MoreMachineOneInputCachedRecipe`, the usage/storage
configs, the block and block-type registries, the block prefab, the jar's
`recycler` recipes, loot table, lang, blockstates, models and textures, plus
Mekanism's `FactoryTier`, `CachedRecipe` and `OneInputCachedRecipe` for the tick
order and 3-process tier. Upstream differences and simplifications are the
deviations above; nothing was copied as code.

## Verification

*The 0.1.0 ladder below is **historical**: it is the retired `.hnmod` bundle's,
kept because its in-game run is the behaviour's evidence. The 0.2.0 SDK package's
own ladder (tests, lints, format, build, `check`, `fits`) is in
[`## Migration to the SDK format`](#migration-to-the-sdk-format-version-020-2026-11-18);
its in-game run was **not** made (no `horizon-launcher` here).*

Pin **`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`**, mixins
**`5a64af938ec86b00d53d16a408aabc0038fb3e57`** (schema 6). Every build, test
and e2e ran through `ports/tools/build-slot.sh` (never a bare `cargo`,
`CARGO_INCREMENTAL=0`, no `--target-dir`), with
`--horizon ports/_shared/pins/horizon-1d63e0a`; the e2e used the pin-matched
shared `horizon-ctl` (`ports/_shared/bin/horizon-1d63e0a5/`). Evidence:
`ports/_shared/evidence/mekmm/`.

### Ladder

| Step | Command (through the slot runner) | Result |
|---|---|---|
| runtime tests (RED) | `cargo test --offline` before the implementation | **110 errors**, the tests' API missing (`unit-red.log`, exit 101) |
| runtime tests | `cargo test --offline --locked` in the port | **19 passed**, 0 failed (`test-green.log`) |
| runtime lints | `cargo clippy --all-targets --locked --offline -- -D warnings` | exit 0, **0 warnings** (`clippy.log`) |
| format | `cargo fmt --all -- --check` + `rustfmt --check` on the injected helper | clean (`fmt.log`, `fmt-helper.log`) |
| stage | `python3 tools/horizon.py stage` | exit 0, `stage_changed: true` (`stage.log`) |
| check | `python3 tools/horizon.py check` | `cargo_exit: 0`, **0 warnings** in `target/horizon-check.log` (`check.log`) |
| embedded test | `python3 tools/horizon.py test` | `test result: ok. **19 passed**; 0 failed; 1240 filtered out`, `cargo_exit: 0` (`test.log`, `test-count.log`) |
| fast build | `python3 tools/horizon.py build` | `cargo_exit: 0` (`build.log`) |
| content + package | `tools/ladder-embed.sh` → `tools/package.sh` | `.hmod` 6,830 B / 17 entries; `.hnmod` 58,547 B / 13 entries; native check passes (`package.log`) |
| e2e (one `.hnmod`, no `--dev-mod`) | `build-slot.sh e2e -- bash tools/e2e-embed.sh` | **PASS, `EXIT:0`**, control leg first (`e2e-embedded.log`) |
| fail-capable run | the same with `SELFTEST=1` | **FAIL as required**, `EXIT:1` (`e2e-embedded-selftest.log`) |
| gates | `check-e2e-logs.py mekmm --strict`, `check-skills.py --notes ports/mekmm` | exit 0 (below) |

### The in-game check (`ports/_shared/evidence/mekmm/e2e-embedded.log`)

*Historical (0.1.0 `.hnmod` bundle, 2026-11-11). The SDK package carries the same
content and code inside one `.hmod`, and the same scenario is in the rewritten
`tools/e2e.sh`; it was not run at 0.2.0.*

One file only: the launched game has **no `--dev-mod`**, the content comes from
the bundle's embedded copy. (`tools/e2e.sh` can also run the same scenario with
`--dev-mod content/` — `EMBEDDED=0`, the default — but that leg was not run for
this slice: the shipping path is the embedded one, and the content it loads is
byte-identical to `content/mekmm.hmod`, which the control leg and
`cargo horizon-mod check` both cover.) The End arena, floor y=63; A `8,64,8`, B `12,64,8`,
C `16,64,8`. Quoted verbatim:

```
== control: the same game with no embedded content file ==
  placed block: minecraft:air
  native command: No block entity there.
  without the embedded file: no factory block; the native command still answers (it is in the binary)
== launch (The End arena, content + native bundle) ==
  embedded content: 6830 B at /tmp/opencode/mekmm-home.MEAE39/.horizon-cache/mekmm-embedded-content.hmod, identical to content/mekmm.hmod
== the factories place and their block entities answer ==
  info(8,64,8): mekmm:basic_recycling_factory: 0/20000 FE, progress [0, 0, 0]/200, cycles [0, 0, 0], in [empty, empty, empty], out [empty, empty, empty]
== control 1: without energy three stones do not recycle ==
  info(12,64,8): mekmm:basic_recycling_factory: 0/20000 FE, progress [0, 0, 0]/200, cycles [0, 0, 0], in [minecraft:stone x1, minecraft:stone x1, minecraft:stone x1], out [empty, empty, empty]
== one powered process: one stone -> one scrap in 200 ticks for 10,000 FE ==
  info(8,64,8): mekmm:basic_recycling_factory: 10000/20000 FE, progress [0, 0, 0]/200, cycles [1, 0, 0], in [empty, empty, empty], out [mekmm:scrap x1, empty, empty], chance forced(true)
== three processes share the buffer: 10,000 FE is 67/67/66 ticks ==
  info(8,64,8): mekmm:basic_recycling_factory: 0/20000 FE, progress [67, 67, 66]/200, cycles [1, 0, 0], in [minecraft:stone x1, minecraft:stone x1, minecraft:stone x1], out [mekmm:scrap x1, empty, empty], chance forced(true)
== the three operations finish on a 20,000 FE top-up ==
  info(8,64,8): mekmm:basic_recycling_factory: 0/20000 FE, progress [0, 0, 0]/200, cycles [2, 1, 1], in [empty, empty, empty], out [mekmm:scrap x2, mekmm:scrap x1, mekmm:scrap x1], chance forced(true)
== control 2: a missed chance roll consumes the input and the energy, and makes no scrap ==
  info(8,64,8): mekmm:basic_recycling_factory: 10000/20000 FE, progress [0, 0, 0]/200, cycles [3, 1, 1], in [empty, empty, empty], out [mekmm:scrap x2, mekmm:scrap x1, mekmm:scrap x1], chance forced(false)
== control 3: an input with no recipe burns nothing and resets the progress ==
  info(16,64,8): mekmm:basic_recycling_factory: 20000/20000 FE, progress [0, 0, 0]/200, cycles [0, 0, 0], in [empty, empty, minecraft:diamond x1], out [empty, empty, empty]
== control 4: a full output slot holds the progress and burns nothing ==
  info(8,64,8): mekmm:basic_recycling_factory: 20000/20000 FE, progress [0, 0, 0]/200, cycles [3, 1, 1], in [minecraft:stone x1, empty, empty], out [mekmm:scrap x64, mekmm:scrap x1, mekmm:scrap x1], chance forced(true)
== the factory opens its 3x3 menu with the nine slots ==
== persistence: a factory process stalled at progress 60 with 0 FE reads back after a relaunch ==
  before save: mekmm:basic_recycling_factory: 0/20000 FE, progress [0, 60, 0]/200, cycles [3, 1, 1], in [minecraft:stone x1, minecraft:stone x1, empty], out [mekmm:scrap x64, mekmm:scrap x1, mekmm:scrap x1], chance forced(true)
  after relaunch: mekmm:basic_recycling_factory: 0/20000 FE, progress [0, 60, 0]/200, cycles [3, 1, 1], in [minecraft:stone x1, minecraft:stone x1, empty], out [mekmm:scrap x64, mekmm:scrap x1, mekmm:scrap x1], chance forced(true)
PASS: ...
EXIT:0
```

What the numbers prove:

- **one operation = 200 ticks × 50 FE/t = 10,000 FE**: the buffer goes
  20,000 → 10,000 and the stone becomes exactly one scrap;
- **three processes are three processes**: with 10,000 FE the three progress
  counters read `[67, 67, 66]` — 66 ticks × 3 × 50 = 9,900 FE spent, then the
  first two processes take the last 100 FE on tick 67 and the third is left at
  66 with no energy. A single-process machine would read `[67, 0, 0]`; a machine
  that spent 50 FE/t in total would have finished 200 ticks. The exact stall is
  the measurement;
- **the buffer is one buffer**: the top-up of 20,000 FE is exactly the 20,000
  the three operations still need (133/133/134 remaining ticks × 50 FE), and
  they finish with the buffer at 0 and three scrap;
- **a missed roll is not a free operation**: with the roll forced off, the
  input and the 10,000 FE are gone and no scrap appears;
- **no recipe, no spend**: a diamond leaves the progress at 0 and all 20,000 FE;
- **a full output holds everything**: 64 scrap in an output slot keeps the
  progress at 0 and the 20,000 FE untouched;
- **the menu is the container**: nine slots open and the 64 scrap is in one;
- **the block entity's compound survives a save**: a process stalled at 60/200
  with 0 FE reads back identically after quit and relaunch, with its cycles and
  slots.
- **the embedded file is what makes it work**: the same binary with
  `.horizon-cache` unusable places no block (the id is unknown) while
  `/mekmm info` still answers "No block entity there." — the run cannot pass
  without the content inside the bundle.

### Fail-capable check (`e2e-embedded-selftest.log`)

`SELFTEST=1 EMBEDDED=1` flips the first operation's assertion (the output must
stay empty). The same scenario then fails as required:

```
FAIL: expected /mekmm info at 8,64,8 to say 'out [empty, empty, empty]', got
'mekmm:basic_recycling_factory: 10000/20000 FE, progress [0, 0, 0]/200, cycles [1, 0, 0], in [empty, empty, empty], out [mekmm:scrap x1, empty, empty], chance forced(true)'
EXIT:1
```

An earlier run failed on a real mismatch too — the script put the control's
stones into upstream's interleaved slots (`0, 2, 4`) while the port groups them
(`0, 1, 2`), and the info line showed the third stone in an output slot
(`e2e-embedded.log` of that run). The assertions bite.

### Artifacts (this pin, 2026-11-11 — the retired 0.1.0 bundle)

| Item | Path | Size | SHA-256 | Entries |
|---|---|---|---:|---|
| content mod | `content/mekmm.hmod` | 6,830 B | `2035b55325af7912838f0c4ba3bb4a0787816b8af946c949ebf52903312e50d0` | 17 |
| the deliverable | `mekmm.hnmod` | 58,547 B | `31b43c0b984a8e2e40a0c75a51702b1b08121437c8e4cafaa189499399418abf` | 13 |

The `.hnmod` was built here and on the second machine and the two files are
**byte-identical** (same size, same SHA-256), and a second local run reproduces
it too.

The `.hnmod` holds `horizon-native.json`, the ten mixin files and
`mixins/mekmm/data/mekmm-content.hmod` (the built content, byte for byte).
`cargo horizon-mod check` reports `mekmm 0.1.0 (native-mod, format
native-1/1d63e0a5…/5a64af93…, for 26.3)`.

## Gates

- `python3 ports/tools/check-e2e-logs.py mekmm --strict` → exit 0.
- `python3 ports/tools/check-skills.py --notes ports/mekmm` → PASS.
- `python3 ports/tools/upstream_ready.py mekmm` → **exit 1**, with exactly two
  blockers, both outside this port's write ownership:

  ```
  BLOCKED  mekmm                        MIT
             - status is pending, not ported
             - add-on whose required parent(s) mekanism are not in the registry - the reviewer closes this (see refinedstorage_quartz_arsenal, #498)
  ready=0 blocked=1 of 1
  ```

  The ledger row is the orchestrator's (it records after re-reading this
  evidence; `ports/ledger.csv` is not mine to edit) and the parent `mekanism`
  is not in `ports/_shared/horizon-mods/registry-ids.txt` yet (an upstream
  submission, which `upstream_ready.py` itself says the reviewer closes). The
  port's own half is satisfied: the tree and the one `.hnmod` exist in the
  port (not `target/`), `notes.md` carries the `Skills:` line, and
  `e2e-embedded.log` ends `PASS` + `EXIT:0` — the earlier “no port directory”
  and “no e2e evidence” blockers are gone. Re-run after the ledger row is set
  and Mekanism is listed.

## Review repairs (PR #675)

1. Staging (`mekmm.hnmod` absent from the registry release/staging) is the
   orchestrator's to answer (horizon-mods#686).
2. *"Include the required copyright/license notice in the distributed
   artifact"* — the packager now ships `mixins/mekmm/LICENSE` and `NOTICE.md`
   inside the `.hnmod`, and both sit at the root of the embedded
   `mekmm-content.hmod`; repackaged twice, same SHA-256.
3. *"Correct the original attribution URL … Also put the material 'requires
   parent #568 for crafting' limitation on the public entry"* — the entry uses
   `https://github.com/lostmyself8/Mekanism-MoreMachine` and states, under
   **Needs Mekanism for crafting**, that horizon-mods#568 is open and the block
   is creative-only without it.

### Verification after the repairs

- Artifact: `mekmm.hnmod`, 62,333 B, SHA-256 `3e0c8a824a2be46e460acce6168b3b65b0b7ce227bea6e22a7852a0b2f3d9545` (rebuilt twice, identical bytes; `tools/package.py`; the embedded content equals `content/mekmm.hmod`).
- `upstream_ready.py mekmm` exits 1 as **BLOCKED**: an add-on whose parent (`mekanism`) is not in the registry yet — the orchestrator answers that (the parent's PR is #568). `check-skills.py --notes ports/mekmm` exits 0.

## Migration to the SDK format (version 0.2.0, 2026-11-18)

Horizon's mod-API merge (#16941, launcher ≥ 0.2.70, PR #17992) retired `.hnmod`, and the
registry closed every `.hnmod` submission with one reason. The unit is now **one flat
`.hmod`**: content, native code, or both. Pins for this migration: nightly
**`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (2026-11-18, `package_format` 6,
`content_api` "0.2"), mixins **`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, launcher 0.2.78.
The recipe followed is `ports/imfast/notes.md` → "Migration to the SDK format" plus the family
traps in `ports/mekanismgenerators/notes.md` (the first content+code migration, same Mekanism
machine shape: one `block_entity_type`, one Part, `block_container`/`block_energy` beside it).

### The line that makes this a migration and not a rewrite

`src/runtime.rs` is `mixins/mekmm/runtime.rs` **byte-identical**: SHA-256
`94dac43c585d392e62a138158ccf01e46a0928b7dec6cee9dc0698c9f63352b3`, `cmp` exit 0 against the
retired file, and the **packaged** entry (`unzip -p mekmm.hmod src/runtime.rs`) hashes to the
same value. Its **19 tests were green before any glue existed** — `src/lib.rs` held only
`pub mod runtime;` (`sdk-runtime-first.log`: `test result: ok. 19 passed; 0 failed`). All 19
still pass with the glue (25 tests).

### What moved, and where the retired tree is

Moved (**not** deleted) to `ports/_shared/evidence/mekmm/retired-hnmod-0.1.0/` **before the
first `cargo horizon-mod build`**, because the project root may hold package paths only
(`imfast` trap 1): `mixins/` (`horizon-mixin.toml`, `runtime.rs`, `support.rs`, the eight
hook helpers, the embedded `data/mekmm-content.hmod`), `mekmm.hnmod` (62,333 B),
`compatibility.json`, `rust-toolchain.toml`, the 0.1.0 `Cargo.toml`/`Cargo.lock`, `content/`
(`horizon-mod.toml`, `assets/`, `data/`, `mekmm.hmod` 6,830 B) and the six retired tools
(`horizon.py`, `package.py`, `package.sh`, `add-notices.py`, `ladder-embed.sh`,
`e2e-embed.sh`).

| 0.1.0 path | 0.2.0 path | Evidence |
|---|---|---|
| `mixins/mekmm/runtime.rs` | `src/runtime.rs`, **`cp` + `cmp`** | retired file kept |
| `mixins/mekmm/support.rs` + 8 helpers | `src/basic_recycling_factory.rs` (glue) + `src/lib.rs` (the one `Hooks`) | retired `mixins/` |
| `mixins/mekmm/horizon-mixin.toml` (7 `[[injection]]`, 1 `content_root` target, 2 `[[module]]`) | no plan: typed hook methods, one Part, two `Registrar` declarations | retired `mixins/` |
| `content/assets/`, `content/data/` | `assets/`, `data/` at the package root — **unchanged** (`diff -r` clean, 16 files) | retired `content/` |
| `content/horizon-mod.toml` | merged into the root `horizon-mod.toml` (+ `[native]`, `api = "0.2"`, `capabilities`) | retired `content/` |
| `content/mekmm.hmod` (6,830 B content carry) | not needed: content is inside the one package | retired `content/` |
| `mekmm.hnmod` (62,333 B) | `mekmm.hmod` (32,735 B, native + content) | retired root |
| `compatibility.json`, `rust-toolchain.toml` | `[native]` in `horizon-mod.toml`; the pinned SDK decides the toolchain | retired root |
| `tools/horizon.py`, `package.py`, `package.sh`, `add-notices.py`, `ladder-embed.sh`, `e2e-embed.sh` | retired: staging, `.hnmod` packaging and the embedded-content ladder | retired `tools/` |
| `Cargo.lock` (157 B) | not kept: it is packed and changes the artifact (`imfast` trap 5) | retired root |

**Kept:** `tools/e2e.sh` (rewritten for a launcher-built game, see below), `notes.md`,
`README.md` (rewritten — it is packed), `FINDINGS.md`, `VALIDATION.md`, `LICENSE`,
`LICENSE-NOTICE.md`.

### File map (0.2.0)

| Path | What it is |
|---|---|
| `horizon-mod.toml` | the only manifest and identity: `id = "mekmm"`, `version = "0.2.0"`, `api = "0.2"`, `side = "both"`, `capabilities = ["registries"]`, licence/homepage/issues, `[native]` `game`/`horizon`/`mixins` |
| `Cargo.toml` | one crate, edition 2024; its only dependency is `horizon-mod-api` from `.horizon/sdk/` |
| `src/lib.rs` | the one `#[mod_api::horizon_mod] impl Hooks`: `register`, `register_commands`, `run_mod_command` |
| `src/basic_recycling_factory.rs` | the glue: `FactoryPart` (a `BlockEntityPart`, plus its empty `BlockPart`), `container()`, `battery()`, `declare()`, the saved-compound codec, `info_line`/`set_energy`/`set_chance`/`progress_line`, and 6 tests of the declarations and the readout |
| `src/runtime.rs` | the pure logic, byte-identical, 19 tests |
| `assets/mekmm/**`, `data/**` | the carried content (blockstate, models, six textures, lang, block/item/creative-tab definitions, loot table, pickaxe tag) |
| `README.md`, `LICENSE`, `LICENSE-NOTICE.md` | packed by the builder |
| `tools/e2e.sh` | the in-game scenario, now for a launcher build (`GAME=`), with `CONTROL_GAME=` and `SELFTEST=1` |
| `mekmm.hmod` | the deliverable, **32,735 B**, 25 entries |

### Helper → hook table

Each former `[[injection]]` helper is now one method, one Part method or one declaration; the
helper file itself is gone. **7 hook injections mapped, 0 recorded as behaviour gaps**; the
eighth (the `content_root` *target*) has no successor because the format removed the reason it
existed.

| 0.1.0 injection (API) | helper | 0.2.0 | kept / gap |
|---|---|---|---|
| `block_entity` (15/129) | `block_entity.rs` | `BlockEntityPart::block_entity` on the type declared by `registrar.block_entity_type("basic_recycling_factory").blocks(&["basic_recycling_factory"]).build()` | **kept** — returns `Some(Compound::new())` |
| `block_entity_ticks` (15) | `ticks.rs` | `BlockEntityPart::block_entity_ticks` sets `event.ticks` | **kept** |
| `block_entity_container` (16/17) | `container.rs` | `Registrar::block_container(&block, ModContainer)` | **kept** — the same nine slots (inputs `0,1,2`, outputs `3,4,5`, energy slot `6`, spare `7,8`), every face, `minecraft:generic_3x3` |
| `block_entity_energy` (142) | `energy.rs` | `Registrar::block_energy(&block, ModEnergyBuffer)` | **kept** — 20,000 FE, fill from every face, no extraction; the amount is still the int `energy` in the saved compound |
| `block_entity_tick` (17) | `tick.rs` | `BlockEntityPart::block_entity_tick` | **kept**; `mark_modified` became the `Changed`/`Unchanged` reply |
| `register_commands` (8) | `register.rs` | `Hooks::register_commands` pushing the same four usages (`info`/`progress` at level 0, `energy`/`chance` at 2) | **kept** |
| `perform_command` (8) | `perform.rs` | `Hooks::run_mod_command`, reading the typed `CommandInvocation` | **kept, simpler**: `<pos:block_pos>` → `BlockPos`, `<amount:integer>`/`<forced:integer>` → `Integer`; no `split_whitespace`, no string positions; feedback through `host.commands().feedback` |
| `content_root` (a target, not a hook) | `embedded_content.rs` + `data/mekmm-content.hmod` | — | **gone by design, not a gap**: one `.hmod` carries `assets/`, `data/` and `src/`, so the helper that wrote an embedded content file out and appended it to the dev list has nothing to work around |

The two `[[module]]`s became two source files: `runtime` → `src/runtime.rs` (byte-identical),
`state` → `src/basic_recycling_factory.rs`.

Host calls, 0.1.0 → 0.2.0:

| 0.1.0 (`Game`) | 0.2.0 |
|---|---|
| `game.native_with_mod_block_entity(pos, closure)` | `host.block_entity(level, pos)` + `BlockEntityRef::{data, item_slot, merge_data, replace_data}`, guarded by `level.block(host, pos)` |
| `entity.nbt` / `entity.items` (in the tick) | `event.entity.nbt` / `event.entity.items` (`ModBlockEntityFields`; a `Unchanged` reply discards this Part's edits) |
| `game.world.modified.insert(chunk)` (`mark_modified`) | nothing: the tick reply dirties the chunk when the payload moved |
| `game.native_dimension()` | `host.dimension()` |
| `game.native_max_stack_size(SCRAP)` | `host.max_stack_size(scrap_key)`, the key resolved once in `register` |
| `game.native_feedback(Text)` | `host.commands().feedback(Text)` |
| `JavaString::from("energy")` + `entity.nbt.get(..)` | `Tag::get("energy")` / `nbt.insert("energy".into(), …)` |

### The ladder (all inside one `build-slot.sh` lease, `CARGO_INCREMENTAL=0`, no `--target-dir`)

`build-slot.sh` still speaks the retired `tools/horizon.py`, so it is used as the **slot guard**,
not as the driver: the commands run directly in the port through
`build-slot.sh run --job mekmm-sdk --port ports/mekmm --step … -- bash -c 'cd ports/mekmm && …'`
with `BUILD_LOCAL=1` (an SDK package compiles in place against `.horizon/sdk/`; there is no
staged tree and nothing for the remote runner's sync to gain).

| Step | Command | Result | Evidence |
|---|---|---|---|
| runtime carried over | `cmp` + `sha256sum` | identical, `94dac43c…52b3` | `runtime-carryover.txt` |
| runtime tests before the glue | `cargo test --offline` with `src/lib.rs` = `pub mod runtime;` | **19 passed**, 0 failed | `sdk-runtime-first.log` |
| glue tests (RED) | the same with `todo!()` bodies | **5 failed** of 25 (`not yet implemented`), 20 passed | `sdk-glue-red.log` |
| all tests | `cargo test --offline` | **25 passed**, 0 failed | `sdk-test.log` |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, **0 warnings** | `sdk-clippy.log` |
| formatting | `cargo fmt --check` (never `--all`: that formats the fetched SDK) | clean, exit 0; `src/runtime.rs` still `cmp`-identical | `sdk-fmt.log` |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | `SDK 0b14f8a … in ./.horizon/sdk`, `Built ./target/horizon-mod/mekmm.hmod` | `sdk-build.log` |
| determinism | a second build, `cmp` | **byte-identical**, `55c476c9…f153c` | `sdk-build-second.log` |
| registry check | `cargo horizon-mod check mekmm.hmod` | exit 0 (quoted below) | `sdk-check.log` |
| fits the pinned build | `cargo horizon-mod fits mekmm.hmod --build nightly` | `0b14f8a  made for`, exit 0 | `sdk-fits.log` |
| archive | `unzip -l` | 25 entries; `LICENSE` + `LICENSE-NOTICE.md` in; **no `Cargo.lock`** | `sdk-entries.txt` |
| surface | `unzip -p mekmm.hmod uses.json` | **265 entries** (137 type, 56 handle, 24 method, 20 hook, 20 trait, 5 part, 2 macro, 1 capability) | `sdk-uses.json`, `sdk-uses-summary.log` |
| skills gate | `check-skills.py --notes ports/mekmm --strict` | PASS | this file's `Skills:` line |

`check` (verbatim):

```
mekmm 0.2.0 (native-mod, format native-1/0b14f8acfd97fa2c750e42795249dcc981178ca4/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  32735 bytes, SHA-256 55c476c91af941d70d4d4c9c7469e5f08b1df1bd36cff2dae2ba09ea7bbf153c, unsigned
  native source: unsandboxed; not executed by the registry check. Compile and test against the exact pinned Horizon and Mixins revisions before review.
```

`fits` (verbatim):

```
Mekanism: MoreMachine:
0b14f8a  made for
```

**Package:** `mekmm.hmod` **32,735 B**, SHA-256
`55c476c91af941d70d4d4c9c7469e5f08b1df1bd36cff2dae2ba09ea7bbf153c`, **25 entries**
(`horizon-mod.toml`, `Cargo.toml`, `LICENSE`, `LICENSE-NOTICE.md`, `README.md`, 11 asset
files, 5 data files, `src/lib.rs`, `src/basic_recycling_factory.rs`, `src/runtime.rs`,
`uses.json`). The packed manifest still carries `license`, `homepage` and `issues`
(`unzip -p mekmm.hmod horizon-mod.toml`).

### Traps new to this port (beyond `imfast`/`enderstorage`/`naturesaura`/`mekanismgenerators`)

1. **`Compound` has no writable removal.** `JavaString` is not exported, and
   `BTreeMap::remove` cannot infer the key type from `&"k".into()`, so "back to the real 17 %
   roll" (`/mekmm chance 2`) drops the key with
   `data.retain(|key, _| key.to_string_lossy() != CHANCE_KEY)` and writes the trimmed compound
   back with `BlockEntityRef::replace_data`. The near miss — storing `Tag::Int(2)` instead —
   would have changed the saved compound the 0.1.0 code kept clean, and the port's own
   `forced_chance` is what would have had to learn the extra value.
2. **`host.block_entity` sees vanilla block entities; the 0.1.0 hook saw only mod ones.** A
   chest is a `BlockEntityRef` now, so `/mekmm info` at a barrel would have answered as a
   factory at 0 FE. Every readout path therefore checks the block id
   (`level.block(host, pos).id(host) == "mekmm:basic_recycling_factory"`) **before** the entity
   read: `energy`, `chance` and `progress` share one guard (`factory_at`), and `info` repeats it
   inline because it prints the id it resolved. That is what keeps the e2e's "no block entity
   there" control meaningful.
3. **`ItemStack` is not the 0.1.0 struct.** `count()` is non-zero by construction
   (`NonZeroU32`), so "empty" is `None` and the old `count > 0` test is dead; `ItemStack::new`
   returns `Option`; and an output write that keeps a stack's components is
   `current.set_count(value.count)` on the existing stack, not a struct rebuild. Since only
   `mekmm:scrap` is ever produced, the output key is resolved once (`OnceLock<ItemKey>`, filled
   in `register`) instead of per tick.
4. **The readout must keep the 0.1.0 arithmetic, not the raw compound.** `progress` and `cycles`
   are printed through `Factory::from_saved` (clamped to `0..=199`, padded to three, cycles
   `u32 → i32`) while `energy` is the **raw** int, exactly as `be_energy` + `from_saved` did;
   printing the stored arrays directly would change a hand-edited `progress 250` from `199` to
   `250` and break the byte-identical readout the e2e asserts on.
5. **The 0.1.0 usage-error text has no successor.** "Usage: /mekmm info <x> <y> <z>", "The
   amount must be a whole number of FE." and the two other usage lines answered *unparsable*
   input; the typed dispatcher refuses a bad `<pos:block_pos>`/`<integer>` before
   `run_mod_command` runs, so Brigadier answers in its own words. The readout lines (the ones
   the in-game check asserts on) are byte-identical; only the invalid-input text differs, and
   it is recorded as a deviation rather than faked.
6. **`ctl chat` is still the right command path.** The retired comment named
   `perform_command`; it is `run_mod_command` now, and the quirk is unchanged: `ctl chat
   "/mekmm …"` reaches the hook, `ctl command` runs vanilla's console path and bypasses it. The
   rewritten `tools/e2e.sh` keeps every `say`.
7. **A `Registrar` declaration is not a hook, and its key must be bound first.** `block_container`
   and `block_energy` take `&BlockKey`, so the key is a local (`let block = registrar.block_key(…)`)
   and the same value feeds both; `block_entity_part` takes the type key from
   `block_entity_type(…).build()` and returns `Result`, which is handed to
   `RegistrationContext::refuse` so a refused Part is reported instead of vanishing.

### Behaviour that changed with the format (deliberate)

- **The embedded-content control leg is gone.** 0.1.0 proved the block came from the bundle's
  `content_root` write-out by launching the same binary with `.horizon-cache` unusable. In the
  SDK format the package *is* the block and the code, so the control is an **unmodified game**:
  the block id is unknown and `/mekmm` does not answer (`CONTROL_GAME` in `tools/e2e.sh`).
- **A non-factory block entity is "No block entity there." by construction** (trap 2), which is
  the same answer 0.1.0 gave for different reasons.
- **Invalid command input** is answered by the server's dispatcher, not by four hand-written
  usage lines (trap 5).

### Gaps after the migration

1. ~~**No in-game run at 0.2.0.**~~ **Closed:** 0.2.1 ran the leg
   (`ports/_shared/evidence/mekmm-e2e/`) and 0.2.2 re-ran it on four tiers. The text below is
   the state at 0.2.0. `horizon-launcher` is not installed in this environment, so a
   game carrying this package cannot be built (an SDK package is compiled **into** the client by
   the launcher; there is no staging path for it). `tools/e2e.sh` was rewritten for
   `GAME=<launcher-built client>` + optional `CONTROL_GAME=` and was **not run**. The 0.1.0
   runs in `../_shared/evidence/mekmm/` (`e2e-embedded.log`, `e2e-embedded-selftest.log`) remain
   the behaviour's evidence: the rules are byte-identical, the saved keys are the same
   (`energy`/`progress`/`cycles`/`test_chance`), the numbers are the same and the readout line is
   the same string. **The first thing the in-game run must confirm** is the one claim the offline
   ladder cannot test: that the *declaration* (`block_entity_type("basic_recycling_factory")` +
   `block_entity_part`) gives the content block its block entity, where 0.1.0 asked the
   `block_entity` hook. Place the block and read `/mekmm info`: a registered entity answers
   `0/20000 FE, progress [0, 0, 0]/200, …`, a missing one answers `No block entity there.` —
   the first assertion the scenario already makes.
2. **The menu is the vanilla `minecraft:generic_3x3`** (unchanged from 0.1.0): a mod menu type
   would need a mod screen (`data/mekmm/horizon/menu/` + a `screen` Part). Whether the vanilla
   menu still opens from `Registrar::block_container` is exactly what the in-game run's
   nine-slot assertion proves.
3. **Invalid-input command text differs** (trap 5).
4. **No log lines.** The SDK exports no logger and a native mod may not add a crate, so nothing
   is logged; diagnostics go through `/mekmm info` only.

Every behaviour gap from 0.1.0 (the other 15 factory families, the eight higher tiers, the
chemical families, the single machines, the multiblocks, `mekanism:substrate`, side config,
upgrades, the ejector, redstone control, the `active` state, the factory's own GUI, the item
component copy on break) stands unchanged: the migration did not touch the slice boundary.

### Changelog for the resubmission (supersedes #803)

> **0.2.0 — republished as an SDK `.hmod`.** Horizon's mod-API merge (launcher ≥ 0.2.70, PR
> #17992) retired `.hnmod` and closed **#803** (`mekmm` 0.1.0); this version supersedes it and
> states what changed for `api = "0.2"`. One flat `mekmm.hmod` now carries the code *and* the
> content (the block, its item, models, textures, lang, loot table, tag and creative tab):
> `src/lib.rs` is the one `impl mod_api::Hooks`, `src/basic_recycling_factory.rs` the glue (one
> `BlockEntityPart`, the nine-slot `block_container` and the 20,000 FE `block_energy`
> declarations, and the typed `register_commands`/`run_mod_command` pair), and
> `src/runtime.rs` is byte-identical to 0.1.0, so the three processes, the 200-tick operation,
> the 50 FE/t cost, the 17 % roll and the saved state behave exactly as before. Built against
> SDK `0b14f8a` / mixins `e59da90` for 26.3, launcher 0.2.78. The `.hnmod`, the mixin plan, the
> eight helper files and the embedded-content injection are gone; the readout commands print
> the same strings. Same feature set and the same gaps (the other factory families and tiers,
> the chemical machines, side config, upgrades, the factory's own GUI). Package: 25 entries,
> **32,735 B**, sha256 `55c476c91af941d70d4d4c9c7469e5f08b1df1bd36cff2dae2ba09ea7bbf153c`;
> `check` and `fits` exit 0. No 0.2.0 in-game run: the launcher is not installed here.

### Publication route (campaign ruling 2026-11-18)

The submission form is gone; publishing is `horizon-rust/horizon-mods#1147` — the artifact is
attached to a release of the author's own repository under the entry's exact `file` name, a PR
adds the `[[versions]]` record and links the file, and a **maintainer** runs `scripts/stage.sh`
(a red `check` before that is the expected `fetch.sh` failure). The orchestrator's
`python3 ports/tools/publish-ready.py mekmm` does the bookkeeping; this port's half is the
package and this record. The port stays **local-only until the maintainer approves publishing**
(rank-0 MIT still needs the approval).

## Review fixes (2026-11-18)

`var/review-all/1219-mekmm.md`, both findings fixed; version **0.2.1** (34,306 B, sha256
`b72a04acdb28029962bc23a0aaae14df2a93fa505ec54ab17886ac94610c1e42`).

- **Info disclosure (src/lib.rs:56, :383).** `/mekmm info` read input/output item ids and counts
  from any loaded factory at caller-supplied coordinates. It now needs an operator (permission
  ≥ 2) or a player in the same level within 8 blocks; any other caller gets a generic refusal.
  Rule: `runtime::may_inspect`, pinned by `a_readout_needs_an_operator_reach_or_the_same_level`.
- **Output rewrite on every changed tick (src/basic_recycling_factory.rs:346, :252).** The tick
  wrote every output slot, and `write_output` replaced an occupied non-scrap stack with scrap of
  the same count even when the recipe had rejected it. Now only a slot whose `Stack` actually
  changed is written (`runtime::output_changed`, pinned by
  `an_untouched_output_slot_is_not_rewritten`), and `write_output` never overwrites a stack of a
  different item: it only updates/clears this mod's result item.

### In-game harness corrections (documented, not a weakening)

- `info_once` grepped `"$BLOCK: "` while every answer reads `"$BLOCK at x,y,z: "`, so it never
  matched; the pattern now accepts the space after the id.
- The scenario stood 8.8 blocks from the first factory, outside the new read gate; it now stands
  at 12.5,64,4.5 among the three.
- Under `--headless` a mod container screen opens but `ctl screen`/`close`/`quit` never answer
  while it is open (no screen tick). The menu assertion now reads the screen from the `ctl
  interact` reply (`.screen == "container"`), runs last, and `cleanup` ends the captured game pid.
  The nine slots/3x3 menu stay pinned by the `container()` unit test; all item movement is still
  exercised in-game through the same slots by `item replace block … container.N`.
- `runtime.rs` gained the reach and output-change rules and their tests (27 tests); it is no
  longer byte-identical to the retired bundle.

Ladder: `cargo test` 27 passed; `cargo clippy --all-targets -- -D warnings` exit 0;
`cargo fmt --check` clean; `cargo horizon-mod build` twice byte-identical; `check` and `fits`
exit 0. In-game: `launcher-game.sh run mekmm` → `PASS` + `EXIT:0`
(`ports/_shared/evidence/mekmm-e2e/e2e-launcher.log`); `GAME=$CONTROL_GAME` → `FAIL`
(`mekmm info … Unknown command`) + `EXIT:1`.

## Slice 2 — the factory's four real tiers (version 0.2.2, 2026-11-21)

Pin **`0a875da91dbb0fdcb7ba8c42c11341aa229af51a`** + horizon-mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`** (Nightly 2026-11-20). The SDK in
`.horizon/sdk` was refreshed to this pin (deleted and re-fetched: a repin does not
refresh a cached SDK), and the package rebuilt from it.

### The family, and the gap line it comes from

`## Gaps` said: *"The other 15 factory families (144 factory blockstates), **the
eight higher tiers of this one** (advanced/elite/ultimate and the
overclocked/quantum/dense/multiversal/creative extras), … are **not ported**."*

This slice is that family: **the recycling factory at every tier the jar
registers without another mod**. The "eight" in that line was an inventory count;
the registrable set is **three** higher tiers, measured:

| evidence | what it says |
|---|---|
| `MoreMachineUtils.getFactoryTier()` | returns `EnumUtils.FACTORY_TIERS` **unless** `Mekmm.hooks.evolvedMekanism.isLoaded()`, in which case it appends `MoreMachineEnumUtils.EM_TIERS` |
| `MoreMachineEnumUtils.java:50-56` | `EM_TIERS = { EMFactoryTier.OVERCLOCKED, QUANTUM, DENSE, MULTIVERSAL, CREATIVE }`, only inside `if (Mekanism.hooks.evolvedMekanism.isLoaded())` |
| Mekanism `common/tier/FactoryTier.java` | `BASIC(3), ADVANCED(5), ELITE(7), ULTIMATE(9)`; `EnumUtils.FACTORY_TIERS = FactoryTier.values()` |

Evolved Mekanism is not in the registry, so the five extra tiers are
`blocked(evolvedmekanism-not-ported)` rather than ported against a type this pin does
not have. It fits the **machines** tier: four block entities with their own process
count, buffer and container, and no renderer, entity or dimension.

### The numbers, and the defect this slice fixes

| value | source | this port |
|---|---|---|
| processes | Mekanism `FactoryTier.processes` | 3 / 5 / 7 / 9 |
| ticks per operation | `TileEntityMoreMachineFactory.BASE_TICKS_REQUIRED` = `10 * TICKS_PER_SECOND` | 200 for every tier |
| energy per process per tick | `MoreMachineUsageConfig.recycler` = 50 | 50 |
| the single machine's storage | `MoreMachineStorageConfig.recycler` = 20,000 | `MACHINE_STORAGE` |
| **the factory's buffer** | `MoreMachineFactory.setMachineData`: `clamp(max(origEnergy.getConfigStorage() * 0.5, origEnergy.getUsage()) * tier.processes)` | `factory_capacity` → **30,000 / 50,000 / 70,000 / 90,000** |

**Port defect fixed (0.2.1 → 0.2.2):** slice 1 shipped the **basic** factory with a
**20,000 FE** buffer — the *single machine's* `StorageConfig.recycler` — while the same
jar's factory builder computes `max(20,000 × 0.5, 50) × 3 = 30,000`. The tier formula
cannot be implemented for advanced/elite/ultimate while leaving basic at a value the
formula contradicts, so basic is corrected to **30,000** here. A player notices a basic
factory that can hold one and a half operations instead of one, and every tier now reads
its own buffer in `/mekmm info` (30,000/50,000/70,000/90,000). Pinned by
`the_factory_buffer_is_half_the_machine_storage_times_its_processes`.

### Red → green (TDD)

Two fail-capable red runs before any of the family's rules existed, both kept verbatim
in `ports/_shared/evidence/mekmm/`:

1. `sdk-tier-red.log` — the tier rules as failing tests (`todo!()`/`panic!` skeletons,
   the crate compiling):
   ```
   thread 'runtime::tests::the_four_tiers_are_the_jars_process_counts' panicked at src/runtime.rs:75:9:
   the tier's process count
   thread 'runtime::tests::five_processes_stall_after_forty_ticks_on_ten_thousand_fe' panicked at src/runtime.rs:180:9:
   not yet implemented: a factory of a tier
   test result: FAILED. 11 passed; 24 failed; 0 ignored; 0 measured; 0 filtered out
   ```
2. `sdk-tier-glue-red.log` — the per-tier *declarations* as failing tests (`container`,
   `battery` and `read` stubbed):
   ```
   thread 'recycling_factory::tests::every_tier_opens_the_smallest_vanilla_grid_that_holds_its_slots' panicked at src/recycling_factory.rs:87:5:
   not yet implemented: a tier's container
   thread 'recycling_factory::tests::every_tier_declares_its_own_input_only_buffer' panicked at src/recycling_factory.rs:93:5:
   not yet implemented: a tier's buffer
   test result: FAILED. 31 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out
   ```

Green: **35 tests** (19 from 0.1.0's runtime + 6 glue + 10 new), `clippy --all-targets -- -D
warnings` 0 warnings, `cargo fmt --check` clean.

The new rules the tests pin:

- `the_four_tiers_are_the_jars_process_counts` — processes, ids, block ids, display names;
- `the_factory_buffer_is_half_the_machine_storage_times_its_processes` — the formula,
  including the usage floor (`factory_capacity(40, 50, 3) == 150`) and the four buffers;
- `every_tier_block_id_resolves_back_to_its_tier` — `Tier::from_block`, and five non-factory
  ids answered `None`;
- `five_processes_stall_after_forty_ticks_on_ten_thousand_fe`,
  `seven_processes_stall_after_twenty_eight_ticks_on_ten_thousand_fe`,
  `nine_processes_stall_after_twenty_two_ticks_on_ten_thousand_fe` — the exact stall each
  tier reaches on 10,000 FE (`[40×5]`, `[29,29,29,29,28,28,28]`, `[23,23,22×7]`);
- `a_higher_tier_finishes_on_exactly_the_energy_it_has_left` — the remaining ticks cost
  exactly 40,000 / 60,000 / 80,000 FE and every process produces one scrap;
- `a_tier_s_factory_runs_exactly_its_process_count`, `saved_arrays_are_padded_and_clamped_per_tier`;
- `every_tier_opens_the_smallest_vanilla_grid_that_holds_its_slots` (the container/menu),
  `every_tier_declares_its_own_input_only_buffer`, `a_fresh_compound_is_an_empty_factory_of_its_tier`,
  `the_tick_s_state_reads_back_from_the_compound_it_wrote` (advanced: five progress slots),
  `the_info_line_names_the_tier_and_its_own_buffer` (basic 30,000 and ultimate 90,000 lines).

### The in-game leg (measured this version)

`ports/_shared/evidence/mekmm-e2e/` — `e2e-launcher.log` (**PASS** + `EXIT:0`),
`e2e-launcher-control.log` (`FAIL` + `EXIT:1`), `e2e-launcher-selftest.log` (`FAIL` +
`EXIT:1`), `native-build.log` (`Compiling mekmm (1 of 1)` — the manifest changed, so trap 15
of the wave's list cannot apply), `check-e2e-logs.py mekmm-e2e --strict` → **logs=3 ok=3**.
The leg runs `--offscreen` (a menu assertion needs the client's screen tick; `--headless`
hangs `ctl screen`).

Quoted from `e2e-launcher.log`:

```
== the factories place and their block entities answer ==
  info(20,64,8): mekmm:advanced_recycling_factory at 20,64,8: 0/50000 FE, progress [0, 0, 0, 0, 0]/200, cycles [0, 0, 0, 0, 0], in [empty, empty, empty, empty, empty], out [empty, empty, empty, empty, empty]
  info(24,64,8): mekmm:elite_recycling_factory at 24,64,8: 0/70000 FE, progress [0, 0, 0, 0, 0, 0, 0]/200, cycles [0, 0, 0, 0, 0, 0, 0], in [empty, empty, empty, empty, empty, empty, empty], out [empty, empty, empty, empty, empty, empty, empty]
  info(28,64,8): mekmm:ultimate_recycling_factory at 28,64,8: 0/90000 FE, progress [0, 0, 0, 0, 0, 0, 0, 0, 0]/200, cycles [0, 0, 0, 0, 0, 0, 0, 0, 0], in [empty, empty, empty, empty, empty, empty, empty, empty, empty], out [empty, empty, empty, empty, empty, empty, empty, empty, empty]
== the higher tiers stall at their own tick pattern on 10,000 FE ==
  info(20,64,8): mekmm:advanced_recycling_factory at 20,64,8: 0/50000 FE, progress [40, 40, 40, 40, 40]/200, cycles [0, 0, 0, 0, 0], in [minecraft:stone x1, minecraft:stone x1, minecraft:stone x1, minecraft:stone x1, minecraft:stone x1], out [empty, empty, empty, empty, empty]
  info(24,64,8): mekmm:elite_recycling_factory at 24,64,8: 0/70000 FE, progress [29, 29, 29, 29, 28, 28, 28]/200, cycles [0, 0, 0, 0, 0, 0, 0]
  info(28,64,8): mekmm:ultimate_recycling_factory at 28,64,8: 0/90000 FE, progress [23, 23, 22, 22, 22, 22, 22, 22, 22]/200, cycles [0, 0, 0, 0, 0, 0, 0, 0, 0]
== each tier finishes every process on exactly the energy it has left ==
  info(20,64,8): mekmm:advanced_recycling_factory at 20,64,8: 0/50000 FE, progress [0, 0, 0, 0, 0]/200, cycles [1, 1, 1, 1, 1], in [empty, empty, empty, empty, empty], out [mekmm:scrap x1, mekmm:scrap x1, mekmm:scrap x1, mekmm:scrap x1, mekmm:scrap x1]
  info(24,64,8): mekmm:elite_recycling_factory at 24,64,8: 0/70000 FE, progress [0, 0, 0, 0, 0, 0, 0]/200, cycles [1, 1, 1, 1, 1, 1, 1], in [empty, empty, empty, empty, empty, empty, empty], out [mekmm:scrap x1, mekmm:scrap x1, mekmm:scrap x1, mekmm:scrap x1, mekmm:scrap x1, mekmm:scrap x1, mekmm:scrap x1]
  info(28,64,8): mekmm:ultimate_recycling_factory at 28,64,8: 0/90000 FE, progress [0, 0, 0, 0, 0, 0, 0, 0, 0]/200, cycles [1, 1, 1, 1, 1, 1, 1, 1, 1], in [empty, empty, empty, empty, empty, empty, empty, empty, empty], out [mekmm:scrap x1, mekmm:scrap x1, mekmm:scrap x1, mekmm:scrap x1, mekmm:scrap x1, mekmm:scrap x1, mekmm:scrap x1, mekmm:scrap x1, mekmm:scrap x1]
== each tier opens its own grid: 9, 18, 18 and 27 slots ==
  mekmm:basic_recycling_factory menu slots: 9 (expected 9)
  mekmm:advanced_recycling_factory menu slots: 18 (expected 18)
  mekmm:elite_recycling_factory menu slots: 18 (expected 18)
  mekmm:ultimate_recycling_factory menu slots: 27 (expected 27)
== the carried loot table and tag: a broken factory drops itself ==
  stone block drop count: 1 (the differential: vanilla's own loot)
  factory drop: {"mekmm:ultimate_recycling_factory":1}
```

What the numbers prove:

- **the process count is real**: 10,000 FE at 50 FE/t per process buys 40 ticks with five
  processes, 28 with seven and 22 with nine, and the leftover 200/100 FE advances exactly
  four/two processes one more tick. A machine with the wrong count or a shared 50 FE/t
  cannot print those arrays;
- **the buffer is per tier**: each top-up is exactly the stalled processes' remaining ticks
  (`800/1,200/1,600 × 50 = 40,000/60,000/80,000`), and every buffer ends at **0** with one
  scrap per process — an over- or under-sized buffer leaves a remainder or blocks a finish;
- **the containers are real**: the four grids open with 9, 18, 18 and 27 slots, which is the
  declared slot count of each tier (a container whose `slots` did not equal the menu's is
  filtered out by the pin, so a wrong declaration opens nothing);
- the basic tier's own controls (no energy, no recipe, full output, missed roll, 67/67/66
  sharing) and the save/relaunch round trip still hold at 30,000 FE;
- **the carried loot table and tag are real**: broken with an iron pickaxe in survival, an
  ultimate factory drops `mekmm:ultimate_recycling_factory` ×1 (its 26.3 singular-condition
  self-drop, with `requires_tool` satisfied by the carried `mineable/pickaxe` entry), and the
  same break on a stone block drops 1 as the differential — a break that cannot read drops
  fails before the factory is even touched.

The control leg is a real differential: both `mekmm:basic_recycling_factory` and
`mekmm:advanced_recycling_factory` place as **air** in the same Nightly without the package
and `/mekmm` does not answer; the `CONTROL=0 GAME=$CONTROL_GAME` run of the whole scenario
fails at the first assertion (`FAIL: expected /mekmm info at 8,64,8 … got ''`, `EXIT:1`).
`SELFTEST=1` flips the first operation's output assertion and fails with the machine's own
line (`… out [mekmm:scrap x1, empty, empty]`, `EXIT:1`).

### Two in-game findings the code review could not see

1. **The tier roll is the real 17 %.** The first tier run showed `cycles [1, 1, 1, 1, 1]`
   with `out [scrap, empty, empty, empty, scrap]` — the port is right, the *scenario* was
   wrong: the chance override is per block entity and had been pinned only on the basic
   factory. The leg now pins `mekmm chance 1` on each tier before its completion phase, and
   the miss path stays proven on the basic tier's control 2.
2. **A command pair is a race, not an atomic step.** `put <stone>` followed by
   `mekmm energy 30000` let one tick run at the *old* buffer, so the operation completed
   after 199 spending ticks and the buffer read `20,050/30,000`. The leg now empties the
   buffer before the input goes in (and then sets the target), which is the same trick the
   three-process phase already used; the energy assertions are about the machine again.

### Deviations added by this slice

8. **One container grid per tier, not upstream's layout.** A mod container's menu must show
   exactly its slot count, and the pin's vanilla grids are 9/18/27 slots, so each tier opens
   the smallest that holds its `2 × processes` working slots plus upstream's energy slot:
   `generic_3x3` (9), `generic_9x2` (18, 18) and `generic_9x3` (27). The spare cells after
   the energy slot are empty. Upstream draws its own 2n+1 screen; that GUI is still a gap.
9. **The tier LED is drawn, not composed.** Upstream's factory model is a
   `neoforge:composite` of the mod's cube and `mekanism:block/factory/front_led/<tier>`,
   which Horizon's model loader does not have. The four fronts are the mod's own
   `recycling_factory_front.png` with the tier's LED row baked in at the top-centre of the
   face, using Mekanism's own palette (`factory/led.png` rows 0-3, exact RGB);
   `tools/gen_fronts.py` is the generator and the record.Without the parent mod installed,
   the composite could not resolve its textures anyway.

### Coverage (every family the jar ships)

| family | state |
|---|---|
| recycling factory, basic tier (3 processes, 30,000 FE) | **tested** — phases "the factories place…", "one powered process…", "three processes share…", "the three operations finish…", control 1–4, persistence |
| recycling factory, advanced tier (5 processes, 50,000 FE) | **tested** — "the higher tiers stall…", "each tier finishes…", "each tier opens its own grid" (18 slots) |
| recycling factory, elite tier (7 processes, 70,000 FE) | **tested** — same phases (18 slots) |
| recycling factory, ultimate tier (9 processes, 90,000 FE) | **tested** — same phases (27 slots) |
| recycling recipe table (`mekmm:recycler`: `c:stones`, dirt, grass_block → scrap 17 %) | **tested** — control 1–4 and the unit table test (the `mekanism:substrate` row is not ported: no such item here) |
| `mekmm:scrap` item | **tested** — produced, counted and read back in every phase |
| the factory blocks' loot tables and the `mineable/pickaxe` tag | **tested** — "the carried loot table and tag": a survival break with an iron pickaxe drops the block, with a stone break as the differential |
| the four tier front textures (the baked LED) | **tested** — the block models are loaded by the client at launch (`block sprites: 1309 layers`); the LED strip itself is verified by `tools/gen_fronts.py`'s palette read, not by a screenshot |
| the other 15 factory families | **not ported** — 13 need Mekanism chemicals (slurries/pigments/`nutrient_solution`), and `lathing`/`rolling_mill`/`stamping`/`pressing` ship only `neoforge:mod_loaded` `compat/` recipes, so they would be machines that can never run |
| `overclocked`/`quantum`/`dense`/`multiversal`/`creative` tiers | **blocked(evolvedmekanism-not-ported)** — `MoreMachineEnumUtils.EM_TIERS` exists only when Evolved Mekanism is loaded |
| chemical tanks (`mid`/`max`) | **not ported** — the parent's chemical system |
| single machines (`recycler`, `planting_station`, `presser`, cnc trio, three replicators) | **not ported** |
| `large_*` multiblocks | **not ported** |
| silver ore/block/item, `scrap_box`, `uu_matter`, crystals | **not ported** |
| side configuration, upgrades, ejector, redstone control, `active` state, the factory's own GUI | **not ported** (deviations 3–6, 8) |

### Ladder at 0.2.2 (evidence in `ports/_shared/evidence/mekmm/`)

| Step | Result | Evidence |
|---|---|---|
| tier rules (RED) | **24 failed** of 35 | `sdk-tier-red.log` |
| tier declarations (RED) | **4 failed** of 35 | `sdk-tier-glue-red.log` |
| tests | **35 passed**, 0 failed | `sdk-test.log` |
| clippy | exit 0, **0 warnings** | `sdk-clippy.log` |
| fmt | clean | `sdk-fmt.log` |
| build ×2 | **byte-identical**, `cmp` exit 0, 44,260 B | `sdk-build.log`, `sdk-build-second.log` |
| check | exit 0, `mekmm 0.2.2 (native-mod, format native-1/0a875da/…, for 26.3)`, 44,260 B, SHA-256 `086d1d51…bf1f` | `sdk-check.log` |
| fits (the pin) | `0a875da  made for`, exit 0 | `sdk-fits.log` |
| **fits (a different build)** | `0b14f8a  fits`, exit 0 | `sdk-fits-other.log` |
| archive | 43 entries, no `Cargo.lock` | `sdk-entries.txt` |
| surface | 261 entries (125 type, 59 handle, 26 method, 21 hook, 22 trait, 5 part, 2 macro, 1 capability); no local `ItemCount`/`RecycleRecipe` recorded against an SDK id | `sdk-uses.json`, `sdk-uses-summary.log` |
| package gate | **1/1** | `verify-sdk-package.py mekmm` |

**Deliverable:** `mekmm.hmod`, **44,260 B**, SHA-256
`086d1d51d7da4bda8f20fe69a0ca92f94b319f57b35f7d9ab6eb7eefbdb0bf1f`, **43 entries**, built
twice byte-identically from a `rm -f Cargo.lock` tree against SDK `0a875da` / mixins
`e59da90`.

### A `fits` false positive this slice removed

0.2.1's pure core named its types `Stack` and `Recipe`, which the usage scanner resolves
against the SDK's own `type:Stack`/`type:Recipe`; that is the documented `fits --build
<other>` false positive (HANDOVER §9d trap 14). They are now `ItemCount` and
`RecycleRecipe`, both absent from `uses.json`, and `fits --build 0b14f8a` is green.

### Files this slice changed

| Path | What changed |
|---|---|
| `src/runtime.rs` | `Tier`, `factory_capacity`, tier-aware `Factory` (Vec progress/cycles), `factory_tick` over slices; `Stack`→`ItemCount`, `Recipe`→`RecycleRecipe`; 10 new tests, the existing ones moved to per-tier energy |
| `src/recycling_factory.rs` (was `src/basic_recycling_factory.rs`) | one `FactoryPart { tier }`, `grid`/`menu`/`container(tier)`/`battery(tier)`, `read(saved, tier)`, `FactoryPart` tick over `2n` slots, `Sight.capacity`; 6 tests |
| `src/lib.rs` | `register` loops `Tier::ALL`: block key, container, buffer, entity type and Part per tier |
| `assets/mekmm/blockstates,models,textures,lang` | three blockstates, three block models, three item models, three tier fronts (+ a green LED on the basic front), six lang entries |
| `data/mekmm/horizon/{block,creative_tab}`, `data/mekmm/loot_table/blocks`, `data/minecraft/tags/block/mineable` | three blocks, three loot tables, the pickaxe tag and the creative tab extended |
| `horizon-mod.toml`, `Cargo.toml` | version 0.2.2, pin `0a875da`, the new description |
| `tools/e2e.sh` | `--offscreen`, per-tier phases, `stand_at`/`menu_slots`, the tier grid assertions, buffer-before-input ordering |
| `tools/gen_fronts.py` | new: the tier LED generator |
| `README.md` | rewritten for four tiers |
| `VALIDATION.md` | the 0.2.2 record |

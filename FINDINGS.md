# mekmm — upstream findings (step 1: the artefact, not the name)

Ported from **Mekanism: MoreMachine (mekmm) 1.4.2**, jar
`ports/_shared/licence-cache/mekmm-1.21.1-1.4.2.jar`
(3,721,605 B, SHA-256 `f019b2d47d4dd6dfd72f066bb86b1572aa9089ac530221ef2b2ff136218f3c53`),
the ATM10 entry `mekmm` (`content` / `machines`, MIT, add-on of **Mekanism**).

Behaviour was read from the upstream repo at the tag matching the jar —
`lostmyself8/Mekanism-MoreMachine`, **`v1.4.2-1.21.1`** = commit
`fd2108ffd998351dc37064ea240736f6c345ceed` (`gradle.properties: mod_version =
1.4.2`). Analysis only; no Java was translated, and the jar was never
decompiled.

## Licence, from the jar

`META-INF/neoforge.mods.toml`: `license = "MIT"`, `modId = "mekmm"`,
`version = "1.4.2"`, required dependencies `neoforge [21.1,)`,
`minecraft [1.21, 1.21.1]` and **`mekanism [10.7.19,)`** (ordering AFTER,
`side = BOTH`), optional `evolvedmekanism`. No bundled LICENSE/NOTICE and no
`ASSETS_LICENSE*` entry, so the code and the assets are covered by the same MIT
declaration → **rank 0** (permissive). Local-only until the maintainer approves
publishing (ARCHITECTURE.md §8).

## Inventory, measured with `unzip -l` (the brief's counts corrected)

The brief said 2,752 entries / 181 blockstates / 202 item models. Measured:

| | Count |
|---|---:|
| entries in the jar | **2,757** |
| classes (`com/**`) | 755 |
| blockstates (`assets/mekmm/blockstates/*.json`) | **180** |
| block models (`assets/mekmm/models/block/**.json`) | 365 |
| item models (`assets/mekmm/models/item/*.json`) | **201** |
| textures (`assets/mekmm/textures/**.png`) | 232 |
| lang files | 8 |
| loot tables (`data/mekmm/loot_table/blocks/*.json`) | 180 |
| recipe files (`data/mekmm/recipe/**.json`) | 503 |

**Sixteen** factory families, each in **nine** tiers (`basic`, `advanced`,
`elite`, `ultimate`, `overclocked`, `quantum`, `dense`, `multiversal`,
`creative`) = 144 factory blockstates, plus `basic/advanced/elite/ultimate`
`*_chemical_tank` (`mid`/`max`), and single machines (`recycler`,
`planting_station`, `presser`, `cnc_lathe`, `cnc_rolling_mill`, `cnc_stamper`,
`replicator`, `fluid_replicator`, `chemical_replicator`, `chemical_replicator`,
the `large_*` multiblock ports and the silver ore/block set).

Families: `centrifuging, crystallizing, dissolving, lathing, liquifying,
oxidizing, painting, pigment_extracting, planting, pressing,
pressurised_reacting, recycling, replicating, rolling_mill, stamping, washing`.

**There is no `smelting_factory` in this jar** (the brief offered one): smelting
is Mekanism's own factory, not mekmm's. The artefact wins.

Recipe types the jar declares itself: `mekmm:recycler` (3 files),
`mekmm:pressing` (18), `mekmm:planting` (209), `mekmm:stamper` (38),
`mekmm:rolling_mill` (4), `mekmm:lathe` (6) — the last four live almost
entirely under `data/mekmm/recipe/compat/<mod>/`, gated by
`neoforge:mod_loaded` (Immersive Engineering, Create Addition, AE2 families…).

## The slice: `mekmm:basic_recycling_factory` (Basic Recycling Factory)

Why this factory, from the jar and not the name:

- an item→item factory needs **no chemical tank or gas system** (this port has
  no chemistry), and the recycling family's recipes are the only ones in the
  jar that work with vanilla 26.3 items: `mekmm:recycler` recipes are
  `c:stones` → scrap ×1 (chance 0.17), `dirt`/`grass_block` → scrap ×1
  (chance 0.17), and `mekanism:substrate` → scrap (chance 0.43, dropped:
  `mekanism:substrate` is not registered by this port).
- the `lathing`/`rolling_mill`/`stamping`/`pressing` families have **no
  recipe that works without another mod's items** (all their recipe files are
  under `compat/` and `mod_loaded`-gated, and the presser's one base recipe,
  `presser/reflector.json`, needs 3 glass panes + 3 silver ingots + 3
  `mekanism:hdpe_sheet`), so they would ship as machines that can never run.
- the `planting`, `crystallizing`, `dissolving`, `oxidizing`,
  `pressurised_reacting`, `centrifuging`, `washing`, `liquifying`,
  `pigment_extracting` and `replicating` families need Mekanism chemicals
  (`mekmm:nutrient_solution`, slurries, pigments…), which is the parent's deep
  tier and out of this slice.
- `basic` is the first tier and the one `FactoryTier.BASIC` configures (3
  processes); the eight higher tiers are the same machine with more processes.

### The numbers (where each came from)

| Value | Value in this port | Source (tag `v1.4.2-1.21.1`) |
|---|---|---|
| processes (parallel operations) | **3** | Mekanism `tier/FactoryTier.java`: `BASIC(BaseTier.BASIC, 3)`; mekmm reads `tier.processes` |
| ticks per operation | **200** | `tile/factory/TileEntityMoreMachineFactory.java`: `BASE_TICKS_REQUIRED = 10 * SharedConstants.TICKS_PER_SECOND`; the recycler has the same (`TileEntityRecycler.java`) |
| energy per operating tick, per process | **50 FE/t** | `config/MoreMachineUsageConfig.java`: `recycler = … "recycler", 50L` |
| energy buffer (shared by the three processes) | **20,000 FE** | `config/MoreMachineStorageConfig.java`: `recycler = … 20_000L` |
| energy per operation | **10,000 FE** (200 × 50) | derived above; upstream `CachedRecipe.process` extracts `energyPerTick` each tick per process |
| item slots | **3 input + 3 output** (+ upstream's 1 energy slot) | `TileEntityRecyclingFactory.addSlots`: one input and one output per `tier.processes`; `ItemSlotsBuilder.addBasicFactorySlots(processes, predicate)` + `.addEnergy()` |
| recipe input consumed | at the **end** of the operation, `recipe.input` (1) | `OneInputCachedRecipe.finishProcessing` → `inputHandler.use(input, operations)` |
| output | `mekmm:scrap` ×1, **chance 0.17** (`dirt`/`grass_block` 0.17, `substrate` 0.43) | `data/mekmm/recipe/recycler/*.json`; `RecyclerRecipe.ChanceOutput` rolled per operation in `MoreMachineOutputHelper` |
| output room check | the *max* chance output (1 scrap) must fit before energy is spent | `MoreMachineOutputHelper.calculateOperationsCanSupport` |
| block hardness / resistance | 3.5 / 16, `requiresCorrectToolForDrops` | Mekanism `block/prefab/BlockTile.java`: `Properties.of().strength(3.5F, 16).requiresCorrectToolForDrops()` |
| block sounds | stone (vanilla `Properties.of()` default) | same |
| block states | `facing` (4) × `active` (2) | `assets/mekmm/blockstates/basic_recycling_factory.json`, 8 variants |
| mineable tag | `minecraft:mineable/pickaxe`, **not** `needs_iron_tool` | the jar's `data/minecraft/tags/block/{mineable/pickaxe,needs_iron_tool}.json` |
| loot | self-drop, `copy_name` + `copy_components` of 9 `mekanism:*` components | `data/mekmm/loot_table/blocks/basic_recycling_factory.json` |
| names | `block.mekmm.basic_recycling_factory` "Basic Recycling Factory", `item.mekmm.scrap` "Scrap", `container.mekmm.basic_recycling_factory` | `assets/mekmm/lang/en_us.json` |

Error semantics read from the cached-recipe base
(`mekanism/api/recipes/cache/CachedRecipe.java`,
`OneInputCachedRecipe.java`): no input (or no recipe, or too few items) **resets**
`operatingTicks` to 0; no energy or no output room **holds** it (positive
`currentMax` is required to tick, otherwise the progress is untouched and
`active` goes false).

## Files read upstream (analysis only)

| File | Read for |
|---|---|
| `common/tile/factory/TileEntityRecyclingFactory.java` | per-process slots, `OUTPUT_CHECK`, recipe type, `getNeededInput` |
| `common/tile/factory/TileEntityMoreMachineFactory.java` | `BASE_TICKS_REQUIRED` 200, `getOperationsPerTick` (1 without upgrades), the shared `MachineEnergyContainer.input` |
| `common/tile/machine/TileEntityRecycler.java` | the single-machine baseline: 200 ticks, input/output/energy slots, `RECYCLING` recipe type |
| `api/recipes/RecyclerRecipe.java`, `api/recipes/outputs/MoreMachineOutputHelper.java`, `api/recipes/cache/MoreMachineOneInputCachedRecipe.java` | the chance output, the max-chance space check, input use at finish |
| `common/config/MoreMachineUsageConfig.java`, `MoreMachineStorageConfig.java` | 50 FE/t, 20,000 FE |
| `common/registries/MoreMachineBlocks.java`, `MoreMachineBlockTypes.java` | the factory block/item registration, `.withEnergyConfig(usage.recycler, storage.recycler)` |
| `common/block/prefab/BlockMoreFactoryMachine.java` (+ Mekanism `BlockTile.java`) | 3.5/16, requires-correct-tool, no fluid-loggable state on factories |
| `data/mekmm/recipe/recycler/*.json`, `recipe/factory/*/*.json`, `loot_table/blocks/basic_recycling_factory.json`, `lang/en_us.json`, `blockstates/*.json`, `models/block/factory/recycling/*.json`, `textures/block/factory/recycling/*.png` | the carried data |
| Mekanism `common/tier/FactoryTier.java`, `api/recipes/cache/CachedRecipe.java`, `api/recipes/cache/OneInputCachedRecipe.java` | 3 processes, the tick/finish ordering |

## Deviations this slice records (details and reasons in `notes.md`)

1. **No chemical system**: the recycling family needs none; every family that
   needs one is out of slice.
2. **No Mekanism items**: `mekanism:substrate` has no recipe here, and the
   factory's own crafting recipe (`mekmm:recycler` + Mekanism alloys/circuits)
   cannot be crafted; the block is given by command in the check.
3. **Chance roll**: upstream rolls a `RandomSource` per operation. This port
   derives the roll from the block position, the process and the operation
   ordinal (a pure SplitMix64 in `runtime.rs`) so a factory is reproducible and
   the rate is exactly 17 % over a run; plus a harness-only
   `/mekmm chance` control so the in-game check can pin the output on and off
   deterministically (the `/mekanism energy` precedent).
4. **Slots**: 3 input + 3 output at 0–5; the upstream energy slot is declared
   at 6 but unused (no item energy in this slice); 7–8 exist because the pin's
   only 9-slot vanilla grid menu is `minecraft:generic_3x3`, and a mod menu's
   slot count must equal the menu's (`native_hooks.rs` filters it otherwise).
   Upstream's own GUI is not drawn.
5. **`active` block state**: the blockstate is carried with `active` → the
   barrel template's `open`, but the port never sets it (single model), as the
   Mekanism port records for its machines.
6. **Loot**: plain self-drop; upstream's `copy_components` of
   `mekanism:energy`/`:items` (machine contents into the item) has no port.

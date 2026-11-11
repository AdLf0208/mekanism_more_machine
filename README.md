# mekmm (Horizon port)

A Horizon port of [mekmm](lostmyself8/Mekanism-MoreMachine) — MIT.

**Status: first slices.** What is below is what the port actually does today.

## Implemented so far

- | | Upstream | This port |
- |---|---|---|
- | machine | `TileEntityRecyclingFactory` on `TileEntityMoreMachineFactory` (Mekanism `FactoryTier.BASIC`) | the same shape in `runtime::factory_tick` |
- | processes | **3** (`FactoryTier.BASIC.processes`) | `BASIC_PROCESSES = 3`, three independent progress counters |
- | ticks per operation | **200** (`BASE_TICKS_REQUIRED = 10 * 20`) | `RECYCLER_TICKS = 200` |
- | energy | **50 FE/t per process** (`UsageConfig.recycler`) | `RECYCLER_USAGE = 50`, spent per process per tick |
- | buffer | **20,000 FE**, shared by the three processes (`StorageConfig.recycler`) | `RECYCLER_CAPACITY = 20_000` via `native_block_entity_energy` |
- | energy per operation | 10,000 FE (200 × 50), pinned by the e2e | same |

## Not yet ported

- The rest of the mod.** The other 15 factory families (144 factory
- blockstates), the eight higher tiers of this one (advanced/elite/ultimate and
- the overclocked/quantum/dense/multiversal/creative extras), the chemical
- tanks (`mid`/`max`), every single machine (`recycler`, `planting_station`,
- `presser`, `cnc_lathe`, `cnc_rolling_mill`, `cnc_stamper`, `replicator`,
- `fluid_replicator`, `chemical_replicator`), the `large_*` multiblocks, the
- silver ore/block/item set, the `scrap_box`/`uu_matter`/crystals and the
- compat recipes are **not ported**.

## Install

Build the bundle with a native-mod capable launcher, or add the file from the
registry once it is listed. Pinned to Horizon `1d63e0a` and horizon-mixins
`5a64af93` for game 26.3.

## Build from source

```sh
cargo test
python3 tools/horizon.py check --horizon <horizon checkout>
python3 tools/horizon.py build --horizon <horizon checkout>
python3 tools/package.py
```

## Credits and licence

Original mod: lostmyself8/Mekanism-MoreMachine. Port: AdLf0208. Licence: MIT — see [LICENSE](LICENSE);
the original notices are preserved.

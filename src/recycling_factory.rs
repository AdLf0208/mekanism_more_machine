//! The Recycling Factory's glue: one block entity Part per tier, each block's
//! container and energy declarations, the saved state and the `/mekmm` command
//! bodies.
//!
//! Every rule lives in [`crate::runtime`]; this file reads the game, calls it
//! and writes the result back. It has no rules of its own, so its tests pin what
//! it *declares* (a grid per tier, one input and one output per process, an
//! input-only buffer sized by the tier), that the state written to the saved
//! compound reads back, and the exact lines `/mekmm` prints.
use mod_api::{
    self, BlockEntityEvent, BlockEntityPart, BlockEntityTickEvent, BlockEntityTickReply,
    BlockEntityTicksEvent, BlockEntityTypeKey, BlockPart, Compound, Host, ItemKey, ItemStack,
    LevelSide, ModContainer, ModEnergyBuffer, ServerHost, Tag,
};

use crate::runtime::{
    Factory, ItemCount, RECYCLER_TICKS, RECYCLING_OPERATION, SCRAP, Tick, Tier, clamp_energy,
    factory_tick, output_changed, recycling,
};

/// The slots a tier's container holds: the smallest vanilla chest grid that
/// holds `2 * processes` working slots and upstream's energy slot.
///
/// `minecraft:generic_3x3` is 9, `generic_9x2` 18 and `generic_9x3` 27
/// (`containers::MOD_MENUS`), and a mod container's menu must show exactly its
/// slot count.
#[must_use]
pub(crate) fn grid(tier: Tier) -> usize {
    match tier {
        Tier::Basic => 9,
        Tier::Advanced | Tier::Elite => 18,
        Tier::Ultimate => 27,
    }
}

/// The menu a tier's container opens: the grid of [`grid`].
#[must_use]
pub(crate) fn menu(tier: Tier) -> &'static str {
    match tier {
        Tier::Basic => "minecraft:generic_3x3",
        Tier::Advanced | Tier::Elite => "minecraft:generic_9x2",
        Tier::Ultimate => "minecraft:generic_9x3",
    }
}

/// A process's input slot: the processes come first (`0..processes`).
#[must_use]
pub(crate) fn input_slot(process: usize) -> usize {
    process
}

/// A process's output slot, after every input (`processes..2 * processes`).
#[must_use]
pub(crate) fn output_slot(tier: Tier, process: usize) -> usize {
    tier.processes() + process
}

/// Upstream's energy slot (an energy tablet goes there), after the outputs.
/// This slice has no item energy, so it is declared and left unused.
#[must_use]
pub(crate) fn energy_slot(tier: Tier) -> usize {
    2 * tier.processes()
}

/// The saved key of the harness's chance override.
const CHANCE_KEY: &str = "test_chance";

/// The item key of `mekmm:scrap`, resolved once in `Hooks::register`: the
/// runtime's recipe table names items by id, and an output stack needs the key.
static SCRAP_KEY: std::sync::OnceLock<ItemKey> = std::sync::OnceLock::new();

/// The block entity type declared for a tier's block, built in `Hooks::register`.
pub(crate) fn declare(registrar: &mut dyn mod_api::Registrar, tier: Tier) -> BlockEntityTypeKey {
    registrar
        .block_entity_type(tier.id())
        .blocks(&[tier.id()])
        .build()
}

/// A tier's container: its grid's slots, every face reaching all of them, with
/// the first `2 * processes` split into that process's input and output (the
/// retired bundle's `native_block_entity_container`).
pub(crate) fn container(tier: Tier) -> ModContainer {
    let slots = grid(tier);
    let all: Vec<usize> = (0..slots).collect();
    let processes = tier.processes();
    debug_assert!(
        energy_slot(tier) < slots,
        "the energy slot must be inside the grid"
    );
    ModContainer {
        slots,
        faces: [
            all.clone(),
            all.clone(),
            all.clone(),
            all.clone(),
            all.clone(),
            all,
        ],
        inputs: (0..processes).map(input_slot).collect(),
        outputs: (0..processes)
            .map(|process| output_slot(tier, process))
            .collect(),
        menu: Some(menu(tier).to_owned()),
    }
}

/// A tier's buffer: its capacity that a side may fill and none may drain, as
/// upstream's input-only `MachineEnergyContainer`.
pub(crate) fn battery(tier: Tier) -> ModEnergyBuffer {
    let capacity = tier.storage();
    ModEnergyBuffer {
        capacity,
        max_insert: capacity,
        max_extract: 0,
        insert_faces: [true; 6],
        extract_faces: [false; 6],
    }
}

/// A saved int, 0 when absent or of another type.
fn int(saved: &Tag, key: &str) -> i32 {
    match saved.get(key) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

/// A saved IntArray, padded to `processes`.
fn array(saved: &Tag, key: &str, processes: usize) -> Vec<i32> {
    let mut out = vec![0; processes];
    if let Some(Tag::IntArray(values)) = saved.get(key) {
        for (slot, value) in out.iter_mut().zip(values) {
            *slot = *value;
        }
    }
    out
}

/// The harness override of the chance roll: `test_chance` 0 never produces, 1
/// always does; absent means the real 17 % roll.
pub(crate) fn forced_chance(saved: &Tag) -> Option<bool> {
    match saved.get(CHANCE_KEY) {
        Some(Tag::Int(0)) => Some(false),
        Some(Tag::Int(1)) => Some(true),
        _ => None,
    }
}

/// The factory saved in a block entity's compound: `energy`, `progress` and
/// `cycles`, padded and clamped as the pure core does.
pub(crate) fn read(saved: &Tag, tier: Tier) -> Factory {
    let processes = tier.processes();
    Factory::from_saved(
        tier,
        &array(saved, "progress", processes),
        &array(saved, "cycles", processes),
        int(saved, "energy"),
    )
}

/// Write the tick's result back: the buffer's amount (`energy`, the int the
/// energy capability reads) and the two IntArrays per process.
pub(crate) fn write(saved: &mut Tag, factory: &Factory) {
    let Tag::Compound(nbt) = saved else {
        return;
    };
    nbt.insert(
        "energy".into(),
        Tag::Int(clamp_energy(factory.energy, factory.capacity())),
    );
    nbt.insert("progress".into(), Tag::IntArray(factory.progress_saved()));
    nbt.insert("cycles".into(), Tag::IntArray(factory.cycles_saved()));
}

/// What `/mekmm info` prints about one block entity.
pub(crate) struct Sight {
    pub(crate) id: String,
    pub(crate) capacity: i32,
    pub(crate) pos: [i32; 3],
    pub(crate) energy: i32,
    pub(crate) progress: Vec<i32>,
    pub(crate) cycles: Vec<i32>,
    pub(crate) inputs: Vec<ItemCount>,
    pub(crate) outputs: Vec<ItemCount>,
    pub(crate) forced: Option<bool>,
}

impl Sight {
    /// The report line the in-game check asserts on.
    pub(crate) fn line(&self) -> String {
        let mut text = format!(
            "{} at {},{},{}: {}/{} FE, progress {}/{RECYCLER_TICKS}, cycles {}, in [{}], out [{}]",
            self.id,
            self.pos[0],
            self.pos[1],
            self.pos[2],
            self.energy,
            self.capacity,
            numbers(&self.progress),
            numbers(&self.cycles),
            stacks(&self.inputs),
            stacks(&self.outputs),
        );
        if let Some(forced) = self.forced {
            text.push_str(&format!(", chance forced({forced})"));
        }
        text
    }
}

/// `[a, b, c]`, one entry per process.
fn numbers(values: &[i32]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(i32::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// A process's stacks as `item xN`, or `empty`.
fn stacks(stacks: &[ItemCount]) -> String {
    stacks
        .iter()
        .map(|stack| {
            if stack.is_empty() {
                "empty".to_owned()
            } else {
                format!("{} x{}", stack.item, stack.count)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// A block position's roll seed, mixed with the dimension so the same
/// coordinates in two levels do not share a sequence.
fn position_seed(dimension: &str, pos: [i32; 3]) -> u64 {
    let mut hash = 0xCBF2_9CE4_8422_2325_u64; // FNV-1a
    for byte in dimension.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    let x = u64::from(pos[0] as u32);
    let y = u64::from(pos[1] as u32);
    let z = u64::from(pos[2] as u32);
    hash ^ (x << 42) ^ (y << 21) ^ z
}

/// A slot as the pure core's stack, empty when the slot is empty.
fn slot_stack(host: &dyn Host, slot: Option<&ItemStack>) -> ItemCount {
    match slot {
        Some(stack) => ItemCount::new(&stack.item().id(host), stack.count()),
        None => ItemCount::default(),
    }
}

/// Write the pure core's output stack back into `slot`, keeping the previous
/// stack's components when it is the same item. Only this mod's result item is
/// ever written: a different stack in the slot is not ours to replace, so the
/// writer leaves it alone instead of turning it into scrap (the review finding).
fn write_output(items: &mut [Option<ItemStack>], slot: usize, value: &ItemCount, key: ItemKey) {
    let Some(cell) = items.get_mut(slot) else {
        return;
    };
    match cell {
        Some(current) => {
            if current.item() != key {
                return;
            }
            if value.is_empty() {
                *cell = None;
            } else {
                let _ = current.set_count(value.count);
            }
        }
        None if !value.is_empty() => {
            *cell = ItemStack::new(key, value.count);
        }
        None => {}
    }
}

/// `ItemStack.shrink`: an input slot only ever loses items, so its stack keeps
/// its components and only the count moves.
fn write_input(items: &mut [Option<ItemStack>], slot: usize, value: &ItemCount) {
    match items.get_mut(slot) {
        Some(cell) if value.is_empty() => *cell = None,
        Some(Some(current)) if current.count() != value.count => {
            let _ = current.set_count(value.count);
        }
        _ => {}
    }
}

/// The result item's stack size, for the output-room check (`SCRAP` is the only
/// result the recipe table has).
fn max_stack(host: &dyn Host) -> u32 {
    SCRAP_KEY
        .get()
        .copied()
        .and_then(|key| host.max_stack_size(key))
        .unwrap_or(64)
        .max(1)
}

/// One tier's factory: a block entity, ticked on the server.
pub(crate) struct FactoryPart {
    pub(crate) tier: Tier,
}

#[mod_api::horizon_part]
impl BlockPart for FactoryPart {}

#[mod_api::horizon_part]
impl BlockEntityPart for FactoryPart {
    /// `EntityBlock.newBlockEntity`: an empty saved compound; the tick fills it.
    fn block_entity(
        &self,
        _level: LevelSide,
        _event: &mut BlockEntityEvent<'_>,
    ) -> Option<Compound> {
        Some(Compound::new())
    }

    /// `EntityBlock.getTicker`: it ticks. Never clear a value an earlier mod set.
    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One server tick: the tier's processes over one buffer. The payload is
    /// left `Changed` only when something moved, so an idle factory does not
    /// dirty its chunk every tick (the retired bundle's `mark_modified`
    /// condition).
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let tier = self.tier;
        let processes = tier.processes();
        let slots = grid(tier);
        let pos = event.pos;
        let resized = event.entity.items.len() != slots;
        if resized {
            event.entity.items.resize(slots, None);
        }
        let mut saved = Tag::Compound(std::mem::take(&mut event.entity.nbt));
        let before = read(&saved, tier);
        let mut factory = before.clone();
        let mut inputs: Vec<ItemCount> = (0..processes)
            .map(|process| slot_stack(&*host, event.entity.items[input_slot(process)].as_ref()))
            .collect();
        let mut outputs: Vec<ItemCount> = (0..processes)
            .map(|process| {
                slot_stack(
                    &*host,
                    event.entity.items[output_slot(tier, process)].as_ref(),
                )
            })
            .collect();
        let before_inputs = inputs.clone();
        let before_outputs = outputs.clone();
        let tick = Tick {
            operation: RECYCLING_OPERATION,
            max_stack: max_stack(&*host),
            seed: position_seed(host.dimension(), pos),
            forced_chance: forced_chance(&saved),
            lookup: &recycling::lookup,
        };
        factory_tick(&mut factory, &tick, &mut inputs, &mut outputs);
        let changed =
            resized || factory != before || inputs != before_inputs || outputs != before_outputs;
        if changed {
            write(&mut saved, &factory);
        }
        if let Tag::Compound(nbt) = saved {
            event.entity.nbt = nbt;
        }
        if !changed {
            *event.reply = BlockEntityTickReply::Unchanged;
            return;
        }
        if let Some(key) = SCRAP_KEY.get().copied() {
            for process in 0..processes {
                write_input(
                    &mut event.entity.items,
                    input_slot(process),
                    &inputs[process],
                );
                // Only a slot the tick changed is written: an untouched slot
                // (including one holding another item) stays exactly as it was.
                if output_changed(&before_outputs[process], &outputs[process]) {
                    write_output(
                        &mut event.entity.items,
                        output_slot(tier, process),
                        &outputs[process],
                        key,
                    );
                }
            }
        }
    }
}

/// The factory at `pos` when what stands there is one of this mod's blocks: a
/// foreign block entity is not a factory (a chest must not be reported at 0 FE).
pub(crate) fn factory_at(
    host: &mut dyn ServerHost,
    pos: [i32; 3],
) -> Option<(Tier, mod_api::BlockEntityRef)> {
    let level = host.level(host.dimension())?;
    let id = level.block(&*host, pos)?.id(&*host);
    let tier = Tier::from_block(&id)?;
    host.block_entity(level, pos).map(|entity| (tier, entity))
}

/// `/mekmm info <pos>`: the report line, or `None` when no factory stands there.
pub(crate) fn info_line(host: &mut dyn ServerHost, pos: [i32; 3]) -> Option<String> {
    let level = host.level(host.dimension())?;
    let id = level.block(&*host, pos)?.id(&*host);
    let tier = Tier::from_block(&id)?;
    let entity = host.block_entity(level, pos)?;
    let saved = Tag::Compound(entity.data(&*host)?);
    let factory = read(&saved, tier);
    let processes = tier.processes();
    let mut inputs = vec![ItemCount::default(); processes];
    let mut outputs = vec![ItemCount::default(); processes];
    for process in 0..processes {
        inputs[process] = slot_stack(
            &*host,
            entity
                .item_slot(&*host, input_slot(process) as u32)
                .ok()
                .flatten()
                .as_ref(),
        );
        outputs[process] = slot_stack(
            &*host,
            entity
                .item_slot(&*host, output_slot(tier, process) as u32)
                .ok()
                .flatten()
                .as_ref(),
        );
    }
    let sight = Sight {
        id,
        capacity: tier.storage(),
        pos,
        energy: int(&saved, "energy"),
        progress: factory.progress_saved(),
        cycles: factory.cycles_saved(),
        inputs,
        outputs,
        forced: forced_chance(&saved),
    };
    Some(sight.line())
}

/// `/mekmm energy <amount> <pos>`: set the buffer's amount, the harness control
/// the in-game check uses. Answers with the amount it stored, or `None` when no
/// factory stands there.
pub(crate) fn set_energy(host: &mut dyn ServerHost, pos: [i32; 3], amount: i32) -> Option<i32> {
    let (tier, entity) = factory_at(host, pos)?;
    let energy = clamp_energy(amount, tier.storage());
    let mut patch = Compound::new();
    patch.insert("energy".into(), Tag::Int(energy));
    // `merge_data` is `/data merge block`: it loads the patch and marks the chunk
    // changed, which is the `setChanged` the retired bundle had to fake.
    entity.merge_data(host, patch).ok().map(|_| energy)
}

/// `/mekmm chance <0|1|2> <pos>`: pin the roll off, on, or back to the real one.
/// Answers with the state it set, or `None` when no factory stands there.
pub(crate) fn set_chance(host: &mut dyn ServerHost, pos: [i32; 3], forced: i32) -> Option<String> {
    let (_tier, entity) = factory_at(host, pos)?;
    let mut data = entity.data(&*host)?;
    let state = match forced {
        0 | 1 => {
            data.insert(CHANCE_KEY.into(), Tag::Int(forced));
            format!("forced({})", forced == 1)
        }
        _ => {
            // Back to the real 17 % roll: the key is absent, as in 0.1.0.
            data.retain(|key, _| key.to_string_lossy() != CHANCE_KEY);
            "the real 17 % roll".to_owned()
        }
    };
    entity.replace_data(host, data).ok().map(|_| state)
}

/// `/mekmm progress <pos>`: the tier's progresses and cycle counters.
pub(crate) fn progress_line(host: &mut dyn ServerHost, pos: [i32; 3]) -> Option<String> {
    let (tier, entity) = factory_at(host, pos)?;
    let factory = read(&Tag::Compound(entity.data(&*host)?), tier);
    Some(format!(
        "Progress {}/{RECYCLER_TICKS}, cycles {}.",
        numbers(&factory.progress_saved()),
        numbers(&factory.cycles_saved())
    ))
}

/// Resolve `mekmm:scrap` once, during registration, for the output stacks.
pub(crate) fn remember_scrap(registrar: &mut dyn mod_api::Registrar) {
    let key = registrar.item_key(SCRAP);
    let _ = SCRAP_KEY.set(key);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tier_opens_the_smallest_vanilla_grid_that_holds_its_slots() {
        for (tier, slots, menu, processes) in [
            (Tier::Basic, 9, "minecraft:generic_3x3", 3),
            (Tier::Advanced, 18, "minecraft:generic_9x2", 5),
            (Tier::Elite, 18, "minecraft:generic_9x2", 7),
            (Tier::Ultimate, 27, "minecraft:generic_9x3", 9),
        ] {
            let container = container(tier);
            assert_eq!(grid(tier), slots, "{tier:?} grid");
            assert_eq!(container.slots, slots, "{tier:?} container slots");
            assert_eq!(container.menu, Some(menu.to_owned()), "{tier:?} menu");
            assert_eq!(
                container.inputs,
                (0..processes).collect::<Vec<_>>(),
                "{tier:?}: one input per process"
            );
            assert_eq!(
                container.outputs,
                (processes..2 * processes).collect::<Vec<_>>(),
                "{tier:?}: one output per process, after the inputs"
            );
            assert!(
                energy_slot(tier) < slots,
                "{tier:?}: the energy slot is inside the grid"
            );
            assert_eq!(
                container.faces,
                std::array::from_fn(|_| (0..slots).collect::<Vec<_>>()),
                "{tier:?}: automation reaches every slot from every face"
            );
        }
    }

    #[test]
    fn every_tier_declares_its_own_input_only_buffer() {
        for tier in Tier::ALL {
            let buffer = battery(tier);
            assert_eq!(buffer.capacity, tier.storage(), "{tier:?} capacity");
            assert_eq!(
                buffer.max_insert,
                tier.storage(),
                "{tier:?}: a side may fill it at the buffer's own rate"
            );
            assert_eq!(buffer.max_extract, 0, "{tier:?}: nothing may drain it");
            assert_eq!(buffer.insert_faces, [true; 6]);
            assert_eq!(buffer.extract_faces, [false; 6]);
        }
    }

    #[test]
    fn a_fresh_compound_is_an_empty_factory_of_its_tier() {
        for tier in Tier::ALL {
            assert_eq!(
                read(&Tag::Compound(Compound::new()), tier),
                Factory::new(tier),
                "{tier:?}"
            );
        }
        assert_eq!(forced_chance(&Tag::Compound(Compound::new())), None);
    }

    #[test]
    fn the_tick_s_state_reads_back_from_the_compound_it_wrote() {
        let mut saved = Tag::Compound(Compound::new());
        let factory = Factory {
            tier: Tier::Advanced,
            energy: 10_000,
            progress: vec![0, 60, 199, 0, 3],
            cycles: vec![3, 1, 0, 2, 5],
        };
        write(&mut saved, &factory);
        assert_eq!(
            read(&saved, Tier::Advanced),
            factory,
            "energy, progress and cycles round-trip"
        );
        assert_eq!(
            saved.get("energy"),
            Some(&Tag::Int(10_000)),
            "the buffer is the int `energy` the energy capability reads"
        );
        assert_eq!(
            saved.get("progress"),
            Some(&Tag::IntArray(vec![0, 60, 199, 0, 3])),
            "the progress is upstream's IntArray"
        );
        assert_eq!(
            saved.get("cycles"),
            Some(&Tag::IntArray(vec![3, 1, 0, 2, 5])),
            "so is the roll's counter"
        );
    }

    #[test]
    fn the_chance_override_is_only_the_two_pinned_values() {
        let mut saved = Tag::Compound(Compound::new());
        for (value, expected) in [(0, Some(false)), (1, Some(true)), (2, None)] {
            if let Tag::Compound(nbt) = &mut saved {
                nbt.insert(CHANCE_KEY.into(), Tag::Int(value));
            }
            assert_eq!(forced_chance(&saved), expected, "test_chance {value}");
        }
    }

    #[test]
    fn the_info_line_names_the_tier_and_its_own_buffer() {
        let sight = Sight {
            id: Tier::Basic.block().to_owned(),
            capacity: Tier::Basic.storage(),
            pos: [8, 64, 8],
            energy: 10_000,
            progress: vec![0, 0, 0],
            cycles: vec![1, 0, 0],
            inputs: vec![ItemCount::default(); 3],
            outputs: vec![
                ItemCount::new("mekmm:scrap", 1),
                ItemCount::default(),
                ItemCount::default(),
            ],
            forced: Some(true),
        };
        assert_eq!(
            sight.line(),
            "mekmm:basic_recycling_factory at 8,64,8: 10000/30000 FE, progress [0, 0, 0]/200, \
             cycles [1, 0, 0], in [empty, empty, empty], out [mekmm:scrap x1, empty, empty], \
             chance forced(true)"
        );
        let plain = Sight {
            forced: None,
            ..sight
        };
        assert!(
            plain.line().ends_with("out [mekmm:scrap x1, empty, empty]"),
            "the real roll adds no suffix"
        );

        let advanced = Sight {
            id: Tier::Ultimate.block().to_owned(),
            capacity: Tier::Ultimate.storage(),
            energy: 0,
            progress: vec![0; 9],
            cycles: vec![0; 9],
            inputs: vec![ItemCount::default(); 9],
            outputs: vec![ItemCount::default(); 9],
            forced: None,
            ..plain
        };
        assert_eq!(
            advanced.line(),
            "mekmm:ultimate_recycling_factory at 8,64,8: 0/90000 FE, progress \
             [0, 0, 0, 0, 0, 0, 0, 0, 0]/200, cycles [0, 0, 0, 0, 0, 0, 0, 0, 0], in [empty, \
             empty, empty, empty, empty, empty, empty, empty, empty], out [empty, empty, empty, \
             empty, empty, empty, empty, empty, empty]"
        );
    }
}

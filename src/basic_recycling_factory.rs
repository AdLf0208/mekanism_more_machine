//! The Basic Recycling Factory's glue: the block entity Part, the block's
//! container and energy declarations, the saved state and the `/mekmm` command
//! bodies.
//!
//! Every rule lives in [`crate::runtime`] (the retired bundle's `runtime.rs`,
//! byte-identical); this file reads the game, calls it and writes the result
//! back. It has no rules of its own, so its tests pin what it *declares* (nine
//! slots of which six work, an input-only buffer), that the state written to the
//! saved compound reads back, and the exact line `/mekmm info` prints.
use mod_api::{
    self, BlockEntityEvent, BlockEntityPart, BlockEntityTickEvent, BlockEntityTickReply,
    BlockEntityTicksEvent, BlockEntityTypeKey, BlockPart, Compound, Host, ItemKey, ItemStack,
    LevelSide, ModContainer, ModEnergyBuffer, ServerHost, Tag,
};

use crate::runtime::{
    BASIC_PROCESSES, Factory, RECYCLER_CAPACITY, RECYCLER_TICKS, RECYCLING_OPERATION, SCRAP, Stack,
    Tick, clamp_energy, factory_tick, recycling,
};

/// The slice's one block; the block entity type declared for it carries the
/// same name in this mod's namespace.
pub(crate) const FACTORY: &str = "mekmm:basic_recycling_factory";

/// The block entity type's path, in this mod's namespace: `/mekmm` has no
/// bearing on it.
const FACTORY_TYPE: &str = "basic_recycling_factory";

/// The container's slots: one input and one output per process, then upstream's
/// energy slot and two of the 3x3 menu's spare cells.
const SLOTS: usize = 9;

/// A process's input slot (`0, 1, 2`).
const fn input_slot(process: usize) -> usize {
    process
}

/// A process's output slot (`3, 4, 5`).
const fn output_slot(process: usize) -> usize {
    BASIC_PROCESSES + process
}

// Slot 6 is upstream's energy slot (an energy tablet goes there); this slice has
// no item energy, so it is declared and left unused, and 7-8 are the menu's
// spare cells.

/// The menu the container opens: the pin's only 9-slot vanilla grid, because a
/// mod container's menu must have exactly its slot count.
const MENU: &str = "minecraft:generic_3x3";

/// The saved key of the harness's chance override.
const CHANCE_KEY: &str = "test_chance";

/// The item key of `mekmm:scrap`, resolved once in `Hooks::register`: the
/// runtime's recipe table names items by id, and an output stack needs the key.
static SCRAP_KEY: std::sync::OnceLock<ItemKey> = std::sync::OnceLock::new();

/// The block entity type declared for the factory, built in `Hooks::register`.
pub(crate) fn declare(registrar: &mut dyn mod_api::Registrar) -> BlockEntityTypeKey {
    registrar
        .block_entity_type(FACTORY_TYPE)
        .blocks(&[FACTORY_TYPE])
        .build()
}

/// The factory's container: nine slots every face reaches, of which the first
/// six are the three inputs and three outputs (upstream's `native_block_entity_
/// container` at the retired pin).
pub(crate) fn container() -> ModContainer {
    let all: Vec<usize> = (0..SLOTS).collect();
    ModContainer {
        slots: SLOTS,
        faces: [
            all.clone(),
            all.clone(),
            all.clone(),
            all.clone(),
            all.clone(),
            all,
        ],
        inputs: (0..BASIC_PROCESSES).map(input_slot).collect(),
        outputs: (0..BASIC_PROCESSES).map(output_slot).collect(),
        menu: Some(MENU.to_owned()),
    }
}

/// The factory's buffer: 20,000 FE that a side may fill and none may drain, as
/// upstream's input-only `MachineEnergyContainer`.
pub(crate) fn battery() -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: RECYCLER_CAPACITY,
        max_insert: RECYCLER_CAPACITY,
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

/// A saved IntArray, padded to the port's process count.
fn array(saved: &Tag, key: &str) -> [i32; BASIC_PROCESSES] {
    let mut out = [0; BASIC_PROCESSES];
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
pub(crate) fn read(saved: &Tag) -> Factory {
    Factory::from_saved(
        &array(saved, "progress"),
        &array(saved, "cycles"),
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
        Tag::Int(clamp_energy(factory.energy, RECYCLER_CAPACITY)),
    );
    nbt.insert(
        "progress".into(),
        Tag::IntArray(factory.progress_saved().to_vec()),
    );
    nbt.insert(
        "cycles".into(),
        Tag::IntArray(factory.cycles_saved().to_vec()),
    );
}

/// What `/mekmm info` prints about one block entity.
pub(crate) struct Sight {
    pub(crate) id: String,
    pub(crate) pos: [i32; 3],
    pub(crate) energy: i32,
    pub(crate) progress: [i32; BASIC_PROCESSES],
    pub(crate) cycles: [i32; BASIC_PROCESSES],
    pub(crate) inputs: [Stack; BASIC_PROCESSES],
    pub(crate) outputs: [Stack; BASIC_PROCESSES],
    pub(crate) forced: Option<bool>,
}

impl Sight {
    /// The report line, byte-identical to the 0.1.0 readout the in-game check
    /// asserts on.
    pub(crate) fn line(&self) -> String {
        let mut text = format!(
            "{} at {},{},{}: {}/{} FE, progress {}/{RECYCLER_TICKS}, cycles {}, in [{}], out [{}]",
            self.id,
            self.pos[0],
            self.pos[1],
            self.pos[2],
            self.energy,
            RECYCLER_CAPACITY,
            numbers(self.progress),
            numbers(self.cycles),
            stacks(&self.inputs),
            stacks(&self.outputs),
        );
        if let Some(forced) = self.forced {
            text.push_str(&format!(", chance forced({forced})"));
        }
        text
    }
}

/// `[a, b, c]`, the shape the 0.1.0 readout used.
fn numbers(values: [i32; BASIC_PROCESSES]) -> String {
    format!("[{}, {}, {}]", values[0], values[1], values[2])
}

/// A process's three stacks as `item xN`, or `empty`.
fn stacks(stacks: &[Stack; BASIC_PROCESSES]) -> String {
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
fn slot_stack(host: &dyn Host, slot: Option<&ItemStack>) -> Stack {
    match slot {
        Some(stack) => Stack::new(&stack.item().id(host), stack.count()),
        None => Stack::default(),
    }
}

/// Write the pure core's output stack back into `slot`, keeping the previous
/// stack's components when it is the same item. A miss leaves the slot empty.
fn write_output(items: &mut [Option<ItemStack>], slot: usize, value: &Stack, key: ItemKey) {
    let Some(cell) = items.get_mut(slot) else {
        return;
    };
    if value.is_empty() {
        *cell = None;
        return;
    }
    if let Some(current) = cell.as_mut()
        && current.item() == key
    {
        let _ = current.set_count(value.count);
        return;
    }
    *cell = ItemStack::new(key, value.count);
}

/// `ItemStack.shrink`: an input slot only ever loses items, so its stack keeps
/// its components and only the count moves.
fn write_input(items: &mut [Option<ItemStack>], slot: usize, value: &Stack) {
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

/// The factory's behaviour: a block entity, ticked on the server.
pub(crate) struct FactoryPart;

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

    /// One server tick: three processes over one buffer. The payload is left
    /// `Changed` only when something moved, so an idle factory does not dirty its
    /// chunk every tick (the retired bundle's `mark_modified` condition).
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let pos = event.pos;
        let resized = event.entity.items.len() != SLOTS;
        if resized {
            event.entity.items.resize(SLOTS, None);
        }
        let mut saved = Tag::Compound(std::mem::take(&mut event.entity.nbt));
        let before = read(&saved);
        let mut factory = before;
        let mut inputs: [Stack; BASIC_PROCESSES] = std::array::from_fn(|process| {
            slot_stack(&*host, event.entity.items[input_slot(process)].as_ref())
        });
        let mut outputs: [Stack; BASIC_PROCESSES] = std::array::from_fn(|process| {
            slot_stack(&*host, event.entity.items[output_slot(process)].as_ref())
        });
        let (before_inputs, before_outputs) = (inputs.clone(), outputs.clone());
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
            for process in 0..BASIC_PROCESSES {
                write_input(
                    &mut event.entity.items,
                    input_slot(process),
                    &inputs[process],
                );
                write_output(
                    &mut event.entity.items,
                    output_slot(process),
                    &outputs[process],
                    key,
                );
            }
        }
    }
}

/// The factory at `pos` when what stands there is this mod's block: a foreign
/// block entity is not a factory (the 0.1.0 readout answered "No block entity
/// there." for one, and a chest must not be reported at 0 FE).
pub(crate) fn factory_at(
    host: &mut dyn ServerHost,
    pos: [i32; 3],
) -> Option<mod_api::BlockEntityRef> {
    let level = host.level(host.dimension())?;
    if level.block(&*host, pos)?.id(&*host) != FACTORY {
        return None;
    }
    host.block_entity(level, pos)
}

/// `/mekmm info <pos>`: the report line, or `None` when no factory stands there.
pub(crate) fn info_line(host: &mut dyn ServerHost, pos: [i32; 3]) -> Option<String> {
    let level = host.level(host.dimension())?;
    let id = level.block(&*host, pos)?.id(&*host);
    if id != FACTORY {
        return None;
    }
    let entity = host.block_entity(level, pos)?;
    let saved = Tag::Compound(entity.data(&*host)?);
    let factory = read(&saved);
    let mut inputs: [Stack; BASIC_PROCESSES] = std::array::from_fn(|_| Stack::default());
    let mut outputs: [Stack; BASIC_PROCESSES] = std::array::from_fn(|_| Stack::default());
    for process in 0..BASIC_PROCESSES {
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
                .item_slot(&*host, output_slot(process) as u32)
                .ok()
                .flatten()
                .as_ref(),
        );
    }
    let sight = Sight {
        id,
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
    let entity = factory_at(host, pos)?;
    let energy = clamp_energy(amount, RECYCLER_CAPACITY);
    let mut patch = Compound::new();
    patch.insert("energy".into(), Tag::Int(energy));
    // `merge_data` is `/data merge block`: it loads the patch and marks the chunk
    // changed, which is the `setChanged` the retired bundle had to fake.
    entity.merge_data(host, patch).ok().map(|_| energy)
}

/// `/mekmm chance <0|1|2> <pos>`: pin the roll off, on, or back to the real one.
/// Answers with the state it set, or `None` when no factory stands there.
pub(crate) fn set_chance(host: &mut dyn ServerHost, pos: [i32; 3], forced: i32) -> Option<String> {
    let entity = factory_at(host, pos)?;
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

/// `/mekmm progress <pos>`: the three progresses and the three cycle counters.
pub(crate) fn progress_line(host: &mut dyn ServerHost, pos: [i32; 3]) -> Option<String> {
    let entity = factory_at(host, pos)?;
    let factory = read(&Tag::Compound(entity.data(&*host)?));
    Some(format!(
        "Progress {}/{RECYCLER_TICKS}, cycles {}.",
        numbers(factory.progress_saved()),
        numbers(factory.cycles_saved())
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
    fn the_nine_slots_are_three_inputs_three_outputs_and_the_menus_spare_cells() {
        let container = container();
        assert_eq!(container.slots, SLOTS, "nine slots, the 3x3 grid");
        assert_eq!(container.inputs, vec![0, 1, 2], "one input per process");
        assert_eq!(container.outputs, vec![3, 4, 5], "one output per process");
        assert_eq!(
            container.faces,
            std::array::from_fn(|_| (0..SLOTS).collect::<Vec<_>>()),
            "automation reaches every slot from every face: there is no side configuration"
        );
        assert_eq!(
            container.menu,
            Some("minecraft:generic_3x3".to_owned()),
            "the pin's only nine-slot vanilla grid"
        );
    }

    #[test]
    fn the_factory_declares_its_input_only_buffer() {
        let buffer = battery();
        assert_eq!(buffer.capacity, RECYCLER_CAPACITY, "the factory's capacity");
        assert_eq!(
            buffer.max_insert, RECYCLER_CAPACITY,
            "a side may fill it at the buffer's own rate"
        );
        assert_eq!(buffer.max_extract, 0, "nothing may drain it: input only");
        assert_eq!(buffer.insert_faces, [true; 6]);
        assert_eq!(buffer.extract_faces, [false; 6]);
    }

    #[test]
    fn a_fresh_compound_is_an_empty_factory() {
        assert_eq!(read(&Tag::Compound(Compound::new())), Factory::default());
        assert_eq!(forced_chance(&Tag::Compound(Compound::new())), None);
    }

    #[test]
    fn the_tick_s_state_reads_back_from_the_compound_it_wrote() {
        let mut saved = Tag::Compound(Compound::new());
        let factory = Factory {
            energy: 10_000,
            progress: [0, 60, 199],
            cycles: [3, 1, 0],
        };
        write(&mut saved, &factory);
        assert_eq!(
            read(&saved),
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
            Some(&Tag::IntArray(vec![0, 60, 199])),
            "the progress is upstream's IntArray"
        );
        assert_eq!(
            saved.get("cycles"),
            Some(&Tag::IntArray(vec![3, 1, 0])),
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
    fn the_info_line_is_the_one_the_e2e_asserts() {
        let sight = Sight {
            id: FACTORY.to_owned(),
            pos: [8, 64, 8],
            energy: 10_000,
            progress: [0, 0, 0],
            cycles: [1, 0, 0],
            inputs: [Stack::default(), Stack::default(), Stack::default()],
            outputs: [
                Stack::new("mekmm:scrap", 1),
                Stack::default(),
                Stack::default(),
            ],
            forced: Some(true),
        };
        assert_eq!(
            sight.line(),
            "mekmm:basic_recycling_factory at 8,64,8: 10000/20000 FE, progress [0, 0, 0]/200, \
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
    }
}

// Game-side glue for the mekmm slice 1 (Basic Recycling Factory).
//
// Injected into `crate::game` as `mekmm_state`; it needs the game, so it is not
// part of the standalone library (only `runtime.rs` is).
//
// Energy lives in the block entity's saved compound as the int `energy`
// (`native_block_entity_energy`, API 142), each process's progress and completed
// operations as the IntArrays `progress` and `cycles`, so all of it survives a
// save and the block's own tick reads it back.

use horizon_client::containers::{ModBlockEntity, ModContainer, ModEnergyBuffer};
use horizon_client::items::ItemStack;
use horizon_world::nbt::{Compound, JavaString, Tag};

use crate::game::mekmm::{
    BASIC_PROCESSES, Factory, RECYCLER_CAPACITY, RECYCLER_TICKS, RECYCLING_OPERATION, SCRAP, Stack,
    Tick, clamp_energy, factory_tick, recycling,
};
use crate::game::native_server::{NativeCommand, NativeServerPlayer};

/// The one block of this slice.
const BLOCK: &str = "mekmm:basic_recycling_factory";

/// The container's slots: one input and one output per process, then the
/// upstream energy slot and two of the 3x3 menu's spare cells.
const SLOTS: usize = 9;

/// A process's input slot (`0, 1, 2`).
const fn input_slot(process: usize) -> usize {
    process
}

/// A process's output slot (`3, 4, 5`).
const fn output_slot(process: usize) -> usize {
    BASIC_PROCESSES + process
}

// Slot 6 is upstream's energy slot (an energy tablet goes there); this slice
// has no item energy, so it is declared and left unused, and 7-8 are the menu's
// spare cells.

/// The menu the container opens: the pin's only 9-slot vanilla grid, because a
/// mod container's menu must have exactly its slot count
/// (`native_hooks.rs` filters a menu of another size out).
const MENU: &str = "minecraft:generic_3x3";

/// A machine's saved energy (`native_block_entity_energy`'s int).
fn be_energy(entity: &ModBlockEntity) -> i32 {
    match entity.nbt.get(&JavaString::from("energy")) {
        Some(Tag::Int(energy)) => *energy,
        _ => 0,
    }
}

fn set_be_energy(entity: &mut ModBlockEntity, energy: i32) {
    entity
        .nbt
        .insert(JavaString::from("energy"), Tag::Int(energy));
}

/// A saved IntArray, padded to the port's process count.
fn be_array(entity: &ModBlockEntity, key: &str) -> [i32; BASIC_PROCESSES] {
    let mut out = [0; BASIC_PROCESSES];
    if let Some(Tag::IntArray(values)) = entity.nbt.get(&JavaString::from(key)) {
        for (slot, value) in out.iter_mut().zip(values) {
            *slot = *value;
        }
    }
    out
}

fn set_be_array(entity: &mut ModBlockEntity, key: &str, values: [i32; BASIC_PROCESSES]) {
    entity
        .nbt
        .insert(JavaString::from(key), Tag::IntArray(values.to_vec()));
}

/// The harness override of the chance roll: `test_chance` 0 never produces, 1
/// always does; absent means the real 17 % roll.
fn be_forced_chance(entity: &ModBlockEntity) -> Option<bool> {
    match entity.nbt.get(&JavaString::from("test_chance")) {
        Some(Tag::Int(0)) => Some(false),
        Some(Tag::Int(1)) => Some(true),
        _ => None,
    }
}

/// A slot as the pure core's stack, empty when the slot is empty.
fn slot_stack(slot: Option<&ItemStack>) -> Stack {
    match slot {
        Some(stack) if stack.count > 0 => Stack::new(&stack.item, stack.count),
        _ => Stack::default(),
    }
}

/// Write the pure core's stack back into `slot`, keeping the previous stack's
/// components when it is the same item.
fn set_slot(items: &mut [Option<ItemStack>], slot: usize, value: &Stack) {
    if value.is_empty() {
        items[slot] = None;
        return;
    }
    let same = items[slot]
        .as_ref()
        .is_some_and(|current| &*current.item == value.item);
    if same {
        if let Some(current) = items[slot].as_mut() {
            current.count = value.count;
        }
    } else {
        items[slot] = Some(ItemStack::new(&value.item, value.count));
    }
}

fn mark_modified(game: &mut crate::game::Game<'_>, pos: [i32; 3]) {
    game.world.modified.insert((pos[0] >> 4, pos[2] >> 4));
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

/// Declare the block entity (`native_block_entity`, API 15/129).
#[allow(private_interfaces)]
pub fn on_block_entity(block: &str, entity: &mut Option<(String, Compound)>) {
    if block == BLOCK {
        *entity = Some((block.to_owned(), Compound::new()));
    }
}

/// The factory ticks (`native_block_entity_ticks`, API 15).
#[allow(private_interfaces)]
pub fn on_block_entity_ticks(block: &str, ticks: &mut bool) {
    if block == BLOCK {
        *ticks = true;
    }
}

/// The energy buffer (`native_block_entity_energy`, API 142): the factory takes
/// energy and gives none, as upstream's input-only `MachineEnergyContainer`.
#[allow(private_interfaces)]
pub fn on_block_entity_energy(block: &str, energy: &mut Option<ModEnergyBuffer>) {
    if block != BLOCK {
        return;
    }
    *energy = Some(ModEnergyBuffer {
        capacity: RECYCLER_CAPACITY,
        max_insert: RECYCLER_CAPACITY,
        max_extract: 0,
        insert_faces: [true; 6],
        extract_faces: [false; 6],
    });
}

/// The 3+3 item slots and the 3x3 menu (`native_block_entity_container`, API
/// 16/17). Hoppers fill the inputs and take the outputs; the energy slot and
/// the menu's two spare cells are not automation slots.
#[allow(private_interfaces)]
pub fn on_block_entity_container(block: &str, container: &mut Option<ModContainer>) {
    if block != BLOCK {
        return;
    }
    let all: Vec<usize> = (0..SLOTS).collect();
    *container = Some(ModContainer {
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
    });
}

/// One tick of the factory (`native_block_entity_tick`, API 17).
#[allow(private_interfaces)]
pub fn on_block_entity_tick(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    entity: &mut ModBlockEntity,
) {
    if entity.id != BLOCK {
        return;
    }
    // The engine sizes a mod entity's slots when a container view is built, so
    // a fresh machine has none yet. Giving them their size here makes them real
    // from the first tick (a hopper, a menu or `ctl container` then finds them).
    if entity.items.len() < SLOTS {
        entity.items.resize(SLOTS, None);
    }
    let mut factory = Factory::from_saved(
        &be_array(entity, "progress"),
        &be_array(entity, "cycles"),
        be_energy(entity),
    );
    let before = factory;
    let mut inputs: [Stack; BASIC_PROCESSES] = std::array::from_fn(|_| Stack::default());
    let mut outputs: [Stack; BASIC_PROCESSES] = std::array::from_fn(|_| Stack::default());
    for process in 0..BASIC_PROCESSES {
        inputs[process] = slot_stack(
            entity
                .items
                .get(input_slot(process))
                .and_then(Option::as_ref),
        );
        outputs[process] = slot_stack(
            entity
                .items
                .get(output_slot(process))
                .and_then(Option::as_ref),
        );
    }
    let (before_inputs, before_outputs) = (inputs.clone(), outputs.clone());
    let max_stack = game.native_max_stack_size(SCRAP).unwrap_or(64).max(1);
    let tick = Tick {
        operation: RECYCLING_OPERATION,
        max_stack,
        seed: position_seed(game.native_dimension(), pos),
        forced_chance: be_forced_chance(entity),
        lookup: &recycling::lookup,
    };
    factory_tick(&mut factory, &tick, &mut inputs, &mut outputs);
    if factory == before && inputs == before_inputs && outputs == before_outputs {
        return;
    }
    set_be_energy(entity, clamp_energy(factory.energy, RECYCLER_CAPACITY));
    set_be_array(entity, "progress", factory.progress_saved());
    set_be_array(entity, "cycles", factory.cycles_saved());
    for process in 0..BASIC_PROCESSES {
        set_slot(&mut entity.items, input_slot(process), &inputs[process]);
        set_slot(&mut entity.items, output_slot(process), &outputs[process]);
    }
    mark_modified(game, pos);
}

/// `/mekmm info|energy|chance|progress` (`register_commands`, API 8).
#[allow(private_interfaces)]
pub fn on_register_commands(commands: &mut Vec<NativeCommand>) {
    commands.push(NativeCommand::new("mekmm info <pos:block_pos>"));
    commands.push(NativeCommand::op(
        "mekmm energy <amount:integer> <pos:block_pos>",
        2,
    ));
    commands.push(NativeCommand::op(
        "mekmm chance <forced:integer> <pos:block_pos>",
        2,
    ));
    commands.push(NativeCommand::new("mekmm progress <pos:block_pos>"));
}

/// A stack as the info line shows it: `item xN`, or `empty`.
fn stack_text(stack: &Stack) -> String {
    if stack.is_empty() {
        "empty".to_owned()
    } else {
        format!("{} x{}", stack.item, stack.count)
    }
}

fn parse_pos(words: &[&str]) -> Option<[i32; 3]> {
    Some([
        words.first()?.parse().ok()?,
        words.get(1)?.parse().ok()?,
        words.get(2)?.parse().ok()?,
    ])
}

fn numbers(values: [i32; BASIC_PROCESSES]) -> String {
    format!("[{}, {}, {}]", values[0], values[1], values[2])
}

/// The `/mekmm` commands (`perform_command`, API 8). `info` prints the machine,
/// `energy` sets its stored FE, `chance` pins the roll on/off and `progress`
/// prints the three processes — the harness controls the in-game check uses.
#[allow(private_interfaces)]
pub fn on_perform_command(
    game: &mut crate::game::Game<'_>,
    _player: &NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    let words: Vec<&str> = command.split_whitespace().collect();
    match words.as_slice() {
        ["mekmm", "info", x, y, z] => {
            *ran = true;
            let Some(pos) = parse_pos(&[x, y, z]) else {
                game.native_feedback(horizon_client::text::Text::literal(
                    "Usage: /mekmm info <x> <y> <z>",
                ));
                return;
            };
            let Some(report) = game.native_with_mod_block_entity(pos, |entity| {
                let id = entity.id.clone();
                let energy = be_energy(entity);
                let progress = Factory::from_saved(
                    &be_array(entity, "progress"),
                    &be_array(entity, "cycles"),
                    energy,
                );
                let mut inputs: [Stack; BASIC_PROCESSES] =
                    std::array::from_fn(|_| Stack::default());
                let mut outputs: [Stack; BASIC_PROCESSES] =
                    std::array::from_fn(|_| Stack::default());
                for process in 0..BASIC_PROCESSES {
                    inputs[process] = slot_stack(
                        entity
                            .items
                            .get(input_slot(process))
                            .and_then(Option::as_ref),
                    );
                    outputs[process] = slot_stack(
                        entity
                            .items
                            .get(output_slot(process))
                            .and_then(Option::as_ref),
                    );
                }
                (
                    id,
                    energy,
                    progress,
                    inputs,
                    outputs,
                    be_forced_chance(entity),
                )
            }) else {
                game.native_feedback(horizon_client::text::Text::literal(
                    "No block entity there.",
                ));
                return;
            };
            let (id, energy, factory, inputs, outputs, forced) = report;
            let capacity = if id == BLOCK { RECYCLER_CAPACITY } else { 0 };
            let mut text = format!(
                "{id}: {energy}/{capacity} FE, progress {}/{RECYCLER_TICKS}, cycles {}, in [{}], out [{}]",
                numbers(factory.progress_saved()),
                numbers(factory.cycles_saved()),
                inputs.map(|stack| stack_text(&stack)).join(", "),
                outputs.map(|stack| stack_text(&stack)).join(", "),
            );
            if let Some(forced) = forced {
                text.push_str(&format!(", chance forced({forced})"));
            }
            game.native_feedback(horizon_client::text::Text::literal(text));
        }
        ["mekmm", "energy", amount, x, y, z] => {
            *ran = true;
            let Some(pos) = parse_pos(&[x, y, z]) else {
                game.native_feedback(horizon_client::text::Text::literal(
                    "Usage: /mekmm energy <amount> <x> <y> <z>",
                ));
                return;
            };
            let Ok(amount) = amount.parse::<i32>() else {
                game.native_feedback(horizon_client::text::Text::literal(
                    "The amount must be a whole number of FE.",
                ));
                return;
            };
            let Some(after) = game.native_with_mod_block_entity(pos, |entity| {
                let energy = clamp_energy(amount, RECYCLER_CAPACITY);
                set_be_energy(entity, energy);
                energy
            }) else {
                game.native_feedback(horizon_client::text::Text::literal(
                    "No block entity there.",
                ));
                return;
            };
            game.native_feedback(horizon_client::text::Text::literal(format!(
                "Energy set to {after} FE."
            )));
        }
        ["mekmm", "chance", forced, x, y, z] => {
            *ran = true;
            let Some(pos) = parse_pos(&[x, y, z]) else {
                game.native_feedback(horizon_client::text::Text::literal(
                    "Usage: /mekmm chance <0|1|2> <x> <y> <z>",
                ));
                return;
            };
            let Ok(forced) = forced.parse::<i32>() else {
                game.native_feedback(horizon_client::text::Text::literal(
                    "Usage: /mekmm chance <0|1|2> <x> <y> <z>",
                ));
                return;
            };
            let Some(state) = game.native_with_mod_block_entity(pos, |entity| {
                let key = JavaString::from("test_chance");
                match forced {
                    0 | 1 => {
                        entity.nbt.insert(key, Tag::Int(forced));
                        format!("forced({})", forced == 1)
                    }
                    _ => {
                        entity.nbt.remove(&key);
                        "the real 17 % roll".to_owned()
                    }
                }
            }) else {
                game.native_feedback(horizon_client::text::Text::literal(
                    "No block entity there.",
                ));
                return;
            };
            game.native_feedback(horizon_client::text::Text::literal(format!(
                "Chance is now {state}."
            )));
        }
        ["mekmm", "progress", x, y, z] => {
            *ran = true;
            let Some(pos) = parse_pos(&[x, y, z]) else {
                game.native_feedback(horizon_client::text::Text::literal(
                    "Usage: /mekmm progress <x> <y> <z>",
                ));
                return;
            };
            let Some((progress, cycles)) = game.native_with_mod_block_entity(pos, |entity| {
                let factory = Factory::from_saved(
                    &be_array(entity, "progress"),
                    &be_array(entity, "cycles"),
                    0,
                );
                (factory.progress_saved(), factory.cycles_saved())
            }) else {
                game.native_feedback(horizon_client::text::Text::literal(
                    "No block entity there.",
                ));
                return;
            };
            game.native_feedback(horizon_client::text::Text::literal(format!(
                "Progress {}/{RECYCLER_TICKS}, cycles {}.",
                numbers(progress),
                numbers(cycles)
            )));
        }
        _ => {}
    }
}

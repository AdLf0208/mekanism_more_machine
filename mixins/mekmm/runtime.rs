// The pure core of the mekmm slice 1 port (clean-room rewrite).
//
// One machine: the **Basic Recycling Factory** (`mekmm:basic_recycling_factory`)
// from Mekanism: MoreMachine 1.4.2 (MIT), an add-on of Mekanism. It is the
// Mekanism factory shape at the basic tier: three parallel processes, each
// spending 50 FE/t for 200 ticks, sharing one 20,000 FE buffer, and each
// turning one item into `mekmm:scrap` on a 17 % chance roll.
//
// Nothing here touches the game: the block entity in `support.rs` reads its
// saved NBT, calls these functions and writes the result back.
//
// Values and semantics come from the upstream tag `v1.4.2-1.21.1`; see
// `FINDINGS.md` for the files read and where every number came from.

/// Ticks one operation takes (`BASE_TICKS_REQUIRED` = 10 s, the factory base
/// and the recycler both).
pub const RECYCLER_TICKS: i32 = 200;

/// Energy one operating tick of **one process** costs
/// (`UsageConfig.recycler`, 50 FE/t).
pub const RECYCLER_USAGE: i32 = 50;

/// The factory's shared buffer (`StorageConfig.recycler`).
pub const RECYCLER_CAPACITY: i32 = 20_000;

/// Processes a basic-tier factory runs in parallel (`FactoryTier.BASIC`).
pub const BASIC_PROCESSES: usize = 3;

/// The one recipe's output item, `mekmm:scrap`.
pub const SCRAP: &str = "mekmm:scrap";

/// How long one operation takes and what one operating tick costs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Operation {
    /// Ticks an operation takes.
    pub ticks: i32,
    /// Energy an operating tick costs, in FE.
    pub usage: i32,
}

/// The recycling factory's operation.
pub const RECYCLING_OPERATION: Operation = Operation {
    ticks: RECYCLER_TICKS,
    usage: RECYCLER_USAGE,
};

/// An item stack as the pure core sees it: an id and a count.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Stack {
    /// The item id (`minecraft:stone`).
    pub item: String,
    /// How many of it.
    pub count: u32,
}

impl Stack {
    /// A stack of `count` of `item`.
    #[must_use]
    pub fn new(item: &str, count: u32) -> Self {
        Self {
            item: item.to_owned(),
            count,
        }
    }

    /// Whether the stack holds nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count == 0 || self.item.is_empty()
    }
}

/// One recipe as the factory runs it: how many input items one operation
/// consumes, what it may produce and how likely that is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recipe {
    /// Items taken from the input slot when the operation finishes.
    pub input: u32,
    /// The result, a stack of the recipe's item and count.
    pub output: Stack,
    /// The chance the result is produced, in whole percent (upstream `chance`
    /// 0.17 is 17).
    pub chance_percent: u32,
}

/// A factory's saved state: its energy, each process's progress in ticks and
/// each process's completed operations (the roll's counter).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Factory {
    /// Stored energy in FE.
    pub energy: i32,
    /// Ticks of the operation in progress, per process, `0..operation.ticks`.
    pub progress: [i32; BASIC_PROCESSES],
    /// Operations completed, per process: the chance roll's counter.
    pub cycles: [u32; BASIC_PROCESSES],
}

impl Factory {
    /// A factory from what its block entity saves: the `progress` and `cycles`
    /// IntArrays and the `energy` int, padded to three processes, clamped.
    #[must_use]
    pub fn from_saved(progress: &[i32], cycles: &[i32], energy: i32) -> Self {
        let mut factory = Self {
            energy: clamp_energy(energy, RECYCLER_CAPACITY),
            ..Self::default()
        };
        for (slot, &value) in factory.progress.iter_mut().zip(progress) {
            *slot = value.clamp(0, RECYCLER_TICKS - 1);
        }
        for (slot, &value) in factory.cycles.iter_mut().zip(cycles) {
            *slot = u32::try_from(value).unwrap_or(0);
        }
        factory
    }

    /// The progress array, as the entity's `progress` IntArray.
    #[must_use]
    pub fn progress_saved(&self) -> [i32; BASIC_PROCESSES] {
        self.progress
    }

    /// The cycle counters, as the entity's `cycles` IntArray.
    #[must_use]
    pub fn cycles_saved(&self) -> [i32; BASIC_PROCESSES] {
        self.cycles
            .map(|cycle| i32::try_from(cycle).unwrap_or(i32::MAX))
    }
}

/// Why a process did nothing on its tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdleReason {
    /// The input slot is empty (the progress resets).
    NoInput,
    /// The input has no recipe (the progress resets).
    NoRecipe,
    /// The input stack is smaller than the recipe needs (the progress resets;
    /// upstream's `NOT_ENOUGH_INPUT`).
    NotEnoughInput,
    /// Less than one tick's energy is stored (the progress is held).
    NoEnergy,
    /// The output slot has no room for the recipe's result (held).
    NoOutputSpace,
}

/// What one process did on its tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// A tick of the operation was made; energy was spent.
    Working,
    /// The operation finished: the input was consumed and the chance rolled.
    /// `output` is the scrap produced, or `None` when the roll missed.
    Produced {
        /// The result the roll produced, `None` on a miss.
        output: Option<Stack>,
    },
    /// Nothing happened.
    Idle(IdleReason),
}

/// What one tick of a factory needs besides its own state.
pub struct Tick<'a> {
    /// The operation's ticks and FE/t.
    pub operation: Operation,
    /// The result item's stack size, for the output-room check.
    pub max_stack: u32,
    /// A block position's seed; mixed with the process and its cycle so a
    /// factory's rolls are reproducible without a world RNG.
    pub seed: u64,
    /// A harness override: `Some(true)` always produces, `Some(false)` never.
    pub forced_chance: Option<bool>,
    /// The recipe table.
    pub lookup: &'a dyn Fn(&str) -> Option<Recipe>,
}

/// `energy` clamped into `0..=capacity`, as a buffer reads its saved int.
#[must_use]
pub fn clamp_energy(energy: i32, capacity: i32) -> i32 {
    energy.clamp(0, capacity.max(0))
}

/// SplitMix64: the port's own deterministic random for the chance roll.
#[must_use]
pub fn splitmix64(state: u64) -> u64 {
    let mut z = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Whether a `chance_percent` roll succeeds for `seed`: an exact percentage
/// (0 never, 100 always) drawn uniformly from SplitMix64.
#[must_use]
pub fn roll(chance_percent: u32, seed: u64) -> bool {
    let chance = chance_percent.min(100);
    chance > 0 && (splitmix64(seed) % 100) < u64::from(chance)
}

/// The roll's seed for one process's next operation: the block's seed mixed
/// with the process index and how many operations it has completed, so two
/// processes and two cycles never share a roll.
#[must_use]
pub fn roll_seed(seed: u64, process: usize, cycle: u32) -> u64 {
    let process = (process as u64)
        .wrapping_add(1)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let cycle = u64::from(cycle)
        .wrapping_add(1)
        .wrapping_mul(0xD1B5_4A32_D192_ED03);
    splitmix64(seed ^ process ^ cycle)
}

/// One tick of all three processes (`CachedRecipe.process`, one per process).
///
/// Each process reads its own input and output, spends `operation.usage` for
/// one progress tick, and on `operation.ticks` consumes the recipe's input and
/// rolls the recipe's chance for its output. No energy or no output room holds
/// a process's progress; an empty input, an input without a recipe or too few
/// items resets it (upstream's `mismatchedRecipe` and `NOT_ENOUGH_INPUT`).
pub fn factory_tick(
    factory: &mut Factory,
    tick: &Tick<'_>,
    inputs: &mut [Stack; BASIC_PROCESSES],
    outputs: &mut [Stack; BASIC_PROCESSES],
) -> [Outcome; BASIC_PROCESSES] {
    let mut outcomes = [
        Outcome::Idle(IdleReason::NoInput),
        Outcome::Idle(IdleReason::NoInput),
        Outcome::Idle(IdleReason::NoInput),
    ];
    for process in 0..BASIC_PROCESSES {
        outcomes[process] = process_tick(
            factory,
            tick,
            process,
            &mut inputs[process],
            &mut outputs[process],
        );
    }
    outcomes
}

/// One process of a factory's tick. Split out so the three are visibly the
/// same machine, sharing only the buffer in `factory.energy`.
fn process_tick(
    factory: &mut Factory,
    tick: &Tick<'_>,
    process: usize,
    input: &mut Stack,
    output: &mut Stack,
) -> Outcome {
    let operation = tick.operation;
    if input.is_empty() {
        factory.progress[process] = 0;
        return Outcome::Idle(IdleReason::NoInput);
    }
    let Some(recipe) = (tick.lookup)(&input.item).filter(|recipe| !recipe.output.is_empty()) else {
        factory.progress[process] = 0;
        return Outcome::Idle(IdleReason::NoRecipe);
    };
    if input.count < recipe.input.max(1) {
        factory.progress[process] = 0;
        return Outcome::Idle(IdleReason::NotEnoughInput);
    }
    // Upstream checks the *maximum* chance output's room before it spends
    // energy (`MoreMachineOutputHelper.calculateOperationsCanSupport`).
    let room = if output.is_empty() {
        tick.max_stack
    } else if output.item == recipe.output.item {
        tick.max_stack.saturating_sub(output.count)
    } else {
        0
    };
    if room < recipe.output.count {
        return Outcome::Idle(IdleReason::NoOutputSpace);
    }
    if factory.energy < operation.usage {
        return Outcome::Idle(IdleReason::NoEnergy);
    }
    factory.energy -= operation.usage;
    factory.progress[process] += 1;
    if factory.progress[process] < operation.ticks {
        return Outcome::Working;
    }
    factory.progress[process] = 0;
    input.count -= recipe.input.max(1);
    let produced = match tick.forced_chance {
        Some(hit) => hit,
        None => roll(
            recipe.chance_percent,
            roll_seed(tick.seed, process, factory.cycles[process]),
        ),
    };
    factory.cycles[process] = factory.cycles[process].wrapping_add(1);
    let output_stack = produced.then(|| recipe.output.clone());
    if let Some(stack) = &output_stack {
        if output.is_empty() {
            *output = stack.clone();
        } else {
            output.count += stack.count;
        }
    }
    Outcome::Produced {
        output: output_stack,
    }
}

/// The recycling recipe table: the jar's `mekmm:recycler` recipes over items
/// this port registers.
///
/// Source: `data/mekmm/recipe/recycler/{from_stone,from_dirt,from_substrate}.json`.
/// `from_stone` matches the `c:stones` tag; 26.3 ships no convention (`c`)
/// tags, so the tag's membership is this port's definition (the same one
/// `ports/cookingforblockheads` carries). `from_substrate` needs
/// `mekanism:substrate`, which this port does not register, so it is dropped.
pub mod recycling {
    /// The rows: `(input item, output item, output count, chance percent)`.
    static TABLE: &[(&str, &str, u32, u32)] = &[
        // `c:stones` (the port's definition of the convention tag).
        ("minecraft:stone", "mekmm:scrap", 1, 17),
        ("minecraft:granite", "mekmm:scrap", 1, 17),
        ("minecraft:diorite", "mekmm:scrap", 1, 17),
        ("minecraft:andesite", "mekmm:scrap", 1, 17),
        ("minecraft:deepslate", "mekmm:scrap", 1, 17),
        ("minecraft:tuff", "mekmm:scrap", 1, 17),
        ("minecraft:calcite", "mekmm:scrap", 1, 17),
        ("minecraft:dripstone_block", "mekmm:scrap", 1, 17),
        ("minecraft:smooth_stone", "mekmm:scrap", 1, 17),
        // `from_dirt`: dirt or grass_block.
        ("minecraft:dirt", "mekmm:scrap", 1, 17),
        ("minecraft:grass_block", "mekmm:scrap", 1, 17),
    ];

    /// The recipe for `item`, `None` when the factory cannot recycle it.
    #[must_use]
    pub fn lookup(item: &str) -> Option<super::Recipe> {
        TABLE.iter().find(|(input, ..)| *input == item).map(
            |&(_, output, count, chance_percent)| super::Recipe {
                input: 1,
                output: super::Stack::new(output, count),
                chance_percent,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(item: &str, count: u32) -> Stack {
        Stack::new(item, count)
    }

    fn empty() -> [Stack; BASIC_PROCESSES] {
        [Stack::default(), Stack::default(), Stack::default()]
    }

    fn inputs(items: [&str; BASIC_PROCESSES]) -> [Stack; BASIC_PROCESSES] {
        items.map(|item| stack(item, 1))
    }

    fn tick_with<'a>(forced: Option<bool>, lookup: &'a dyn Fn(&str) -> Option<Recipe>) -> Tick<'a> {
        Tick {
            operation: RECYCLING_OPERATION,
            max_stack: 64,
            seed: 0x1234_5678,
            forced_chance: forced,
            lookup,
        }
    }

    fn run(
        factory: &mut Factory,
        tick: &Tick<'_>,
        inputs: &mut [Stack; 3],
        outputs: &mut [Stack; 3],
        ticks: i32,
    ) {
        for _ in 0..ticks {
            factory_tick(factory, tick, inputs, outputs);
        }
    }

    #[test]
    fn the_constants_are_the_jars_numbers() {
        assert_eq!(RECYCLER_TICKS, 200, "BASE_TICKS_REQUIRED = 10 s");
        assert_eq!(RECYCLER_USAGE, 50, "UsageConfig.recycler");
        assert_eq!(RECYCLER_CAPACITY, 20_000, "StorageConfig.recycler");
        assert_eq!(BASIC_PROCESSES, 3, "FactoryTier.BASIC.processes");
        assert_eq!(RECYCLING_OPERATION.ticks, RECYCLER_TICKS);
        assert_eq!(RECYCLING_OPERATION.usage, RECYCLER_USAGE);
    }

    #[test]
    fn the_operation_costs_ten_thousand_fe() {
        assert_eq!(
            i64::from(RECYCLER_TICKS) * i64::from(RECYCLER_USAGE),
            10_000,
            "200 ticks at 50 FE/t"
        );
    }

    #[test]
    fn no_energy_holds_the_operation_and_spends_nothing() {
        let mut factory = Factory::default();
        let mut in_slots = inputs(["minecraft:stone", "", ""]);
        let mut out_slots = empty();
        let outcomes = factory_tick(
            &mut factory,
            &tick_with(Some(true), &recycling::lookup),
            &mut in_slots,
            &mut out_slots,
        );
        assert_eq!(outcomes[0], Outcome::Idle(IdleReason::NoEnergy));
        assert_eq!(factory.progress, [0, 0, 0], "the progress is held");
        assert_eq!(factory.energy, 0);
        assert_eq!(in_slots[0].count, 1, "nothing was consumed");
    }

    #[test]
    fn one_tick_spends_fifty_fe_and_advances_one_tick() {
        let mut factory = Factory {
            energy: 20_000,
            ..Default::default()
        };
        let mut in_slots = inputs(["minecraft:stone", "", ""]);
        let mut out_slots = empty();
        let outcomes = factory_tick(
            &mut factory,
            &tick_with(Some(true), &recycling::lookup),
            &mut in_slots,
            &mut out_slots,
        );
        assert_eq!(outcomes[0], Outcome::Working);
        assert_eq!(factory.energy, 19_950);
        assert_eq!(factory.progress, [1, 0, 0]);
    }

    #[test]
    fn an_operation_takes_two_hundred_ticks_and_ten_thousand_fe() {
        let mut factory = Factory {
            energy: 20_000,
            ..Default::default()
        };
        let mut in_slots = inputs(["minecraft:stone", "", ""]);
        let mut out_slots = empty();
        let mut last = Outcome::Working;
        for i in 0..RECYCLER_TICKS {
            last = factory_tick(
                &mut factory,
                &tick_with(Some(true), &recycling::lookup),
                &mut in_slots,
                &mut out_slots,
            )[0]
            .clone();
            if i + 1 < RECYCLER_TICKS {
                assert_eq!(last, Outcome::Working, "tick {} is not the finish", i + 1);
                assert_eq!(
                    in_slots[0].count, 1,
                    "the input is consumed at the end only"
                );
            }
        }
        assert_eq!(
            last,
            Outcome::Produced {
                output: Some(stack("mekmm:scrap", 1))
            }
        );
        assert_eq!(factory.energy, 10_000, "exactly one operation's energy");
        assert_eq!(factory.progress, [0, 0, 0]);
        assert_eq!(in_slots[0].count, 0);
        assert_eq!(out_slots[0], stack("mekmm:scrap", 1));
        assert_eq!(factory.cycles, [1, 0, 0]);
    }

    #[test]
    fn three_processes_each_cost_fifty_fe_from_the_one_buffer() {
        let mut factory = Factory {
            energy: 10_000,
            ..Default::default()
        };
        let mut in_slots = inputs(["minecraft:stone"; BASIC_PROCESSES]);
        let mut out_slots = empty();
        run(
            &mut factory,
            &tick_with(Some(true), &recycling::lookup),
            &mut in_slots,
            &mut out_slots,
            67,
        );
        // 67 x 150 = 10,050 wanted from 10,000: the first two processes take
        // the last 100 FE on tick 67, the third finds none.
        assert_eq!(factory.energy, 0);
        assert_eq!(factory.progress, [67, 67, 66]);
        assert_eq!(
            in_slots.map(|s| s.count),
            [1, 1, 1],
            "no operation finished"
        );
    }

    #[test]
    fn each_process_runs_its_own_recipe() {
        let mut factory = Factory {
            energy: 20_000,
            ..Default::default()
        };
        let mut in_slots = inputs(["minecraft:stone", "minecraft:dirt", "minecraft:diamond"]);
        let mut out_slots = empty();
        let outcomes = factory_tick(
            &mut factory,
            &tick_with(Some(true), &recycling::lookup),
            &mut in_slots,
            &mut out_slots,
        );
        assert_eq!(outcomes[0], Outcome::Working);
        assert_eq!(outcomes[1], Outcome::Working);
        assert_eq!(outcomes[2], Outcome::Idle(IdleReason::NoRecipe));
        assert_eq!(factory.progress, [1, 1, 0]);
        assert_eq!(factory.energy, 19_900, "two processes spent");
        assert_eq!(in_slots[2].count, 1, "the diamond is left alone");
    }

    #[test]
    fn an_input_without_a_recipe_resets_the_progress() {
        let mut factory = Factory {
            energy: 20_000,
            progress: [50, 0, 0],
            ..Default::default()
        };
        let mut in_slots = inputs(["minecraft:iron_ingot", "", ""]);
        let mut out_slots = empty();
        let outcomes = factory_tick(
            &mut factory,
            &tick_with(Some(true), &recycling::lookup),
            &mut in_slots,
            &mut out_slots,
        );
        assert_eq!(outcomes[0], Outcome::Idle(IdleReason::NoRecipe));
        assert_eq!(factory.progress, [0, 0, 0], "the progress resets");
        assert_eq!(factory.energy, 20_000, "no energy was spent");
        assert_eq!(in_slots[0].count, 1);
    }

    #[test]
    fn an_empty_input_resets_the_progress() {
        let mut factory = Factory {
            energy: 20_000,
            progress: [50, 0, 0],
            ..Default::default()
        };
        let mut in_slots = empty();
        let mut out_slots = empty();
        let outcomes = factory_tick(
            &mut factory,
            &tick_with(Some(true), &recycling::lookup),
            &mut in_slots,
            &mut out_slots,
        );
        assert_eq!(outcomes[0], Outcome::Idle(IdleReason::NoInput));
        assert_eq!(factory.progress, [0, 0, 0]);
        assert_eq!(factory.energy, 20_000);
    }

    #[test]
    fn too_few_items_holds_nothing_and_resets() {
        // A row whose recipe needs more than the slot holds would reset; the
        // recycling table needs one, so exercise the guard through a custom
        // table instead of guessing a second upstream recipe.
        fn three(_item: &str) -> Option<Recipe> {
            Some(Recipe {
                input: 3,
                output: stack("mekmm:scrap", 1),
                chance_percent: 100,
            })
        }
        let mut factory = Factory {
            energy: 20_000,
            progress: [50, 0, 0],
            ..Default::default()
        };
        let mut in_slots = inputs(["minecraft:stone", "", ""]);
        in_slots[0].count = 2;
        let mut out_slots = empty();
        let outcomes = factory_tick(
            &mut factory,
            &tick_with(None, &three),
            &mut in_slots,
            &mut out_slots,
        );
        assert_eq!(outcomes[0], Outcome::Idle(IdleReason::NotEnoughInput));
        assert_eq!(factory.progress, [0, 0, 0]);
        assert_eq!(factory.energy, 20_000);
    }

    #[test]
    fn a_full_output_holds_the_progress_without_spending() {
        let mut factory = Factory {
            energy: 20_000,
            progress: [10, 0, 0],
            ..Default::default()
        };
        let mut in_slots = inputs(["minecraft:stone", "", ""]);
        let mut out_slots = empty();
        out_slots[0] = stack("mekmm:scrap", 64);
        let outcomes = factory_tick(
            &mut factory,
            &tick_with(Some(true), &recycling::lookup),
            &mut in_slots,
            &mut out_slots,
        );
        assert_eq!(outcomes[0], Outcome::Idle(IdleReason::NoOutputSpace));
        assert_eq!(factory.progress, [10, 0, 0], "held, not reset");
        assert_eq!(factory.energy, 20_000, "no energy was spent");
        assert_eq!(in_slots[0].count, 1);
    }

    #[test]
    fn another_item_in_the_output_slot_blocks_the_result() {
        let mut factory = Factory {
            energy: 20_000,
            ..Default::default()
        };
        let mut in_slots = inputs(["minecraft:stone", "", ""]);
        let mut out_slots = empty();
        out_slots[0] = stack("minecraft:iron_ingot", 1);
        let outcomes = factory_tick(
            &mut factory,
            &tick_with(Some(true), &recycling::lookup),
            &mut in_slots,
            &mut out_slots,
        );
        assert_eq!(outcomes[0], Outcome::Idle(IdleReason::NoOutputSpace));
        assert_eq!(factory.energy, 20_000);
    }

    #[test]
    fn a_missed_roll_still_consumes_the_input_and_the_energy() {
        let mut factory = Factory {
            energy: 20_000,
            ..Default::default()
        };
        let mut in_slots = inputs(["minecraft:stone", "", ""]);
        let mut out_slots = empty();
        run(
            &mut factory,
            &tick_with(Some(false), &recycling::lookup),
            &mut in_slots,
            &mut out_slots,
            RECYCLER_TICKS,
        );
        assert_eq!(factory.energy, 10_000);
        assert_eq!(in_slots[0].count, 0, "the input is consumed either way");
        assert_eq!(out_slots[0], Stack::default(), "and nothing is produced");
        assert_eq!(factory.cycles, [1, 0, 0]);
    }

    #[test]
    fn the_roll_is_the_recipes_chance() {
        assert_eq!(
            recycling::lookup("minecraft:stone").map(|r| r.chance_percent),
            Some(17)
        );
        assert!(!roll(0, 12345), "0 % never succeeds");
        assert!(roll(100, 12345), "100 % always succeeds");
        let hits = (0..10_000u64).filter(|seed| roll(17, *seed)).count();
        assert!(
            (1_550..=1_850).contains(&hits),
            "17 % of 10,000 seeds, got {hits}"
        );
    }

    #[test]
    fn two_processes_and_two_cycles_never_share_a_roll_seed() {
        let mut seeds = std::collections::BTreeSet::new();
        for process in 0..BASIC_PROCESSES {
            for cycle in 0..64 {
                seeds.insert(roll_seed(0xABCD, process, cycle));
            }
        }
        assert_eq!(seeds.len(), BASIC_PROCESSES * 64, "every seed is distinct");
    }

    #[test]
    fn stone_dirt_and_grass_block_recycle_and_other_items_do_not() {
        for item in [
            "minecraft:stone",
            "minecraft:granite",
            "minecraft:diorite",
            "minecraft:andesite",
            "minecraft:deepslate",
            "minecraft:tuff",
            "minecraft:calcite",
            "minecraft:dripstone_block",
            "minecraft:smooth_stone",
            "minecraft:dirt",
            "minecraft:grass_block",
        ] {
            let recipe = recycling::lookup(item).unwrap_or_else(|| panic!("{item} recycles"));
            assert_eq!(recipe.input, 1);
            assert_eq!(recipe.output, stack("mekmm:scrap", 1));
            assert_eq!(recipe.chance_percent, 17);
        }
        for item in [
            "minecraft:iron_ingot",
            "minecraft:diamond",
            "minecraft:cobblestone",
            "mekanism:substrate",
            "",
        ] {
            assert_eq!(recycling::lookup(item), None, "{item} has no recipe");
        }
    }

    #[test]
    fn a_stack_runs_one_operation_at_a_time() {
        let mut factory = Factory {
            energy: 20_000,
            ..Default::default()
        };
        let mut in_slots = empty();
        in_slots[0] = stack("minecraft:stone", 5);
        let mut out_slots = empty();
        run(
            &mut factory,
            &tick_with(Some(true), &recycling::lookup),
            &mut in_slots,
            &mut out_slots,
            RECYCLER_TICKS,
        );
        assert_eq!(in_slots[0].count, 4, "one item per operation");
        assert_eq!(out_slots[0].count, 1);
    }

    #[test]
    fn energy_is_clamped_into_the_buffer() {
        assert_eq!(clamp_energy(-5, RECYCLER_CAPACITY), 0);
        assert_eq!(clamp_energy(25_000, RECYCLER_CAPACITY), 20_000);
        assert_eq!(clamp_energy(4_321, RECYCLER_CAPACITY), 4_321);
        assert_eq!(clamp_energy(4_321, 0), 0);
    }

    #[test]
    fn saved_arrays_are_padded_and_clamped() {
        let factory = Factory::from_saved(&[3, -1], &[7], 25_000);
        assert_eq!(factory.progress, [3, 0, 0], "a short array pads and clamps");
        assert_eq!(factory.cycles, [7, 0, 0]);
        assert_eq!(factory.energy, RECYCLER_CAPACITY, "the buffer caps the int");
        let round_trip = Factory::from_saved(
            &factory.progress_saved(),
            &factory.cycles_saved(),
            factory.energy,
        );
        assert_eq!(round_trip, factory);
    }
}

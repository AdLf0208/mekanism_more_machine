//! Mekanism: MoreMachine for Horizon — the Recycling Factory's four tiers.
//!
//! One flat SDK package (mod API 0.2): the carried blocks, their items, models,
//! textures, lang, loot tables, creative tab and tags are the `assets/` and
//! `data/` here, and the behaviour is the one `impl Hooks` below. The rules
//! themselves live in [`runtime`]; [`recycling_factory`] is the glue.
//!
//! What the retired bundle expressed as eight injected helpers plus an embedded
//! content file is here three shapes: each tier's block entity is a **Part**
//! (`recycling_factory::FactoryPart { tier }`, attached to the type declared for
//! that block), its grid container and its FE buffer are **declarations** on the
//! block key, and the commands are the typed `register_commands`/
//! `run_mod_command` pair. Its embedded-content helper has no successor: one
//! `.hmod` carries the content and the code, which is what that helper existed to
//! work around.
use mod_api::{
    self, Command, CommandArgumentValue, CommandSource, Flow, Hooks, LevelRef,
    RegisterCommandsEvent, Registrar, RegistrationContext, RunModCommandEvent, ServerHost, Text,
};

use recycling_factory::FactoryPart;
use runtime::Tier;

mod recycling_factory;

pub mod runtime;

/// Whether this command source may read the factory at `pos`: an operator
/// (permission ≥ 2), or a player in the same level within
/// [`runtime::INSPECT_RANGE`] of the block.
fn may_read(host: &dyn ServerHost, source: &CommandSource, level: LevelRef, pos: [i32; 3]) -> bool {
    let op = source.permission().level() >= runtime::LEVEL_ADMINS;
    match source {
        CommandSource::Player { player, .. } => {
            let same_level = player.level(host) == Some(level);
            let distance = player
                .entity()
                .position(host)
                .map(|at| {
                    let block = [
                        f64::from(pos[0]) + 0.5,
                        f64::from(pos[1]) + 0.5,
                        f64::from(pos[2]) + 0.5,
                    ];
                    (0..3)
                        .map(|axis| (at[axis] - block[axis]).powi(2))
                        .sum::<f64>()
                        .sqrt()
                })
                .unwrap_or(f64::MAX);
            runtime::may_inspect(op, same_level, distance)
        }
        _ => op,
    }
}

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Declare the four tiers: each is one block with a block entity, one input
    /// and one output per process, and its own input-only buffer.
    ///
    /// The retired bundle wrote the buffer into the `block_entity_energy` hook's
    /// shared output and had to be careful to leave a foreign block's entry
    /// alone. A declaration is keyed by the block, so that hazard cannot recur:
    /// another mod's block is another key.
    fn register(&self, registrar: &mut dyn Registrar, context: &RegistrationContext) {
        recycling_factory::remember_scrap(registrar);
        for tier in Tier::ALL {
            let block = registrar.block_key(tier.block());
            registrar.block_container(&block, recycling_factory::container(tier));
            registrar.block_energy(&block, recycling_factory::battery(tier));
            let kind = recycling_factory::declare(registrar, tier);
            if let Err(error) = registrar.block_entity_part(&kind, FactoryPart { tier }) {
                context.refuse(&error.to_string());
            }
        }
    }

    /// `/mekmm info|energy|chance|progress`: the harness controls the in-game
    /// check uses, since the slice has no GUI of its own.
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        event
            .commands
            .push(Command::new("mekmm info <pos:block_pos>"));
        event.commands.push(Command::op(
            "mekmm energy <amount:integer> <pos:block_pos>",
            2,
        ));
        event.commands.push(Command::op(
            "mekmm chance <forced:integer> <pos:block_pos>",
            2,
        ));
        event
            .commands
            .push(Command::new("mekmm progress <pos:block_pos>"));
    }

    /// The command bodies. The arguments are parsed and resolved by the server's
    /// dispatcher, so this only reads the block entity and answers; a factory
    /// that is not there answers "No block entity there." as it did in 0.1.0.
    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        let literals: Vec<&str> = event.command.literals.iter().map(String::as_str).collect();
        let Some(CommandArgumentValue::BlockPos(pos)) = event.command.argument("pos") else {
            return Flow::Continue;
        };
        let line = match literals.as_slice() {
            ["mekmm", "info"] => {
                let Some(level) = host.level(host.dimension()) else {
                    return Flow::Continue;
                };
                if !may_read(&*host, event.source, level, *pos) {
                    let line = format!(
                        "You must be an operator or within {:.0} blocks of the factory.",
                        runtime::INSPECT_RANGE
                    );
                    let _ = host.commands().feedback(Text::literal(line));
                    return Flow::Cancel;
                }
                recycling_factory::info_line(host, *pos)
                    .unwrap_or_else(|| "No block entity there.".to_owned())
            }
            ["mekmm", "energy"] => {
                let Some(CommandArgumentValue::Integer(amount)) = event.command.argument("amount")
                else {
                    return Flow::Continue;
                };
                match recycling_factory::set_energy(host, *pos, *amount) {
                    Some(after) => format!("Energy set to {after} FE."),
                    None => "No block entity there.".to_owned(),
                }
            }
            ["mekmm", "chance"] => {
                let Some(CommandArgumentValue::Integer(forced)) = event.command.argument("forced")
                else {
                    return Flow::Continue;
                };
                match recycling_factory::set_chance(host, *pos, *forced) {
                    Some(state) => format!("Chance is now {state}."),
                    None => "No block entity there.".to_owned(),
                }
            }
            ["mekmm", "progress"] => recycling_factory::progress_line(host, *pos)
                .unwrap_or_else(|| "No block entity there.".to_owned()),
            _ => return Flow::Continue,
        };
        let _ = host.commands().feedback(Text::literal(line));
        Flow::Cancel
    }
}

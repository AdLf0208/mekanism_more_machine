// One tick of the factory.
fn observe_block_entity_tick(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    entity: &mut horizon_client::containers::ModBlockEntity,
) {
    crate::game::mekmm_state::on_block_entity_tick(game, pos, entity);
}

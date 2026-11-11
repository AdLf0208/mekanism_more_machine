// The factory's six item slots and its 3x3 menu.
fn observe_block_entity_container(
    block: &str,
    container: &mut Option<horizon_client::containers::ModContainer>,
) {
    crate::game::mekmm_state::on_block_entity_container(block, container);
}

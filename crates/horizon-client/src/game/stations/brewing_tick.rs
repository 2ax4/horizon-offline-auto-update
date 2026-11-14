//! `BrewingStandBlockEntity.serverTick` after the model tick: the native
//! hooks, `setChanged`, the bottle bits, the drops and sleeping. Kept out of
//! `stations.rs`.
use super::super::Game;
use horizon_client::{Result, brewing::BrewTick, containers::BlockEntity};

impl Game<'_> {
    pub(super) fn finish_brewing_tick(
        &mut self,
        pos: (i32, i32, i32),
        was_idle: bool,
        fuel_item: Option<std::sync::Arc<str>>,
        out: BrewTick,
    ) -> Result<()> {
        let (started, ingredient) = match self.world.block_entities.get(&pos) {
            Some(BlockEntity::Brewing(b)) => (
                was_idle && b.brew_time > 0,
                (was_idle && b.brew_time > 0)
                    .then(|| b.ingredient.clone())
                    .flatten(),
            ),
            _ => (false, None),
        };
        if out.fuel_taken
            && let Some(item) = fuel_item
        {
            let (x, y, z) = pos;
            self.native_brewing_fuel([x, y, z], &item);
        }
        if let Some(ingredient) = ingredient {
            let (x, y, z) = pos;
            self.native_brewing_start([x, y, z], &ingredient);
        }
        let out_changed = out != BrewTick::default();
        // `BrewingStandBlockEntity.serverTick`: setChanged when fuel was
        // taken, while a brew runs, when one starts, and when the pots or a
        // dropped remainder changed. The block state only carries the bottle
        // bits, so most of these had no other mark.
        if !was_idle || started || out_changed {
            self.container_changed(pos);
        }
        let (x, y, z) = pos;
        // `doBrew` drops the ingredient's remainder through
        // `Containers.dropItemStack`, before the brew's level event: no
        // `block_drops` test and no pickup delay.
        for d in out.drops {
            self.drop_item_stack_containers([x, y, z], d);
        }
        if out.brewed {
            self.native_brewing_complete([x, y, z]);
            self.play_at("minecraft:block.brewing_stand.brew", [x, y, z], 1.0, 1.0);
        }
        if let Some(bottles) = out.bottles {
            let state = self.world.block(x, y, z);
            if let Some(mut s) = self.pack.assets.registry.state(state).cloned()
                && AsRef::<str>::as_ref(&s.name) == "minecraft:brewing_stand"
            {
                for (i, has) in bottles.iter().enumerate() {
                    s.properties
                        .insert(format!("has_bottle_{i}"), has.to_string());
                }
                if let Ok(id) = self.pack.assets.registry.resolve(&s) {
                    self.level_set_with_flags(
                        [x, y, z],
                        id,
                        horizon_world::level::flags::UPDATE_CLIENTS,
                    )?;
                }
            }
        }
        if matches!(self.world.block_entities.get(&pos), Some(BlockEntity::Brewing(b)) if b.idle())
        {
            self.world.block_entities.sleep(pos);
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "brewing_tick_tests.rs"]
mod brewing_tick_tests;

//! `MobEvent::Convert`: `Mob.convertTo` with `ConversionType.SINGLE` and the
//! `ConversionParams.AfterConversion` step the converting mob asked for.
use super::*;
use horizon_client::mobs::conversion;

impl Game<'_> {
    /// A mob turned into `kind` in its place (`conversion::single`), then
    /// `after`: `Pig.thunderHit`'s gear and persistence, `Villager.thunderHit`'s
    /// finalization, the nausea a zombifying piglin or hoglin leaves behind.
    pub(super) fn convert_event(
        &mut self,
        from: &mut Mob,
        kind: &str,
        params: conversion::Params,
        after: conversion::After,
    ) {
        let Some(id) = self.convert_to(from, kind, params, after.pre_checked()) else {
            return;
        };
        match after {
            // `AbstractPiglin.finishConversion` and `Hoglin.finishConversion`:
            // the new mob is nauseous for 10 seconds.
            conversion::After::Nausea => {
                if let Some(m) = self.mobs.iter_mut().find(|m| m.id == id) {
                    m.add_effect("minecraft:nausea", 200, 0);
                }
            }
            // `Pig.thunderHit`: `populateDefaultEquipmentSlots(this.getRandom(),
            // ..)` draws from the pig's own random, then `setPersistenceRequired`.
            conversion::After::ZombifiedPiglinGear => {
                let item = horizon_client::mobs::piglin::default_equipment(&mut from.random);
                if let Some(m) = self.mobs.iter_mut().find(|m| m.id == id) {
                    m.extra.set_main_hand(Some(item.into()));
                    m.persistent = true;
                }
            }
            // `Villager.thunderHit` and `Tadpole.ageUp`: the spawn finalization
            // of a conversion, then `setPersistenceRequired`.
            conversion::After::Witch | conversion::After::Finalize => {
                self.finalize_converted(id);
            }
            conversion::After::None => {}
        }
    }
    /// `MushroomCow.shear`'s `convertTo(COW, single(this, false, false))`: the
    /// cow takes the mooshroom's place with its name, flags, effects and
    /// persistence, and its gear stays behind.
    pub(super) fn convert_mooshroom(&mut self, from: &Mob) -> bool {
        self.convert_to(from, "minecraft:cow", conversion::Params::NONE, false)
            .is_some()
    }
}

//! The breeding parent's own random in `getBreedOffspring`: the offspring's
//! coat coin and a wolf pup's voice.
use super::super::Game;
use horizon_client::mobs::{Mob, Species};
use horizon_core::geometry::Vec3;
use horizon_core::random::RandomSource;

impl Game<'_> {
    /// The baby's variant: a parent's variant (pigs, cows, chickens and foxes
    /// take either parent's, unless the breeding parent's coin picked its own),
    /// or `Rabbit.getBreedOffspring`'s own draws. Called before the child
    /// exists, where vanilla draws it.
    pub(super) fn bred_offspring_variant(
        &mut self,
        parent: u32,
        p: &Mob,
        q: Option<&Mob>,
        coin: bool,
    ) -> u8 {
        let own = p.extra.variant;
        if p.species() == Species::Rabbit {
            // `partner instanceof Rabbit`: a partner of another species draws no coin.
            let mate = q
                .filter(|q| q.species() == Species::Rabbit)
                .map(|q| q.extra.variant);
            return self.bred_rabbit_variant(parent, p.position, own, mate);
        }
        if matches!(
            p.species(),
            Species::Fox | Species::Pig | Species::Cow | Species::Chicken
        ) && !coin
        {
            return q.map_or(own, |q| q.extra.variant);
        }
        own
    }
    /// `Rabbit.getBreedOffspring`: a fresh biome variant
    /// (`Rabbit.getRandomRabbitVariant`, from the level's random) one time in
    /// twenty, else a parent's colour by the breeding parent's own coin toss.
    fn bred_rabbit_variant(&mut self, parent: u32, at: Vec3, own: u8, mate: Option<u8>) -> u8 {
        let at = [at.x, at.y, at.z].map(|c| c.floor() as i32);
        let facts = self.biome_facts(at[0], at[1], at[2]);
        let fresh = horizon_client::mobs::rabbit::random_variant(
            facts.white_rabbit,
            facts.gold_rabbit,
            &mut self.world.random,
        );
        let Some(m) = self.mobs.iter_mut().find(|m| m.id == parent) else {
            return own;
        };
        if m.random.next_int_bound(20).unwrap_or(1) == 0 {
            return fresh;
        }
        // The coin is drawn only for a rabbit partner.
        match mate {
            Some(mate) if m.random.next_boolean() => mate,
            _ => own,
        }
    }
    /// Cow, Pig, Chicken, Fox, Cat and Wolf.getBreedOffspring: the offspring's
    /// variant coin is the breeding parent's own first draw, not the level's.
    /// (Mooshroom draws its own coin in getOffspringVariant.)
    pub(super) fn bred_coat_coin(&mut self, parent: u32, species: Species) -> bool {
        matches!(
            species,
            Species::Cow
                | Species::Pig
                | Species::Chicken
                | Species::Fox
                | Species::Cat
                | Species::Wolf
        ) && self
            .mobs
            .iter_mut()
            .find(|m| m.id == parent)
            .is_some_and(|m| m.random.next_boolean())
    }
    /// WolfSoundVariants.pickRandomSoundVariant: Registry.getRandom over the
    /// wolf voice registry, from the parent's own random, after the collar
    /// colour's mix.
    pub(super) fn pick_bred_wolf_voice(&mut self, parent: u32, child: u32) {
        let voices = horizon_client::mobs::finalize::sound_variants(Species::Wolf);
        let voice = self.mobs.iter_mut().find(|m| m.id == parent).and_then(|m| {
            let i = m.random.next_int_bound(voices.len() as i32).ok()?;
            Some(i as u8)
        });
        if let Some(c) = self.mobs.iter_mut().find(|m| m.id == child) {
            c.extra.sound_variant = voice;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;

    /// `Cow`, `Pig`, `Chicken`, `Fox`, `Cat` and `Wolf.getBreedOffspring` draw
    /// the offspring's coat from the breeding parent's own random, and only a
    /// wolf picks a voice there (`WolfSoundVariants.pickRandomSoundVariant`,
    /// that same random, after the collar colour's mix). The level's random
    /// answers neither: two runs that differ only in the level's seed must
    /// agree, and the parent's stream must read as vanilla's own draws in
    /// vanilla's order.
    #[test]
    #[ignore = "needs prepared resources (HORIZON_TEST_RESOURCES)"]
    fn a_pup_takes_its_coat_and_voice_from_the_parents_own_random() {
        use horizon_core::random::{LegacyRandom, RandomSource};
        // The voice registry, not a hardcoded count of wolf voices.
        let voices = horizon_client::mobs::finalize::sound_variants(Species::Wolf).len() as i32;
        assert!(voices > 1, "a wolf has several voices");
        let cases = [
            ("minecraft:wolf", 2u8, 8u8),
            ("minecraft:wolf", 6, 0),
            ("minecraft:cow", 1, 0),
            ("minecraft:pig", 2, 0),
            ("minecraft:chicken", 0, 1),
            ("minecraft:fox", 1, 0),
            ("minecraft:cat", 4, 0),
        ];
        let mut heard = std::collections::BTreeSet::new();
        // One server game: its resources load once, and only the level's seed
        // and the pair change between runs.
        let mut game = super::super::super::native_machines::tests::server_game();
        for (n, &(kind, own, mate)) in cases.iter().enumerate() {
            // Independent entity randoms, kept clear of the ids the spawner
            // hands out: the parent is `part`, the partner `mate_id`.
            let parent_seed = 4_242 + 1_009 * n as i64;
            let mut outcomes = Vec::new();
            for (r, level_seed) in [11i64, 2_000_003].into_iter().enumerate() {
                game.world.random =
                    horizon_core::random::LevelRandom::for_level(LegacyRandom::new(level_seed));
                game.mobs.clear();
                let (part, mate_id) = (7u32, 8u32);
                // Ids the spawner hands out stay clear of the pair's.
                game.next_entity = 1_000 + 10 * n as u32 + 2 * r as u32;
                let t = game
                    .res
                    .mob_types()
                    .get(kind)
                    .unwrap_or_else(|| panic!("a {kind}"))
                    .clone();
                for (id, variant) in [(part, own), (mate_id, mate)] {
                    let mut m = Mob::new(id, t.clone(), Vec3::new(8.5, 64.0, 8.5), 0.0);
                    m.random = horizon_core::random::EntityRandom::own(LegacyRandom::new(
                        parent_seed + i64::from(id),
                    ));
                    m.extra.variant = variant;
                    m.extra.in_love = 600;
                    game.mobs.push(m);
                }
                game.breed(part, mate_id, None);
                let child = game
                    .mobs
                    .iter()
                    .find(|m| m.id != part && m.id != mate_id)
                    .unwrap_or_else(|| panic!("{kind}: a pup"));
                assert_eq!(child.extra.age, -24000, "{kind}: a baby");
                // What the parent's own stream answers, draw for draw.
                let mut expected = LegacyRandom::new(parent_seed + i64::from(part));
                let coin = expected.next_boolean();
                let coat = if coin { own } else { mate };
                let voice = (kind == "minecraft:wolf").then(|| {
                    let i = expected.next_int_bound(voices).unwrap() as u8;
                    heard.insert(i);
                    i
                });
                assert_eq!(
                    child.extra.variant, coat,
                    "{kind}: the parent's coin picks the coat (level seed {level_seed})"
                );
                assert_eq!(
                    child.extra.sound_variant, voice,
                    "{kind}: the parent's next draw picks the voice (level seed {level_seed})"
                );
                // The stream ends where vanilla's does: the coin, a wolf's
                // voice, then finalizeSpawnChildFromBreeding's 1-to-7 roll.
                if game.ui.rules.bool("mob_drops") {
                    let _ = expected.next_int_bound(7);
                }
                let mut parent = game
                    .mobs
                    .iter()
                    .find(|m| m.id == part)
                    .unwrap()
                    .random
                    .clone();
                for _ in 0..4 {
                    assert_eq!(
                        parent.next_int(),
                        expected.next_int(),
                        "{kind}: the parent's stream goes on where vanilla leaves it"
                    );
                }
                outcomes.push((child.extra.variant, child.extra.sound_variant));
            }
            assert_eq!(
                outcomes[0], outcomes[1],
                "{kind}: the level's seed does not decide the pup"
            );
        }
        assert!(heard.len() > 1, "the wolf's voices vary: {heard:?}");
    }
}

//! What the player is good at. Every skill has its own experience, and
//! rises by being used; there are no classes.

/// The highest level a skill reaches.
pub const LEVEL_CAP: u8 = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Skill {
    Melee,
    Ranged,
    Magic,
    Defence,
    Vitality,
    Mining,
    Herbalism,
    Smithing,
    Alchemy,
}

impl Skill {
    pub const ALL: [Skill; 9] = [Skill::Melee, Skill::Ranged, Skill::Magic, Skill::Defence, Skill::Vitality, Skill::Mining, Skill::Herbalism, Skill::Smithing, Skill::Alchemy];

    /// Its name as the player reads it.
    pub const fn label(self) -> &'static str {
        match self {
            Skill::Melee => "Melee",
            Skill::Ranged => "Ranged",
            Skill::Magic => "Magic",
            Skill::Defence => "Defence",
            Skill::Vitality => "Vitality",
            Skill::Mining => "Mining",
            Skill::Herbalism => "Herbalism",
            Skill::Smithing => "Smithing",
            Skill::Alchemy => "Alchemy",
        }
    }

    /// What it is called in a save file. Never change one.
    pub const fn name(self) -> &'static str {
        match self {
            Skill::Melee => "melee",
            Skill::Ranged => "ranged",
            Skill::Magic => "magic",
            Skill::Defence => "defence",
            Skill::Vitality => "vitality",
            Skill::Mining => "mining",
            Skill::Herbalism => "herbalism",
            Skill::Smithing => "smithing",
            Skill::Alchemy => "alchemy",
        }
    }

    pub fn from_name(name: &str) -> Option<Skill> {
        Skill::ALL.into_iter().find(|s| s.name() == name)
    }

    const fn index(self) -> usize {
        self as usize
    }
}

/// The experience it takes, in all, to be `level`: nothing for level 1,
/// 28 for level 2, and each level a good deal more than the last, to
/// 135,436 for level 50. (Twenty a level, plus eight times the levels
/// gained to the power of two and a half.)
pub fn xp_for_level(level: u8) -> u32 {
    let n = level.clamp(1, LEVEL_CAP) as u64 - 1;
    (20 * n + 8 * n * n * (n * 10_000).isqrt() / 100) as u32
}

/// The level that much experience makes.
pub fn level_for_xp(xp: u32) -> u8 {
    (1..=LEVEL_CAP).rev().find(|&level| xp >= xp_for_level(level)).unwrap_or(1)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Skills {
    xp: [u32; Skill::ALL.len()],
}

impl Skills {
    pub fn xp(&self, skill: Skill) -> u32 {
        self.xp[skill.index()]
    }

    pub fn level(&self, skill: Skill) -> u8 {
        level_for_xp(self.xp(skill))
    }

    /// How far through its level a skill is, `0..1`; 1 at the cap.
    pub fn progress(&self, skill: Skill) -> f64 {
        let level = self.level(skill);
        if level >= LEVEL_CAP {
            return 1.0;
        }
        let (floor, next) = (xp_for_level(level), xp_for_level(level + 1));
        (self.xp(skill) - floor) as f64 / (next - floor) as f64
    }

    /// Every level added up: how far a character has come, in a number.
    pub fn total_level(&self) -> u32 {
        Skill::ALL.iter().map(|&skill| self.level(skill) as u32).sum()
    }

    /// Make a skill's experience exactly `xp`: for a character read
    /// back from its save.
    pub fn set_xp(&mut self, skill: Skill, xp: u32) {
        self.xp[skill.index()] = xp;
    }

    /// Gain experience. Returns the new level if that raised it.
    pub fn gain(&mut self, skill: Skill, amount: u32) -> Option<u8> {
        let before = self.level(skill);
        self.xp[skill.index()] = self.xp(skill).saturating_add(amount);
        Some(self.level(skill)).filter(|&after| after > before)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_curve_starts_quick_and_every_level_takes_more_than_the_last() {
        assert_eq!((xp_for_level(1), xp_for_level(2), xp_for_level(3)), (0, 28, 85));
        assert_eq!(xp_for_level(LEVEL_CAP), 135_436);
        let steps: Vec<u32> = (1..LEVEL_CAP).map(|l| xp_for_level(l + 1) - xp_for_level(l)).collect();
        assert!(steps.windows(2).all(|w| w[1] > w[0]), "{steps:?}");
        // Past the cap asks no more.
        assert_eq!(xp_for_level(200), xp_for_level(LEVEL_CAP));
    }

    #[test]
    fn levels_and_experience_agree_at_every_boundary() {
        for level in 1..=LEVEL_CAP {
            let at = xp_for_level(level);
            assert_eq!(level_for_xp(at), level);
            if level > 1 {
                assert_eq!(level_for_xp(at - 1), level - 1);
            }
        }
        assert_eq!(level_for_xp(u32::MAX), LEVEL_CAP);
    }

    #[test]
    fn gaining_reports_a_level_only_when_one_is_reached() {
        let mut skills = Skills::default();
        assert_eq!(skills.level(Skill::Mining), 1);
        assert_eq!(skills.gain(Skill::Mining, 27), None);
        assert_eq!(skills.gain(Skill::Mining, 1), Some(2));
        assert_eq!(skills.gain(Skill::Mining, 1), None);
        // Enough at once for several: the level reached is what is said.
        assert_eq!(skills.gain(Skill::Mining, 2000), Some(level_for_xp(2029)));
        // And only that skill moved.
        assert_eq!(skills.level(Skill::Smithing), 1);
        assert_eq!(skills.xp(Skill::Mining), 2029);
    }

    #[test]
    fn progress_runs_from_nothing_to_all_through_a_level() {
        let mut skills = Skills::default();
        assert_eq!(skills.progress(Skill::Mining), 0.0);
        skills.gain(Skill::Mining, 14);
        assert_eq!(skills.progress(Skill::Mining), 0.5);
        skills.gain(Skill::Mining, 14);
        assert_eq!(skills.progress(Skill::Mining), 0.0, "a new level starts empty");
        skills.gain(Skill::Mining, u32::MAX);
        assert_eq!((skills.level(Skill::Mining), skills.progress(Skill::Mining)), (LEVEL_CAP, 1.0));
    }

    #[test]
    fn every_skill_has_its_own_name() {
        for names in [Skill::ALL.map(Skill::label), Skill::ALL.map(Skill::name)] {
            let mut names = names.to_vec();
            names.sort();
            names.dedup();
            assert_eq!(names.len(), Skill::ALL.len());
        }
        for skill in Skill::ALL {
            assert_eq!(Skill::from_name(skill.name()), Some(skill));
        }
        assert_eq!(Skill::from_name("juggling"), None);
    }

    #[test]
    fn the_total_level_adds_every_skill() {
        let mut skills = Skills::default();
        assert_eq!(skills.total_level(), 9);
        skills.set_xp(Skill::Mining, xp_for_level(7));
        skills.set_xp(Skill::Alchemy, xp_for_level(3));
        assert_eq!(skills.total_level(), 9 + 6 + 2);
    }
}

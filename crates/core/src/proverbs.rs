//! Daily inspiration proverbs shown on the Today page.

use crate::DateKey;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Proverb {
    pub japanese: &'static str,
    pub reading: &'static str,
    pub translation: &'static str,
}

pub const DAILY_PROVERBS: [Proverb; 5] = [
    Proverb {
        japanese: "継続は力なり",
        reading: "けいぞくはちからなり",
        translation: "Continuity is power. (Perseverance pays off)",
    },
    Proverb {
        japanese: "一期一会",
        reading: "いちごいちえ",
        translation: "Once in a lifetime encounter. (Treasure every meeting)",
    },
    Proverb {
        japanese: "七転び八起き",
        reading: "ななころびやおき",
        translation: "Fall seven times, stand up eight.",
    },
    Proverb {
        japanese: "千里の道も一歩から",
        reading: "せんりのみちもいっぽから",
        translation: "A journey of a thousand miles begins with a single step.",
    },
    Proverb {
        japanese: "初心忘るべからず",
        reading: "しょしんわするべからず",
        translation: "Never forget your beginner's spirit.",
    },
];

/// The same proverb all day, changing at the next day.
pub fn proverb_for(day: DateKey) -> &'static Proverb {
    let seed = day.year() as usize * 372 + day.month() as usize * 31 + day.day() as usize;
    &DAILY_PROVERBS[seed % DAILY_PROVERBS.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proverb_changes_day_to_day() {
        let a: DateKey = "2026-11-11".parse().unwrap();
        assert_eq!(proverb_for(a), proverb_for(a));
        assert_ne!(proverb_for(a), proverb_for(a.next()));
    }
}

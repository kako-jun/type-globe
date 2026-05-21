//! Enemy table for the Hack RPG (#37).
//!
//! Phase 2 introduces a cosmetic enemy lookup keyed by ordinal within a
//! 10-encounter run. The actual encounter shape (regular at 1-4 / 6-9,
//! miniboss at 5, boss at 10) already lives on `ListeningRpgRun`; this
//! table just attaches a name + emoji + HP value to each beat so the
//! battle log and the listening UIs can say "🟢 Slime" instead of
//! "Encounter 1".
//!
//! The `hp` field is currently cosmetic — the v0.2.0 RPG has no failure
//! state ("失敗概念なし" per CLAUDE.md), so the UI never decrements it.
//! It's exposed so a follow-up "show enemy HP shrinking" effect can hook
//! in without re-shaping the table.

/// One row of the enemy table. `key` stays stable for save / i18n
/// purposes; `display` is what the player sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnemySpec {
    pub key: &'static str,
    pub display: &'static str,
    pub hp: u32,
}

/// Regular encounter pool. `enemy_for_ordinal` cycles through this slice
/// for the non-boss beats so a single 10-encounter run sees five distinct
/// silhouettes plus the miniboss/boss.
pub const REGULAR_ENEMIES: &[EnemySpec] = &[
    EnemySpec {
        key: "slime",
        display: "🟢 Slime",
        hp: 20,
    },
    EnemySpec {
        key: "goblin",
        display: "👹 Goblin",
        hp: 30,
    },
    EnemySpec {
        key: "bat",
        display: "🦇 Bat",
        hp: 15,
    },
    EnemySpec {
        key: "wolf",
        display: "🐺 Wolf",
        hp: 40,
    },
    EnemySpec {
        key: "skeleton",
        display: "💀 Skeleton",
        hp: 35,
    },
];

/// Miniboss slot (encounter 5).
pub const MINIBOSS_ENEMY: EnemySpec = EnemySpec {
    key: "centurion",
    display: "👾 Centurion",
    hp: 80,
};

/// Boss slot (encounter 10).
pub const BOSS_ENEMY: EnemySpec = EnemySpec {
    key: "dragon",
    display: "🐲 Final Dragon",
    hp: 150,
};

/// Resolve the enemy displayed for a given 1-indexed ordinal within a
/// `ListeningRpgRun`. The mapping matches `ListeningRpgRun::build`:
/// 1-4 / 6-9 → regular pool (cycled), 5 → miniboss, 10 → boss.
///
/// For `ordinal == 0` we fall back to the first regular enemy so the
/// function is total — callers should never pass 0 in practice, but
/// returning a sentinel beats panicking on an off-by-one elsewhere.
pub fn enemy_for_ordinal(ordinal: usize) -> &'static EnemySpec {
    match ordinal {
        5 => &MINIBOSS_ENEMY,
        10 => &BOSS_ENEMY,
        0 => &REGULAR_ENEMIES[0],
        n => &REGULAR_ENEMIES[(n - 1) % REGULAR_ENEMIES.len()],
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinal_five_is_miniboss() {
        assert_eq!(enemy_for_ordinal(5).key, "centurion");
        assert_eq!(enemy_for_ordinal(5).display, "👾 Centurion");
    }

    #[test]
    fn ordinal_ten_is_boss() {
        assert_eq!(enemy_for_ordinal(10).key, "dragon");
        assert_eq!(enemy_for_ordinal(10).display, "🐲 Final Dragon");
    }

    #[test]
    fn regular_ordinals_resolve_to_regular_pool() {
        for ordinal in [1usize, 2, 3, 4, 6, 7, 8, 9] {
            let enemy = enemy_for_ordinal(ordinal);
            assert!(
                REGULAR_ENEMIES.iter().any(|e| e.key == enemy.key),
                "ordinal {ordinal} produced {} which is not in REGULAR_ENEMIES",
                enemy.key
            );
        }
    }

    #[test]
    fn regular_pool_is_cycled_by_ordinal() {
        // Ordinals 1..=4 hit pool index 0..=3; ordinal 6 wraps back to 0.
        assert_eq!(enemy_for_ordinal(1).key, REGULAR_ENEMIES[0].key);
        assert_eq!(enemy_for_ordinal(2).key, REGULAR_ENEMIES[1].key);
        assert_eq!(enemy_for_ordinal(3).key, REGULAR_ENEMIES[2].key);
        assert_eq!(enemy_for_ordinal(4).key, REGULAR_ENEMIES[3].key);
        // Ordinal 6 = index (6-1) % 5 = 0
        assert_eq!(enemy_for_ordinal(6).key, REGULAR_ENEMIES[0].key);
        assert_eq!(enemy_for_ordinal(7).key, REGULAR_ENEMIES[1].key);
        assert_eq!(enemy_for_ordinal(8).key, REGULAR_ENEMIES[2].key);
        assert_eq!(enemy_for_ordinal(9).key, REGULAR_ENEMIES[3].key);
    }

    #[test]
    fn ordinal_zero_falls_back_to_first_regular() {
        assert_eq!(enemy_for_ordinal(0).key, REGULAR_ENEMIES[0].key);
    }

    #[test]
    fn enemy_displays_include_an_emoji_prefix() {
        for spec in REGULAR_ENEMIES {
            // A loose check: every regular display starts with a
            // non-ASCII char (the emoji). Catches accidental "Slime"
            // edits without locking the exact glyph.
            assert!(
                !spec.display.chars().next().unwrap().is_ascii(),
                "{} display should start with an emoji",
                spec.key
            );
        }
        assert!(!MINIBOSS_ENEMY.display.chars().next().unwrap().is_ascii());
        assert!(!BOSS_ENEMY.display.chars().next().unwrap().is_ascii());
    }

    #[test]
    fn keys_are_unique_across_all_enemies() {
        let mut keys: Vec<&str> = REGULAR_ENEMIES.iter().map(|e| e.key).collect();
        keys.push(MINIBOSS_ENEMY.key);
        keys.push(BOSS_ENEMY.key);
        let before = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), before, "all enemy keys must be unique");
    }
}

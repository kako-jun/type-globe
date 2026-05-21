//! Title / rank table for the Hack RPG (#35).
//!
//! Phase 2 hangs cosmetic titles off `RpgStats.level`. The table is a
//! flat `&'static` slice on purpose:
//!
//! - the entries are intentionally few (6 ranks across Lv 1–50) so a
//!   linear scan in `newly_unlocked_titles` is trivially fast,
//! - keeping it as plain consts (no `lazy_static`) lets the test suite
//!   exercise it without any runtime setup,
//! - the table doubles as a sort of i18n anchor — `key` stays stable
//!   across UI rewrites, `display` is what the player sees.
//!
//! The persistence side (`RpgStats.titles_unlocked: Vec<String>`) stores
//! the `key`. UIs / battle-log lines pick up the `display` via
//! `TITLE_TABLE` when they need to render.

/// One row of the title table. See module docs for the role of `key` vs
/// `display`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TitleSpec {
    pub level: u32,
    pub key: &'static str,
    pub display: &'static str,
}

/// Canonical title table. Sorted by `level` ascending — `newly_unlocked_titles`
/// relies on the scan order to emit unlocks in chronological progression.
pub const TITLE_TABLE: &[TitleSpec] = &[
    TitleSpec {
        level: 2,
        key: "apprentice",
        display: "Apprentice",
    },
    TitleSpec {
        level: 5,
        key: "veteran",
        display: "Veteran",
    },
    TitleSpec {
        level: 10,
        key: "champion",
        display: "Champion",
    },
    TitleSpec {
        level: 20,
        key: "master",
        display: "Master",
    },
    TitleSpec {
        level: 30,
        key: "grandmaster",
        display: "Grandmaster",
    },
    TitleSpec {
        level: 50,
        key: "legend",
        display: "Legend",
    },
];

/// A title newly unlocked by a level-up. `key` is what callers push into
/// `RpgStats.titles_unlocked`; `display` is what UIs render in the battle
/// log (`🏆 Title unlocked: {display}`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnlockedTitle {
    pub key: &'static str,
    pub display: &'static str,
}

/// Compute the titles newly available at `reached_level`, excluding any
/// the player has already unlocked (`current` holds the keys, matched by
/// case-sensitive string equality). Returned in `TITLE_TABLE` order,
/// which is monotonic in `level`.
///
/// Pure — no allocation beyond the returned `Vec`. `current` is taken by
/// slice to avoid forcing callers to clone their `titles_unlocked`.
pub fn newly_unlocked_titles(reached_level: u32, current: &[String]) -> Vec<UnlockedTitle> {
    TITLE_TABLE
        .iter()
        .filter(|t| t.level <= reached_level && !current.iter().any(|c| c == t.key))
        .map(|t| UnlockedTitle {
            key: t.key,
            display: t.display,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_one_unlocks_nothing() {
        let unlocked = newly_unlocked_titles(1, &[]);
        assert!(unlocked.is_empty());
    }

    #[test]
    fn level_two_unlocks_apprentice_only() {
        let unlocked = newly_unlocked_titles(2, &[]);
        assert_eq!(unlocked.len(), 1);
        assert_eq!(unlocked[0].key, "apprentice");
        assert_eq!(unlocked[0].display, "Apprentice");
    }

    #[test]
    fn level_four_still_only_unlocks_apprentice() {
        // 4 is past Lv 2 (Apprentice) but not Lv 5 (Veteran).
        let unlocked = newly_unlocked_titles(4, &[]);
        assert_eq!(unlocked.len(), 1);
        assert_eq!(unlocked[0].key, "apprentice");
    }

    #[test]
    fn level_five_unlocks_apprentice_and_veteran_when_no_current() {
        let unlocked = newly_unlocked_titles(5, &[]);
        let keys: Vec<&str> = unlocked.iter().map(|t| t.key).collect();
        assert_eq!(keys, vec!["apprentice", "veteran"]);
    }

    #[test]
    fn existing_titles_are_not_re_emitted() {
        let current = vec!["apprentice".to_string()];
        let unlocked = newly_unlocked_titles(5, &current);
        // Only Veteran should be unlocked — Apprentice is filtered out.
        assert_eq!(unlocked.len(), 1);
        assert_eq!(unlocked[0].key, "veteran");
    }

    #[test]
    fn level_fifty_unlocks_full_chain_when_starting_fresh() {
        let unlocked = newly_unlocked_titles(50, &[]);
        let keys: Vec<&str> = unlocked.iter().map(|t| t.key).collect();
        assert_eq!(
            keys,
            vec![
                "apprentice",
                "veteran",
                "champion",
                "master",
                "grandmaster",
                "legend"
            ]
        );
    }

    #[test]
    fn level_far_above_table_caps_at_legend() {
        let unlocked = newly_unlocked_titles(9_999, &[]);
        // Same as Lv 50 — table has no entries past Lv 50.
        assert_eq!(unlocked.len(), TITLE_TABLE.len());
        assert_eq!(unlocked.last().unwrap().key, "legend");
    }

    #[test]
    fn table_is_sorted_by_level_ascending() {
        let levels: Vec<u32> = TITLE_TABLE.iter().map(|t| t.level).collect();
        let mut sorted = levels.clone();
        sorted.sort();
        assert_eq!(levels, sorted, "TITLE_TABLE must be sorted by level");
    }

    #[test]
    fn keys_are_unique() {
        let mut keys: Vec<&str> = TITLE_TABLE.iter().map(|t| t.key).collect();
        let before = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), before, "TITLE_TABLE keys must be unique");
    }
}

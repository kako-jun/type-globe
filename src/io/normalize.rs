//! Romaji canonicalization for permissive input matching (#96).
//!
//! Both ja_typings candidates and user input are mapped to the same
//! canonical form before comparison, so a player typing `shi`, `chi`,
//! `tsu`, `wo`, etc. is accepted as equivalent to `si`, `ti`, `tu`, `o`
//! without requiring every variant to be enumerated in question data.

pub fn canonical_romaji(s: &str) -> String {
    let mut out = s.to_lowercase();
    // 長いパターンから順に。同一の対象文字列に複数ルールが当たらないように。
    out = out.replace("texi", "thi");
    out = out.replace("teli", "thi");
    out = out.replace("dji", "di");
    out = out.replace("dzu", "du");
    out = out.replace("tsu", "tu");
    out = out.replace("shi", "si");
    out = out.replace("chi", "ti");
    out = out.replace("fu", "hu");
    out = out.replace("ji", "zi");
    out = out.replace("wo", "o");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hepburn_collapses_to_kunrei() {
        assert_eq!(canonical_romaji("shi"), "si");
        assert_eq!(canonical_romaji("chi"), "ti");
        assert_eq!(canonical_romaji("tsu"), "tu");
        assert_eq!(canonical_romaji("fu"), "hu");
        assert_eq!(canonical_romaji("ji"), "zi");
    }

    #[test]
    fn wo_collapses_to_o() {
        assert_eq!(canonical_romaji("wo"), "o");
        assert_eq!(canonical_romaji("kawawo"), "kawao");
    }

    #[test]
    fn foreign_thi_aliases() {
        assert_eq!(canonical_romaji("thi"), "thi");
        assert_eq!(canonical_romaji("texi"), "thi");
        assert_eq!(canonical_romaji("teli"), "thi");
    }

    #[test]
    fn di_dzu_aliases() {
        assert_eq!(canonical_romaji("dji"), "di");
        assert_eq!(canonical_romaji("dzu"), "du");
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(canonical_romaji("SHI"), "si");
        assert_eq!(canonical_romaji("Chi"), "ti");
    }

    #[test]
    fn idempotent() {
        let once = canonical_romaji("hashi tsuru chiba");
        let twice = canonical_romaji(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn hepburn_and_kunrei_collide() {
        // Issue #96 の意図: shi も si も同じ正準形になる
        assert_eq!(canonical_romaji("hashi"), canonical_romaji("hasi"));
        assert_eq!(canonical_romaji("tokyo"), canonical_romaji("tokyo")); // 自明
        assert_eq!(canonical_romaji("tsugi"), canonical_romaji("tugi"));
    }

    #[test]
    fn preserves_unrelated_characters() {
        assert_eq!(canonical_romaji("tokyo"), "tokyo");
        assert_eq!(canonical_romaji("nagoya"), "nagoya");
        assert_eq!(canonical_romaji("123 test"), "123 test");
    }
}

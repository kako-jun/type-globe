//! Romaji canonicalization for permissive input matching (#96).
//!
//! Both ja_typings candidates and user input are mapped to the same
//! canonical form before comparison, so a player typing `shi`, `chi`,
//! `tsu`, `wo`, etc. is accepted as equivalent to `si`, `ti`, `tu`, `o`
//! without requiring every variant to be enumerated in question data.
//! The long-vowel mark `ー` typed as `-` after a vowel is collapsed too,
//! since question data registers long vowels in collapsed form (#93).

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
    // 長音符 `ー` を `-` で打った場合の正規化 (#93)。
    // 母音直後の `-` は `ー` と同じ意図なので、データが長音を畳んで
    // 登録している (例: サーバー → `saba`) 前提に合わせて落とす。
    // 子音直後や行頭の `-` は実在しない綴りなのでそのまま残す
    // (mistype 判定で弾かれる)。
    out = collapse_long_vowel_dash(&out);
    out
}

fn collapse_long_vowel_dash(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c == '-' {
            if let Some(prev) = out.chars().last() {
                if matches!(prev, 'a' | 'i' | 'u' | 'e' | 'o') {
                    continue;
                }
            }
        }
        out.push(c);
    }
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
        assert_eq!(canonical_romaji("tsugi"), canonical_romaji("tugi"));
    }

    #[test]
    fn rules_apply_in_sequence_without_clobbering_each_other() {
        // 同一文字列内で複数ルールが連続適用されることのリグレッションガード。
        // 置換順序が壊れると tsushi → tushi のように片方しか潰れなくなる。
        assert_eq!(canonical_romaji("tsushi"), "tusi");
        assert_eq!(canonical_romaji("jishi"), "zisi");
        assert_eq!(canonical_romaji("chitsu"), "titu");
    }

    #[test]
    fn preserves_unrelated_characters() {
        assert_eq!(canonical_romaji("tokyo"), "tokyo");
        assert_eq!(canonical_romaji("nagoya"), "nagoya");
        assert_eq!(canonical_romaji("123 test"), "123 test");
    }

    #[test]
    fn long_vowel_dash_after_vowel_drops() {
        // Issue #93: サーバー を `sa-` `sa-ba-` で打てるようにする。
        assert_eq!(canonical_romaji("sa-"), "sa");
        assert_eq!(canonical_romaji("sa-ba-"), "saba");
        assert_eq!(canonical_romaji("ko-hi-"), "kohi");
        // 連続した長音もすべて落ちる。
        assert_eq!(canonical_romaji("a--"), "a");
    }

    #[test]
    fn dash_without_preceding_vowel_is_kept() {
        // 子音直後・行頭の `-` は長音意図ではないので残す
        // (ヘボン式に存在しない綴りなので prefix 判定で弾かれる)。
        assert_eq!(canonical_romaji("-foo"), "-foo");
        assert_eq!(canonical_romaji("k-"), "k-");
    }

    #[test]
    fn long_vowel_dash_combines_with_hepburn_collapse() {
        // 既存ルールと組み合わせても破綻しないこと。
        // shi → si のあと i- が i に潰れる。
        assert_eq!(canonical_romaji("shi-"), "si");
        assert_eq!(canonical_romaji("chi-zu"), "tizu");
        // wo → o のあと o- が o に潰れる (ウォー の入力ケース)。
        assert_eq!(canonical_romaji("wo-"), "o");
    }
}

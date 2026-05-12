//! Romaji canonicalization for permissive input matching (#96).
//!
//! Both ja_typings candidates and user input are mapped to the same
//! canonical form before comparison, so a player typing `shi`, `chi`,
//! `tsu`, `wo`, etc. is accepted as equivalent to `si`, `ti`, `tu`, `o`
//! without requiring every variant to be enumerated in question data.
//! Katakana ー は `-` キー入力が必須 (IME 仕様)。データ側も `bo-ru`
//! 形で登録するため、ここでは `-` を畳まず素通しする (#93)。
//! ン+子音/末尾 の `nn` は IME で許容される冗長入力なので canonical
//! では `n` に畳む。ただし `nn` の直後が母音 / `y` / `n` のときは
//! ナ行・ヤ行と区別するために残す (例: かんおん = `kannon`)。

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
    out = collapse_redundant_nn(&out);
    out
}

/// ン+子音/末尾 の `nn` を `n` に畳む。`nn` の直後が母音 / `y` / `n`
/// のときは「ン+母音」と「ナ行+母音」を区別するために残す。
fn collapse_redundant_nn(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        if i + 1 < chars.len() && chars[i] == 'n' && chars[i + 1] == 'n' {
            let keep = matches!(
                chars.get(i + 2),
                Some('a' | 'i' | 'u' | 'e' | 'o' | 'y' | 'n')
            );
            if keep {
                out.push('n');
                out.push('n');
            } else {
                out.push('n');
            }
            i += 2;
        } else {
            out.push(chars[i]);
            i += 1;
        }
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
    fn dash_is_passed_through_for_katakana_long_vowel() {
        // Issue #93: カタカナ ー は IME 入力で `-` キー必須なので、
        // canonical_romaji は `-` を畳まず素通しする。データ側に
        // `bo-ru` 形で登録されているので、入力もそのまま比較される。
        assert_eq!(canonical_romaji("bo-ru"), "bo-ru");
        assert_eq!(canonical_romaji("sa-ba-"), "sa-ba-");
        // hepburn 書き換えは適用される。
        assert_eq!(canonical_romaji("shi-zu"), "si-zu");
    }

    #[test]
    fn nn_before_consonant_collapses_to_n() {
        // ン+子音 は IME 入力で `n` でも `nn` でも通る。データは `n` で
        // 登録するので、`nn` を `n` に畳んで比較する。
        assert_eq!(canonical_romaji("doragonnbo-ru"), "doragonbo-ru");
        assert_eq!(canonical_romaji("shinnbashi"), "sinbasi");
        assert_eq!(canonical_romaji("tennpura"), "tenpura");
    }

    #[test]
    fn nn_at_end_collapses_to_n() {
        // ン 末尾も同様に `nn` → `n`。
        assert_eq!(canonical_romaji("doragonn"), "doragon");
        assert_eq!(canonical_romaji("rapann"), "rapan");
    }

    #[test]
    fn nn_before_vowel_or_y_is_kept() {
        // ン+母音/y は `nn` を残さないと ナ行/ヤ行 と区別できない。
        // 例: かんおん (観音) = `kannon` であり `kanon` (カノン) と別物。
        assert_eq!(canonical_romaji("kannon"), "kannon");
        assert_eq!(canonical_romaji("tennen"), "tennen");
        assert_eq!(canonical_romaji("honnyaku"), "honnyaku");
    }

}

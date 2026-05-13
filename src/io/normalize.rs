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
    // IME で全角記号を出す ASCII キーを剥がす。データ側は記号を含めない
    // 慣行 (`・` `、` `。` をローマ字に書き起こさない) なので、打鍵時に
    // 対応キー (`/`,`、` で `,`、`。` で `.`) を押しても押さなくても受理する。
    out = out.replace('/', "");
    out = out.replace(',', "");
    out = out.replace('.', "");
    // 非標準 digraph の IME 別経路を標準形へ寄せる。`vu` 系・`fu` 系は
    // `fu→hu` より先に適用しないと `fuxi → huxi → fi` の連鎖が途切れる
    // ため、4文字以上のパターンをここで先に処理する。
    out = out.replace("vuxa", "va");
    out = out.replace("vula", "va");
    out = out.replace("vuxi", "vi");
    out = out.replace("vuli", "vi");
    out = out.replace("vuxe", "ve");
    out = out.replace("vule", "ve");
    out = out.replace("vuxo", "vo");
    out = out.replace("vulo", "vo");
    out = out.replace("huxa", "fa");
    out = out.replace("hula", "fa");
    out = out.replace("huxi", "fi");
    out = out.replace("huli", "fi");
    out = out.replace("huxe", "fe");
    out = out.replace("hule", "fe");
    out = out.replace("huxo", "fo");
    out = out.replace("hulo", "fo");
    out = out.replace("fuxa", "fa");
    out = out.replace("fula", "fa");
    out = out.replace("fuxi", "fi");
    out = out.replace("fuli", "fi");
    out = out.replace("fuxe", "fe");
    out = out.replace("fule", "fe");
    out = out.replace("fuxo", "fo");
    out = out.replace("fulo", "fo");
    // 長いパターンから順に。同一の対象文字列に複数ルールが当たらないように。
    out = out.replace("texi", "thi");
    out = out.replace("teli", "thi");
    out = out.replace("dexi", "di");
    out = out.replace("deli", "di");
    out = out.replace("dhi", "di");
    out = out.replace("dji", "di");
    out = out.replace("dzi", "di");
    out = out.replace("dzu", "du");
    out = out.replace("tsu", "tu");
    out = out.replace("shi", "si");
    out = out.replace("chi", "ti");
    // ウェ 系: `we/uxe/ule` を同一視。`uxa/ula → wa` は ウァ ≠ ワ なので
    // 除外。`uxo/ulo → wo` は ウォ も `wo` で書く慣行があるため同一視 OK。
    out = out.replace("uxi", "wi");
    out = out.replace("uli", "wi");
    out = out.replace("uxe", "we");
    out = out.replace("ule", "we");
    out = out.replace("uxo", "wo");
    out = out.replace("ulo", "wo");
    out = out.replace("fu", "hu");
    out = out.replace("ji", "zi");
    out = collapse_redundant_nn(&out);
    out
}

/// n の連続を文脈に応じて正準化する:
/// - 母音 / `y` の直前にある ≥2連 → `nn` に正規化 (ん + ナ行/ヤ行 を表現)。
///   例: `sennnorikyuu` (Wapuro 3連) ≡ `sennorikyuu` (Hepburn 2連) → どちらも
///   `sennorikyuu` に寄せる。これにより IME 経由で打鍵した人と Hepburn 表記の
///   候補が一致する。
/// - 子音 / 末尾 の直前にある ≥2連 → `n` に畳む (ん のみ)。
///   例: `tennpura` → `tenpura`、`doragonn` → `doragon`。
/// - 単独 `n` はそのまま (ナ行の頭子音 として次音節と結合)。
fn collapse_redundant_nn(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == 'n' {
            let start = i;
            while i < chars.len() && chars[i] == 'n' {
                i += 1;
            }
            let run = i - start;
            let next = chars.get(i).copied();
            if run == 1 {
                out.push('n');
            } else if matches!(next, Some('a' | 'i' | 'u' | 'e' | 'o' | 'y')) {
                out.push('n');
                out.push('n');
            } else {
                out.push('n');
            }
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
    fn wo_is_distinct_from_o() {
        // IME 入力で `wo` キーストロークは を を出すが、`o` 単独は お を出す。
        // 別文字なので canonical でも別物に保つ。
        assert_eq!(canonical_romaji("wo"), "wo");
        assert_eq!(canonical_romaji("kawawo"), "kawawo");
        assert_ne!(canonical_romaji("kawawo"), canonical_romaji("kawao"));
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
        // ディ / ぢ の Wapuro 系別綴り (Windows IME 既定など)
        assert_eq!(canonical_romaji("dhi"), "di");
        assert_eq!(canonical_romaji("dexi"), "di");
        assert_eq!(canonical_romaji("deli"), "di");
        assert_eq!(canonical_romaji("dzi"), "di");
    }

    #[test]
    fn ime_punctuation_keys_are_stripped() {
        // ・ → `/`、、 → `,`、。 → `.`。データ側は記号を持たないので、
        // 打鍵側で押された記号は剥がして候補と整合させる。
        assert_eq!(canonical_romaji("sukuwea/enikkusu"), "sukuweaenikkusu");
        assert_eq!(canonical_romaji("hello,world"), "helloworld");
        assert_eq!(canonical_romaji("nodejs.org"), "nodejsorg");
    }

    #[test]
    fn ime_alt_digraphs_wo_we_wi() {
        // ウェ / ウィ / ウォ の IME 別経路 (`uxe/ule`, `uxi/uli`, `uxo/ulo`)
        // を標準形 (`we/wi/wo`) へ寄せる。`uxa/ula → wa` は ウァ ≠ ワ
        // なので意図的に除外している。
        assert_eq!(canonical_romaji("sukuuxea"), canonical_romaji("sukuwea"));
        assert_eq!(canonical_romaji("sukuulea"), canonical_romaji("sukuwea"));
        assert_eq!(canonical_romaji("uxisuki-"), canonical_romaji("wisuki-"));
        assert_eq!(
            canonical_romaji("uxoruhuredo"),
            canonical_romaji("woruhuredo")
        );
    }

    #[test]
    fn ime_alt_digraphs_v_and_f() {
        // ヴ / フ 系の小文字-合成入力を標準形に統一。
        assert_eq!(canonical_romaji("vuxaiorin"), canonical_romaji("vaiorin"));
        assert_eq!(canonical_romaji("vulaiorin"), canonical_romaji("vaiorin"));
        assert_eq!(canonical_romaji("vuxeneto"), canonical_romaji("veneto"));
        assert_eq!(canonical_romaji("huxiripin"), canonical_romaji("firipin"));
        assert_eq!(canonical_romaji("huliripin"), canonical_romaji("firipin"));
        assert_eq!(canonical_romaji("fuxiripin"), canonical_romaji("firipin"));
        assert_eq!(canonical_romaji("fuliripin"), canonical_romaji("firipin"));
        assert_eq!(
            canonical_romaji("huxeruma-ta"),
            canonical_romaji("feruma-ta")
        );
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

    #[test]
    fn triple_n_before_vowel_collapses_to_double() {
        // Hepburn は ん+ナ行 を `nn` 連で書き、IME-wapuro は `nnn` (ん用 `nn`
        // + ナ行の n) で打つ。両方の流儀を受け入れるため 3 連以上の n は
        // 2 連へ正規化する。例: せんのりきゅう = `sennorikyuu` (Hepburn)
        // ≡ `sennnorikyuu` (IME-wapuro)。
        assert_eq!(canonical_romaji("sennnorikyuu"), "sennorikyuu");
        assert_eq!(canonical_romaji("gomennnasai"), "gomennasai");
        // 3 連 n が子音 / 末尾 の前にあれば 1 連 (ん) に畳む。
        assert_eq!(canonical_romaji("tennnpura"), "tenpura");
        assert_eq!(canonical_romaji("doragonnn"), "doragon");
    }
}

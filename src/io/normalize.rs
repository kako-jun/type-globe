//! Romaji canonicalization for permissive input matching (#96).
//!
//! Both ja_typings candidates and user input are mapped to the same
//! canonical form before comparison, so a player typing `shi`, `chi`,
//! `tsu`, etc. is accepted as equivalent to `si`, `ti`, `tu`
//! without requiring every variant to be enumerated in question data.
//! Katakana ー は `-` キー入力が必須 (IME 仕様)。データ側も `bo-ru`
//! 形で登録するため、ここでは `-` を畳まず素通しする (#93)。
//! ン+子音/末尾 の `nn` は IME で許容される冗長入力なので canonical
//! では `n` に畳む。ただし `nn` の直後が母音 / `y` / `n` のときは
//! ナ行・ヤ行と区別するために残す (例: かんおん = `kannon`)。

/// 注意: ルールの **適用順序が正しさに直結する** ため安易に並び替えない。
/// 依存関係:
/// - 4文字 digraph (`vuxa/huxa/fuxa/...`) は `fu → hu` より前に処理する
///   (`fuxa → fa` の連鎖を `fu` が先に潰さないため)。
/// - 拗音 Hepburn 形 (`sha/cha/ja`) は単音 (`shi/chi/ji`) より前に処理する
///   (`sha → si...` のような暴発を避けるため; `sha → sya` を先に当てる)。
/// - 単音 Hepburn (`shi/chi/ji`) は `ixya/ilya` (拗音 IME 別経路) より前に
///   処理する (`shixya → sixya → sya` の連鎖が成立する順序)。
/// - `apply_explicit_small_tsu` は子音二重化を伴うため、子音側のすべての
///   書き換えが済んだ後に実行する。
/// - `collapse_redundant_nn` は最後 (途中で挿入された n の連続にも対応する
///   ため)。
pub fn canonical_romaji(s: &str) -> String {
    let mut out = s.to_lowercase();
    // IME 記号キー (`/` = ・, `,` = 、, `.` = 。) は **commit トリガー** でも
    // あるので、直前の単独 `n` を `nn` に二重化する。記号自体は剥がさず
    // 保持する: そうしないと「`/` を打鍵できる位置が ・ の位置に限らず
    // どこでも通る」誤動作になる (例: `to/kyo` が `tokyo` と等価になる)。
    // データ側に `/` を含めて位置一致で判定する厳密仕様。
    for punct in ['/', ',', '.'] {
        let n_punct = format!("n{punct}");
        let nn_punct = format!("nn{punct}");
        let sentinel = format!("\x01{punct}");
        out = out.replace(&nn_punct, &sentinel); // protect already-doubled n
        out = out.replace(&n_punct, &nn_punct); // double single n + keep /
        out = out.replace(&sentinel, &nn_punct); // restore protected form
    }
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
    out = out.replace("dexi", "dhi");
    out = out.replace("deli", "dhi");
    out = out.replace("dji", "di");
    out = out.replace("dzi", "di");
    out = out.replace("dzu", "du");
    // 拗音 (きゃ/しゃ/ちゃ 等) の Hepburn 形を kunrei 形に寄せる:
    // `sha → sya`, `cha → tya`, `ja → zya` 等。`shi/chi/ji` 単音版より前に
    // 処理する (`shixya` の `shi` を `si` に潰す前に `sha` 系を捌くため)。
    out = out.replace("sha", "sya");
    out = out.replace("shu", "syu");
    out = out.replace("sho", "syo");
    out = out.replace("cha", "tya");
    out = out.replace("chu", "tyu");
    out = out.replace("cho", "tyo");
    out = out.replace("ja", "zya");
    out = out.replace("ju", "zyu");
    out = out.replace("jo", "zyo");
    out = out.replace("tsu", "tu");
    out = out.replace("shi", "si");
    out = out.replace("chi", "ti");
    out = out.replace("ji", "zi");
    // ウェ 系: `we/uxe/ule` を同一視。`uxa/ula → wa` は ウァ ≠ ワ なので
    // 除外。`uxo/ulo → wo` は ウォ も `wo` で書く慣行があるため同一視 OK。
    out = out.replace("uxi", "wi");
    out = out.replace("uli", "wi");
    out = out.replace("uxe", "we");
    out = out.replace("ule", "we");
    out = out.replace("uxo", "wo");
    out = out.replace("ulo", "wo");
    // 拗音の IME 別経路: `Cixya/Cilya` 等 (子音+i+小ゃ) を `Cya` に寄せる。
    // 例: `kixya` (=きゃ via 明示的小ゃ) ≡ `kya`。`shi/chi/ji` を先に
    // `si/ti/zi` に潰してから `ixya → ya` を当てると `shixya → sixya → sya`、
    // `chixya → tixya → tya`、`jixya → zixya → zya` の連鎖が成立する。
    out = out.replace("ixya", "ya");
    out = out.replace("ixyu", "yu");
    out = out.replace("ixyo", "yo");
    out = out.replace("ilya", "ya");
    out = out.replace("ilyu", "yu");
    out = out.replace("ilyo", "yo");
    out = out.replace("fu", "hu");
    // 促音 (っ) の IME 別経路: `ltu/xtu/ltsu/xtsu + 子音C → CC` に変換。
    // 例: `rokeltsuto` (明示的小っ) ≡ `roketto` (標準二重子音)。
    out = apply_explicit_small_tsu(&out);
    out = collapse_redundant_nn(&out);
    out
}

/// 促音 IME 別経路: `ltu/xtu/ltsu/xtsu` の直後に子音があれば、その子音を
/// 二重化した形に置換する。これにより `roke[ltu]to` → `roketto` のように
/// 標準形へ寄せられる。直後が母音 / 末尾 の場合は別意味になるので変換しない。
fn apply_explicit_small_tsu(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        let prefix_len = if i + 4 <= chars.len()
            && (chars[i] == 'l' || chars[i] == 'x')
            && chars[i + 1] == 't'
            && chars[i + 2] == 's'
            && chars[i + 3] == 'u'
        {
            Some(4)
        } else if i + 3 <= chars.len()
            && (chars[i] == 'l' || chars[i] == 'x')
            && chars[i + 1] == 't'
            && chars[i + 2] == 'u'
        {
            Some(3)
        } else {
            None
        };
        if let Some(plen) = prefix_len {
            if let Some(&next) = chars.get(i + plen) {
                if next.is_ascii_alphabetic() && !matches!(next, 'a' | 'i' | 'u' | 'e' | 'o') {
                    out.push(next);
                    i += plen;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// ン+子音/末尾 の `nn` を `n` に畳む。`nn` の直後が母音 / `y` / `n`
/// のときは「ン+母音」と「ナ行+母音」を区別するために残す。
///
/// 3 連以上の n は IME-wapuro と Hepburn の流儀差を表す (Hepburn `nn` ≡
/// Wapuro `nnn`) が、type-globe はタイピング練習として IME 流儀を「正」
/// と定義する。よってここでは正規化せず、2 連と 3 連は異なる正準形のまま
/// 残し、データ側を IME 正解形に揃える運用とする (`hiragana_to_hepburn` も
/// IME 流儀で出力)。
fn collapse_redundant_nn(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        if i + 1 < chars.len() && chars[i] == 'n' && chars[i + 1] == 'n' {
            // 直後が母音 / `y` / `n` (ナ行/ヤ行 と区別が必要) または IME
            // 記号 `/,.` (commit boundary なので `nn` が ん として確定する)
            // のときは `nn` を保持する。それ以外の子音/末尾は `n` に畳む。
            let keep = matches!(
                chars.get(i + 2),
                Some('a' | 'i' | 'u' | 'e' | 'o' | 'y' | 'n' | '/' | ',' | '.')
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
    fn di_and_dhi_stay_distinct() {
        assert_eq!(canonical_romaji("dji"), "di");
        assert_eq!(canonical_romaji("dzu"), "du");
        // IME では `di` は ぢ、`dhi` は ディ。混ぜない。
        assert_ne!(canonical_romaji("di"), canonical_romaji("dhi"));
        assert_eq!(canonical_romaji("dhi"), "dhi");
        assert_eq!(canonical_romaji("dexi"), "dhi");
        assert_eq!(canonical_romaji("deli"), "dhi");
        assert_eq!(canonical_romaji("dzi"), "di");
    }

    #[test]
    fn ime_punctuation_keys_preserved_with_n_commit() {
        // v0.7.1+: `/,.` は IME 記号として位置一致が必要 (剥がさない)。
        // ただし IME 上では commit トリガーでもあるので、直前の単独 `n` は
        // `nn` に二重化される (ん として確定する挙動を再現)。既に `nn` の
        // 場合は重複二重化しない。
        assert_eq!(canonical_romaji("burendan/aiku"), "burendann/aiku");
        assert_eq!(canonical_romaji("burendann/aiku"), "burendann/aiku");
        assert_eq!(canonical_romaji("sukuwea/enikkusu"), "sukuwea/enikkusu");
        // 記号自体は通常文字として canonical に残るため、位置不一致は弾く。
        assert_ne!(canonical_romaji("to/kyo"), canonical_romaji("tokyo"));
        // 末尾の n + / も同様。
        assert_eq!(canonical_romaji("don/a"), "donn/a");
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
    fn yoon_hepburn_kunrei_unified() {
        // しゃ・ちゃ・じゃ 系は Hepburn (sha/cha/ja) と Kunrei (sya/tya/zya)
        // を同一視。`shi/chi/ji` 単音版と整合。
        assert_eq!(canonical_romaji("sha"), canonical_romaji("sya"));
        assert_eq!(canonical_romaji("shu"), canonical_romaji("syu"));
        assert_eq!(canonical_romaji("sho"), canonical_romaji("syo"));
        assert_eq!(canonical_romaji("cha"), canonical_romaji("tya"));
        assert_eq!(canonical_romaji("cho"), canonical_romaji("tyo"));
        assert_eq!(canonical_romaji("ja"), canonical_romaji("zya"));
        assert_eq!(canonical_romaji("jo"), canonical_romaji("zyo"));
    }

    #[test]
    fn yoon_ime_alt_path_via_small_ya() {
        // `kixya/kilya` 等 (明示的小ゃ) を `kya` と同一視。
        assert_eq!(canonical_romaji("kixya"), canonical_romaji("kya"));
        assert_eq!(canonical_romaji("kilyu"), canonical_romaji("kyu"));
        assert_eq!(canonical_romaji("kixyo"), canonical_romaji("kyo"));
        // しゃ 系も Hepburn → Kunrei → 小ゃ展開 で統一される。
        assert_eq!(canonical_romaji("shixya"), canonical_romaji("sha"));
        assert_eq!(canonical_romaji("chixya"), canonical_romaji("cha"));
        assert_eq!(canonical_romaji("jixya"), canonical_romaji("ja"));
    }

    #[test]
    fn sokuon_ime_alt_path_via_explicit_small_tsu() {
        // 明示的小っ (`ltu/xtu/ltsu/xtsu` + 子音) を子音二重化 (`kk/tt/...`)
        // と同一視。例: `rokeltsuto` ≡ `roketto`。
        assert_eq!(canonical_romaji("rokeltsuto"), canonical_romaji("roketto"));
        assert_eq!(canonical_romaji("rokextsuto"), canonical_romaji("roketto"));
        assert_eq!(canonical_romaji("rokeltuto"), canonical_romaji("roketto"));
        assert_eq!(canonical_romaji("rokextuto"), canonical_romaji("roketto"));
        // っ + ち系: `matcha` (まっちゃ) は kunrei `mattya` (m+a+tt+ya) と同一。
        // `cha → tya` で `mat+cha → mat+tya = mattya`。
        assert_eq!(canonical_romaji("matcha"), "mattya");
        assert_eq!(canonical_romaji("maltsucha"), "mattya");
        // 直後が母音や末尾の `ltu/xtu` は変換しない (意味的に異なる)。
        assert_eq!(canonical_romaji("xtu"), "xtu");
    }

    #[test]
    fn trailing_n_run_preserves_count_for_prefix_match() {
        // `sennnorikyuu` を 1 文字ずつ打鍵中、`sennn` (途中5文字) は末尾に
        // 3連 n が来る。canonical はペア単位の collapse なので 3連目は
        // 単独 `n` として残り `sennn` が保たれる。これにより candidate
        // `sennnorikyuu` の prefix としてマッチする。
        assert_eq!(canonical_romaji("sennn"), "sennn");
        // 末尾 2連 n は通常通り `n` に畳まれる (ん 末尾の正準形)。
        assert_eq!(canonical_romaji("senn"), "sen");
        // 4連以上の n は末尾に来ると最後のペアが畳まれて 3連へ縮む
        // (`nn` ペア + `nn` ペア → 前ペアは次が `n` なので keep、後ペアは
        // 次が None なので collapse)。実プレイで 4連 n を打つことは無いが、
        // 仕様メモとして記録。
        assert_eq!(canonical_romaji("sennnn"), "sennn");
    }

    #[test]
    fn n_runs_preserve_ime_distinction() {
        // type-globe は IME 流儀を「正」と定義 (タイピング練習として)。
        // ん+ナ行 (3 連) と ん+母音 (2 連) は IME で異なる入力経路なので、
        // canonical でも別物に保つ。
        // - `sennnorikyuu` は せんのりきゅう (3 連 = ん + の)
        // - `sennorikyuu` は せんおりきゅう (2 連 = ん + お)
        assert_ne!(
            canonical_romaji("sennnorikyuu"),
            canonical_romaji("sennorikyuu")
        );
        assert_eq!(canonical_romaji("sennnorikyuu"), "sennnorikyuu");
    }
}

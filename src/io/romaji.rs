/// Convert Japanese kana text into lowercase Hepburn-style ASCII.
///
/// This is intentionally pragmatic rather than linguistically complete:
/// it targets type-globe's JA input mode (#69), where players need a
/// no-IME typing target for quiz/listening answers. The key project
/// rules are:
/// - ASCII only
/// - Hepburn spellings (`shi`, `chi`, `tsu`, `fu`, `ji`)
/// - no macrons
/// - long `o` collapsed (`とうきょう` -> `tokyo`, `おおさか` -> `osaka`)
/// - katakana ー is emitted as `-` (IME 入力で `-` キー必須に合わせる、
///   例: `ボール` -> `bo-ru`)
/// - `ん` always maps to `n`, even before `b/m/p`
#[allow(dead_code)]
pub fn hiragana_to_hepburn(input: &str) -> String {
    let raw = hiragana_to_hepburn_raw(input);
    collapse_long_o(&raw)
}

pub fn hiragana_to_hepburn_variants(input: &str) -> Vec<String> {
    let raw = hiragana_to_hepburn_raw(input);
    if raw.is_empty() {
        return Vec::new();
    }
    let collapsed = collapse_long_o(&raw);
    if raw == collapsed {
        vec![collapsed]
    } else {
        vec![collapsed, raw]
    }
}

fn hiragana_to_hepburn_raw(input: &str) -> String {
    let chars: Vec<char> = input.chars().map(normalize_kana).collect();
    let mut out = String::new();
    let mut i = 0usize;
    let mut geminate = false;

    while i < chars.len() {
        let c = chars[i];

        if is_space_like(c) {
            push_space(&mut out);
            geminate = false;
            i += 1;
            continue;
        }

        if c == 'ー' {
            // カタカナ長音は IME 入力で `-` キーが必須なので、ローマ字側
            // にも `-` を残す。データ側は `bo-ru` 形で登録されている前提。
            out.push('-');
            geminate = false;
            i += 1;
            continue;
        }

        if c == 'っ' {
            geminate = true;
            i += 1;
            continue;
        }

        if should_preserve_ascii_punctuation(c) {
            out.push(c);
            geminate = false;
            i += 1;
            continue;
        }

        if is_separator_like(c) {
            push_space(&mut out);
            geminate = false;
            i += 1;
            continue;
        }

        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            geminate = false;
            i += 1;
            continue;
        }

        // ん は IME-wapuro 仕様で出力する: 後続が母音 / ヤ行 / ナ行 のとき
        // `nn` で打たないと ん が消費されて次音節と結合してしまうため、
        // ローマ字側でも `nn` を出力する。それ以外 (子音 / 末尾) は `n` で
        // 充分 (IME は次の子音 or 確定キーで ん をコミットする)。
        if c == 'ん' {
            let next_needs_nn = matches!(
                chars.get(i + 1).copied(),
                Some(
                    'あ' | 'い'
                        | 'う'
                        | 'え'
                        | 'お'
                        | 'や'
                        | 'ゆ'
                        | 'よ'
                        | 'ゃ'
                        | 'ゅ'
                        | 'ょ'
                        | 'な'
                        | 'に'
                        | 'ぬ'
                        | 'ね'
                        | 'の'
                )
            );
            out.push_str(if next_needs_nn { "nn" } else { "n" });
            geminate = false;
            i += 1;
            continue;
        }

        if let Some((roman, consumed)) = romanize_chunk(&chars[i..]) {
            if geminate {
                if let Some(prefix) = geminate_prefix(roman) {
                    out.push(prefix);
                }
                geminate = false;
            }
            out.push_str(roman);
            i += consumed;
            continue;
        }

        // Drop unknown characters rather than smuggling non-ASCII
        // into the typing target.
        geminate = false;
        i += 1;
    }

    squash_spaces(&out)
}

fn normalize_kana(c: char) -> char {
    if ('ァ'..='ヶ').contains(&c) {
        char::from_u32(c as u32 - 0x60).unwrap_or(c)
    } else {
        c
    }
}

fn is_space_like(c: char) -> bool {
    c.is_whitespace() || c == '　'
}

fn is_separator_like(c: char) -> bool {
    matches!(
        c,
        '・' | '･'
            | '（'
            | '）'
            | '('
            | ')'
            | '['
            | ']'
            | '{'
            | '}'
            | '「'
            | '」'
            | '『'
            | '』'
            | '、'
            | '。'
            | ','
            | '.'
            | ':'
            | ';'
            | '!'
            | '?'
    )
}

fn should_preserve_ascii_punctuation(c: char) -> bool {
    matches!(c, '.' | '/')
}

fn push_space(out: &mut String) {
    if !out.is_empty() && !out.ends_with(' ') {
        out.push(' ');
    }
}

fn squash_spaces(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn collapse_long_o(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        let c = chars[i];
        out.push(c);
        if c == 'o' && i + 1 < chars.len() && matches!(chars[i + 1], 'o' | 'u') {
            i += 2;
            continue;
        }
        i += 1;
    }

    out
}

fn geminate_prefix(roman: &str) -> Option<char> {
    if roman.starts_with("ch") {
        Some('t')
    } else {
        roman
            .chars()
            .next()
            .filter(|c| c.is_ascii_alphabetic() && !matches!(c, 'a' | 'i' | 'u' | 'e' | 'o'))
    }
}

fn romanize_chunk(chars: &[char]) -> Option<(&'static str, usize)> {
    if chars.len() >= 2 {
        let pair = [chars[0], chars[1]];
        if let Some(roman) = romanize_pair(pair) {
            return Some((roman, 2));
        }
    }
    romanize_single(chars[0]).map(|roman| (roman, 1))
}

fn romanize_pair(pair: [char; 2]) -> Option<&'static str> {
    Some(match pair {
        ['き', 'ゃ'] => "kya",
        ['き', 'ゅ'] => "kyu",
        ['き', 'ょ'] => "kyo",
        ['ぎ', 'ゃ'] => "gya",
        ['ぎ', 'ゅ'] => "gyu",
        ['ぎ', 'ょ'] => "gyo",
        ['し', 'ゃ'] => "sha",
        ['し', 'ゅ'] => "shu",
        ['し', 'ょ'] => "sho",
        ['じ', 'ゃ'] => "ja",
        ['じ', 'ゅ'] => "ju",
        ['じ', 'ょ'] => "jo",
        ['ち', 'ゃ'] => "cha",
        ['ち', 'ゅ'] => "chu",
        ['ち', 'ょ'] => "cho",
        ['ぢ', 'ゃ'] => "ja",
        ['ぢ', 'ゅ'] => "ju",
        ['ぢ', 'ょ'] => "jo",
        ['に', 'ゃ'] => "nya",
        ['に', 'ゅ'] => "nyu",
        ['に', 'ょ'] => "nyo",
        ['ひ', 'ゃ'] => "hya",
        ['ひ', 'ゅ'] => "hyu",
        ['ひ', 'ょ'] => "hyo",
        ['び', 'ゃ'] => "bya",
        ['び', 'ゅ'] => "byu",
        ['び', 'ょ'] => "byo",
        ['ぴ', 'ゃ'] => "pya",
        ['ぴ', 'ゅ'] => "pyu",
        ['ぴ', 'ょ'] => "pyo",
        ['み', 'ゃ'] => "mya",
        ['み', 'ゅ'] => "myu",
        ['み', 'ょ'] => "myo",
        ['り', 'ゃ'] => "rya",
        ['り', 'ゅ'] => "ryu",
        ['り', 'ょ'] => "ryo",
        ['ふ', 'ぁ'] => "fa",
        ['ふ', 'ぃ'] => "fi",
        ['ふ', 'ぇ'] => "fe",
        ['ふ', 'ぉ'] => "fo",
        ['て', 'ぃ'] => "ti",
        ['で', 'ぃ'] => "di",
        ['と', 'ぅ'] => "tu",
        ['ど', 'ぅ'] => "du",
        ['し', 'ぇ'] => "she",
        ['じ', 'ぇ'] => "je",
        ['ち', 'ぇ'] => "che",
        ['つ', 'ぁ'] => "tsa",
        ['つ', 'ぃ'] => "tsi",
        ['つ', 'ぇ'] => "tse",
        ['つ', 'ぉ'] => "tso",
        ['う', 'ぁ'] => "wa",
        ['う', 'ぃ'] => "wi",
        ['う', 'ぇ'] => "we",
        ['う', 'ぉ'] => "wo",
        ['ゔ', 'ぁ'] => "va",
        ['ゔ', 'ぃ'] => "vi",
        ['ゔ', 'ぇ'] => "ve",
        ['ゔ', 'ぉ'] => "vo",
        ['い', 'ぇ'] => "ye",
        _ => return None,
    })
}

fn romanize_single(c: char) -> Option<&'static str> {
    Some(match c {
        'あ' | 'ぁ' => "a",
        'い' | 'ぃ' => "i",
        'う' | 'ぅ' => "u",
        'え' | 'ぇ' => "e",
        'お' | 'ぉ' => "o",
        'か' => "ka",
        'き' => "ki",
        'く' => "ku",
        'け' => "ke",
        'こ' => "ko",
        'が' => "ga",
        'ぎ' => "gi",
        'ぐ' => "gu",
        'げ' => "ge",
        'ご' => "go",
        'さ' => "sa",
        'し' => "shi",
        'す' => "su",
        'せ' => "se",
        'そ' => "so",
        'ざ' => "za",
        'じ' => "ji",
        'ず' => "zu",
        'ぜ' => "ze",
        'ぞ' => "zo",
        'た' => "ta",
        'ち' => "chi",
        'つ' => "tsu",
        'て' => "te",
        'と' => "to",
        'だ' => "da",
        'ぢ' => "ji",
        'づ' => "zu",
        'で' => "de",
        'ど' => "do",
        'な' => "na",
        'に' => "ni",
        'ぬ' => "nu",
        'ね' => "ne",
        'の' => "no",
        'は' => "ha",
        'ひ' => "hi",
        'ふ' => "fu",
        'へ' => "he",
        'ほ' => "ho",
        'ば' => "ba",
        'び' => "bi",
        'ぶ' => "bu",
        'べ' => "be",
        'ぼ' => "bo",
        'ぱ' => "pa",
        'ぴ' => "pi",
        'ぷ' => "pu",
        'ぺ' => "pe",
        'ぽ' => "po",
        'ま' => "ma",
        'み' => "mi",
        'む' => "mu",
        'め' => "me",
        'も' => "mo",
        'や' | 'ゃ' => "ya",
        'ゆ' | 'ゅ' => "yu",
        'よ' | 'ょ' => "yo",
        'ら' => "ra",
        'り' => "ri",
        'る' => "ru",
        'れ' => "re",
        'ろ' => "ro",
        'わ' => "wa",
        'を' => "o",
        'ん' => "n",
        'ゔ' => "vu",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::{hiragana_to_hepburn, hiragana_to_hepburn_variants};

    #[test]
    fn converts_basic_hiragana() {
        assert_eq!(hiragana_to_hepburn("りんご"), "ringo");
        assert_eq!(hiragana_to_hepburn("ふゆ"), "fuyu");
    }

    #[test]
    fn collapses_long_o() {
        assert_eq!(hiragana_to_hepburn("とうきょう"), "tokyo");
        assert_eq!(hiragana_to_hepburn("おおさか"), "osaka");
        assert_eq!(hiragana_to_hepburn("きょうと"), "kyoto");
    }

    #[test]
    fn exposes_uncollapsed_long_vowel_variant() {
        assert_eq!(
            hiragana_to_hepburn_variants("とうきょう"),
            vec!["tokyo".to_string(), "toukyou".to_string()]
        );
    }

    #[test]
    fn keeps_n_before_bmp_as_plain_n() {
        assert_eq!(hiragana_to_hepburn("しんばし"), "shinbashi");
        assert_eq!(hiragana_to_hepburn("てんぷら"), "tenpura");
    }

    #[test]
    fn doubles_n_before_vowel_y_or_n_row_for_ime() {
        // IME-wapuro 仕様: ん + 母音 / ヤ行 / ナ行 は `nn` で打たないと
        // 次音節と結合して ん が消える。例:
        // - せんのりきゅう (千利休): ん+の → `sennnorikyuu` (3連)
        // - ごめんなさい: ん+な → `gomennnasai` (3連)
        // - かんおん (観音): ん+お → `kannon` (2連)
        // - ほんやく (翻訳): ん+や → `honnyaku` (2連)
        assert_eq!(hiragana_to_hepburn("せんのりきゅう"), "sennnorikyuu");
        assert_eq!(hiragana_to_hepburn("ごめんなさい"), "gomennnasai");
        assert_eq!(hiragana_to_hepburn("かんおん"), "kannon");
        assert_eq!(hiragana_to_hepburn("ほんやく"), "honnyaku");
        // 末尾の ん は単独 `n` (IME は次キー/確定で ん 化)
        assert_eq!(hiragana_to_hepburn("どらごん"), "doragon");
    }

    #[test]
    fn handles_sokuon_and_small_kana() {
        assert_eq!(hiragana_to_hepburn("ろけっと"), "roketto");
        assert_eq!(hiragana_to_hepburn("がっこう"), "gakko");
        assert_eq!(hiragana_to_hepburn("ちぇっく"), "chekku");
    }

    #[test]
    fn normalizes_katakana_and_punctuation() {
        assert_eq!(hiragana_to_hepburn("エル（Lawliet）"), "eru lawliet");
        // 長音 ー は IME 入力で必須の `-` キーに合わせてローマ字側にも残す。
        assert_eq!(hiragana_to_hepburn("おーぷん そーす"), "o-pun so-su");
        assert_eq!(hiragana_to_hepburn("エレン・イェーガー"), "eren ye-ga-");
    }

    #[test]
    fn preserves_decimal_and_slash_in_mixed_text() {
        assert_eq!(hiragana_to_hepburn("やく1.5おくkm"), "yaku1.5okukm");
        assert_eq!(hiragana_to_hepburn("やく300km/s"), "yaku300km/s");
    }
}

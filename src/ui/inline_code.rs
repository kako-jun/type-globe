//! Inline Markdown `code` rendering helpers (Issue #97).
//!
//! Question texts and choices stored in `data/questions_*.json` may contain
//! Markdown-style inline code spans wrapped in single backticks, e.g.
//! ``HTMLの `alt` 属性は何のためにある？``. The raw backtick characters
//! should not leak into the TUI; instead, the wrapped text is rendered with
//! a distinct color/weight while the backticks themselves are stripped.
//!
//! This module is intentionally minimal — it only handles single-backtick
//! pairs. Double / triple backticks and escape sequences (`` \` ``) are out
//! of scope for #97. If a `` ` `` has no matching closer the input is
//! returned as a single non-code segment with the backtick preserved
//! verbatim, so malformed data degrades gracefully instead of disappearing.
//!
//! The functions here operate purely on the *display* layer; the typing
//! match path (`ja_typings`, `current_correct_typing_candidates`, etc.) is
//! never routed through them.

use unicode_segmentation::UnicodeSegmentation;

/// One run of "code" or "regular" text produced by [`parse_inline_code`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineSegment {
    pub text: String,
    pub is_code: bool,
}

/// Split `s` into alternating `code` / non-code segments by single
/// backtick pairs. See the module docs for the exact rules.
pub fn parse_inline_code(s: &str) -> Vec<InlineSegment> {
    if s.is_empty() {
        return Vec::new();
    }

    let mut out: Vec<InlineSegment> = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0usize;
    let mut buf = String::new();

    while i < bytes.len() {
        if bytes[i] == b'`' {
            // Look for the matching closing backtick.
            if let Some(rel) = bytes[i + 1..].iter().position(|&b| b == b'`') {
                let close = i + 1 + rel;
                if !buf.is_empty() {
                    out.push(InlineSegment {
                        text: std::mem::take(&mut buf),
                        is_code: false,
                    });
                }
                // SAFETY: backticks are ASCII so slicing on byte indices
                // lands on UTF-8 boundaries.
                let code_text = &s[i + 1..close];
                out.push(InlineSegment {
                    text: code_text.to_string(),
                    is_code: true,
                });
                i = close + 1;
                continue;
            } else {
                // Unmatched opener — fall through and keep the byte
                // verbatim. The rest of the string can still contain
                // matched pairs but a lone backtick that never closes
                // shouldn't swallow the tail of the input.
                buf.push('`');
                i += 1;
                continue;
            }
        }
        // Push one UTF-8 scalar so multibyte text stays intact.
        let ch_len = utf8_char_len(bytes[i]);
        buf.push_str(&s[i..i + ch_len]);
        i += ch_len;
    }

    if !buf.is_empty() {
        out.push(InlineSegment {
            text: buf,
            is_code: false,
        });
    }

    out
}

/// Return (plain-text-without-backticks, list of grapheme index ranges
/// `(start, end_exclusive)` covering the code spans inside that plain
/// text). Used to drive the typewriter reveal animation, which needs a
/// backtick-free string up front but still has to know which graphemes
/// belong to a code span so they can be styled later.
pub fn strip_and_locate(s: &str) -> (String, Vec<(usize, usize)>) {
    let segments = parse_inline_code(s);
    let mut stripped = String::new();
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let mut grapheme_cursor: usize = 0;
    for seg in segments {
        let count = seg.text.graphemes(true).count();
        if seg.is_code && count > 0 {
            ranges.push((grapheme_cursor, grapheme_cursor + count));
        }
        stripped.push_str(&seg.text);
        grapheme_cursor += count;
    }
    (stripped, ranges)
}

/// Convenience: return a per-grapheme boolean mask aligned with the
/// stripped text returned by [`strip_and_locate`]. `mask[i] == true`
/// means grapheme `i` belongs to an inline-code span.
///
/// Currently exercised only by tests — the renderer uses the
/// `(start, end)` range form from [`strip_and_locate`] directly because
/// the number of code spans per question is tiny. Kept public as part
/// of the Issue #97 API surface.
#[allow(dead_code)]
pub fn code_grapheme_mask(s: &str) -> Vec<bool> {
    let (stripped, ranges) = strip_and_locate(s);
    let n = stripped.graphemes(true).count();
    let mut mask = vec![false; n];
    for (start, end) in ranges {
        for slot in mask.iter_mut().take(end.min(n)).skip(start) {
            *slot = true;
        }
    }
    mask
}

#[inline]
fn utf8_char_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b < 0xC0 {
        // Continuation byte — shouldn't be the start of a scalar; treat
        // defensively as length 1 so we keep advancing.
        1
    } else if b < 0xE0 {
        2
    } else if b < 0xF0 {
        3
    } else {
        4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_string() {
        assert_eq!(parse_inline_code(""), vec![]);
    }

    #[test]
    fn plain_text_no_backticks() {
        assert_eq!(
            parse_inline_code("hello world"),
            vec![InlineSegment {
                text: "hello world".into(),
                is_code: false
            }]
        );
    }

    #[test]
    fn single_code_span() {
        assert_eq!(
            parse_inline_code("<img> の `alt` 属性"),
            vec![
                InlineSegment {
                    text: "<img> の ".into(),
                    is_code: false
                },
                InlineSegment {
                    text: "alt".into(),
                    is_code: true
                },
                InlineSegment {
                    text: " 属性".into(),
                    is_code: false
                },
            ]
        );
    }

    #[test]
    fn multiple_code_spans() {
        assert_eq!(
            parse_inline_code("use `let` and `mut`"),
            vec![
                InlineSegment {
                    text: "use ".into(),
                    is_code: false
                },
                InlineSegment {
                    text: "let".into(),
                    is_code: true
                },
                InlineSegment {
                    text: " and ".into(),
                    is_code: false
                },
                InlineSegment {
                    text: "mut".into(),
                    is_code: true
                },
            ]
        );
    }

    #[test]
    fn unclosed_backtick_is_kept_as_literal() {
        assert_eq!(
            parse_inline_code("missing `close"),
            vec![InlineSegment {
                text: "missing `close".into(),
                is_code: false
            }]
        );
    }

    #[test]
    fn empty_code_span_is_collapsed_or_kept() {
        // `` parses as: "a" (plain) + "" (code) + "b" (plain). The empty
        // code segment is preserved so the downstream renderer can choose
        // to skip it; the test only requires that the surrounding text is
        // not lost.
        let segs = parse_inline_code("a``b");
        assert!(segs.iter().any(|s| s.text == "a"));
        assert!(segs.iter().any(|s| s.text == "b"));
    }

    #[test]
    fn strip_and_locate_simple() {
        let (stripped, ranges) = strip_and_locate("`alt` 属性");
        assert_eq!(stripped, "alt 属性");
        assert_eq!(ranges, vec![(0, 3)]);
    }

    #[test]
    fn strip_and_locate_japanese() {
        let (stripped, ranges) = strip_and_locate("使う `let` 文");
        assert_eq!(stripped, "使う let 文");
        // "使う " = 3 graphemes, "let" = 3 graphemes → range (3, 6)
        assert_eq!(ranges, vec![(3, 6)]);
    }

    #[test]
    fn strip_and_locate_no_code() {
        let (stripped, ranges) = strip_and_locate("hello");
        assert_eq!(stripped, "hello");
        assert!(ranges.is_empty());
    }

    #[test]
    fn strip_and_locate_multiple_spans() {
        let (stripped, ranges) = strip_and_locate("a `b` c `d` e");
        assert_eq!(stripped, "a b c d e");
        assert_eq!(ranges, vec![(2, 3), (6, 7)]);
    }

    #[test]
    fn code_grapheme_mask_matches_strip() {
        let input = "a `b` c";
        let (stripped, ranges) = strip_and_locate(input);
        let mask = code_grapheme_mask(input);
        let grapheme_count = stripped.graphemes(true).count();
        assert_eq!(mask.len(), grapheme_count);
        for (i, b) in mask.iter().enumerate() {
            let in_range = ranges.iter().any(|&(s, e)| i >= s && i < e);
            assert_eq!(*b, in_range, "mask[{i}] = {b}, expected {in_range}");
        }
    }

    #[test]
    fn unclosed_backtick_after_valid_pair() {
        // Verify partial-match parsing doesn't swallow the tail.
        let segs = parse_inline_code("use `let` and `oops");
        // First pair is matched; the second backtick has no closer, so
        // " and `oops" stays as one plain segment.
        assert_eq!(
            segs,
            vec![
                InlineSegment {
                    text: "use ".into(),
                    is_code: false
                },
                InlineSegment {
                    text: "let".into(),
                    is_code: true
                },
                InlineSegment {
                    text: " and `oops".into(),
                    is_code: false
                },
            ]
        );
    }
}

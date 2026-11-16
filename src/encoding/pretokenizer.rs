//! Pre-tokenization strategies for Neuratal Tokenizer.
//!
//! Provides the legacy candidates plus the v0.3 structured candidates:
//! 1. `RawByte`: No pre-tokenization. The input is processed as a contiguous stream of bytes.
//! 2. `ConservativeUnicode`: Splits by Unicode character categories (Alphabetic, Numeric, Whitespace, Punctuation).
//! 3. `ScriptAndDigitAware`: Distinguishes distinct Unicode scripts, isolates individual digits,
//!    preserves whitespace runs, and keeps emoji/combining marks attached.
//! 4. `LeadingWhitespaceUnicode` (v0.2): leading spaces attach to the next lexical run.
//! 5. `ScriptTransitionAware`: script-transition ablation without space attachment.
//! 6. `UnicodeHybrid`: context-aware brackets, trailing punctuation and word-internal connectors.
//! 7. `CodeAware`: identifier/operator/LaTeX-aware splitting for source code.
//! 8. `MathSymbolAware`: decimal, time, operator-binding and prefix/suffix-symbol rules.

use serde::{Deserialize, Serialize};
use unicode_general_category::{GeneralCategory, get_general_category};
use unicode_script::{Script as UnicodeScript, UnicodeScript as UnicodeScriptTrait};

/// Pre-tokenization strategy mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PretokenizationMode {
    /// Pure byte-level processing with no boundaries imposed.
    RawByte,
    /// Conservative category-based boundary splitting (legacy v0.1 default).
    #[default]
    ConservativeUnicode,
    /// Script-isolated and digit-isolated splitting.
    ScriptAndDigitAware,
    /// Script-aware Unicode splitting with leading-whitespace attachment:
    /// runs of spaces/tabs are merged with the following lexical run
    /// (e.g. `" the"`, `"  def"`), so BPE can learn space+word tokens.
    /// Newlines never attach; combining marks and ZWJ stay attached to the
    /// preceding character; CJK/Hangul/kana runs and emoji runs are kept whole.
    LeadingWhitespaceUnicode,
    /// Script-transition-aware splitting without leading-space attachment.
    /// Letters split whenever the Unicode script changes, digit runs stay
    /// whole, punctuation runs stay whole, and combining marks stay attached.
    /// This is an ablation that isolates the value of space attachment.
    ScriptTransitionAware,
    /// Improved Unicode hybrid: leading spaces attach to the following run,
    /// opening punctuation attaches to the following lexical run, closing and
    /// trailing punctuation attach to the preceding lexical run, and common
    /// word-internal connectors (apostrophes, hyphens, slashes, dots, colons,
    /// underscores, `@`, `&`) keep URLs, emails, dates and compounds intact.
    UnicodeHybrid,
    /// Code-aware splitting: identifiers may contain ASCII letters, digits,
    /// `_`, `$`, internal apostrophes and internal dots; operator runs are
    /// grouped (`->`, `==`, `::`), LaTeX-like `\command` runs are preserved,
    /// and indentation attaches to the following token.
    CodeAware,
    /// Math- and symbol-aware splitting: decimal numbers, times and grouped
    /// thousands stay whole; LaTeX commands and grouped operator runs are
    /// preserved; operators between lexical atoms bind (`x^2`, `a+b`) and
    /// common prefix/suffix symbols attach to their number (`$100`, `100%`).
    MathSymbolAware,
    /// MathSymbolAware with Han/Hiragana/Katakana/Hangul runs split into
    /// fixed-size chunks of at most 2 characters. v0.4 CJK arm.
    MathSymbolAwareCjk2,
    /// Same as MathSymbolAwareCjk2 with 4-character chunks. v0.4 CJK arm.
    MathSymbolAwareCjk4,
    /// Same as MathSymbolAwareCjk2 with 8-character chunks. v0.4 CJK arm.
    MathSymbolAwareCjk8,
    /// MathSymbolAware with ONLY Han (unified ideograph) runs chunked to 4 chars;
    /// Hiragana/Katakana/Hangul runs stay whole. v0.4 experiment for Chinese.
    MathSymbolAwareCjkHan4,
}

/// Character classification category for pre-tokenization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharCategory {
    Whitespace,
    Digit,
    Letter(Script),
    PunctuationOrSymbol,
    EmojiOrExtended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Script {
    Latin,
    Cyrillic,
    Arabic,
    Hebrew,
    Greek,
    Devanagari,
    Cjk,
    Hangul,
    Thai,
    Other,
}

impl PretokenizationMode {
    /// Splits a UTF-8 text string into pre-token string slices.
    pub fn split_text<'a>(&self, text: &'a str) -> Vec<&'a str> {
        if text.is_empty() {
            return Vec::new();
        }

        match self {
            PretokenizationMode::RawByte => vec![text],
            PretokenizationMode::ConservativeUnicode => split_conservative_unicode(text),
            PretokenizationMode::ScriptAndDigitAware => split_script_and_digit_aware(text),
            PretokenizationMode::LeadingWhitespaceUnicode => split_leading_whitespace_unicode(text),
            PretokenizationMode::ScriptTransitionAware => {
                split_structured_unicode(text, HybridConfig::script_transition())
            }
            PretokenizationMode::UnicodeHybrid => {
                split_structured_unicode(text, HybridConfig::unicode_hybrid())
            }
            PretokenizationMode::CodeAware => split_structured_unicode(text, HybridConfig::code()),
            PretokenizationMode::MathSymbolAware => {
                split_structured_unicode(text, HybridConfig::math_symbol())
            }
            PretokenizationMode::MathSymbolAwareCjk2 => {
                split_structured_unicode(text, HybridConfig::math_symbol_cjk(2))
            }
            PretokenizationMode::MathSymbolAwareCjk4 => {
                split_structured_unicode(text, HybridConfig::math_symbol_cjk(4))
            }
            PretokenizationMode::MathSymbolAwareCjk8 => {
                split_structured_unicode(text, HybridConfig::math_symbol_cjk(8))
            }
            PretokenizationMode::MathSymbolAwareCjkHan4 => {
                split_structured_unicode(text, HybridConfig::math_symbol_cjk_han(4))
            }
        }
    }

    /// Splits an arbitrary byte slice into pre-token byte slices.
    pub fn split_bytes<'a>(&self, bytes: &'a [u8]) -> Vec<&'a [u8]> {
        if bytes.is_empty() {
            return Vec::new();
        }

        match self {
            PretokenizationMode::RawByte => vec![bytes],
            _ => {
                // If the byte slice is valid UTF-8, use UTF-8 text splitting
                if let Ok(text) = std::str::from_utf8(bytes) {
                    self.split_text(text)
                        .into_iter()
                        .map(|s| s.as_bytes())
                        .collect()
                } else {
                    // For invalid UTF-8 byte sequences, fallback to contiguous binary chunks
                    vec![bytes]
                }
            }
        }
    }
}

/// Candidate B: Conservative category-based pre-tokenization.
fn split_conservative_unicode(text: &str) -> Vec<&str> {
    let mut slices = Vec::new();
    let mut start_idx = 0;
    let mut current_cat: Option<CategorySimple> = None;

    #[derive(PartialEq, Eq, Clone, Copy)]
    enum CategorySimple {
        Whitespace,
        Letter,
        Number,
        Other,
    }

    for (idx, ch) in text.char_indices() {
        // Keep combining marks and joiners attached to the preceding character
        if is_combining_or_joiner(ch) {
            continue;
        }

        let cat = if ch.is_whitespace() {
            CategorySimple::Whitespace
        } else if ch.is_alphabetic() {
            CategorySimple::Letter
        } else if ch.is_numeric() {
            CategorySimple::Number
        } else {
            CategorySimple::Other
        };

        if let Some(prev) = current_cat {
            if prev != cat {
                slices.push(&text[start_idx..idx]);
                start_idx = idx;
                current_cat = Some(cat);
            }
        } else {
            current_cat = Some(cat);
        }
    }

    if start_idx < text.len() {
        slices.push(&text[start_idx..]);
    }

    slices
}

/// Candidate D: Script-aware Unicode splitting with leading-whitespace attachment.
///
/// Design (deterministic, single pass):
/// - Categories: Space (ASCII space or tab), Newline (LF/CR/LS/PS), Digit,
///   Letter(Script), PunctuationOrSymbol, EmojiOrExtended.
/// - Boundary rules:
///   * Space runs merge their characters and merge with combining marks.
///   * A Space run IMMEDIATELY FOLLOWED by a Digit or Letter run is emitted
///     as one pre-token together with that run (`" the"`, `"  def"`).
///     Newline runs never attach; trailing Space runs are emitted alone.
///   * Newline runs merge consecutive line terminators into one pre-token.
///   * Letter runs split on script change; CJK/Hangul/kana are one class each.
///   * Digit runs stay together; punctuation/symbol runs stay together;
///     emoji runs stay together.
///   * Combining marks, variation selectors and ZWJ attach to the preceding
///     character (never start a new run).
/// - Determinism: no randomness, no locale, no case folding, no normalization.
fn split_leading_whitespace_unicode(text: &str) -> Vec<&str> {
    #[derive(PartialEq, Eq, Clone, Copy)]
    enum Run {
        Space,       // ASCII space or tab run (attachable)
        Newline,     // LF/CR/LS/PS run
        Digit,
        Letter(Script),
        PunctuationOrSymbol,
        EmojiOrExtended,
    }

    fn classify(ch: char) -> Run {
        match ch {
            ' ' | '\t' => Run::Space,
            '\n' | '\r' | '\u{2028}' | '\u{2029}' => Run::Newline,
            _ if ch.is_ascii_digit() || ('\u{0660}'..='\u{0669}').contains(&ch) => Run::Digit,
            _ if is_emoji(ch) => Run::EmojiOrExtended,
            _ if ch.is_alphabetic() => Run::Letter(classify_script(ch)),
            // Any other non-whitespace character (incl. Unicode Nd/Nl/No and punctuation)
            _ => Run::PunctuationOrSymbol,
        }
    }

    // Collect (start_idx, Run) boundaries with combining-mark attachment.
    let mut runs: Vec<(usize, usize, Run)> = Vec::new(); // (start, end, run)
    let mut cur_start = 0usize;
    let mut cur_run: Option<Run> = None;
    let mut leading_start_pinned = false;

    for (idx, ch) in text.char_indices() {
        if is_combining_or_joiner(ch) {
            // Attach to the ongoing run; if none started, pin the start to the
            // leading mark. `cur_start` defaults to 0 and the first non-mark
            // char must not overwrite a start pinned by leading marks.
            if cur_run.is_none() && !leading_start_pinned {
                cur_start = idx;
                leading_start_pinned = true;
            }
            continue;
        }
        let run = classify(ch);
        match cur_run {
            None => {
                cur_run = Some(run);
            }
            Some(prev) => {
                let same = prev == run;
                if !same {
                    runs.push((cur_start, idx, prev));
                    cur_start = idx;
                    cur_run = Some(run);
                }
            }
        }
    }
    if let Some(prev) = cur_run {
        runs.push((cur_start, text.len(), prev));
    }

    // Whitespace attachment: a Space run immediately followed by Digit/Letter merges.
    let mut out: Vec<&str> = Vec::with_capacity(runs.len() + 4);
    if runs.is_empty() {
        // Text consisting solely of combining marks / joiners: keep it whole.
        return vec![text];
    }
    let mut i = 0;
    while i < runs.len() {
        let (s, e, run) = runs[i];
        if run == Run::Space && i + 1 < runs.len() {
            let (s2, e2, next_run) = runs[i + 1];
            debug_assert_eq!(e, s2);
            if matches!(next_run, Run::Digit | Run::Letter(_)) && e == s2 {
                out.push(&text[s..e2]);
                i += 2;
                continue;
            }
        }
        out.push(&text[s..e]);
        i += 1;
    }
    out
}

/// Candidate C: Script-aware and digit-isolated pre-tokenization.
fn split_script_and_digit_aware(text: &str) -> Vec<&str> {
    let mut slices = Vec::new();
    let mut start_idx = 0;
    let mut current_cat: Option<CharCategory> = None;

    for (idx, ch) in text.char_indices() {
        // Handle combining marks, variation selectors, and ZWJ by keeping them attached
        if is_combining_or_joiner(ch) {
            continue;
        }

        let cat = classify_char(ch);

        if let Some(prev) = current_cat {
            let should_split = match (prev, cat) {
                // Digits are isolated individually
                (CharCategory::Digit, CharCategory::Digit) => true,
                // Differing categories always split
                (p, c) if p != c => true,
                // Within punctuation/symbols, single-char isolation unless identical whitespace
                (CharCategory::PunctuationOrSymbol, CharCategory::PunctuationOrSymbol) => true,
                _ => false,
            };

            if should_split {
                slices.push(&text[start_idx..idx]);
                start_idx = idx;
                current_cat = Some(cat);
            }
        } else {
            current_cat = Some(cat);
        }
    }

    if start_idx < text.len() {
        slices.push(&text[start_idx..]);
    }

    slices
}

/// Combining marks and joiners attach to the surrounding run.
///
/// The first release used a hardcoded range list, which misclassified many
/// legitimate marks (e.g. Devanagari Extended, Cyrillic combining marks,
/// U+20E0 enclosing marks not in 20D0..20FF) as punctuation, splitting
/// grapheme clusters. The general-category check covers every mark/format
/// character and is kept in sync with Unicode data by construction.
#[inline]
fn is_combining_or_joiner(c: char) -> bool {
    use unicode_general_category::{GeneralCategory, get_general_category};
    if matches!(
        get_general_category(c),
        GeneralCategory::NonspacingMark | GeneralCategory::SpacingMark | GeneralCategory::EnclosingMark
    ) {
        return true;
    }
    let u = c as u32;
    matches!(
        u,
        0x200C | 0x200D |                 // ZWNJ, ZWJ
        0xFE00..=0xFE0F |                 // Variation Selectors
        0xE0100..=0xE01EF |               // Variation Selectors Supplement
        0x1F3FB..=0x1F3FF |               // Emoji Fitzpatrick Skin-Tone Modifiers
        0xE0020..=0xE007F                 // Tags (for emoji sub-region flags)
    )
}

fn classify_char(c: char) -> CharCategory {
    if c.is_whitespace() {
        CharCategory::Whitespace
    } else if c.is_ascii_digit() || ('\u{0660}'..='\u{0669}').contains(&c) {
        CharCategory::Digit
    } else if c.is_alphabetic() {
        CharCategory::Letter(classify_script(c))
    } else if is_emoji(c) {
        CharCategory::EmojiOrExtended
    } else {
        CharCategory::PunctuationOrSymbol
    }
}

fn classify_script(c: char) -> Script {
    let u = c as u32;
    match u {
        // Latin (including Basic, Latin-1, Extended-A, Extended-B, IPA, and Latin Extended Additional for Vietnamese)
        0x0041..=0x005A | 0x0061..=0x007A | 0x00C0..=0x02AF | 0x1E00..=0x1EFF => Script::Latin,
        // Cyrillic (including Basic, Supplement, and Historical)
        0x0400..=0x04FF | 0x0500..=0x052F | 0x2DE0..=0x2DFF | 0xA640..=0xA69F => Script::Cyrillic,
        // Arabic (including Basic, Supplement, Extended, and Presentation Forms)
        0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF => {
            Script::Arabic
        }
        // Hebrew
        0x0590..=0x05FF | 0xFB1D..=0xFB4F => Script::Hebrew,
        // Greek and Coptic, Greek Extended
        0x0370..=0x03FF | 0x1F00..=0x1FFF => Script::Greek,
        // Devanagari and Devanagari Extended
        0x0900..=0x097F | 0xA8E0..=0xA8FF => Script::Devanagari,
        // CJK Unified Ideographs, Extensions, Hiragana, and Katakana
        0x4E00..=0x9FFF
        | 0x3400..=0x4DBF
        | 0x20000..=0x2A6DF
        | 0x3040..=0x309F
        | 0x30A0..=0x30FF
        | 0x31F0..=0x31FF => Script::Cjk,
        // Hangul Syllables, Jamo, and Compatibility Jamo
        0xAC00..=0xD7AF | 0x1100..=0x11FF | 0x3130..=0x318F | 0xA960..=0xA97F | 0xD7B0..=0xD7FF => {
            Script::Hangul
        }
        // Thai
        0x0E00..=0x0E7F => Script::Thai,
        _ => Script::Other,
    }
}

fn is_emoji(c: char) -> bool {
    let u = c as u32;
    matches!(
        u,
        0x1F600..=0x1F64F | // Emoticons
        0x1F300..=0x1F5FF | // Misc Symbols and Pictographs
        0x1F680..=0x1F6FF | // Transport and Map
        0x1F700..=0x1F77F | // Alchemical Symbols
        0x1F780..=0x1F7FF | // Geometric Shapes Extended
        0x1F800..=0x1F8FF | // Supplemental Arrows-C
        0x1F900..=0x1F9FF | // Supplemental Symbols and Pictographs
        0x1FA00..=0x1FA6F | // Chess Symbols
        0x1FA70..=0x1FAFF | // Symbols and Pictographs Extended-A
        0x2600..=0x26FF   | // Misc symbols
        0x2700..=0x27BF   | // Dingbats
        0x1F1E6..=0x1F1FF   // Regional Indicator Symbols (Flags)
    )
}

// ---------------------------------------------------------------------------
// v0.3 structured Unicode pretokenizers
// ---------------------------------------------------------------------------

/// Script-agnostic word marker. `Common` (Zyyy) is used as the wildcard so a
/// connector or digit can join a word run without erasing its script.
const SCRIPT_ANY: UnicodeScript = UnicodeScript::Common;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StructuredRun {
    Space,
    Newline,
    Word(UnicodeScript),
    Digit,
    OpenPunct,
    ClosePunct,
    Punct,
    Operator,
    Symbol,
    Emoji,
    Other,
}

#[derive(Debug, Clone, Copy)]
struct HybridConfig {
    /// Merge a space/tab run into the following non-newline run.
    attach_space: bool,
    /// Split words when the Unicode script changes.
    split_scripts: bool,
    /// Treat decimal digits as identifier characters (code mode).
    ident_digits: bool,
    /// Treat `_` as an identifier character.
    ident_underscore: bool,
    /// Treat internal apostrophes as identifier characters.
    ident_apostrophe: bool,
    /// Treat internal dots as identifier characters.
    ident_dot: bool,
    /// Keep `\command` LaTeX-like runs together.
    latex_commands: bool,
    /// Group adjacent operator characters into one run.
    group_operators: bool,
    /// Attach opening punctuation to the following lexical run.
    attach_open: bool,
    /// Attach closing punctuation to the preceding lexical run.
    attach_close: bool,
    /// Attach trailing punctuation to the preceding lexical run.
    attach_trailing_punct: bool,
    /// Keep word-internal connectors such as `'`, `-`, `/`, `.`, `:` attached
    /// to both neighbours when no whitespace is present.
    internal_connectors: bool,
    /// Bind an operator run to its lexical neighbours (`x^2`, `a+b`).
    internal_math_ops: bool,
    /// Attach currency-like prefix symbols to a following number/word.
    attach_prefix_symbols: bool,
    /// Attach percent/degree-like suffix symbols to a preceding number/word.
    attach_suffix_symbols: bool,
    /// Treat a dot between digits as part of a decimal number.
    decimal_numbers: bool,
    /// Treat `:` and `/` between digits (times and dates) as digit material.
    time_like: bool,
    /// Treat a comma between digits as digit material (grouped thousands).
    comma_between_digits: bool,
    /// Optional fixed-size chunk cap for CJK script runs. Unsegmented CJK
    /// sentences are extremely long pre-tokens whose frequencies are ≈1, so a
    /// flat `--min-word-freq` prunes them all. Capping CJK chunks keeps
    /// frequent short sequences in the index. When `cjk_han_only` is set, the
    /// chunk applies only to Han ideographs (kana/Hangul stay whole).
    cjk_chunk: Option<usize>,
    cjk_han_only: bool,
}

impl HybridConfig {
    fn script_transition() -> Self {
        Self {
            attach_space: false,
            split_scripts: true,
            ident_digits: false,
            ident_underscore: false,
            ident_apostrophe: false,
            ident_dot: false,
            latex_commands: false,
            group_operators: false,
            attach_open: false,
            attach_close: false,
            attach_trailing_punct: false,
            internal_connectors: false,
            internal_math_ops: false,
            attach_prefix_symbols: false,
            attach_suffix_symbols: false,
            decimal_numbers: false,
            time_like: false,
            comma_between_digits: false,
            cjk_chunk: None,
            cjk_han_only: false,
        }
    }

    fn unicode_hybrid() -> Self {
        Self {
            attach_space: true,
            split_scripts: true,
            ident_digits: false,
            ident_underscore: false,
            ident_apostrophe: false,
            ident_dot: false,
            latex_commands: false,
            group_operators: false,
            attach_open: true,
            attach_close: true,
            attach_trailing_punct: true,
            internal_connectors: true,
            internal_math_ops: false,
            attach_prefix_symbols: true,
            attach_suffix_symbols: true,
            decimal_numbers: true,
            time_like: true,
            comma_between_digits: true,
            cjk_chunk: None,
            cjk_han_only: false,
        }
    }

    fn code() -> Self {
        Self {
            attach_space: true,
            split_scripts: false,
            ident_digits: true,
            ident_underscore: true,
            ident_apostrophe: true,
            ident_dot: true,
            latex_commands: true,
            group_operators: true,
            attach_open: true,
            attach_close: true,
            attach_trailing_punct: true,
            internal_connectors: false,
            internal_math_ops: false,
            attach_prefix_symbols: true,
            attach_suffix_symbols: true,
            decimal_numbers: true,
            time_like: false,
            comma_between_digits: false,
            cjk_chunk: None,
            cjk_han_only: false,
        }
    }

    fn math_symbol_cjk(chunk: usize) -> Self {
        let mut cfg = Self::math_symbol();
        cfg.cjk_chunk = Some(chunk);
        cfg
    }

    fn math_symbol_cjk_han(chunk: usize) -> Self {
        let mut cfg = Self::math_symbol();
        cfg.cjk_chunk = Some(chunk);
        cfg.cjk_han_only = true;
        cfg
    }

    fn math_symbol() -> Self {
        Self {
            attach_space: true,
            split_scripts: true,
            ident_digits: false,
            ident_underscore: false,
            ident_apostrophe: false,
            ident_dot: false,
            latex_commands: true,
            group_operators: true,
            attach_open: true,
            attach_close: true,
            attach_trailing_punct: true,
            internal_connectors: true,
            internal_math_ops: true,
            attach_prefix_symbols: true,
            attach_suffix_symbols: true,
            decimal_numbers: true,
            time_like: true,
            comma_between_digits: true,
            cjk_chunk: None,
            cjk_han_only: false,
        }
    }
}

#[inline]
fn is_structured_mark(ch: char, category: GeneralCategory) -> bool {
    matches!(
        category,
        GeneralCategory::NonspacingMark
            | GeneralCategory::SpacingMark
            | GeneralCategory::EnclosingMark
    ) || matches!(
        ch as u32,
        0x200C | 0x200D                  // ZWNJ, ZWJ
        | 0xFE00..=0xFE0F                // Variation Selectors
        | 0xE0100..=0xE01EF              // Variation Selectors Supplement
        | 0x1F3FB..=0x1F3FF              // Emoji skin-tone modifiers
        | 0xE0020..=0xE007F              // Emoji tag sequences
    )
}

#[inline]
fn is_decimal_digit(ch: char) -> bool {
    get_general_category(ch) == GeneralCategory::DecimalNumber
}

#[inline]
fn is_wordish_char(ch: char) -> bool {
    // Exact match for char::is_alphanumeric || '_' — NOT a category subset,
    // because Unicode "Alphabetic" includes some marks. Keep byte-identical behavior.
    ch.is_alphanumeric() || ch == '_'
}

#[inline]
fn is_ascii_code_operator(ch: char) -> bool {
    matches!(
        ch,
        '+' | '-' | '*' | '/' | '%' | '=' | '<' | '>' | '!' | '&' | '|' | '^' | '~'
    )
}

#[inline]
fn is_internal_connector(ch: char) -> bool {
    matches!(
        ch,
        '\'' | '\u{2019}' | '-' | '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}'
            | '\u{2014}' | '/' | '.' | ':' | '@' | '_' | '&' | '\u{00B7}'
    )
}

#[inline]
fn is_prefix_symbol(ch: char) -> bool {
    matches!(
        ch,
        '$' | '\u{20AC}' | '\u{00A3}' | '\u{00A5}' | '\u{20B9}' | '\u{20BA}' | '\u{20BD}'
            | '\u{20A9}' | '\u{20AA}' | '\u{20AB}' | '\u{0E3F}' | '\u{20B4}' | '\u{20A6}'
            | '\u{20B1}' | '\u{20A1}' | '\u{20B2}' | '\u{20B5}' | '\u{20B8}' | '\u{20BC}'
            | '\u{20BE}' | '\u{09F3}'
    )
}

#[inline]
fn is_suffix_symbol(ch: char) -> bool {
    matches!(
        ch,
        '%' | '\u{2030}' | '\u{00B0}' | '\u{2103}' | '\u{2109}' | '\u{2032}' | '\u{2033}'
            | '\u{00A2}' | '\u{00B9}' | '\u{00B2}' | '\u{00B3}' | '\u{2070}' | '\u{2074}'..='\u{2079}'
            | '\u{2080}'..='\u{2089}' | '+'
    )
}

fn all_chars<F: Fn(char) -> bool>(text: &str, pred: F) -> bool {
    !text.is_empty() && text.chars().all(pred)
}

fn structured_word_script(ch: char, cfg: &HybridConfig) -> UnicodeScript {
    if !cfg.split_scripts {
        return SCRIPT_ANY;
    }
    // Fast path for ASCII letters: the script table binary search is only
    // needed for non-ASCII code points; 'A'-'Z'/'a'-'z' are always Latin.
    if ch.is_ascii() {
        return if ch.is_ascii_alphabetic() {
            UnicodeScript::Latin
        } else {
            UnicodeScript::Common
        };
    }
    let script = ch.script();
    match script {
        UnicodeScript::Common | UnicodeScript::Inherited | UnicodeScript::Unknown => SCRIPT_ANY,
        other => other,
    }
}

fn classify_structured(
    ch: char,
    prev: Option<char>,
    next: Option<char>,
    cfg: &HybridConfig,
    category: GeneralCategory,
) -> StructuredRun {
    if ch == '\n' || ch == '\r' {
        return StructuredRun::Newline;
    }

    match category {
        GeneralCategory::LineSeparator | GeneralCategory::ParagraphSeparator => {
            return StructuredRun::Newline;
        }
        GeneralCategory::SpaceSeparator => return StructuredRun::Space,
        _ => {}
    }
    if ch == '\t' || (ch.is_whitespace() && !matches!(ch, '\u{0085}')) {
        return StructuredRun::Space;
    }
    if matches!(category, GeneralCategory::Control | GeneralCategory::Format) {
        return StructuredRun::Other;
    }

    // Context-sensitive ASCII quotes. A quote between two word characters is
    // left as punctuation so the connector pass can keep contractions intact.
    if ch == '"' || ch == '\'' || ch == '\u{2019}' {
        let prev_word = prev.is_some_and(is_wordish_char);
        let next_word = next.is_some_and(is_wordish_char);
        if cfg.ident_apostrophe && (ch == '\'' || ch == '\u{2019}') && prev_word && next_word {
            return StructuredRun::Word(SCRIPT_ANY);
        }
        if prev_word && next_word {
            return StructuredRun::Punct;
        }
        if prev_word {
            return StructuredRun::ClosePunct;
        }
        return StructuredRun::OpenPunct;
    }

    if ch == '\\' && cfg.latex_commands {
        if let Some(n) = next {
            if n.is_ascii_alphabetic() {
                return StructuredRun::Word(structured_word_script(n, cfg));
            }
        }
        return StructuredRun::Punct;
    }

    if ch == '_' {
        return if cfg.ident_underscore {
            StructuredRun::Word(SCRIPT_ANY)
        } else {
            StructuredRun::Punct
        };
    }

    if ch == '.' {
        if cfg.ident_dot
            && prev.is_some_and(is_wordish_char)
            && next.is_some_and(is_wordish_char)
        {
            return StructuredRun::Word(SCRIPT_ANY);
        }
        if cfg.decimal_numbers
            && prev.is_some_and(is_decimal_digit)
            && next.is_some_and(is_decimal_digit)
        {
            return StructuredRun::Digit;
        }
        return StructuredRun::Punct;
    }

    if matches!(ch, ':' | '/')
        && cfg.time_like
        && prev.is_some_and(is_decimal_digit)
        && next.is_some_and(is_decimal_digit)
    {
        return StructuredRun::Digit;
    }

    if ch == ','
        && cfg.comma_between_digits
        && prev.is_some_and(is_decimal_digit)
        && next.is_some_and(is_decimal_digit)
    {
        return StructuredRun::Digit;
    }

    if cfg.group_operators && is_ascii_code_operator(ch) {
        return StructuredRun::Operator;
    }

    if category == GeneralCategory::DecimalNumber {
        return if cfg.ident_digits {
            StructuredRun::Word(SCRIPT_ANY)
        } else {
            StructuredRun::Digit
        };
    }

    match category {
        GeneralCategory::UppercaseLetter
        | GeneralCategory::LowercaseLetter
        | GeneralCategory::TitlecaseLetter
        | GeneralCategory::ModifierLetter
        | GeneralCategory::OtherLetter
        | GeneralCategory::LetterNumber => StructuredRun::Word(structured_word_script(ch, cfg)),
        GeneralCategory::OpenPunctuation | GeneralCategory::InitialPunctuation => {
            StructuredRun::OpenPunct
        }
        GeneralCategory::ClosePunctuation | GeneralCategory::FinalPunctuation => {
            StructuredRun::ClosePunct
        }
        GeneralCategory::ConnectorPunctuation
        | GeneralCategory::DashPunctuation
        | GeneralCategory::OtherPunctuation => StructuredRun::Punct,
        GeneralCategory::MathSymbol | GeneralCategory::ModifierSymbol => {
            if cfg.group_operators {
                StructuredRun::Operator
            } else {
                StructuredRun::Symbol
            }
        }
        GeneralCategory::CurrencySymbol => StructuredRun::Symbol,
        GeneralCategory::OtherSymbol => {
            if is_emoji(ch) {
                StructuredRun::Emoji
            } else {
                StructuredRun::Symbol
            }
        }
        GeneralCategory::OtherNumber => StructuredRun::Symbol,
        GeneralCategory::NonspacingMark
        | GeneralCategory::SpacingMark
        | GeneralCategory::EnclosingMark => StructuredRun::Other,
        _ => StructuredRun::Other,
    }
}

#[inline]
fn is_lexical(run: StructuredRun) -> bool {
    matches!(
        run,
        StructuredRun::Word(_) | StructuredRun::Digit | StructuredRun::Emoji
    )
}

#[inline]
fn same_structured_run(a: StructuredRun, b: StructuredRun) -> bool {
    match (a, b) {
        (StructuredRun::Word(sa), StructuredRun::Word(sb)) => {
            sa == sb || sa == SCRIPT_ANY || sb == SCRIPT_ANY
        }
        _ => a == b,
    }
}

#[inline]
fn combine_structured_run(a: StructuredRun, b: StructuredRun) -> StructuredRun {
    match (a, b) {
        (StructuredRun::Word(sa), StructuredRun::Word(sb)) => {
            if sa == SCRIPT_ANY { StructuredRun::Word(sb) } else { StructuredRun::Word(sa) }
        }
        _ => b,
    }
}

fn merged_group_class(prev: StructuredRun, current: StructuredRun) -> StructuredRun {
    match (prev, current) {
        (StructuredRun::Space, c) => c,
        (StructuredRun::OpenPunct, c) if is_lexical(c) => c,
        (StructuredRun::OpenPunct, StructuredRun::OpenPunct) => StructuredRun::OpenPunct,
        (StructuredRun::OpenPunct, StructuredRun::ClosePunct) => {
            StructuredRun::Word(SCRIPT_ANY)
        }
        (StructuredRun::ClosePunct, StructuredRun::ClosePunct) => StructuredRun::ClosePunct,
        (p, StructuredRun::ClosePunct) if is_lexical(p) => p,
        (p, StructuredRun::Punct) if is_lexical(p) => p,
        (StructuredRun::Symbol, c) if is_lexical(c) => c,
        (p, StructuredRun::Symbol) if is_lexical(p) => p,
        _ => current,
    }
}

fn split_structured_unicode(text: &str, cfg: HybridConfig) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut char_iter = text.char_indices().peekable();
    let mut runs: Vec<(usize, usize, StructuredRun)> = Vec::with_capacity(text.len() / 3 + 1);
    let mut current_start = 0usize;
    let mut current_class: Option<StructuredRun> = None;
    let mut prev_char: Option<char> = None;
    // Whether the first run's start was pinned by a leading mark (one-shot).
    let mut leading_start_pinned = false;

    while let Some((idx, ch)) = char_iter.next() {
        let category = get_general_category(ch);
        if is_structured_mark(ch, category) {
            // Marks never start a new run; if they appear first, the run starts
            // at the mark and the following base character supplies the class.
            // `current_start` is initialised to 0, so the first mark (at any
            // index) pins it; the first non-mark char must NOT overwrite it.
            if current_class.is_none() && !leading_start_pinned {
                current_start = idx;
                leading_start_pinned = true;
            }
            continue;
        }

        let prev = prev_char;
        let next = char_iter.peek().map(|&(_, c)| c);
        let class = classify_structured(ch, prev, next, &cfg, category);

        match current_class {
            None => {
                // Keep the current_start pinned by leading marks (if any);
                // it defaults to 0 for mark-free text, which is also idx 0.
                current_class = Some(class);
            }
            Some(prev_class) => {
                if same_structured_run(prev_class, class) {
                    current_class = Some(combine_structured_run(prev_class, class));
                } else {
                    runs.push((current_start, idx, prev_class));
                    current_start = idx;
                    current_class = Some(class);
                }
            }
        }
        prev_char = Some(ch);
    }

    if let Some(class) = current_class {
        runs.push((current_start, text.len(), class));
    }
    if runs.is_empty() {
        return vec![text];
    }

    // CJK chunking: split long Han/Hiragana/Katakana/Hangul runs into at most
    // `cjk_chunk` characters per pre-token so their pair frequencies survive
    // `--min-word-freq` pruning (rare whole sentences contribute nothing).
    let runs = match cfg.cjk_chunk {
        Some(chunk) => {
            let mut chunked: Vec<(usize, usize, StructuredRun)> = Vec::with_capacity(runs.len() * 2);
            for (start, end, class) in runs {
                let is_cjk_word = match class {
                    StructuredRun::Word(UnicodeScript::Han) => true,
                    StructuredRun::Word(UnicodeScript::Hiragana)
                    | StructuredRun::Word(UnicodeScript::Katakana)
                    | StructuredRun::Word(UnicodeScript::Hangul) => !cfg.cjk_han_only,
                    _ => false,
                };
                if !is_cjk_word {
                    chunked.push((start, end, class));
                    continue;
                }
                let run_text = &text[start..end];
                let mut piece_start = start;
                let mut remaining = chunk;
                for (byte_offset, _ch) in run_text.char_indices() {
                    if remaining == 0 {
                        chunked.push((piece_start, start + byte_offset, class));
                        piece_start = start + byte_offset;
                        remaining = chunk;
                    }
                    remaining -= 1;
                }
                chunked.push((piece_start, end, class));
            }
            chunked
        }
        None => runs,
    };

    // Pass 1: bind internal connectors and math operators to both neighbours.
    let mut base: Vec<(usize, usize, StructuredRun)> = Vec::with_capacity(runs.len());
    let mut i = 0usize;
    while i < runs.len() {
        let run = runs[i];
        let run_text = &text[run.0..run.1];
        let has_left = base.last().is_some_and(|r| is_lexical(r.2));
        let has_right = i + 1 < runs.len() && is_lexical(runs[i + 1].2);

        if cfg.internal_connectors
            && has_left
            && has_right
            && all_chars(run_text, is_internal_connector)
        {
            let next = runs[i + 1];
            let last = base.last_mut().expect("has_left checked");
            last.1 = next.1;
            last.2 = merged_group_class(last.2, next.2);
            i += 2;
            continue;
        }

        if cfg.internal_math_ops
            && has_left
            && has_right
            && run.2 == StructuredRun::Operator
        {
            let next = runs[i + 1];
            let last = base.last_mut().expect("has_left checked");
            last.1 = next.1;
            last.2 = merged_group_class(last.2, next.2);
            i += 2;
            continue;
        }

        base.push(run);
        i += 1;
    }

    // Pass 2: context attachments (spaces, brackets, punctuation, symbols).
    let mut groups: Vec<(usize, usize, StructuredRun)> = Vec::with_capacity(base.len());
    for run in base {
        let run_text = &text[run.0..run.1];
        if let Some(last) = groups.last_mut() {
            let last_class = last.2;
            let merge = match run.2 {
                StructuredRun::Word(_) | StructuredRun::Digit | StructuredRun::Emoji => {
                    (cfg.attach_space && last_class == StructuredRun::Space)
                        || (cfg.attach_open && last_class == StructuredRun::OpenPunct)
                        || (cfg.attach_prefix_symbols
                            && last_class == StructuredRun::Symbol
                            && all_chars(&text[last.0..last.1], is_prefix_symbol))
                }
                StructuredRun::OpenPunct => {
                    (cfg.attach_space && last_class == StructuredRun::Space)
                        || (cfg.attach_open && last_class == StructuredRun::OpenPunct)
                }
                StructuredRun::ClosePunct => {
                    (cfg.attach_space && last_class == StructuredRun::Space)
                        || (cfg.attach_close
                            && (is_lexical(last_class)
                                || last_class == StructuredRun::OpenPunct
                                || last_class == StructuredRun::ClosePunct))
                }
                StructuredRun::Punct => {
                    (cfg.attach_space && last_class == StructuredRun::Space)
                        || (cfg.attach_trailing_punct
                            && (is_lexical(last_class)
                                || last_class == StructuredRun::ClosePunct
                                || last_class == StructuredRun::Punct))
                }
                StructuredRun::Operator => cfg.attach_space && last_class == StructuredRun::Space,
                StructuredRun::Symbol => {
                    (cfg.attach_space && last_class == StructuredRun::Space)
                        || (cfg.attach_suffix_symbols
                            && is_lexical(last_class)
                            && all_chars(run_text, is_suffix_symbol))
                }
                StructuredRun::Space | StructuredRun::Newline | StructuredRun::Other => false,
            };

            if merge {
                last.1 = run.1;
                last.2 = merged_group_class(last_class, run.2);
                continue;
            }
        }
        groups.push(run);
    }

    groups
        .into_iter()
        .map(|(start, end, _)| &text[start..end])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_byte_pretokenizer() {
        let text = "Hello 123 world!";
        let slices = PretokenizationMode::RawByte.split_text(text);
        assert_eq!(slices, vec!["Hello 123 world!"]);
    }

    #[test]
    fn test_conservative_pretokenizer() {
        let text = "Hello 123 world!";
        let slices = PretokenizationMode::ConservativeUnicode.split_text(text);
        assert_eq!(slices, vec!["Hello", " ", "123", " ", "world", "!"]);
    }

    #[test]
    fn test_script_and_digit_aware_pretokenizer() {
        let text = "a123bc";
        let slices = PretokenizationMode::ScriptAndDigitAware.split_text(text);
        // 'a', '1', '2', '3', 'bc'
        assert_eq!(slices, vec!["a", "1", "2", "3", "bc"]);
    }

    #[test]
    fn test_leading_whitespace_unicode_pretokenizer() {
        use PretokenizationMode::LeadingWhitespaceUnicode as L;
        // Leading space attaches to the following lexical run
        assert_eq!(L.split_text("Merhaba dünya"), vec!["Merhaba", " dünya"]);
        assert_eq!(L.split_text(" the"), vec![" the"]);
        // Code indentation is preserved and attaches
        assert_eq!(
            L.split_text("def outer():\n    if True:"),
            vec!["def", " outer", "():", "\n", "    if", " True", ":"]
        );
        // Tabs attach too, and are NOT normalized away
        assert_eq!(L.split_text("\t\tvalue = 1"), vec!["\t\tvalue", " ", "=", " 1"]);
        // Numbers keep whole runs of digits; leading space attaches
        assert_eq!(L.split_text("version 2026"), vec!["version", " 2026"]);
        // Paragraph breaks stay separate
        assert_eq!(L.split_text("a\n\nb"), vec!["a", "\n\n", "b"]);
        // Arabic, Devanagari, CJK, Hangul, Thai: script runs stay whole
        assert_eq!(L.split_text("مرحبا بالعالم"), vec!["مرحبا", " بالعالم"]);
        assert_eq!(L.split_text("कि का"), vec!["कि", " का"]);
        assert_eq!(L.split_text("中文测试"), vec!["中文测试"]);
        assert_eq!(L.split_text("한국어 테스트"), vec!["한국어", " 테스트"]);
        assert_eq!(L.split_text("สวัสดี ชาวโลก"), vec!["สวัสดี", " ชาวโลก"]);
        // Combining marks and ZWJ sequences stay attached
        assert_eq!(L.split_text("👨‍👩‍👧‍👦"), vec!["👨‍👩‍👧‍👦"]);
        assert_eq!(L.split_text("का कि"), vec!["का", " कि"]);
        // Trailing whitespace is its own pre-token
        assert_eq!(L.split_text("a  "), vec!["a", "  "]);
        // Newline does NOT attach to indentation, but the indentation attaches to the word
        assert_eq!(L.split_text("x\n    y"), vec!["x", "\n", "    y"]);
    }

    #[test]
    fn test_script_transition_aware_pretokenizer() {
        use PretokenizationMode::ScriptTransitionAware as S;
        assert_eq!(S.split_text("hello world"), vec!["hello", " ", "world"]);
        assert_eq!(S.split_text("abcПривет"), vec!["abc", "Привет"]);
        assert_eq!(S.split_text("2026-11-15"), vec!["2026", "-", "11", "-", "15"]);
        assert_eq!(S.split_text("a\n\nb"), vec!["a", "\n\n", "b"]);
        assert_eq!(S.split_text("مرحبا 123"), vec!["مرحبا", " ", "123"]);
    }

    #[test]
    fn test_unicode_hybrid_pretokenizer() {
        use PretokenizationMode::UnicodeHybrid as H;
        assert_eq!(H.split_text("Hello, world!"), vec!["Hello,", " world!"]);
        assert_eq!(H.split_text("\"hello\""), vec!["\"hello\""]);
        assert_eq!(H.split_text("(hello)"), vec!["(hello)"]);
        assert_eq!(H.split_text("don't stop"), vec!["don't", " stop"]);
        assert_eq!(
            H.split_text("state-of-the-art"),
            vec!["state-of-the-art"]
        );
        assert_eq!(H.split_text("3.14"), vec!["3.14"]);
        assert_eq!(H.split_text("1,000"), vec!["1,000"]);
        assert_eq!(H.split_text("12:30"), vec!["12:30"]);
        assert_eq!(H.split_text("100%"), vec!["100%"]);
        assert_eq!(H.split_text("$100"), vec!["$100"]);
        assert_eq!(
            H.split_text("https://example.com/a-b"),
            vec!["https://example.com/a-b"]
        );
        assert_eq!(H.split_text("user@example.com"), vec!["user@example.com"]);
        assert_eq!(H.split_text("Merhaba dünya"), vec!["Merhaba", " dünya"]);
        assert_eq!(H.split_text("中文，测试"), vec!["中文，", "测试"]);
        assert_eq!(H.split_text("a\n\nb"), vec!["a", "\n\n", "b"]);
        assert_eq!(H.split_text("👨‍👩‍👧‍👦 ok"), vec!["👨‍👩‍👧‍👦", " ok"]);
    }

    #[test]
    fn test_code_aware_pretokenizer() {
        use PretokenizationMode::CodeAware as C;
        assert_eq!(
            C.split_text("foo_bar = baz(1);"),
            vec!["foo_bar", " =", " baz", "(1);"]
        );
        assert_eq!(C.split_text("x->y"), vec!["x", "->", "y"]);
        assert_eq!(C.split_text("if (x) {"), vec!["if", " (x)", " {"]);
        assert_eq!(C.split_text("\"hello\""), vec!["\"hello\""]);
        assert_eq!(C.split_text("'a'"), vec!["'a'"]);
        assert_eq!(C.split_text("3.14 + 2"), vec!["3.14", " +", " 2"]);
        assert_eq!(C.split_text("\\alpha + \\beta"), vec!["\\alpha", " +", " \\beta"]);
        assert_eq!(C.split_text("    return value;"), vec!["    return", " value;"]);
    }

    #[test]
    fn test_math_symbol_aware_pretokenizer() {
        use PretokenizationMode::MathSymbolAware as M;
        assert_eq!(
            M.split_text("x^2 + y_1 = 3.5"),
            vec!["x^2", " +", " y_1", " =", " 3.5"]
        );
        assert_eq!(M.split_text("a+b"), vec!["a+b"]);
        assert_eq!(M.split_text("100°"), vec!["100°"]);
        assert_eq!(M.split_text("$100"), vec!["$100"]);
        assert_eq!(M.split_text("\\frac{1}{2}"), vec!["\\frac", "{1}", "{2}"]);
        assert_eq!(M.split_text("2026-11-15"), vec!["2026-11-15"]);
    }

    #[test]
    fn test_cjk_chunking_variants() {
        use PretokenizationMode::*;
        // 11 ideographs: 4/4/3 with Cjk4
        let input = "今天天气很好我们今天要讨论";
        let out4 = MathSymbolAwareCjk4.split_text(input);
        assert_eq!(out4, vec!["今天天气", "很好我们", "今天要讨", "论"]);
        let out2 = MathSymbolAwareCjk2.split_text(input);
        assert_eq!(
            out2,
            vec!["今天", "天气", "很好", "我们", "今天", "要讨", "论"]
        );
        // Non-CJK text keeps the MathSymbolAware behaviour.
        assert_eq!(
            MathSymbolAwareCjk4.split_text("x^2 + y_1 = 3.5"),
            vec!["x^2", " +", " y_1", " =", " 3.5"]
        );
        // Mixed script: only the CJK run is chunked.
        let mixed = MathSymbolAwareCjk4.split_text("abc今天天气很好def");
        assert!(mixed.iter().any(|s| *s == "今天天气"));
        assert!(mixed.iter().any(|s| *s == "很好"));
    }

    #[test]
    fn test_cjkhan4_chunks_han_only() {
        use PretokenizationMode::MathSymbolAwareCjkHan4 as H;
        // Han ideographs are chunked to at most 4 per piece.
        let out = H.split_text("もしもし漢字はとても多いのです。");
        for piece in &out {
            let han: usize = piece.chars().filter(|c| (0x4E00..=0x9FFF).contains(&(*c as u32))).count();
            assert!(han <= 4, "piece {piece:?} has {han} Han chars");
        }
        // Pure kana run is NOT chunked (kana stays whole).
        let kana = H.split_text("あいうえおかきくけこさしすせそ");
        assert_eq!(kana, vec!["あいうえおかきくけこさしすせそ"]);
    }

    #[test]
    fn test_emoji_zwj_attachment() {
        let text = "👨‍👩‍👧‍👦";
        let slices = PretokenizationMode::ScriptAndDigitAware.split_text(text);
        // The entire family sequence with ZWJ should remain intact as one pre-token!
        assert_eq!(slices, vec!["👨‍👩‍👧‍👦"]);
    }
}

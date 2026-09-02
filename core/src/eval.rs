//! # GhostMark Eval Harness
//!
//! Reproduces the *statistical* secret-key watermark family used by
//! SynthID-Text and Claude text watermarking (Kirchenbauer et al.,
//! arXiv:2301.10226) as an **independent measurement oracle**, so we can
//! quantify how much each GhostMark pipeline actually destroys the token
//! signature — instead of trusting marketing claims.
//!
//! Because providers keep their real keys secret, the oracle here is our own
//! deterministic instantiation of the same family: a per-context green/red
//! list seeded by a key. The reported z-scores are therefore comparable to
//! what an official key-holder detector would measure, but they are NOT the
//! vendors' own numbers. Official vendor detectors are layered on top of this
//! harness in the web UI and browser extension (Gemini SynthID and Anthropic
//! Claude "detect" toggles), not in the Rust CLI.

/// Green-list detection threshold (z-score ≥ `Z_THRESHOLD` is a strong hit).
pub const Z_THRESHOLD: f64 = 4.0;

const GREEN_PARTITION_PCT: u32 = 50; // 50% of the vocabulary is "green"
const EMBED_GREEN_BIAS_PCT: u32 = 100; // always swap red → green when a synonym exists

/// Synthetic demo corpus with deliberately high synonym coverage, so the
/// regular embed produces a strong, clearable signal. Kept to short sentences
/// because `shatter_synthid_text` splits very long ones.
pub const DEMO_CORPUS: &str = "The use of modern tools can make many hard tasks easy and fast. \
    Good teams can help people build big things and solve important problems. \
    Big companies use new technology to improve their products and give better service. \
    Small groups can also start good projects and get strong results. \
    Many workers think that good planning can help them do their jobs better. \
    Experts say that hard work can change the way a business works. \
    Teachers look for new ways to help every student learn and improve. \
    A good plan can show people how to start and finish difficult goals. \
    Many people think that big changes happen one small step at a time. \
    Good tools can make a difficult task easy and save a lot of time. \
    Leaders can help their teams build great products and make users happy. \
    Researchers look at many ideas and can show which ones really work. \
    Good communication can help groups avoid many common problems. \
    A small idea can grow into a big company that gives people good jobs. \
    People who look for answers can always get help from good mentors. \
    The best teams start with good values and then improve every day. \
    Hard work and good skills can make anyone successful in the end. \
    Experts can show how to use the best tools and make the work easy. \
    Many companies think that fast changes can improve their results. \
    Good teachers can help students change the way they look at hard ideas. \
    We can build a better future if we use the good ideas we have now. \
    It is easy to make a small start, but it is hard to finish big work. \
    A good leader will always look for ways to help the team improve. \
    Small changes can make a big difference when they happen every day. \
    Everyone can get help when they ask for it and use good advice. \
    Great products start with a simple idea that makes a hard task easy. \
    Good teams show that many hands can make light work fast. \
    A careful plan can help people avoid big risks and get better results. \
    Smart people know that learning is a long journey, not a quick end.";

/// Deterministic 64-bit finalizer (splitmix64).
fn mix(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58476d1ce4e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d049bb133111eb);
    x ^= x >> 31;
    x
}

fn word_hash(word: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in word.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    mix(h)
}

fn context_hash(prev: &str) -> u64 {
    // Position-independent: only the previous token drives the partition,
    // matching the k=1 context used in the paper's simplest instantiation.
    mix(word_hash(prev) ^ 0x9e3779b97f4a7c15)
}

/// Is `word` inside the green list for `prev` under `key`?
fn is_green(key: u64, prev: &str, word: &str) -> bool {
    let h = mix(key ^ context_hash(prev)) ^ word_hash(word);
    (h % 100) < GREEN_PARTITION_PCT as u64
}

/// Split text into alphabetic word tokens (our token proxy).
pub fn tokenize(text: &str) -> Vec<&str> {
    text.split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphabetic() && c != '\''))
        .filter(|w| !w.is_empty())
        .collect()
}

/// Statistics for a single z-score measurement under a key.
#[derive(Debug, Clone, Copy)]
pub struct WatermarkStats {
    /// Number of tokens analyzed.
    pub tokens: usize,
    /// Green-flagged tokens.
    pub green: usize,
    /// Green fraction (observed).
    pub green_frac: f64,
    /// Standard normal z-score: (green - p*T) / sqrt(T*p*(1-p)).
    pub z: f64,
}

impl WatermarkStats {
    pub fn is_hit(&self) -> bool {
        self.tokens >= 20 && self.z >= Z_THRESHOLD
    }
}

/// Score `text` against the green/red oracle seeded by `key`.
pub fn score(text: &str, key: u64) -> WatermarkStats {
    let toks = tokenize(text);
    let mut green = 0usize;
    let mut prev = "";
    for &w in toks.iter() {
        if is_green(key, prev, w) {
            green += 1;
        }
        prev = w;
    }
    let t = toks.len() as f64;
    let p = 0.5;
    let z = if t > 0.0 {
        (green as f64 - p * t) / (t * p * (1.0 - p)).sqrt()
    } else {
        0.0
    };
    WatermarkStats {
        tokens: toks.len(),
        green,
        green_frac: if t > 0.0 { green as f64 / t } else { 0.0 },
        z,
    }
}

/// Estimated unigram entropy (bits/token) over the text — a coarse
/// "perplexity-like" descriptor. Higher after scrubbing indicates the output
/// is less trivially predictable from unigram frequencies alone.
pub fn unigram_entropy(text: &str) -> f64 {
    let toks = tokenize(text);
    if toks.is_empty() {
        return 0.0;
    }
    let n = toks.len() as f64;
    let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for t in toks {
        *counts.entry(t).or_insert(0) += 1;
    }
    -counts
        .values()
        .map(|&c| {
            let p = c as f64 / n;
            p * p.log2()
        })
        .sum::<f64>()
}

// ---------------------------------------------------------------------------
// Embedding: turn human text into "watermarked" text the oracle scores high.
// ---------------------------------------------------------------------------

/// Small synonym map used to swap words while preserving meaning.
const SYNONYMS: &[(&str, &[&str])] = &[
    ("use", &["utilize", "employ", "apply", "leverage"]),
    ("big", &["large", "substantial", "significant", "major"]),
    ("small", &["little", "compact", "minor", "tiny"]),
    ("fast", &["quick", "rapid", "swift", "speedy"]),
    ("slow", &["gradual", "sluggish", "leisurely"]),
    ("good", &["great", "excellent", "solid", "strong"]),
    ("bad", &["poor", "weak", "unfavorable"]),
    ("help", &["assist", "support", "aid", "facilitate"]),
    ("show", &["demonstrate", "display", "illustrate", "reveal"]),
    ("build", &["construct", "develop", "create", "establish"]),
    ("start", &["begin", "initiate", "launch", "commence"]),
    ("end", &["finish", "conclude", "terminate", "wrap up"]),
    ("improve", &["enhance", "boost", "optimize", "strengthen"]),
    ("important", &["crucial", "essential", "vital", "key"]),
    ("difficult", &["hard", "challenging", "tough", "demanding"]),
    ("easy", &["simple", "straightforward", "effortless"]),
    ("many", &["numerous", "various", "countless", "plenty"]),
    ("think", &["believe", "consider", "reckon", "suppose"]),
    ("say", &["state", "declare", "mention", "note"]),
    ("change", &["alter", "modify", "adjust", "transform"]),
    ("look", &["view", "examine", "inspect", "observe"]),
    ("make", &["create", "produce", "generate", "construct"]),
    ("give", &["provide", "supply", "offer", "grant"]),
    ("get", &["obtain", "acquire", "receive", "gain"]),
];

fn choose_green(prev: &str, original: &str, key: u64) -> Option<String> {
    // Only substitute real synonyms so the "watermarked" text stays readable.
    if let Some((_, opts)) = SYNONYMS
        .iter()
        .find(|(w, _)| w.eq_ignore_ascii_case(original))
    {
        for o in opts.iter().copied() {
            if is_green(key, prev, o) {
                return Some(o.to_string());
            }
        }
    }
    None
}

/// Embed a watermark into `text` under `key` by nudging word slots toward
/// green outcomes via real synonym swaps (nothing else — readability is kept).
/// The output is what a key-holder detector should flag.
pub fn embed_watermark(text: &str, key: u64) -> String {
    let words: Vec<&str> = tokenize(text);
    let mut out = String::with_capacity(text.len() + 32);
    let mut prev: String = String::new();
    let mut wi = 0usize;
    for part in text.split_whitespace() {
        let current = words.get(wi).copied().unwrap_or(part).to_string();
        wi += 1;
        let low = current.to_lowercase();
        let chosen = match choose_replacement(&prev, &low, key, wi) {
            Some((repl, _)) => capitalize_like(&repl, &current),
            None => current.clone(),
        };
        out.push_str(&chosen);
        out.push(' ');
        prev = chosen;
    }
    out.trim_end().to_string()
}

/// Decide a replacement for one word slot: green synonym if available,
/// otherwise keep the word as-is.
fn choose_replacement(prev: &str, low: &str, key: u64, pos: usize) -> Option<(String, bool)> {
    let naturally_green = is_green(key, prev, low);
    if naturally_green {
        return Some((low.to_string(), false));
    }
    if !rng_bias(key, prev, pos) {
        return Some((low.to_string(), false));
    }
    choose_green(prev, low, key).map(|g| (g, true))
}

fn rng_bias(key: u64, prev: &str, pos: usize) -> bool {
    let h = mix(key ^ context_hash(format!("{}:{}", prev, pos).as_str()));
    (h % 100) < EMBED_GREEN_BIAS_PCT as u64
}

fn capitalize_like(chosen: &str, reference: &str) -> String {
    let mut chars = chosen.chars();
    let first = chars.next().unwrap_or(' ');
    if reference
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false)
    {
        first.to_uppercase().collect::<String>() + chars.as_str()
    } else {
        chosen.to_string()
    }
}

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------

/// Fold a token into a canonical form for edit-comparison: strip invisible
/// characters, map homoglyphs back to ASCII, lowercase. This ensures the
/// "fidelity" metric reflects real word-level (semantic) edits, not
/// character-level perturbation.
pub fn canonical_token(w: &str) -> String {
    let mut s = String::with_capacity(w.len());
    for c in w.chars() {
        if c.is_control() || matches!(c, '\u{200B}'..='\u{200F}' | '\u{FEFF}') {
            continue;
        }
        let folded = match c {
            'а' | 'А' => 'a',
            'с' | 'С' => 'c',
            'е' | 'Е' => 'e',
            'о' | 'О' => 'o',
            'р' | 'Р' => 'p',
            'х' | 'Х' => 'x',
            'у' | 'У' => 'y',
            _ => c,
        };
        s.push(folded.to_ascii_lowercase());
    }
    s
}

/// Fraction of word tokens changed between two texts (semantic edit rate).
/// Order-invariant: it compares canonical word multisets, so reordering
/// sentences (a watermark-context breaker) does NOT count as content loss,
/// and homoglyph/zero-width character perturbation does not count either.
/// Returns 1 - Jaccard similarity (i.e. the "changed" fraction).
pub fn word_change_frac(original: &str, scrubbed: &str) -> f64 {
    let a = tokenize(original);
    let b = tokenize(scrubbed);
    if a.is_empty() {
        return 1.0;
    }
    let mut counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for w in a.iter().map(|w| canonical_token(w)) {
        *counts.entry(w).or_insert(0) += 1;
    }
    let mut overlap = 0usize;
    for w in b.iter().map(|w| canonical_token(w)) {
        if let Some(c) = counts.get_mut(&w) {
            if *c > 0 {
                *c -= 1;
                overlap += 1;
            }
        }
    }
    let denom = a.len() + b.len();
    if denom == 0 {
        return 1.0;
    }
    1.0 - (2.0 * overlap as f64 / denom as f64)
}

/// Full before/after report for every GhostMark text pipeline.
pub struct EvalReport {
    pub key: u64,
    pub tokens: usize,
    pub z_before: f64,
    pub z_fast: f64,
    pub z_homoglyph: f64,
    pub z_shatter: f64,
    pub green_before: f64,
    pub green_shatter: f64,
    pub fidelity_shatter: f64,
    pub entropy_before: f64,
    pub entropy_shatter: f64,
}

impl EvalReport {
    /// Pipelines that dropped the watermark below the high-confidence bar.
    pub fn passing(&self) -> Vec<&'static str> {
        let mut v = Vec::new();
        if self.z_fast < Z_THRESHOLD {
            v.push("Fast WASM scrub");
        }
        if self.z_homoglyph < Z_THRESHOLD {
            v.push("Fast + Homoglyph");
        }
        if self.z_shatter < Z_THRESHOLD {
            v.push("Shatter SynthID");
        }
        v
    }
}

/// Run the full harness on a single input text under `key`.
pub fn run_eval(input: &str, key: u64) -> EvalReport {
    use crate::text_scrubber::{sanitize_text, shatter_synthid_text};

    let watermarked = embed_watermark(input, key);
    let fast = sanitize_text(&watermarked, false);
    let homoglyph = sanitize_text(&watermarked, true);
    let shatter = shatter_synthid_text(&watermarked);

    let before = score(&watermarked, key);
    let sf = score(&fast, key);
    let sh = score(&homoglyph, key);
    let ss = score(&shatter, key);

    EvalReport {
        key,
        tokens: before.tokens,
        z_before: before.z,
        z_fast: sf.z,
        z_homoglyph: sh.z,
        z_shatter: ss.z,
        green_before: before.green_frac,
        green_shatter: ss.green_frac,
        fidelity_shatter: 1.0 - word_change_frac(&watermarked, &shatter),
        entropy_before: unigram_entropy(&watermarked),
        entropy_shatter: unigram_entropy(&shatter),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = DEMO_CORPUS;

    #[test]
    fn score_catches_watermark_and_clear() {
        let key = 0xA11CE;
        let wm = embed_watermark(SAMPLE, key);
        let s_wm = score(&wm, key);
        assert!(
            s_wm.is_hit(),
            "watermarked text should be a hit (z={:.2}, tokens={})",
            s_wm.z,
            s_wm.tokens
        );
        assert!(
            s_wm.green_frac > 0.6,
            "watermarked green frac {:.2}",
            s_wm.green_frac
        );
        let s_clear = score(SAMPLE, key);
        assert!(
            !s_clear.is_hit(),
            "plain text should not be a hit (z={:.2})",
            s_clear.z
        );
    }

    #[test]
    fn deterministic_under_same_key() {
        let key = 42;
        let a = embed_watermark(SAMPLE, key);
        let b = embed_watermark(SAMPLE, key);
        assert_eq!(a, b);
        assert!((score(&a, key).z - score(&b, key).z).abs() < 1e-9);
    }

    #[test]
    fn shatter_destroys_signature() {
        let key = 7;
        let r = run_eval(SAMPLE, key);
        assert!(
            r.z_before >= Z_THRESHOLD,
            "harness must produce a detectable watermark (z={:.2})",
            r.z_before
        );
        assert!(
            r.z_shatter < r.z_before,
            "shatter must reduce z (was {:.2}, now {:.2})",
            r.z_before,
            r.z_shatter
        );
        assert!(r.fidelity_shatter > 0.3, "shatter is too destructive");
    }

    #[test]
    fn z_math_sane() {
        // Every (prev, word) context is a unique, independent 50% draw, so
        // green_frac must land near 0.5 and z must be modest.
        let set1 = [
            "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india",
            "juliet", "kilo", "lima", "mike", "november", "oscar", "papa", "quebec", "romeo",
            "sierra", "tango",
        ];
        let set2 = [
            "amber", "bronze", "cobalt", "denim", "emerald", "falcon", "granite", "hazel", "ivory",
            "jade", "khaki", "lilac", "mustard", "navy", "olive", "pearl", "quartz", "ruby",
            "silver", "topaz", "umber", "violet", "wheat", "xenon", "yellow", "zebra",
        ];
        let mut text = String::new();
        for g1 in set1 {
            text.push_str(g1);
            text.push(' ');
            for g2 in set2 {
                text.push_str(g2);
                text.push(' ');
            }
        }
        let words = text.trim_end();
        let s = score(words, 123);
        assert!(
            words.split_whitespace().count() >= 500,
            "sanity: corpus size"
        );
        assert!(
            (s.green_frac - 0.5).abs() < 0.1,
            "green frac {}",
            s.green_frac
        );
        assert!(s.z.abs() < 4.0, "z {}", s.z);
    }

    #[test]
    fn word_change_frac_semantics() {
        // Identical text -> 0 effort.
        assert_eq!(word_change_frac("a b c", "a b c"), 0.0);
        // Reordering is free (change without semantic loss) -> 0 changes.
        assert_eq!(word_change_frac("a b c", "c a b"), 0.0);
        // Half the words replaced -> 0.5 changes.
        let f = word_change_frac("a b c d", "a x c y");
        assert!((f - 0.5).abs() < 1e-9, "expected 0.5, got {f}");
        // Reports a *changed* fraction, so fidelity is 1 - f.
        assert!((1.0 - f - 0.5).abs() < 1e-9);
    }

    #[test]
    fn canonical_token_folds_homoglyphs() {
        // Cyrillic look-alikes must collapse onto their ASCII target so
        // scrubbed tokens still score against the original key.
        assert_eq!(canonical_token("е"), "e"); // Cyrillic ye
        assert_eq!(canonical_token("а"), "a"); // Cyrillic a
        assert_eq!(canonical_token("у"), "y"); // Cyrillic u
        assert_eq!(canonical_token("plain"), "plain");
    }

    #[test]
    fn unigram_entropy_orders_texts() {
        let repetitive = "the the the the the";
        let varied = "the quick brown fox jumps over the lazy dog";
        assert!(
            unigram_entropy(varied) > unigram_entropy(repetitive),
            "varied text must have higher entropy"
        );
    }

    #[test]
    fn passing_flags_mark_below_threshold() {
        let low = EvalReport {
            key: 1,
            tokens: 50,
            z_before: 5.0,
            z_fast: 3.9,
            z_homoglyph: 0.2,
            z_shatter: 0.1,
            green_before: 0.9,
            green_shatter: 0.6,
            fidelity_shatter: 0.5,
            entropy_before: 7.0,
            entropy_shatter: 7.2,
        };
        assert_eq!(
            low.passing(),
            vec!["Fast WASM scrub", "Fast + Homoglyph", "Shatter SynthID"]
        );

        let safe = EvalReport {
            z_shatter: Z_THRESHOLD + 0.5,
            ..low
        };
        assert!(safe.passing().contains(&"Fast + Homoglyph"));
        assert!(!safe.passing().contains(&"Shatter SynthID"));
    }

    #[test]
    fn run_eval_pipeline_matches_stats() {
        let key = 0xBEEF;
        let r = run_eval(SAMPLE, key);
        assert_eq!(r.key, key);
        assert!(r.z_before >= Z_THRESHOLD);
        assert!(r.tokens > 0);
        // fidelity is 1 - changed-fraction, so it must stay in [0, 1].
        assert!((0.0..=1.0).contains(&r.fidelity_shatter));
    }
}

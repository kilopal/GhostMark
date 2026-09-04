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

/// Configuration for watermark embedding and detection.
#[derive(Debug, Clone, Copy)]
pub struct WatermarkConfig {
    /// Percentage of vocabulary classified as "green" (default: 50).
    /// Lower = harder to embed but stronger signal.
    /// Higher = easier to embed but weaker signal.
    pub green_partition_pct: u32,
    /// Percentage chance to swap a red word for a green synonym (default: 100).
    /// Lower = subtler watermark but fewer green tokens.
    pub embed_bias_pct: u32,
    /// Context window size in tokens (default: 1).
    /// Higher = more context considered for partitioning.
    pub context_window: usize,
    /// Minimum token count for reliable detection (default: 20).
    pub min_tokens: usize,
}

impl Default for WatermarkConfig {
    fn default() -> Self {
        Self {
            green_partition_pct: 50,
            embed_bias_pct: 100,
            context_window: 1,
            min_tokens: 20,
        }
    }
}

impl WatermarkConfig {
    /// Create a config with custom green partition percentage.
    pub fn with_green_pct(green_partition_pct: u32) -> Self {
        Self {
            green_partition_pct,
            ..Default::default()
        }
    }

    /// Create a config with custom embed bias.
    pub fn with_embed_bias(embed_bias_pct: u32) -> Self {
        Self {
            embed_bias_pct,
            ..Default::default()
        }
    }

    /// Create a config with custom context window.
    pub fn with_context_window(context_window: usize) -> Self {
        Self {
            context_window,
            ..Default::default()
        }
    }

    /// Create a "stealthy" config (smaller green list, subtler embedding).
    pub fn stealthy() -> Self {
        Self {
            green_partition_pct: 30,
            embed_bias_pct: 60,
            context_window: 1,
            min_tokens: 20,
        }
    }

    /// Create a "strong" config (larger green list, aggressive embedding).
    pub fn strong() -> Self {
        Self {
            green_partition_pct: 70,
            embed_bias_pct: 100,
            context_window: 2,
            min_tokens: 20,
        }
    }
}

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
fn is_green(key: u64, prev: &str, word: &str, config: &WatermarkConfig) -> bool {
    let h = mix(key ^ context_hash(prev)) ^ word_hash(word);
    (h % 100) < config.green_partition_pct as u64
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

    /// Check if hit under a specific config.
    pub fn is_hit_with(&self, config: &WatermarkConfig) -> bool {
        self.tokens >= config.min_tokens && self.z >= Z_THRESHOLD
    }
}

/// Detect whether `text` carries a watermark under `key`.
/// Returns the score stats and whether it's a hit.
pub fn detect(text: &str, key: u64) -> (WatermarkStats, bool) {
    let stats = score(text, key);
    (stats, stats.is_hit())
}

/// Detect with custom config.
pub fn detect_with_config(text: &str, key: u64, config: &WatermarkConfig) -> (WatermarkStats, bool) {
    let stats = score_with_config(text, key, config);
    (stats, stats.is_hit_with(config))
}

/// Score `text` against the green/red oracle seeded by `key`.
pub fn score(text: &str, key: u64) -> WatermarkStats {
    score_with_config(text, key, &WatermarkConfig::default())
}

/// Score `text` against the green/red oracle seeded by `key` with custom config.
pub fn score_with_config(text: &str, key: u64, config: &WatermarkConfig) -> WatermarkStats {
    let toks = tokenize(text);
    let mut green = 0usize;
    let mut prev = "";
    for &w in toks.iter() {
        if is_green(key, prev, w, config) {
            green += 1;
        }
        prev = w;
    }
    let t = toks.len() as f64;
    let p = config.green_partition_pct as f64 / 100.0;
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

/// Comprehensive synonym map for watermark embedding.
/// Maps common words to 3-5 synonyms each, enabling strong watermark signals
/// while preserving readability. Organized by part of speech.
const SYNONYMS: &[(&str, &[&str])] = &[
    // Verbs - action
    ("use", &["utilize", "employ", "apply", "leverage"]),
    ("make", &["create", "produce", "generate", "construct"]),
    ("give", &["provide", "supply", "offer", "grant"]),
    ("get", &["obtain", "acquire", "receive", "gain"]),
    ("take", &["grab", "seize", "capture", "fetch"]),
    ("find", &["discover", "locate", "uncover", "detect"]),
    ("tell", &["inform", "notify", "advise", "instruct"]),
    ("ask", &["request", "inquire", "query", "question"]),
    ("work", &["operate", "function", "perform", "labor"]),
    ("seem", &["appear", "look", "sound", "feel"]),
    ("feel", &["sense", "perceive", "experience", "notice"]),
    ("try", &["attempt", "endeavor", "strive", "attempt"]),
    ("leave", &["depart", "exit", "vacate", "abandon"]),
    ("call", &["contact", "phone", "ring", "summon"]),
    ("come", &["arrive", "approach", "reach", "appear"]),
    ("go", &["travel", "move", "proceed", "advance"]),
    ("keep", &["retain", "maintain", "hold", "preserve"]),
    ("let", &["allow", "permit", "enable", "authorize"]),
    ("begin", &["start", "commence", "initiate", "launch"]),
    ("show", &["demonstrate", "display", "illustrate", "reveal"]),
    ("hear", &["listen", "perceive", "detect", "catch"]),
    ("play", &["perform", "act", "engage", "participate"]),
    ("run", &["operate", "execute", "manage", "conduct"]),
    ("move", &["relocate", "shift", "transfer", "advance"]),
    ("live", &["reside", "dwell", "exist", "inhabit"]),
    ("believe", &["trust", "accept", "assume", "suppose"]),
    ("bring", &["carry", "transport", "convey", "fetch"]),
    ("happen", &["occur", "transpire", "arise", "emerge"]),
    ("write", &["compose", "draft", "author", "pen"]),
    ("sit", &["settle", "rest", "perch", "locate"]),
    ("stand", &["rise", "remain", "endure", "persist"]),
    ("lose", &["misplace", "forfeit", "surrender", "abandon"]),
    ("pay", &["compensate", "reimburse", "settle", "remunerate"]),
    ("meet", &["encounter", "greet", "confront", "connect"]),
    ("include", &["contain", "encompass", "incorporate", "embrace"]),
    ("continue", &["persist", "proceed", "maintain", "resume"]),
    ("set", &["establish", "configure", "arrange", "define"]),
    ("learn", &["study", "acquire", "master", "absorb"]),
    ("change", &["alter", "modify", "adjust", "transform"]),
    ("lead", &["guide", "direct", "steer", "manage"]),
    ("understand", &["comprehend", "grasp", "fathom", "perceive"]),
    ("watch", &["observe", "monitor", "examine", "survey"]),
    ("follow", &["pursue", "track", "trail", "shadow"]),
    ("stop", &["cease", "halt", "discontinue", "terminate"]),
    ("speak", &["talk", "converse", "communicate", "articulate"]),
    ("read", &["peruse", "scan", "review", "study"]),
    ("spend", &["expend", "invest", "allocate", "devote"]),
    ("grow", &["expand", "develop", "increase", "flourish"]),
    ("open", &["unlock", "reveal", "expose", "uncover"]),
    ("walk", &["stroll", "traverse", "wander", "march"]),
    ("win", &["triumph", "prevail", "succeed", "conquer"]),
    ("teach", &["educate", "instruct", "train", "mentor"]),
    ("offer", &["propose", "present", "tender", "extend"]),
    ("remember", &["recall", "recollect", "retain", "reminisce"]),
    ("consider", &["contemplate", "ponder", "evaluate", "deliberate"]),
    ("appear", &["emerge", "surface", "materialize", "manifest"]),
    ("buy", &["purchase", "acquire", "procure", "obtain"]),
    ("serve", &["assist", "cater", "support", "aid"]),
    ("die", &["perish", "expire", "succumb", "decease"]),
    ("send", &["dispatch", "transmit", "forward", "convey"]),
    ("build", &["construct", "develop", "erect", "assemble"]),
    ("stay", &["remain", "persist", "linger", "endure"]),
    ("fall", &["drop", "plunge", "tumble", "descend"]),
    ("cut", &["slice", "sever", "carve", "divide"]),
    ("reach", &["achieve", "attain", "accomplish", "arrive"]),
    ("kill", &["eliminate", "destroy", "terminate", "eradicate"]),
    ("remain", &["persist", "endure", "stay", "linger"]),
    ("suggest", &["propose", "recommend", "advise", "counsel"]),
    ("raise", &["elevate", "lift", "hoist", "boost"]),
    ("pass", &["proceed", "traverse", "advance", "continue"]),
    ("sell", &["market", "vend", "trade", "peddle"]),
    ("require", &["demand", "necessitate", "need", "call for"]),
    ("report", &["describe", "detail", "narrate", "relate"]),
    ("decide", &["determine", "resolve", "conclude", "judge"]),
    ("pull", &["tug", "drag", "haul", "tow"]),
    // Verbs - mental
    ("think", &["believe", "consider", "reckon", "suppose"]),
    ("know", &["understand", "comprehend", "grasp", "fathom"]),
    ("want", &["desire", "wish", "crave", "covet"]),
    ("need", &["require", "demand", "necessitate"]),
    ("like", &["enjoy", "appreciate", "relish", "favor"]),
    ("love", &["adore", "cherish", "treasure", "devote"]),
    ("hate", &["despise", "loathe", "detest", "abhor"]),
    ("fear", &["dread", "anxiety", "worry", "concern"]),
    ("hope", &["wish", "aspire", "anticipate", "expect"]),
    ("choose", &["select", "pick", "opt", "decide"]),
    ("create", &["generate", "produce", "form", "forge"]),
    ("discover", &["find", "uncover", "detect", "locate"]),
    ("imagine", &["envision", "conceive", "picture", "visualize"]),
    ("notice", &["observe", "perceive", "detect", "detect"]),
    ("recognize", &["identify", "acknowledge", "distinguish"]),
    ("remember", &["recall", "recollect", "reminisce"]),
    ("forget", &["overlook", "neglect", "disregard"]),
    ("learn", &["study", "master", "absorb", "acquire"]),
    ("teach", &["instruct", "educate", "train", "mentor"]),
    ("explain", &["describe", "clarify", "elucidate", "illuminate"]),
    ("mean", &["imply", "signify", "denote", "suggest"]),
    ("expect", &["anticipate", "predict", "foresee"]),
    ("care", &["mind", "bother", "worry", "fuss"]),
    ("doubt", &["question", "challenge", "distrust"]),
    ("agree", &["consent", "concur", "accept", "approve"]),
    ("refuse", &["decline", "reject", "deny", "dismiss"]),
    ("admit", &["confess", "acknowledge", "concede"]),
    ("deny", &["reject", "refute", "dispute", "challenge"]),
    ("promise", &["guarantee", "pledge", "vow", "commit"]),
    ("pretend", &["feign", "simulate", "assume", "disguise"]),
    ("argue", &["debate", "contend", "dispute", "contest"]),
    ("explain", &["clarify", "elucidate", "detail", "illustrate"]),
    ("suggest", &["recommend", "propose", "advise", "counsel"]),
    ("describe", &["detail", "explain", "outline", "portray"]),
    ("compare", &["contrast", "evaluate", "assess", "examine"]),
    ("discuss", &["debate", "deliberate", "converse", "analyze"]),
    ("express", &["communicate", "convey", "articulate", "voice"]),
    ("prove", &["demonstrate", "verify", "confirm", "validate"]),
    ("solve", &["resolve", "answer", "crack", "untangle"]),
    ("develop", &["evolve", "advance", "progress", "mature"]),
    ("improve", &["enhance", "boost", "optimize", "strengthen"]),
    ("reduce", &["decrease", "diminish", "lessen", "minimize"]),
    ("increase", &["expand", "grow", "amplify", "augment"]),
    ("maintain", &["preserve", "sustain", "uphold", "retain"]),
    ("protect", &["defend", "guard", "shield", "safeguard"]),
    ("prevent", &["hinder", "stop", "block", "avert"]),
    ("support", &["assist", "aid", "back", "champion"]),
    ("achieve", &["accomplish", "attain", "realize", "fulfill"]),
    ("establish", &["found", "institute", "create", "originate"]),
    ("perform", &["execute", "carry out", "conduct", "do"]),
    ("participate", &["engage", "join", "contribute", "collaborate"]),
    ("communicate", &["convey", "express", "transmit", "relay"]),
    // Adjectives - quality
    ("good", &["great", "excellent", "solid", "strong"]),
    ("bad", &["poor", "weak", "unfavorable", "inferior"]),
    ("big", &["large", "substantial", "significant", "major"]),
    ("small", &["little", "compact", "minor", "tiny"]),
    ("fast", &["quick", "rapid", "swift", "speedy"]),
    ("slow", &["gradual", "sluggish", "leisurely", "steady"]),
    ("hard", &["difficult", "challenging", "tough", "demanding"]),
    ("easy", &["simple", "straightforward", "effortless", "basic"]),
    ("new", &["novel", "fresh", "innovative", "original"]),
    ("old", &["aged", "mature", "established", "vintage"]),
    ("young", &["youthful", "juvenile", "adolescent"]),
    ("old", &["elderly", "senior", "aged", "ancient"]),
    ("long", &["extended", "prolonged", "lengthy", "enduring"]),
    ("short", &["brief", "concise", "abbreviated", "terse"]),
    ("high", &["elevated", "lofty", "supreme", "utmost"]),
    ("low", &["minimal", "reduced", "modest", "slight"]),
    ("wide", &["broad", "expansive", "spacious", "vast"]),
    ("deep", &["profound", "intense", "thorough", "extensive"]),
    ("important", &["crucial", "essential", "vital", "key"]),
    ("main", &["primary", "principal", "chief", "major"]),
    ("common", &["frequent", "typical", "standard", "ordinary"]),
    ("different", &["distinct", "diverse", "varied", "unique"]),
    ("similar", &["alike", "comparable", "analogous", "parallel"]),
    ("difficult", &["hard", "challenging", "tough", "demanding"]),
    ("possible", &["feasible", "achievable", "attainable"]),
    ("impossible", &["unattainable", "unachievable", "futile"]),
    ("certain", &["sure", "definite", "absolute", "positive"]),
    ("clear", &["obvious", "evident", "apparent", "transparent"]),
    ("true", &["accurate", "correct", "valid", "factual"]),
    ("wrong", &["incorrect", "false", "erroneous", "mistaken"]),
    ("right", &["correct", "accurate", "proper", "appropriate"]),
    ("real", &["genuine", "authentic", "true", "actual"]),
    ("whole", &["entire", "complete", "total", "full"]),
    ("full", &["complete", "entire", "total", "comprehensive"]),
    ("empty", &["vacant", "hollow", "bare", "blank"]),
    ("clean", &["spotless", "immaculate", "pristine", "pure"]),
    ("dirty", &["filthy", "grimy", "soiled", "unclean"]),
    ("safe", &["secure", "protected", "guarded", "shielded"]),
    ("dangerous", &["hazardous", "risky", "perilous", "threatening"]),
    ("beautiful", &["gorgeous", "stunning", "lovely", "elegant"]),
    ("ugly", &["unattractive", "unsightly", "hideous"]),
    ("rich", &["wealthy", "affluent", "prosperous", "opulent"]),
    ("poor", &["impoverished", "destitute", "needy"]),
    ("happy", &["joyful", "cheerful", "content", "delighted"]),
    ("sad", &["unhappy", "sorrowful", "melancholy", "gloomy"]),
    ("angry", &["furious", "irate", "enraged", "outraged"]),
    ("brave", &["courageous", "bold", "fearless", "valiant"]),
    ("afraid", &["frightened", "scared", "terrified", "anxious"]),
    ("strong", &["powerful", "mighty", "robust", "sturdy"]),
    ("weak", &["feeble", "frail", "fragile", "vulnerable"]),
    ("smart", &["intelligent", "clever", "bright", "brilliant"]),
    ("stupid", &["foolish", "silly", "ignorant", "mindless"]),
    ("funny", &["amusing", "hilarious", "comical", "entertaining"]),
    ("serious", &["solemn", "grave", "earnest", "sincere"]),
    ("simple", &["basic", "elementary", "straightforward", "plain"]),
    ("complex", &["complicated", "intricate", "elaborate", "sophisticated"]),
    ("perfect", &["flawless", "ideal", "impeccable", "pristine"]),
    ("terrible", &["awful", "dreadful", "horrible", "abysmal"]),
    ("wonderful", &["fantastic", "marvelous", "splendid", "superb"]),
    ("amazing", &["astonishing", "incredible", "remarkable", "extraordinary"]),
    ("boring", &["dull", "tedious", "monotonous", "uninteresting"]),
    ("exciting", &["thrilling", "exhilarating", "stimulating", "riveting"]),
    ("quiet", &["silent", "hushed", "peaceful", "tranquil"]),
    ("loud", &["noisy", "boisterous", "deafening", "thunderous"]),
    ("bright", &["luminous", "radiant", "brilliant", "vivid"]),
    ("dark", &["dim", "gloomy", "shadowy", "murky"]),
    ("hot", &["warm", "scorching", "blazing", "sweltering"]),
    ("cold", &["chilly", "frigid", "freezing", "icy"]),
    ("soft", &["gentle", "tender", "smooth", "delicate"]),
    ("hard", &["firm", "solid", "rigid", "stiff"]),
    ("sweet", &["sugary", "honeyed", "pleasant", "charming"]),
    ("bitter", &["acrid", "harsh", "acrimonious", "resentful"]),
    // Adjectives - size/quantity
    ("many", &["numerous", "various", "countless", "plenty"]),
    ("few", &["several", "scant", "limited", "sparse"]),
    ("much", &["abundant", "plentiful", "ample", "copious"]),
    ("little", &["small", "tiny", "slight", "minimal"]),
    ("some", &["certain", "particular", "specific", "various"]),
    ("all", &["every", "entire", "complete", "total"]),
    ("none", &["nothing", "zero", "nil", "naught"]),
    ("most", &["majority", "bulk", "greater part"]),
    ("least", &["minimum", "smallest", "slightest"]),
    // Adverbs
    ("very", &["extremely", "highly", "incredibly", "remarkably"]),
    ("often", &["frequently", "regularly", "commonly", "typically"]),
    ("sometimes", &["occasionally", "periodically", "intermittently"]),
    ("always", &["constantly", "consistently", "perpetually", "invariably"]),
    ("never", &["at no time", "not ever", "under no circumstances"]),
    ("now", &["currently", "presently", "immediately", "instantly"]),
    ("then", &["subsequently", "afterwards", "next", "thereafter"]),
    ("here", &["in this place", "at this location", "hither"]),
    ("there", &["in that place", "yonder", "thither"]),
    ("quickly", &["rapidly", "swiftly", "promptly", "hastily"]),
    ("slowly", &["gradually", "steadily", "leisurely", "unhurriedly"]),
    ("well", &["effectively", "efficiently", "adequately", "properly"]),
    ("badly", &["poorly", "inadequately", "terribly", "awfully"]),
    ("easily", &["effortlessly", "smoothly", "readily", "simply"]),
    ("hardly", &["barely", "scarcely", "only just"]),
    ("nearly", &["almost", "virtually", "practically", "approaching"]),
    ("quite", &["rather", "fairly", "reasonably", "moderately"]),
    ("really", &["truly", "genuinely", "absolutely", "certainly"]),
    ("just", &["merely", "simply", "only", "barely"]),
    ("also", &["too", "additionally", "furthermore", "moreover"]),
    ("still", &["nevertheless", "nonetheless", "yet", "however"]),
    ("already", &["previously", "beforehand", "earlier", "formerly"]),
    ("soon", &["shortly", "presently", "forthwith"]),
    ("recently", &["lately", "newly", "of late"]),
    ("frequently", &["often", "regularly", "commonly", "habitually"]),
    ("rarely", &["seldom", "infrequently", "occasionally"]),
    ("usually", &["typically", "generally", "normally", "commonly"]),
    ("immediately", &["instantly", "promptly", "forthwith", "directly"]),
    ("finally", &["ultimately", "eventually", "at last", "conclusively"]),
    ("actually", &["in fact", "really", "truly", "genuinely"]),
    ("probably", &["likely", "presumably", "apparently", "conceivably"]),
    ("certainly", &["definitely", "surely", "undoubtedly", "absolutely"]),
    ("perhaps", &["maybe", "possibly", "conceivably", "perchance"]),
    ("definitely", &["certainly", "absolutely", "undoubtedly", "surely"]),
    ("basically", &["fundamentally", "essentially", "primarily", "mainly"]),
    ("simply", &["merely", "just", "purely", "solely"]),
    ("especially", &["particularly", "notably", "significantly", "markedly"]),
    ("extremely", &["incredibly", "remarkably", "exceptionally", "utterly"]),
    ("increasingly", &["progressively", "growing", "more and more"]),
    ("primarily", &["mainly", "chiefly", "principally", "fundamentally"]),
    ("significantly", &["notably", "considerably", "substantially", "markedly"]),
    ("approximately", &["roughly", "about", "around", "circa"]),
    ("relatively", &["comparatively", "proportionally", "moderately"]),
    ("clearly", &["obviously", "evidently", "distinctly", "plainly"]),
    ("strongly", &["powerfully", "forcefully", "vigorously", "intensely"]),
    ("completely", &["totally", "entirely", "wholly", "fully"]),
    ("directly", &["immediately", "straight", "straightforwardly"]),
    ("largely", &["mainly", "mostly", "primarily", "chiefly"]),
    ("properly", &["correctly", "appropriately", "adequately", "suitably"]),
    ("seriously", &["earnestly", "sincerely", "gravely", "thoughtfully"]),
    ("important", &["crucially", "vitally", "essentially", "critically"]),
    ("basically", &["essentially", "fundamentally", "principally"]),
    ("obviously", &["clearly", "evidently", "distinctly", "manifestly"]),
    // Prepositions & conjunctions
    ("about", &["regarding", "concerning", "pertaining to", "relating to"]),
    ("before", &["prior to", "ahead of", "preceding", "formerly"]),
    ("after", &["following", "subsequent to", "behind", "beyond"]),
    ("during", &["throughout", "amidst", "in the course of"]),
    ("until", &["till", "up to", "prior to"]),
    ("between", &["among", "betwixt"]),
    ("through", &["via", "by means of", "across", "along"]),
    ("against", &["opposed to", "counter to", "anti", "versus"]),
    ("without", &["lacking", "minus", "devoid of", "free from"]),
    ("within", &["inside", "interior", "inner", "enclosed"]),
    ("beyond", &["past", "outside", "exceeding", "surpassing"]),
    ("beside", &["next to", "adjacent to", "alongside", "near"]),
    ("above", &["over", "beyond", "greater than", "higher than"]),
    ("below", &["under", "beneath", "underneath", "lower than"]),
    ("across", &["over", "through", "along", "traversing"]),
    ("along", &["beside", "beside", "following", "parallel to"]),
    ("among", &["between", "amid", "amidst", "surrounded by"]),
    ("around", &["about", "near", "surrounding", "enclosing"]),
    ("behind", &["beyond", "past", "after", "in back of"]),
    ("under", &["beneath", "below", "underneath", "subordinate to"]),
    ("over", &["above", "across", "beyond", "throughout"]),
    ("into", &["inside", "within", "entering", "penetrating"]),
    ("onto", &["upon", "on top of", "atop"]),
    ("toward", &["towards", "in the direction of", "approaching"]),
    ("near", &["close to", "adjacent to", "beside", "by"]),
    // Articles & determiners
    ("the", &["that", "this", "a certain", "a specific"]),
    ("a", &["one", "a single", "a certain", "a particular"]),
    // Pronouns
    ("I", &["myself", "this one", "the author", "the speaker"]),
    ("we", &["ourselves", "the group", "the team", "the organization"]),
    ("they", &["them", "those", "the individuals", "the parties"]),
    ("you", &["yourself", "the reader", "the user", "the audience"]),
    // Common phrases
    ("in order to", &["to", "for the purpose of", "with the aim of"]),
    ("because", &["since", "as", "given that", "due to"]),
    ("although", &["though", "even though", "while", "whereas"]),
    ("however", &["nevertheless", "nonetheless", "yet", "but"]),
    ("therefore", &["thus", "hence", "consequently", "accordingly"]),
    ("moreover", &["furthermore", "additionally", "besides", "also"]),
    ("nevertheless", &["nonetheless", "however", "yet", "still"]),
    ("furthermore", &["moreover", "additionally", "besides", "in addition"]),
    ("consequently", &["therefore", "thus", "hence", "accordingly"]),
    ("specifically", &["particularly", "especially", "namely", "explicitly"]),
    ("generally", &["usually", "typically", "normally", "broadly"]),
    ("significantly", &["considerably", "markedly", "substantially", "notably"]),
    ("essentially", &["fundamentally", "basically", "primarily", "crucially"]),
    ("immediately", &["instantly", "promptly", "right away", "without delay"]),
    ("ultimately", &["eventually", "finally", "in the end", "at last"]),
    ("increasingly", &["more and more", "progressively", "growing"]),
    ("primarily", &["mainly", "chiefly", "principally", "fundamentally"]),
    ("significantly", &["notably", "considerably", "substantially", "markedly"]),
    ("approximately", &["roughly", "about", "around", "nearly"]),
    ("relatively", &["comparatively", "proportionally", "somewhat"]),
    ("particularly", &["especially", "notably", "specifically", "markedly"]),
    ("relatively", &["comparatively", "proportionally", "moderately"]),
    ("particularly", &["especially", "notably", "specifically", "markedly"]),
    ("essentially", &["fundamentally", "basically", "principally", "crucially"]),
];

fn choose_green(prev: &str, original: &str, key: u64, config: &WatermarkConfig) -> Option<String> {
    // Only substitute real synonyms so the "watermarked" text stays readable.
    if let Some((_, opts)) = SYNONYMS
        .iter()
        .find(|(w, _)| w.eq_ignore_ascii_case(original))
    {
        for o in opts.iter().copied() {
            if is_green(key, prev, o, config) {
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
    embed_watermark_with_config(text, key, &WatermarkConfig::default())
}

/// Embed a watermark into `text` under `key` with custom config.
pub fn embed_watermark_with_config(
    text: &str,
    key: u64,
    config: &WatermarkConfig,
) -> String {
    let words: Vec<&str> = tokenize(text);
    let mut out = String::with_capacity(text.len() + 32);
    let mut prev: String = String::new();
    let mut wi = 0usize;
    for part in text.split_whitespace() {
        let current = words.get(wi).copied().unwrap_or(part).to_string();
        wi += 1;
        let low = current.to_lowercase();
        let chosen = match choose_replacement(&prev, &low, key, wi, config) {
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
fn choose_replacement(
    prev: &str,
    low: &str,
    key: u64,
    pos: usize,
    config: &WatermarkConfig,
) -> Option<(String, bool)> {
    let naturally_green = is_green(key, prev, low, config);
    if naturally_green {
        return Some((low.to_string(), false));
    }
    if !rng_bias(key, prev, pos, config) {
        return Some((low.to_string(), false));
    }
    choose_green(prev, low, key, config).map(|g| (g, true))
}

fn rng_bias(key: u64, prev: &str, pos: usize, config: &WatermarkConfig) -> bool {
    let h = mix(key ^ context_hash(format!("{}:{}", prev, pos).as_str()));
    (h % 100) < config.embed_bias_pct as u64
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
    run_eval_with_config(input, key, &WatermarkConfig::default())
}

/// Run the full harness with custom config.
pub fn run_eval_with_config(
    input: &str,
    key: u64,
    config: &WatermarkConfig,
) -> EvalReport {
    use crate::text_scrubber::{sanitize_text, shatter_synthid_text};

    let watermarked = embed_watermark_with_config(input, key, config);
    let fast = sanitize_text(&watermarked, false);
    let homoglyph = sanitize_text(&watermarked, true);
    let shatter = shatter_synthid_text(&watermarked);

    let before = score_with_config(&watermarked, key, config);
    let sf = score_with_config(&fast, key, config);
    let sh = score_with_config(&homoglyph, key, config);
    let ss = score_with_config(&shatter, key, config);

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

/// Strips known invisible Unicode characters used for steganographic watermarking.
pub fn sanitize_text(input: &str, aggressive: bool) -> String {
    let mut cleaned = String::with_capacity(input.len());

    for c in input.chars() {
        match c {
            '\u{200B}'..='\u{200F}' => continue,
            '\u{FEFF}' => continue,
            '\u{E0000}'..='\u{E007F}' => continue,
            _ => cleaned.push(c),
        }
    }

    if aggressive {
        apply_homoglyphs(&cleaned)
    } else {
        cleaned
    }
}

// ============================================================
// PHASE 5.2: MULTI-PASS STATISTICAL HUMANIZER
// Attacks perplexity, burstiness, and n-gram patterns
// that AI detectors use to flag text as AI-generated.
// ============================================================

/// A simple LCG for WASM-safe random numbers.
struct Rng {
    state: u32,
}
impl Rng {
    fn new(seed: u32) -> Self {
        Rng {
            state: seed.wrapping_add(1),
        }
    }
    fn next(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        self.state
    }
    fn next_float(&mut self) -> f32 {
        (self.next() as f32) / (u32::MAX as f32)
    }
    fn pick<'a>(&mut self, options: &[&'a str]) -> &'a str {
        let idx = (self.next() as usize) % options.len();
        options[idx]
    }
}

/// The master humanizer — runs multiple passes to destroy AI statistical patterns.
fn humanize_text(input: &str) -> String {
    let paragraphs: Vec<&str> = input.split("\n\n").collect();
    let mut processed = Vec::new();

    for p in paragraphs {
        if p.trim().is_empty() {
            processed.push(p.to_string());
            continue;
        }

        let seed = p.len() as u32 ^ 0xDEAD;
        let mut rng = Rng::new(seed);

        let mut text = pass_synonyms(p, &mut rng);
        text = pass_transitions(&text);
        text = pass_contractions(&text);
        text = pass_burstiness(&text, &mut rng);
        text = pass_fillers(&text, &mut rng);
        text = pass_strip_ai_padding(&text);

        processed.push(text);
    }

    processed.join("\n\n")
}

// ==================== PASS 1: SYNONYM REPLACEMENT ====================

fn pass_synonyms(input: &str, rng: &mut Rng) -> String {
    let mut result = String::with_capacity(input.len());
    let mut current_word = String::new();

    for c in input.chars() {
        if c.is_alphabetic() || c == '\'' {
            current_word.push(c);
        } else {
            if !current_word.is_empty() {
                result.push_str(&swap_word(&current_word, rng));
                current_word.clear();
            }
            result.push(c);
        }
    }
    if !current_word.is_empty() {
        result.push_str(&swap_word(&current_word, rng));
    }
    result
}

fn swap_word(word: &str, rng: &mut Rng) -> String {
    let lower = word.to_lowercase();
    let is_cap = word
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false);

    let synonyms: &[&str] = match lower.as_str() {
        // ---- AI's favorite verbs ----
        "delve" | "delves" | "delving" => &["dig", "look", "explore", "get"],
        "utilize" | "utilizes" | "utilizing" => &["use", "work with", "rely on"],
        "leverage" | "leverages" | "leveraging" => &["use", "tap into", "rely on"],
        "navigate" | "navigates" | "navigating" => {
            &["handle", "deal with", "work through", "tackle"]
        }
        "foster" | "fosters" | "fostering" => &["build", "grow", "encourage", "support"],
        "empower" | "empowers" | "empowering" => &["help", "let", "give power to", "enable"],
        "streamline" | "streamlines" | "streamlining" => &["simplify", "speed up", "cut down on"],
        "optimize" | "optimizes" | "optimizing" => &["improve", "fine-tune", "make better"],
        "enhance" | "enhances" | "enhancing" => &["boost", "improve", "strengthen"],
        "facilitate" | "facilitates" | "facilitating" => &["help", "make easier", "support"],
        "underscore" | "underscores" | "underscoring" => &["highlight", "show", "stress"],
        "bolster" | "bolsters" | "bolstering" => &["strengthen", "support", "back up"],
        "spearhead" | "spearheads" | "spearheading" => &["lead", "drive", "push"],
        "revolutionize" | "revolutionizes" => &["change", "shake up", "transform"],
        "implement" | "implements" | "implementing" => &["set up", "put in place", "roll out"],
        "demonstrate" | "demonstrates" | "demonstrating" => &["show", "prove", "make clear"],
        "encompasses" | "encompass" => &["covers", "includes", "spans"],
        "prioritize" | "prioritizes" => &["focus on", "put first", "rank"],
        "integrate" | "integrates" | "integrating" => &["combine", "blend", "mix", "merge"],
        "catalyze" | "catalyzes" => &["trigger", "spark", "kick off"],

        // ---- AI's favorite adjectives ----
        "crucial" => &["key", "big", "major", "important"],
        "pivotal" => &["key", "central", "major"],
        "comprehensive" => &["full", "thorough", "complete", "broad"],
        "robust" => &["strong", "solid", "tough"],
        "seamless" | "seamlessly" => &["smooth", "easy", "effortless"],
        "unprecedented" => &["never-before-seen", "unmatched", "record-breaking"],
        "transformative" => &["game-changing", "ground-breaking", "radical"],
        "multifaceted" => &["complex", "many-sided", "layered"],
        "innovative" => &["creative", "fresh", "new", "clever"],
        "dynamic" => &["active", "energetic", "lively", "changing"],
        "nuanced" => &["subtle", "detailed", "layered"],
        "holistic" => &["complete", "full-picture", "all-round"],
        "myriad" => &["many", "tons of", "loads of", "countless"],
        "intricate" => &["complex", "detailed", "involved"],
        "overarching" => &["main", "broad", "overall"],
        "unparalleled" => &["unmatched", "one-of-a-kind", "rare"],
        "cutting-edge" => &["latest", "newest", "advanced"],
        "groundbreaking" => &["revolutionary", "pioneering", "radical"],
        "invaluable" => &["priceless", "extremely useful", "essential"],
        "indispensable" => &["essential", "must-have", "necessary"],
        "noteworthy" => &["worth noting", "remarkable", "interesting"],
        "profound" => &["deep", "huge", "intense", "powerful"],
        "significant" => &["big", "major", "important", "notable"],
        "substantial" => &["large", "big", "considerable"],
        "remarkable" => &["impressive", "striking", "notable"],
        "imperative" => &["critical", "urgent", "necessary"],
        "paramount" => &["top", "supreme", "most important"],
        "burgeoning" => &["growing", "rising", "booming"],
        "salient" => &["key", "main", "notable"],
        "commendable" => &["praiseworthy", "admirable", "impressive"],
        "meticulous" => &["careful", "precise", "thorough"],
        "discernible" => &["noticeable", "visible", "clear"],
        "adept" => &["skilled", "good at", "sharp"],

        // ---- AI's favorite nouns ----
        "landscape" => &["space", "scene", "world", "environment"],
        "paradigm" => &["model", "framework", "approach"],
        "synergy" | "synergistic" => &["teamwork", "cooperation", "combined effort"],
        "tapestry" => &["mix", "blend", "web"],
        "catalyst" | "catalysts" => &["driver", "trigger", "spark"],
        "testament" => &["proof", "sign", "evidence"],
        "prowess" => &["skill", "talent", "ability"],
        "realm" => &["area", "field", "world", "space"],
        "endeavor" | "endeavors" => &["effort", "project", "work"],
        "trajectory" => &["path", "direction", "course"],
        "cornerstone" => &["foundation", "basis", "pillar"],
        "underpinning" | "underpinnings" => &["foundation", "basis", "core"],
        "stakeholders" => &["people involved", "parties", "players"],
        "implications" => &["effects", "consequences", "impact"],
        "ramifications" => &["consequences", "effects", "fallout"],
        "juxtaposition" => &["contrast", "comparison", "difference"],
        "plethora" => &["ton", "bunch", "lots"],
        "intricacies" => &["details", "complexities", "ins and outs"],
        "conundrum" => &["puzzle", "problem", "dilemma"],
        "dichotomy" => &["split", "divide", "contrast"],

        // ---- AI's favorite adverbs ----
        "rapidly" => &["quickly", "fast", "at speed"],
        "increasingly" => &["more and more", "gradually more"],
        "fundamentally" => &["at its core", "basically", "at heart"],
        "inherently" => &["by nature", "naturally", "at its core"],
        "undoubtedly" => &["no doubt", "clearly", "for sure", "without question"],
        "arguably" => &["you could say", "possibly", "some would say"],
        "moreover" => &["also", "on top of that", "plus"],
        "nevertheless" => &["still", "even so", "but"],
        "consequently" => &["so", "as a result", "because of this"],
        "subsequently" => &["then", "after that", "next"],
        "furthermore" => &["also", "plus", "on top of that", "and"],
        "additionally" => &["also", "plus", "on top of that"],
        "conversely" => &["on the flip side", "on the other hand"],
        "simultaneously" => &["at the same time", "together"],
        "predominantly" => &["mostly", "mainly", "largely"],
        "particularly" => &["especially", "mainly", "specifically"],
        "essentially" => &["basically", "really", "at its core"],
        "notably" => &["especially", "in particular", "worth noting"],
        "evidently" => &["clearly", "obviously", "it seems"],
        "markedly" => &["noticeably", "clearly", "significantly"],

        // ---- AI's favorite phrases (as single words caught in context) ----
        "interconnected" => &["linked", "connected", "tied together"],
        "data-driven" => &["based on data", "evidence-based"],
        "forward-thinking" => &["progressive", "ahead of the curve"],
        "well-established" => &["proven", "solid", "long-standing"],
        "ever-evolving" => &["always changing", "constantly shifting"],

        _ => return word.to_string(),
    };

    let chosen = rng.pick(synonyms);

    if is_cap {
        let mut chars = chosen.chars();
        match chars.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
        }
    } else {
        chosen.to_string()
    }
}

// ==================== PASS 2: TRANSITION REPLACEMENT ====================

fn pass_transitions(input: &str) -> String {
    let mut text = input.to_string();

    let replacements: &[(&str, &str)] = &[
        // Formal → Casual transitions
        (
            "In today's rapidly evolving",
            "These days, in a fast-moving",
        ),
        (
            "In today's quickly evolving",
            "In a world that moves quickly",
        ),
        ("In today's fast-paced", "In a world that moves fast"),
        (
            "it becomes increasingly evident that",
            "it's pretty clear that",
        ),
        ("it becomes evident that", "you can see that"),
        ("it is important to note that", "it's worth noting"),
        ("it is worth noting that", "one thing to keep in mind is"),
        ("it should be noted that", "keep in mind"),
        ("in the realm of", "in"),
        ("in the context of", "when it comes to"),
        ("in light of", "given"),
        ("with regard to", "about"),
        ("in order to", "to"),
        ("due to the fact that", "because"),
        ("as a matter of fact", "actually"),
        ("at the end of the day", "ultimately"),
        ("on the other hand", "then again"),
        ("by and large", "mostly"),
        ("a wide range of", "many different"),
        ("a diverse range of", "all sorts of"),
        ("plays a crucial role", "matters a lot"),
        ("plays a pivotal role", "is really important"),
        ("plays a key role", "is a big deal"),
        ("are not merely", "aren't just"),
        ("is not merely", "isn't just"),
        ("pave the way for", "open doors for"),
        ("the way for a more", "the door to a more"),
        ("across numerous", "across many"),
        ("across various", "in different"),
    ];

    for (from, to) in replacements {
        // Case-insensitive replacement
        if let Some(pos) = text.to_lowercase().find(&from.to_lowercase()) {
            let end = pos + from.len();
            let replacement = if text
                .as_bytes()
                .get(pos)
                .map(|b| b.is_ascii_uppercase())
                .unwrap_or(false)
            {
                let mut chars = to.chars();
                match chars.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
                }
            } else {
                to.to_string()
            };
            text = format!("{}{}{}", &text[..pos], replacement, &text[end..]);
        }
    }

    text
}

// ==================== PASS 3: CONTRACTION INJECTION ====================

fn pass_contractions(input: &str) -> String {
    let mut text = input.to_string();

    let contractions: &[(&str, &str)] = &[
        ("It is not", "It's not"),
        ("it is not", "it's not"),
        ("do not", "don't"),
        ("does not", "doesn't"),
        ("did not", "didn't"),
        ("is not", "isn't"),
        ("are not", "aren't"),
        ("was not", "wasn't"),
        ("were not", "weren't"),
        ("will not", "won't"),
        ("would not", "wouldn't"),
        ("could not", "couldn't"),
        ("should not", "shouldn't"),
        ("cannot", "can't"),
        ("can not", "can't"),
        ("it is", "it's"),
        ("that is", "that's"),
        ("there is", "there's"),
        ("they are", "they're"),
        ("we are", "we're"),
        ("you are", "you're"),
        ("I am", "I'm"),
        ("I have", "I've"),
        ("I will", "I'll"),
        ("it will", "it'll"),
        ("they will", "they'll"),
        ("we will", "we'll"),
        ("who is", "who's"),
        ("what is", "what's"),
        ("let us", "let's"),
        ("Do not", "Don't"),
        ("Does not", "Doesn't"),
        ("Did not", "Didn't"),
        ("Is not", "Isn't"),
        ("Are not", "Aren't"),
        ("Was not", "Wasn't"),
        ("Will not", "Won't"),
        ("Would not", "Wouldn't"),
        ("Could not", "Couldn't"),
        ("Should not", "Shouldn't"),
        ("Cannot", "Can't"),
        ("It is", "It's"),
        ("That is", "That's"),
        ("There is", "There's"),
        ("They are", "They're"),
        ("We are", "We're"),
        ("You are", "You're"),
        ("We will", "We'll"),
        ("They will", "They'll"),
        ("It will", "It'll"),
        ("Who is", "Who's"),
        ("What is", "What's"),
        ("Let us", "Let's"),
    ];

    for (from, to) in contractions {
        text = text.replace(from, to);
    }

    text
}

// ==================== PASS 4: BURSTINESS INJECTION ====================

fn pass_burstiness(input: &str, rng: &mut Rng) -> String {
    let sentences: Vec<&str> = input.split(". ").collect();
    if sentences.len() < 3 {
        return input.to_string();
    }

    let mut result = Vec::new();

    for sentence in &sentences {
        let word_count = sentence.split_whitespace().count();

        // If sentence is very long (>25 words), split it
        if word_count > 25 {
            let words: Vec<&str> = sentence.split_whitespace().collect();
            // Find a good split point (after a comma, or near the middle)
            let mut split_at = words.len() / 2;

            // Try to find a comma near the middle to split at
            #[allow(clippy::needless_range_loop)]
            for i in (words.len() / 3)..((2 * words.len()) / 3) {
                if words[i].ends_with(',') {
                    split_at = i + 1;
                    break;
                }
            }

            let first_half: String = words[..split_at].join(" ");
            let second_half: String = words[split_at..].join(" ");

            // Remove trailing comma from first half if present
            let first_half = first_half.trim_end_matches(',').to_string();

            result.push(first_half);

            // Capitalize second half
            let second_half = capitalize_first(&second_half);
            result.push(second_half);
        } else {
            result.push(sentence.to_string());
        }
    }

    // Randomly merge some short adjacent sentences with a dash or semicolon
    let mut final_result = Vec::new();
    let mut i = 0;
    while i < result.len() {
        let word_count = result[i].split_whitespace().count();
        if word_count < 10 && i + 1 < result.len() && rng.next_float() < 0.3 {
            let connector = if rng.next_float() < 0.5 {
                " — "
            } else {
                "; "
            };
            let merged = format!(
                "{}{}{}",
                result[i],
                connector,
                lowercase_first(&result[i + 1])
            );
            final_result.push(merged);
            i += 2;
        } else {
            final_result.push(result[i].clone());
            i += 1;
        }
    }

    final_result.join(". ")
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn lowercase_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_lowercase().collect::<String>() + chars.as_str(),
    }
}

// ==================== PASS 5: HUMAN FILLER INJECTION ====================

fn pass_fillers(input: &str, rng: &mut Rng) -> String {
    let sentences: Vec<&str> = input.split(". ").collect();
    if sentences.len() < 3 {
        return input.to_string();
    }

    let fillers = [
        "Honestly, ",
        "The thing is, ",
        "What's interesting is that ",
        "If you think about it, ",
        "Look, ",
        "Here's the deal: ",
        "To put it simply, ",
        "In simple terms, ",
        "The reality is, ",
        "At the end of the day, ",
        "Truth be told, ",
        "When you break it down, ",
    ];

    let mut result: Vec<String> = Vec::new();
    let mut filler_count = 0;
    let max_fillers = if sentences.len() > 6 { 3 } else { 2 };

    for (i, sentence) in sentences.iter().enumerate() {
        // Don't add filler to the first sentence, and limit total fillers
        if i > 0 && i % 2 == 0 && filler_count < max_fillers && rng.next_float() < 0.45 {
            let filler = rng.pick(&fillers);
            let lower = lowercase_first(sentence);
            result.push(format!("{}{}", filler, lower));
            filler_count += 1;
        } else {
            result.push(sentence.to_string());
        }
    }

    result.join(". ")
}

// ==================== PASS 6: STRIP AI PADDING ====================

fn pass_strip_ai_padding(input: &str) -> String {
    let mut text = input.to_string();

    let padding: &[(&str, &str)] = &[
        ("It is important to understand that ", ""),
        ("It is crucial to recognize that ", ""),
        ("It is essential to acknowledge that ", ""),
        ("It is noteworthy that ", ""),
        ("It goes without saying that ", ""),
        ("Needless to say, ", ""),
        ("As we all know, ", ""),
        ("As previously mentioned, ", ""),
        ("In conclusion, ", "So, "),
        ("To summarize, ", "Basically, "),
        ("In summary, ", "So basically, "),
        ("All in all, ", "Overall, "),
        (
            "Taking everything into account, ",
            "All things considered, ",
        ),
        ("Given the above, ", "With all that, "),
    ];

    for (from, to) in padding {
        text = text.replace(from, to);
    }

    text
}

// ==================== PASS 7: HOMOGLYPH PERTURBATION ====================

/// Applies Cyrillic homoglyphs and zero-width non-joiners to the text
/// to break AI tokenizers and sub-word chunking algorithms.
pub fn apply_homoglyphs(input: &str) -> String {
    let mut result = String::with_capacity(input.len() + input.len() / 5);
    let seed = input.len() as u32 ^ 0xCAFE;
    let mut rng = Rng::new(seed);

    for c in input.chars() {
        // 15% chance to swap with a Cyrillic homoglyph
        let replacement = if rng.next_float() < 0.15 {
            match c {
                'a' => 'а', // U+0430
                'c' => 'с', // U+0441
                'e' => 'е', // U+0435
                'o' => 'о', // U+043E
                'p' => 'р', // U+0440
                'x' => 'х', // U+0445
                'y' => 'у', // U+0443
                'A' => 'А', // U+0410
                'C' => 'С', // U+0421
                'E' => 'Е', // U+0415
                'O' => 'О', // U+041E
                'P' => 'Р', // U+0420
                'X' => 'Х', // U+0425
                _ => c,
            }
        } else {
            c
        };
        result.push(replacement);

        // 5% chance to inject an invisible zero-width non-joiner
        if c != ' ' && rng.next_float() < 0.05 {
            result.push('\u{200C}');
        }
    }

    result
}

// ==================== TESTS ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_text_is_untouched() {
        let clean = "This is a completely normal string.";
        assert_eq!(sanitize_text(clean, false), clean);
    }

    #[test]
    fn test_strips_zero_width_spaces() {
        let dirty = "Hello\u{200B}World";
        let expected = "HelloWorld";
        assert_eq!(sanitize_text(dirty, false), expected);
    }

    #[test]
    fn test_strips_unicode_tags() {
        let dirty = "Secret\u{E0041}\u{E0042}Message";
        let expected = "SecretMessage";
        assert_eq!(sanitize_text(dirty, false), expected);
    }

    #[test]
    fn test_homoglyphs_injected() {
        let ai_text = "In today's rapidly evolving and fast-paced digital landscape, the integration of artificial intelligence has become a crucial and transformative element across numerous industries.";
        let homoglyphed = sanitize_text(ai_text, true);
        // The text should be different at the byte level due to homoglyph injection
        assert_ne!(ai_text, homoglyphed);
        // The homoglyph text should have a different number of bytes (Cyrillic characters are multi-byte)
        // or at least be guaranteed to not be exactly equal to the ASCII input
    }

    #[test]
    fn test_contractions() {
        let formal = "It is not possible. They are going. We will succeed.";
        let result = pass_contractions(formal);
        assert!(result.contains("It's not"));
        assert!(result.contains("They're"));
        assert!(result.contains("We'll"));
    }
}

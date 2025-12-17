pub struct FuzzyMatcher;

impl FuzzyMatcher {
    /// Generate LIKE patterns from a fly name for fuzzy database searching
    /// Example: "Woolly Bugger" → ["%Woolly%", "%Bugger%", "%Woolly%Bugger%"]
    pub fn generate_patterns(fly_name: &str) -> Vec<String> {
        let mut patterns = Vec::new();

        // Trim and normalize the input
        let normalized = fly_name.trim();

        if normalized.is_empty() {
            return patterns;
        }

        // Add pattern for the full name
        patterns.push(format!("%{}%", normalized));

        // Split by spaces and create patterns for individual words
        let words: Vec<&str> = normalized.split_whitespace()
            .filter(|w| w.len() > 2) // Skip very short words like "fly", "a", etc.
            .collect();

        // Add patterns for each significant word
        for word in &words {
            let pattern = format!("%{}%", word);
            if !patterns.contains(&pattern) {
                patterns.push(pattern);
            }
        }

        // If multiple words, create combination patterns
        if words.len() >= 2 {
            // Create pattern with all words (wildcards between)
            let combined = words.join("%");
            let combined_pattern = format!("%{}%", combined);
            if !patterns.contains(&combined_pattern) {
                patterns.push(combined_pattern);
            }
        }

        // Limit to prevent pattern explosion
        patterns.truncate(5);

        patterns
    }

    /// Calculate a relevance score for a match (0.0 to 1.0)
    /// Higher score means better match
    pub fn calculate_match_score(
        recommendation: &str,
        product_title: &str,
        _matched_pattern: &str,
    ) -> f32 {
        let rec_lower = recommendation.to_lowercase();
        let title_lower = product_title.to_lowercase();

        // Exact match (case insensitive)
        if rec_lower == title_lower {
            return 1.0;
        }

        // Contains exact recommendation as substring
        if title_lower.contains(&rec_lower) {
            // Longer match relative to title length is better
            let ratio = rec_lower.len() as f32 / title_lower.len() as f32;
            return 0.8 + (ratio * 0.15); // 0.8 to 0.95
        }

        // Split into words and check overlap
        let rec_words: Vec<&str> = rec_lower.split_whitespace().collect();
        let title_words: Vec<&str> = title_lower.split_whitespace().collect();

        if rec_words.is_empty() || title_words.is_empty() {
            return 0.3;
        }

        // Count matching words
        let mut matching_words = 0;
        for rec_word in &rec_words {
            if title_words.iter().any(|t| t.contains(rec_word) || rec_word.contains(t)) {
                matching_words += 1;
            }
        }

        // Calculate word overlap ratio
        let overlap_ratio = matching_words as f32 / rec_words.len() as f32;

        // Score based on word overlap (0.3 to 0.75)
        0.3 + (overlap_ratio * 0.45)
    }

    /// Extract clean fly name from ChatGPT response line
    /// Removes numbers, bullets, extra whitespace
    pub fn clean_fly_name(line: &str) -> Option<String> {
        let cleaned = line
            .trim()
            // Remove leading numbers and dots (1. 2. 3.)
            .trim_start_matches(|c: char| c.is_numeric() || c == '.' || c == ')' || c == '-')
            .trim()
            // Remove bullets
            .trim_start_matches(|c: char| c == '•' || c == '*' || c == '▪' || c == '◦')
            .trim()
            // Remove trailing punctuation
            .trim_end_matches(|c: char| c == '!' || c == '.' || c == '?')
            .trim();

        if cleaned.is_empty() || cleaned.len() < 3 {
            return None;
        }

        // Filter out non-fly responses - conversational text
        let lower = cleaned.to_lowercase();

        // Common conversational starters
        if lower.starts_with("based on")
            || lower.starts_with("here")
            || lower.starts_with("i recommend")
            || lower.starts_with("sure")
            || lower.starts_with("certainly")
            || lower.starts_with("of course")
            || lower.starts_with("gotcha")
            || lower.starts_with("great")
            || lower.starts_with("perfect")
            || lower.starts_with("for ")
            || lower.starts_with("try ")
            || lower.starts_with("consider")
        {
            return None;
        }

        // Common conversational phrases
        if lower.contains("question")
            || lower.contains("recommendation")
            || lower.contains("suggest")
            || lower.contains("would be")
            || lower.contains("you could")
            || lower.contains("you should")
            || lower == "here are"
            || lower == "here's"
            || lower == "these are"
        {
            return None;
        }

        Some(cleaned.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_patterns() {
        let patterns = FuzzyMatcher::generate_patterns("Woolly Bugger");
        assert!(patterns.contains(&"%Woolly Bugger%".to_string()));
        assert!(patterns.contains(&"%Woolly%".to_string()));
        assert!(patterns.contains(&"%Bugger%".to_string()));
    }

    #[test]
    fn test_calculate_match_score() {
        // Exact match
        let score = FuzzyMatcher::calculate_match_score("Woolly Bugger", "woolly bugger", "");
        assert!(score > 0.95);

        // Substring match
        let score = FuzzyMatcher::calculate_match_score("Woolly Bugger", "Black Woolly Bugger #6", "");
        assert!(score > 0.7 && score < 0.95);

        // Partial word match
        let score = FuzzyMatcher::calculate_match_score("Woolly Bugger", "Bugger Fly", "");
        assert!(score > 0.3 && score < 0.7);
    }

    #[test]
    fn test_clean_fly_name() {
        assert_eq!(FuzzyMatcher::clean_fly_name("1. Woolly Bugger"), Some("Woolly Bugger".to_string()));
        assert_eq!(FuzzyMatcher::clean_fly_name("• Adams Dry Fly"), Some("Adams Dry Fly".to_string()));
        assert_eq!(FuzzyMatcher::clean_fly_name("  - Elk Hair Caddis  "), Some("Elk Hair Caddis".to_string()));
        assert_eq!(FuzzyMatcher::clean_fly_name("Based on your question..."), None);
    }
}

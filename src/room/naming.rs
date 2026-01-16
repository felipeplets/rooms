#![allow(dead_code)]

use rand::prelude::IndexedRandom;
use rand::Rng;

/// Adjectives for room name generation.
const ADJECTIVES: &[&str] = &[
    "quick", "lazy", "happy", "calm", "bold", "bright", "cool", "warm", "swift", "keen", "fresh",
    "crisp", "gentle", "vivid", "steady", "clever", "witty", "merry", "lively", "peaceful",
    "cosmic", "lunar", "solar", "stellar", "amber", "azure", "coral", "golden", "silver",
    "emerald", "rustic", "modern", "classic", "noble", "humble",
];

/// Nouns for room name generation.
const NOUNS: &[&str] = &[
    "fox", "owl", "bear", "wolf", "hawk", "deer", "hare", "seal", "crow", "swan", "oak", "pine",
    "elm", "maple", "cedar", "river", "stream", "lake", "pond", "brook", "peak", "ridge", "vale",
    "grove", "meadow", "stone", "crystal", "ember", "frost", "breeze", "dawn", "dusk", "noon",
    "tide", "wave",
];

/// Generate a unique room name.
///
/// Format: `<adjective>-<noun>-<4hex>`
///
/// The name is:
/// - Lowercase
/// - Contains only [a-z0-9-]
/// - Max 40 characters
/// - Collision-safe via random suffix
pub fn generate_room_name() -> String {
    let mut rng = rand::rng();

    let adjective = ADJECTIVES.choose(&mut rng).unwrap_or(&"quick");
    let noun = NOUNS.choose(&mut rng).unwrap_or(&"fox");
    let suffix: u16 = rng.random_range(0..=0xFFFF);
    let hex_suffix = format!("{:04x}", suffix);

    format!("{}-{}-{}", adjective, noun, hex_suffix)
}

/// Generate a room name, ensuring it doesn't collide with existing names.
pub fn generate_unique_room_name<F>(exists: F) -> String
where
    F: Fn(&str) -> bool,
{
    // Try up to 100 times to find a unique name
    for _ in 0..100 {
        let name = generate_room_name();
        if !exists(&name) {
            return name;
        }
    }

    // Fallback: use timestamp-based name
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    format!("room-{}", timestamp)
}

/// Validate a room name.
///
/// Valid names:
/// - Are lowercase
/// - Contain only [a-z0-9-]
/// - Are 1-40 characters
/// - Don't start or end with -
pub fn validate_room_name(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("name cannot be empty");
    }

    if name.len() > 40 {
        return Err("name cannot exceed 40 characters");
    }

    if name.starts_with('-') || name.ends_with('-') {
        return Err("name cannot start or end with a hyphen");
    }

    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err("name can only contain lowercase letters, digits, and hyphens");
    }

    Ok(())
}

/// Sanitize a string to be a valid room name.
pub fn sanitize_room_name(name: &str) -> String {
    let sanitized: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();

    // Remove leading/trailing hyphens and collapse multiple hyphens
    let mut result = String::new();
    let mut last_was_hyphen = true; // Treat start as if preceded by hyphen

    for c in sanitized.chars() {
        if c == '-' {
            if !last_was_hyphen {
                result.push(c);
                last_was_hyphen = true;
            }
        } else {
            result.push(c);
            last_was_hyphen = false;
        }
    }

    // Remove trailing hyphen
    if result.ends_with('-') {
        result.pop();
    }

    // Truncate to 40 chars
    if result.len() > 40 {
        result.truncate(40);
        // Make sure we don't end with a hyphen after truncation
        while result.ends_with('-') {
            result.pop();
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_room_name_format() {
        let name = generate_room_name();

        // Should be lowercase with hyphens
        assert!(name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'));

        // Should have format: word-word-xxxx
        let parts: Vec<&str> = name.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[2].len(), 4); // hex suffix
    }

    #[test]
    fn test_generate_room_name_max_length() {
        for _ in 0..100 {
            let name = generate_room_name();
            assert!(name.len() <= 40);
        }
    }

    #[test]
    fn test_generate_unique_room_name() {
        let existing = ["quick-fox-0000".to_string()];
        let name = generate_unique_room_name(|n| existing.contains(&n.to_string()));

        // Should not match the existing name (extremely unlikely anyway)
        assert!(!existing.contains(&name));
    }

    #[test]
    fn test_validate_room_name_valid() {
        assert!(validate_room_name("my-room").is_ok());
        assert!(validate_room_name("room123").is_ok());
        assert!(validate_room_name("a").is_ok());
        assert!(validate_room_name("feature-branch-fix").is_ok());
    }

    #[test]
    fn test_validate_room_name_invalid() {
        assert!(validate_room_name("").is_err());
        assert!(validate_room_name("-starts-with-hyphen").is_err());
        assert!(validate_room_name("ends-with-hyphen-").is_err());
        assert!(validate_room_name("Has-Uppercase").is_err());
        assert!(validate_room_name("has spaces").is_err());
        assert!(validate_room_name("has_underscore").is_err());

        let long_name = "a".repeat(41);
        assert!(validate_room_name(&long_name).is_err());
    }

    #[test]
    fn test_sanitize_room_name() {
        assert_eq!(sanitize_room_name("My Feature"), "my-feature");
        assert_eq!(sanitize_room_name("hello_world"), "hello-world");
        assert_eq!(sanitize_room_name("--double--hyphens--"), "double-hyphens");
        assert_eq!(sanitize_room_name("UPPERCASE"), "uppercase");
        assert_eq!(sanitize_room_name("feat/branch"), "feat-branch");
    }

    #[test]
    fn test_sanitize_room_name_truncates() {
        let long_name = "a".repeat(50);
        let sanitized = sanitize_room_name(&long_name);
        assert!(sanitized.len() <= 40);
    }
}

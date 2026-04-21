//! Contract broker -- firm-internal specialist routing for Civilian Reserve.
//!
//! When a military agent contracts a firm, the firm must choose which of its
//! internal specialists handles the work. For Slice B, routing is purely
//! **deterministic**: the broker reads a `## Routing rules` section from the
//! firm's doctrine markdown and matches contract-brief keywords against the
//! specialist mapping. No LLM involvement; no non-determinism.
//!
//! # Routing rules format
//!
//! The firm's `firm.md` must contain a second-level heading exactly
//! `## Routing rules` followed by a list of bullet items, one per specialist:
//!
//! ```markdown
//! ## Routing rules
//!
//! - `specialist_slug` keywords: kw1, kw2, kw3
//! - `other_slug` keywords: kw4, kw5
//! ```
//!
//! The broker scans the contract brief case-insensitively for any matching
//! keyword; specialists are tried in document order (earlier bullet wins on
//! collision). If no keyword matches, the first specialist listed in the
//! firm's frontmatter `specialists:` field is used as the default fallback.

use crate::traits::CivilianDoctrine;

/// A deterministic specialist router for a given firm.
pub struct ContractBroker;

impl ContractBroker {
    /// Pick the specialist slug that should handle a contract brief.
    ///
    /// Returns `None` only if the firm has neither explicit routing rules nor
    /// any specialists declared in its frontmatter (a malformed firm).
    pub fn route(firm_doctrine: &CivilianDoctrine, brief: &str) -> Option<String> {
        let rules = parse_routing_rules(&firm_doctrine.body);
        let lc_brief = brief.to_lowercase();

        for (specialist, keywords) in &rules {
            for kw in keywords {
                if lc_brief.contains(&kw.to_lowercase()) {
                    return Some(specialist.clone());
                }
            }
        }

        // Fallback: first specialist listed in the firm's frontmatter.
        firm_doctrine.specialists.first().cloned()
    }

    /// Expose the parsed routing rules for inspection (useful for
    /// `cr_firm_detail` and for tests).
    pub fn rules(firm_doctrine: &CivilianDoctrine) -> Vec<(String, Vec<String>)> {
        parse_routing_rules(&firm_doctrine.body)
    }
}

/// Parse the `## Routing rules` section of a firm doctrine body.
///
/// Returns a vector of (specialist_slug, keywords) pairs in document order.
/// Bullet items that do not match the expected pattern are skipped silently;
/// a malformed entry does not block routing on the well-formed ones.
fn parse_routing_rules(body: &str) -> Vec<(String, Vec<String>)> {
    let mut rules: Vec<(String, Vec<String>)> = Vec::new();
    let mut in_section = false;

    for raw_line in body.lines() {
        let line = raw_line.trim_end();
        let trimmed = line.trim_start();

        if trimmed.starts_with("## ") {
            // Any second-level heading delimits the section.
            in_section = trimmed.eq_ignore_ascii_case("## Routing rules")
                || trimmed.eq_ignore_ascii_case("## Routing Rules");
            continue;
        }
        if trimmed.starts_with("# ") {
            // A first-level heading ends any active section.
            in_section = false;
            continue;
        }
        if !in_section {
            continue;
        }

        let bullet = match trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
            Some(rest) => rest,
            None => continue,
        };

        // Expected form:  `specialist_slug` keywords: kw1, kw2, kw3
        let (specialist, keyword_text) = match extract_rule(bullet) {
            Some(pair) => pair,
            None => continue,
        };

        let keywords: Vec<String> = keyword_text
            .split(',')
            .map(|k| k.trim().trim_matches('`').to_string())
            .filter(|k| !k.is_empty())
            .collect();
        if keywords.is_empty() {
            continue;
        }
        rules.push((specialist, keywords));
    }

    rules
}

/// Pull `specialist_slug` and the keyword substring out of a bullet body.
///
/// Accepts the canonical form `` `slug` keywords: k1, k2, k3 `` and the
/// backtick-less variant `slug keywords: k1, k2`.
fn extract_rule(bullet: &str) -> Option<(String, String)> {
    let (head, tail) = bullet.split_once(':')?;
    let head = head.trim();
    let tail = tail.trim();

    // head is expected to be either `` `slug` keywords `` or `slug keywords`.
    // Strip the trailing "keywords" word (case-insensitive) to isolate slug.
    let lower_head = head.to_lowercase();
    let slug_part = lower_head
        .strip_suffix("keywords")
        .map(|s| s.trim_end())
        .unwrap_or(&lower_head);

    // Strip surrounding backticks if present.
    let slug = slug_part.trim().trim_matches('`').trim().to_string();
    if slug.is_empty() {
        return None;
    }

    Some((slug, tail.to_string()))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::CivilianTier;

    fn doctrine(body: &str, specialists: Vec<&str>) -> CivilianDoctrine {
        CivilianDoctrine {
            slug: "firm".into(),
            kind: CivilianTier::Firm,
            firm: None,
            project_path: None,
            upstream_url: None,
            languages: Vec::new(),
            tier_override: None,
            contract_scope: Vec::new(),
            specialists: specialists.into_iter().map(String::from).collect(),
            body: body.to_string(),
        }
    }

    #[test]
    fn routes_by_first_matching_keyword() {
        let body = r#"
## Routing rules

- `morphlex` keywords: tokenizer, BLAKE3, morphology
- `jasterish` keywords: compiler, x86-64, jstar
- `custom-llm` keywords: LLM, inference, training
"#;
        let d = doctrine(body, vec!["morphlex", "jasterish", "custom-llm"]);
        assert_eq!(
            ContractBroker::route(&d, "review the BLAKE3 truncation strategy"),
            Some("morphlex".to_string())
        );
        assert_eq!(
            ContractBroker::route(&d, "x86-64 codegen for the JStar backend"),
            Some("jasterish".to_string())
        );
        assert_eq!(
            ContractBroker::route(&d, "attention pattern for the custom LLM"),
            Some("custom-llm".to_string())
        );
    }

    #[test]
    fn case_insensitive_match() {
        let body = r#"
## Routing rules

- `morphlex` keywords: Tokenizer
"#;
        let d = doctrine(body, vec!["morphlex"]);
        assert_eq!(
            ContractBroker::route(&d, "tokenizer change"),
            Some("morphlex".to_string())
        );
        assert_eq!(
            ContractBroker::route(&d, "TOKENIZER change"),
            Some("morphlex".to_string())
        );
    }

    #[test]
    fn earlier_bullet_wins_on_collision() {
        let body = r#"
## Routing rules

- `morphlex` keywords: compile
- `jasterish` keywords: compile, codegen
"#;
        let d = doctrine(body, vec!["morphlex", "jasterish"]);
        assert_eq!(
            ContractBroker::route(&d, "compile the dictionary"),
            Some("morphlex".to_string())
        );
    }

    #[test]
    fn fallback_to_first_specialist_when_no_rules_match() {
        let body = r#"
## Routing rules

- `morphlex` keywords: tokenizer
"#;
        let d = doctrine(body, vec!["morphlex", "jasterish"]);
        assert_eq!(
            ContractBroker::route(&d, "ambiguous contract with no matching keywords"),
            Some("morphlex".to_string())
        );
    }

    #[test]
    fn returns_none_when_firm_has_no_specialists_and_no_rules() {
        let d = doctrine("## Scope\nonly scope here.\n", vec![]);
        assert_eq!(ContractBroker::route(&d, "whatever"), None);
    }

    #[test]
    fn section_ends_at_next_h1() {
        let body = r#"
## Routing rules

- `morphlex` keywords: tokenizer

# Another top-level section

- `jasterish` keywords: compiler
"#;
        let d = doctrine(body, vec!["morphlex"]);
        // jasterish rule is after an h1, so it should NOT apply.
        assert_eq!(
            ContractBroker::route(&d, "compiler question"),
            Some("morphlex".to_string())
        );
    }

    #[test]
    fn rules_accessor_returns_parsed() {
        let body = r#"
## Routing rules

- `a` keywords: k1, k2
- `b` keywords: k3
"#;
        let d = doctrine(body, vec!["a", "b"]);
        let rules = ContractBroker::rules(&d);
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].0, "a");
        assert_eq!(rules[0].1, vec!["k1".to_string(), "k2".to_string()]);
        assert_eq!(rules[1].0, "b");
        assert_eq!(rules[1].1, vec!["k3".to_string()]);
    }
}

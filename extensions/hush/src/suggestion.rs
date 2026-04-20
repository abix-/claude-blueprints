//! Suggestion shape builder.
//!
//! The single place a [`Suggestion`] is constructed. Every detector path
//! in the engine hands a [`BuildSuggestionInput`] to [`build_suggestion`]
//! and receives a fully-populated output. Centralizing the shape here is
//! the direct answer to the class of bug that produced the 0.5.0 emit()
//! regression: one field only has to be added in one place.

use crate::types::{BuildSuggestionInput, Suggestion, SuggestionDiag, SuggestionLayer};
#[cfg(test)]
use std::sync::Arc;

/// Build a suggestion. Pure function; all inputs are passed in.
pub fn build_suggestion(input: &BuildSuggestionInput) -> Suggestion {
    let frame_hostname = input.from_frame.clone();
    let tab_hostname = input.tab_hostname.clone();
    let is_from_iframe = frame_hostname
        .as_ref()
        .is_some_and(|h| !h.is_empty() && h != &tab_hostname);

    let existing_for_layer: &[String] = match input.layer {
        SuggestionLayer::Block => &input.existing_block,
        SuggestionLayer::Remove => &input.existing_remove,
        SuggestionLayer::Hide => &input.existing_hide,
        // Neuter / silence suggestions don't dedup yet — the
        // detector pipeline doesn't populate their "existing"
        // lists. Always-emit for now; revisit when the
        // replay-listener detector upgrades to Neuter.
        SuggestionLayer::Neuter | SuggestionLayer::Silence => &[],
    };
    let dedup_result = if existing_for_layer.iter().any(|v| v == &input.value) {
        "MATCH (should have been filtered)".to_string()
    } else {
        "no match".to_string()
    };

    let existing_block_sample = input
        .existing_block
        .iter()
        .take(10)
        .cloned()
        .collect::<Vec<_>>();

    let diag = SuggestionDiag {
        value: input.value.clone(),
        layer: input.layer,
        tab_hostname: tab_hostname.clone(),
        frame_hostname: frame_hostname.clone().unwrap_or_else(|| tab_hostname.clone()),
        is_from_iframe,
        matched_key: input.matched_key.clone(),
        config_has_site: input.config_has_site,
        existing_block_count: input.existing_block.len(),
        existing_block_sample,
        dedup_result,
    };

    Suggestion {
        key: input.key.clone(),
        layer: input.layer,
        value: input.value.clone(),
        reason: input.reason.clone(),
        confidence: input.confidence,
        count: input.count,
        evidence: input.evidence.clone(),
        from_iframe: is_from_iframe,
        frame_hostname,
        diag,
        learn: input.learn.clone(),
        kind: input.kind.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_input() -> BuildSuggestionInput {
        BuildSuggestionInput {
            key: "block::||example.com".into(),
            layer: SuggestionLayer::Block,
            value: "||example.com".into(),
            reason: "because".into(),
            confidence: 95,
            count: 3,
            evidence: vec!["https://example.com/a".into()],
            from_frame: None,
            learn: "learn text".into(),
            tab_hostname: "site.test".into(),
            matched_key: Some("site.test".into()),
            config_has_site: true,
            existing_block: Arc::from([] as [String; 0]),
            existing_remove: Arc::from([] as [String; 0]),
            existing_hide: Arc::from([] as [String; 0]),
            kind: String::new(),
        }
    }

    #[test]
    fn top_frame_sets_from_iframe_false_and_frame_matches_tab() {
        let s = build_suggestion(&base_input());
        assert!(!s.from_iframe);
        assert_eq!(s.frame_hostname, None);
        assert_eq!(s.diag.frame_hostname, "site.test");
        assert!(!s.diag.is_from_iframe);
    }

    #[test]
    fn iframe_frame_different_from_tab_sets_from_iframe_true() {
        let mut input = base_input();
        input.from_frame = Some("embed.other.test".into());
        let s = build_suggestion(&input);
        assert!(s.from_iframe);
        assert_eq!(s.frame_hostname.as_deref(), Some("embed.other.test"));
        assert!(s.diag.is_from_iframe);
        assert_eq!(s.diag.frame_hostname, "embed.other.test");
    }

    #[test]
    fn iframe_same_host_as_tab_is_not_flagged_as_iframe() {
        let mut input = base_input();
        input.from_frame = Some("site.test".into());
        let s = build_suggestion(&input);
        assert!(!s.from_iframe);
        assert!(!s.diag.is_from_iframe);
    }

    #[test]
    fn dedup_result_block_layer_matches_existing_block() {
        let mut input = base_input();
        input.existing_block = Arc::from(["||example.com".into(), "||other.test".into()]);
        let s = build_suggestion(&input);
        assert_eq!(s.diag.dedup_result, "MATCH (should have been filtered)");
        assert_eq!(s.diag.existing_block_count, 2);
    }

    #[test]
    fn dedup_result_remove_layer_checks_existing_remove() {
        let mut input = base_input();
        input.layer = SuggestionLayer::Remove;
        input.value = "iframe[src*=\"x\"]".into();
        input.existing_remove = Arc::from(["iframe[src*=\"x\"]".into()]);
        // existing_block is irrelevant for a Remove-layer suggestion.
        input.existing_block = Arc::from(["something-unrelated".into()]);
        let s = build_suggestion(&input);
        assert_eq!(s.diag.dedup_result, "MATCH (should have been filtered)");
    }

    #[test]
    fn existing_block_sample_caps_at_ten() {
        let mut input = base_input();
        input.existing_block = Arc::from((0..50).map(|i| format!("rule{i}")).collect::<Vec<_>>());
        let s = build_suggestion(&input);
        assert_eq!(s.diag.existing_block_sample.len(), 10);
        assert_eq!(s.diag.existing_block_count, 50);
    }

    #[test]
    fn learn_text_passes_through() {
        let s = build_suggestion(&base_input());
        assert_eq!(s.learn, "learn text");
    }
}

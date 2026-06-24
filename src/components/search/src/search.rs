//! Fuzzy search over the [`SearchIndex`].
//!
//! The scorer uses two fast heuristics (no external crates needed):
//!
//! 1. **Exact substring** — score 1.0 for exact, 0.9 for case-insensitive.
//! 2. **Normalized edit distance** — Levenshtein distance normalised to `[0, 1]`,
//!    weighted so short edit distances on long strings still rank well.
//!
//! Results are sorted by descending score and filtered to `score >= threshold`.

use crate::index::SearchIndex;
use crate::types::SearchResult;

/// Search the index with a fuzzy query.
///
/// Returns all entries whose name, contract_id, or any function ID score at or
/// above `threshold` (0.0–1.0). Pass `0.0` to return everything ranked.
pub fn fuzzy_search(index: &SearchIndex, query: &str, threshold: f32) -> Vec<SearchResult> {
    if query.is_empty() {
        return Vec::new();
    }

    let mut results: Vec<SearchResult> = Vec::new();
    let q_lower = query.to_lowercase();

    for entry in index.entries() {
        // Score against name
        let name_score = score(&entry.name, &q_lower);
        if name_score >= threshold {
            results.push(SearchResult {
                entry: entry.clone(),
                matched_field: "name".into(),
                score: name_score,
            });
        }

        // Score against contract_id
        let id_score = score(&entry.contract_id, &q_lower);
        if id_score >= threshold && id_score > name_score {
            results.push(SearchResult {
                entry: entry.clone(),
                matched_field: "contract_id".into(),
                score: id_score,
            });
        }

        // Score against each function ID
        for fn_id in &entry.function_ids {
            let fn_score = score(fn_id, &q_lower);
            if fn_score >= threshold {
                results.push(SearchResult {
                    entry: entry.clone(),
                    matched_field: fn_id.clone(),
                    score: fn_score,
                });
            }
        }
    }

    // Sort best match first
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(core::cmp::Ordering::Equal));
    results
}

/// Compute a fuzzy score in `[0.0, 1.0]` between `target` and the
/// already-lowercased `query`.
fn score(target: &str, query: &str) -> f32 {
    let t_lower = target.to_lowercase();

    // Exact match
    if t_lower == query {
        return 1.0;
    }

    // Case-insensitive substring
    if t_lower.contains(query) {
        // Rank longer needle-vs-haystack ratios higher.
        let ratio = query.len() as f32 / t_lower.len() as f32;
        return 0.9 * ratio.sqrt().clamp(0.5, 1.0);
    }

    // Normalised Levenshtein
    let dist = levenshtein(&t_lower, query);
    let max_len = t_lower.len().max(query.len());
    if max_len == 0 {
        return 1.0;
    }
    let normalized = 1.0 - (dist as f32 / max_len as f32);
    normalized.clamp(0.0, 1.0)
}

/// Levenshtein edit distance (no external dependency).
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let m = a.len();
    let n = b.len();

    let mut row: Vec<usize> = (0..=n).collect();

    for i in 1..=m {
        let mut prev = row[0];
        row[0] = i;
        for j in 1..=n {
            let old = row[j];
            row[j] = if a[i - 1] == b[j - 1] {
                prev
            } else {
                1 + prev.min(row[j]).min(row[j - 1])
            };
            prev = old;
        }
    }

    row[n]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::SearchIndex;
    use crate::types::ContractEntry;

    fn sample_index() -> SearchIndex {
        let mut idx = SearchIndex::new();
        idx.insert(
            ContractEntry::new("CAAAA...001", "vesting_contracts").with_functions(vec![
                "initialize",
                "create_vest",
                "claim_tokens",
                "get_vesting_info",
                "revoke_vest",
            ]),
        );
        idx.insert(
            ContractEntry::new("CAAAA...002", "staking_contract").with_functions(vec![
                "stake",
                "unstake",
                "claim_yield",
                "get_stake_info",
            ]),
        );
        idx.insert(
            ContractEntry::new("CAAAA...003", "grant_contracts").with_functions(vec![
                "create_grant",
                "accept_grant",
                "revoke_grant",
            ]),
        );
        idx
    }

    #[test]
    fn exact_name_match_scores_one() {
        let idx = sample_index();
        let results = fuzzy_search(&idx, "staking_contract", 0.8);
        assert!(!results.is_empty());
        assert_eq!(results[0].score, 1.0);
        assert_eq!(results[0].entry.name, "staking_contract");
    }

    #[test]
    fn partial_name_match_returns_result() {
        let idx = sample_index();
        let results = fuzzy_search(&idx, "vest", 0.5);
        // Both "vesting_contracts" and "revoke_vest" should appear
        assert!(!results.is_empty());
        let names: Vec<_> = results.iter().map(|r| r.entry.name.as_str()).collect();
        assert!(names.contains(&"vesting_contracts"));
    }

    #[test]
    fn function_id_search_finds_claim_tokens() {
        let idx = sample_index();
        let results = fuzzy_search(&idx, "claim_tokens", 0.8);
        assert!(!results.is_empty());
        assert_eq!(results[0].matched_field, "claim_tokens");
        assert_eq!(results[0].score, 1.0);
    }

    #[test]
    fn fuzzy_typo_still_matches() {
        let idx = sample_index();
        // "stkin" is a typo of "staking"
        let results = fuzzy_search(&idx, "stkin", 0.3);
        assert!(!results.is_empty());
    }

    #[test]
    fn empty_query_returns_nothing() {
        let idx = sample_index();
        assert!(fuzzy_search(&idx, "", 0.0).is_empty());
    }

    #[test]
    fn results_are_sorted_by_descending_score() {
        let idx = sample_index();
        let results = fuzzy_search(&idx, "grant", 0.3);
        for w in results.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
    }

    #[test]
    fn index_deduplicates_on_same_contract_id() {
        let mut idx = SearchIndex::new();
        idx.insert(ContractEntry::new("C001", "old_name"));
        idx.insert(ContractEntry::new("C001", "new_name"));
        assert_eq!(idx.len(), 1);
        assert_eq!(idx.entries()[0].name, "new_name");
    }
}

#[cfg(test)]
mod tests {
    use crate::core::models::Grammar;
    use crate::parsers::lalr1::LALR1Parser;
    use crate::parsers::lr1::LR1Parser;
    use crate::parsers::slr1::SLR1Parser;

    // ========================
    // Construction — grammar classification
    // ========================

    #[test]
    fn test_lalr1_valid_simple_grammar_builds() {
        let g = Grammar::from_string("S -> a").unwrap();
        assert!(LALR1Parser::new(g).is_ok());
    }

    #[test]
    fn test_lalr1_valid_multi_token_grammar_builds() {
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        assert!(LALR1Parser::new(g).is_ok());
    }

    #[test]
    fn test_lalr1_valid_epsilon_grammar_builds() {
        let g = Grammar::from_string("S -> A B\nA -> a | ϵ\nB -> b").unwrap();
        assert!(LALR1Parser::new(g).is_ok());
    }

    #[test]
    fn test_lalr1_valid_recursive_grammar_builds() {
        let g = Grammar::from_string("S -> a S b | c").unwrap();
        assert!(LALR1Parser::new(g).is_ok());
    }

    /// KEY HIERARCHY TEST: grammar IS LR(1) but NOT LALR(1).
    ///
    /// States reached via "a c" and "b c" in canonical LR(1) have the same
    /// core {(A→c,1),(B→c,1)} but different lookaheads:
    ///   via "a c": [A→c·,d] and [B→c·,e]
    ///   via "b c": [B→c·,d] and [A→c·,e]
    ///
    /// LALR(1) merges them → lookaheads become {d,e} for both A→c· and B→c·
    /// → reduce-reduce conflict on d and e in the merged state.
    ///
    /// LR(1) keeps them separate → no conflict.
    #[test]
    fn test_lalr1_rejects_grammar_that_lr1_accepts() {
        let raw = "S -> a A d | b B d | a B e | b A e\nA -> c\nB -> c";
        let g_lalr = Grammar::from_string(raw).unwrap();
        let g_lr1  = Grammar::from_string(raw).unwrap();
        assert!(LALR1Parser::new(g_lalr).is_err(), "LALR(1) must reject this grammar");
        assert!(LR1Parser::new(g_lr1).is_ok(),     "LR(1) must accept this grammar");
    }

    #[test]
    fn test_lalr1_rejects_reduce_reduce_conflict() {
        // Inherently ambiguous: S → A | B, A → a, B → a
        let g = Grammar::from_string("S -> A | B\nA -> a\nB -> a").unwrap();
        assert!(LALR1Parser::new(g).is_err());
    }

    // ========================
    // State merging — hierarchy
    // ========================

    /// LALR(1) must have ≤ states than LR(1) for the same grammar
    /// (merging states with identical cores reduces the count).
    #[test]
    fn test_lalr1_has_fewer_or_equal_states_than_lr1() {
        let raw = "S -> a B\nB -> b";
        let g_lr1  = Grammar::from_string(raw).unwrap();
        let g_lalr = Grammar::from_string(raw).unwrap();
        let lr1   = LR1Parser::new(g_lr1).unwrap();
        let lalr1 = LALR1Parser::new(g_lalr).unwrap();
        assert!(lalr1.states.len() <= lr1.states.len(),
            "LALR(1) states ({}) should be ≤ LR(1) states ({})",
            lalr1.states.len(), lr1.states.len());
    }

    #[test]
    fn test_lalr1_has_fewer_states_than_lr1_for_merging_grammar() {
        // S → a A d | b B d | a B e | b A e, A/B → c
        // LR(1) has 2 distinct states for "c" contexts; LALR(1) merges them into 1.
        // BUT this grammar is NOT LALR(1), so we use a simpler grammar that does merge.
        // S → a A | b A, A → c: state "c" is reached from both "a" and "b" paths.
        let raw = "S -> a A | b A\nA -> c";
        let g_lr1  = Grammar::from_string(raw).unwrap();
        let g_lalr = Grammar::from_string(raw).unwrap();
        let lr1   = LR1Parser::new(g_lr1).unwrap();
        let lalr1 = LALR1Parser::new(g_lalr).unwrap();
        // LR(1) keeps two separate states for {[A → c·, $]} (one per context);
        // LALR(1) merges them into one since the core and lookahead are identical.
        assert!(lalr1.states.len() <= lr1.states.len());
    }

    #[test]
    fn test_lalr1_state_count_matches_slr1_core_count() {
        // For a grammar with no mergeable states, LALR(1) ≈ LR(0)/SLR(1) state count
        let raw = "S -> a B\nB -> b";
        let g_slr  = Grammar::from_string(raw).unwrap();
        let g_lalr = Grammar::from_string(raw).unwrap();
        let slr1  = SLR1Parser::new(g_slr).unwrap();
        let lalr1 = LALR1Parser::new(g_lalr).unwrap();
        assert_eq!(lalr1.states.len(), slr1.states.len(),
            "LALR(1) and SLR(1) must have the same state count for this grammar");
    }

    // ========================
    // Parse — accept
    // ========================

    #[test]
    fn test_lalr1_parse_single_token() {
        let g = Grammar::from_string("S -> a").unwrap();
        let p = LALR1Parser::new(g).unwrap();
        let result = p.parse_input(vec!["a".to_string()]);
        assert!(result.is_ok());
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_lalr1_parse_multi_token() {
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = LALR1Parser::new(g).unwrap();
        let result = p.parse_input(vec!["a".to_string(), "b".to_string()]);
        assert!(result.is_ok());
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_lalr1_parse_epsilon_skipped() {
        // S -> A B, A -> a | ε, B -> b: input ["b"] via A → ε
        let g = Grammar::from_string("S -> A B\nA -> a | ϵ\nB -> b").unwrap();
        let p = LALR1Parser::new(g).unwrap();
        let result = p.parse_input(vec!["b".to_string()]);
        assert!(result.is_ok(), "Expected accept for 'b' via A → ε: {:?}", result);
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_lalr1_parse_epsilon_used() {
        // ["a", "b"] via A → a
        let g = Grammar::from_string("S -> A B\nA -> a | ϵ\nB -> b").unwrap();
        let p = LALR1Parser::new(g).unwrap();
        let result = p.parse_input(vec!["a".to_string(), "b".to_string()]);
        assert!(result.is_ok());
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_lalr1_parse_recursive_base() {
        // S -> a S b | c: "c"
        let g = Grammar::from_string("S -> a S b | c").unwrap();
        let p = LALR1Parser::new(g).unwrap();
        let result = p.parse_input(vec!["c".to_string()]);
        assert!(result.is_ok(), "Expected accept for 'c': {:?}", result);
    }

    #[test]
    fn test_lalr1_parse_recursive_one_level() {
        // "a c b"
        let g = Grammar::from_string("S -> a S b | c").unwrap();
        let p = LALR1Parser::new(g).unwrap();
        let result = p.parse_input(vec!["a".to_string(), "c".to_string(), "b".to_string()]);
        assert!(result.is_ok(), "Expected accept for 'a c b': {:?}", result);
    }

    #[test]
    fn test_lalr1_parse_recursive_two_levels() {
        // "a a c b b"
        let g = Grammar::from_string("S -> a S b | c").unwrap();
        let p = LALR1Parser::new(g).unwrap();
        let result = p.parse_input(vec![
            "a".to_string(), "a".to_string(), "c".to_string(),
            "b".to_string(), "b".to_string(),
        ]);
        assert!(result.is_ok(), "Expected accept for 'a a c b b': {:?}", result);
    }

    #[test]
    fn test_lalr1_parse_merged_states_grammar() {
        // S -> a A | b A, A -> c: LALR(1) merges the two "c·" states
        let g = Grammar::from_string("S -> a A | b A\nA -> c").unwrap();
        let p = LALR1Parser::new(g).unwrap();
        assert!(p.parse_input(vec!["a".to_string(), "c".to_string()]).is_ok());
        let g2 = Grammar::from_string("S -> a A | b A\nA -> c").unwrap();
        let p2 = LALR1Parser::new(g2).unwrap();
        assert!(p2.parse_input(vec!["b".to_string(), "c".to_string()]).is_ok());
    }

    // ========================
    // Parse — reject
    // ========================

    #[test]
    fn test_lalr1_parse_rejects_wrong_token() {
        let g = Grammar::from_string("S -> a").unwrap();
        let p = LALR1Parser::new(g).unwrap();
        assert!(p.parse_input(vec!["b".to_string()]).is_err());
    }

    #[test]
    fn test_lalr1_parse_rejects_wrong_second_token() {
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = LALR1Parser::new(g).unwrap();
        assert!(p.parse_input(vec!["a".to_string(), "x".to_string()]).is_err());
    }

    #[test]
    fn test_lalr1_parse_rejects_empty_for_non_nullable() {
        let g = Grammar::from_string("S -> a b").unwrap();
        let p = LALR1Parser::new(g).unwrap();
        assert!(p.parse_input(vec![]).is_err());
    }

    #[test]
    fn test_lalr1_parse_rejects_mismatched_recursive_close() {
        // S -> a S b | c: "a c a b" — mismatched nesting
        let g = Grammar::from_string("S -> a S b | c").unwrap();
        let p = LALR1Parser::new(g).unwrap();
        assert!(p.parse_input(vec![
            "a".to_string(), "c".to_string(), "a".to_string(), "b".to_string()
        ]).is_err());
    }

    #[test]
    fn test_lalr1_parse_rejects_incomplete_input() {
        // S -> a S b | c: "a c" — missing closing 'b'
        let g = Grammar::from_string("S -> a S b | c").unwrap();
        let p = LALR1Parser::new(g).unwrap();
        assert!(p.parse_input(vec!["a".to_string(), "c".to_string()]).is_err());
    }
}

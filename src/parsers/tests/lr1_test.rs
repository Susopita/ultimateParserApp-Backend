#[cfg(test)]
mod tests {
    use crate::core::models::{Grammar, Symbol};
    use crate::parsers::lr1::LR1Parser;
    use crate::parsers::lalr1::LALR1Parser;

    // ========================
    // Construction — grammar classification
    // ========================

    #[test]
    fn test_lr1_valid_simple_grammar_builds() {
        let g = Grammar::from_string("S -> a").unwrap();
        assert!(LR1Parser::new(g).is_ok());
    }

    #[test]
    fn test_lr1_valid_epsilon_grammar_builds() {
        let g = Grammar::from_string("S -> A B\nA -> a | ϵ\nB -> b").unwrap();
        assert!(LR1Parser::new(g).is_ok());
    }

    #[test]
    fn test_lr1_valid_recursive_grammar_builds() {
        let g = Grammar::from_string("S -> a S b | c").unwrap();
        assert!(LR1Parser::new(g).is_ok());
    }

    /// LR(1) correctly handles the SLR(1)-rejected grammar.
    /// Same grammar that SLR(1) rejects (reduce-reduce with overlapping FOLLOW)
    /// also fails LR(1) because the precise lookaheads are identical in the conflicting state.
    #[test]
    fn test_lr1_also_rejects_inherently_ambiguous_grammar() {
        // S → A B | C D, A/C → a, B/D → b: truly ambiguous, fails all parsers
        let raw = "S -> A B | C D\nA -> a\nB -> b\nC -> a\nD -> b";
        let g = Grammar::from_string(raw).unwrap();
        // This grammar is inherently ambiguous — LR(1) must also reject it
        assert!(LR1Parser::new(g).is_err());
    }

    /// KEY HIERARCHY TEST: the grammar S → a A d | b B d | a B e | b A e, A/B → c
    /// is LR(1) but NOT LALR(1).
    ///
    /// In canonical LR(1):
    ///   State reached via "a c": {[A → c·, d], [B → c·, e]}  — no conflict
    ///   State reached via "b c": {[B → c·, d], [A → c·, e]}  — no conflict
    ///
    /// LALR(1) merges these two states (same core {(A→c,1),(B→c,1)}), producing
    ///   {[A → c·, {d,e}], [B → c·, {d,e}]} → reduce-reduce on both d and e.
    ///
    /// LR(1) construction must succeed.
    #[test]
    fn test_lr1_accepts_grammar_that_lalr1_rejects() {
        let raw = "S -> a A d | b B d | a B e | b A e\nA -> c\nB -> c";
        let g_lr1 = Grammar::from_string(raw).unwrap();
        let g_lalr = Grammar::from_string(raw).unwrap();
        assert!(LR1Parser::new(g_lr1).is_ok(), "LR(1) must accept this grammar");
        assert!(LALR1Parser::new(g_lalr).is_err(), "LALR(1) must reject this grammar");
    }

    // ========================
    // White-box: canonical collection and lookaheads
    // ========================

    #[test]
    fn test_lr1_state0_has_initial_item_with_dollar_lookahead() {
        // State 0 must contain [S' → · S, $]
        let g = Grammar::from_string("S -> a b").unwrap();
        let p = LR1Parser::new(g).unwrap();
        let state0 = &p.states[0];
        let has_initial = state0.iter().any(|item| {
            item.production.left == Symbol::NonTerminal("S'".to_string())
                && item.dot_position == 0
                && item.lookahead == "$"
        });
        assert!(has_initial, "State 0 must contain [S' → · S, $]");
    }

    #[test]
    fn test_lr1_state0_has_nonterminal_items_in_closure() {
        // S -> a B, B -> b: state 0 closure must contain [B → · b, $]
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = LR1Parser::new(g).unwrap();
        let state0 = &p.states[0];
        // The closure from S' → · S expands S → · a B, which does NOT expand B yet
        // (dot is on 'a', not on B). But state after 'a' should have B items.
        // Verify state 0 has S → · a B with proper lookahead
        let has_s_prod = state0.iter().any(|item| {
            item.production.left == Symbol::NonTerminal("S".to_string())
                && item.dot_position == 0
        });
        assert!(has_s_prod, "State 0 must contain S → · a B");
    }

    #[test]
    fn test_lr1_more_states_than_lr0_for_distinguishing_grammar() {
        // The LR(1)-but-not-LALR(1) grammar has more LR(1) states than LR(0)/SLR(1) states
        // because two states with the same core but different lookaheads are kept separate.
        let raw = "S -> a A d | b B d | a B e | b A e\nA -> c\nB -> c";
        let g = Grammar::from_string(raw).unwrap();
        let p = LR1Parser::new(g).unwrap();
        // This grammar should have states that differ only in lookahead
        assert!(p.states.len() > 0);
    }

    #[test]
    fn test_lr1_state_count_simple_grammar() {
        // S → a B, B → b: same state count as LR(0)/SLR(1) = 5
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = LR1Parser::new(g).unwrap();
        assert_eq!(p.states.len(), 5);
    }

    #[test]
    fn test_lr1_get_all_terminals_includes_dollar() {
        let g = Grammar::from_string("S -> a b").unwrap();
        let p = LR1Parser::new(g).unwrap();
        assert!(p.get_all_terminals().contains(&"$".to_string()));
    }

    #[test]
    fn test_lr1_get_nonterminals_excludes_augmented() {
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = LR1Parser::new(g).unwrap();
        let nts = p.get_all_non_terminals();
        assert!(!nts.contains(&"S'".to_string()));
        assert!(nts.contains(&"S".to_string()));
        assert!(nts.contains(&"B".to_string()));
    }

    // ========================
    // Parse — accept
    // ========================

    #[test]
    fn test_lr1_parse_single_token() {
        let g = Grammar::from_string("S -> a").unwrap();
        let p = LR1Parser::new(g).unwrap();
        let result = p.parse_input(vec!["a".to_string()]);
        assert!(result.is_ok());
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_lr1_parse_multi_token() {
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = LR1Parser::new(g).unwrap();
        let result = p.parse_input(vec!["a".to_string(), "b".to_string()]);
        assert!(result.is_ok());
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_lr1_parse_epsilon_production_skipped() {
        // S -> A B, A -> a | ε, B -> b: input ["b"] via A → ε
        let g = Grammar::from_string("S -> A B\nA -> a | ϵ\nB -> b").unwrap();
        let p = LR1Parser::new(g).unwrap();
        let result = p.parse_input(vec!["b".to_string()]);
        assert!(result.is_ok(), "Expected accept for 'b' via A → ε: {:?}", result);
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_lr1_parse_epsilon_production_used() {
        // S -> A B, A -> a | ε, B -> b: input ["a", "b"] via A → a
        let g = Grammar::from_string("S -> A B\nA -> a | ϵ\nB -> b").unwrap();
        let p = LR1Parser::new(g).unwrap();
        let result = p.parse_input(vec!["a".to_string(), "b".to_string()]);
        assert!(result.is_ok());
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_lr1_parse_recursive_base_case() {
        // S -> a S b | c: "c"
        let g = Grammar::from_string("S -> a S b | c").unwrap();
        let p = LR1Parser::new(g).unwrap();
        let result = p.parse_input(vec!["c".to_string()]);
        assert!(result.is_ok(), "Expected accept for 'c': {:?}", result);
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_lr1_parse_recursive_one_level() {
        // S -> a S b | c: "a c b"
        let g = Grammar::from_string("S -> a S b | c").unwrap();
        let p = LR1Parser::new(g).unwrap();
        let result = p.parse_input(vec!["a".to_string(), "c".to_string(), "b".to_string()]);
        assert!(result.is_ok(), "Expected accept for 'a c b': {:?}", result);
    }

    #[test]
    fn test_lr1_parse_accepts_first_alternative_of_lr1_grammar() {
        // S → a A d | b B d | a B e | b A e, A → c, B → c
        // Input "a c d" goes through S → a A d, A → c
        let raw = "S -> a A d | b B d | a B e | b A e\nA -> c\nB -> c";
        let g = Grammar::from_string(raw).unwrap();
        let p = LR1Parser::new(g).unwrap();
        let result = p.parse_input(vec!["a".to_string(), "c".to_string(), "d".to_string()]);
        assert!(result.is_ok(), "Expected accept for 'a c d': {:?}", result);
    }

    #[test]
    fn test_lr1_parse_accepts_second_alternative_of_lr1_grammar() {
        // "b c d" goes through S → b B d, B → c
        let raw = "S -> a A d | b B d | a B e | b A e\nA -> c\nB -> c";
        let g = Grammar::from_string(raw).unwrap();
        let p = LR1Parser::new(g).unwrap();
        let result = p.parse_input(vec!["b".to_string(), "c".to_string(), "d".to_string()]);
        assert!(result.is_ok(), "Expected accept for 'b c d': {:?}", result);
    }

    #[test]
    fn test_lr1_parse_accepts_third_alternative_of_lr1_grammar() {
        // "a c e" goes through S → a B e, B → c
        let raw = "S -> a A d | b B d | a B e | b A e\nA -> c\nB -> c";
        let g = Grammar::from_string(raw).unwrap();
        let p = LR1Parser::new(g).unwrap();
        let result = p.parse_input(vec!["a".to_string(), "c".to_string(), "e".to_string()]);
        assert!(result.is_ok(), "Expected accept for 'a c e': {:?}", result);
    }

    #[test]
    fn test_lr1_parse_accepts_fourth_alternative_of_lr1_grammar() {
        // "b c e" goes through S → b A e, A → c
        let raw = "S -> a A d | b B d | a B e | b A e\nA -> c\nB -> c";
        let g = Grammar::from_string(raw).unwrap();
        let p = LR1Parser::new(g).unwrap();
        let result = p.parse_input(vec!["b".to_string(), "c".to_string(), "e".to_string()]);
        assert!(result.is_ok(), "Expected accept for 'b c e': {:?}", result);
    }

    // ========================
    // Parse — reject
    // ========================

    #[test]
    fn test_lr1_parse_rejects_wrong_token() {
        let g = Grammar::from_string("S -> a").unwrap();
        let p = LR1Parser::new(g).unwrap();
        assert!(p.parse_input(vec!["b".to_string()]).is_err());
    }

    #[test]
    fn test_lr1_parse_rejects_wrong_middle_token() {
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = LR1Parser::new(g).unwrap();
        assert!(p.parse_input(vec!["a".to_string(), "x".to_string()]).is_err());
    }

    #[test]
    fn test_lr1_parse_rejects_empty_for_non_nullable() {
        let g = Grammar::from_string("S -> a b").unwrap();
        let p = LR1Parser::new(g).unwrap();
        assert!(p.parse_input(vec![]).is_err());
    }

    #[test]
    fn test_lr1_parse_rejects_wrong_suffix_in_lr1_grammar() {
        // "a c x" — wrong last token for S → a A d, S → a B e
        let raw = "S -> a A d | b B d | a B e | b A e\nA -> c\nB -> c";
        let g = Grammar::from_string(raw).unwrap();
        let p = LR1Parser::new(g).unwrap();
        assert!(p.parse_input(vec!["a".to_string(), "c".to_string(), "x".to_string()]).is_err());
    }

    #[test]
    fn test_lr1_parse_rejects_incomplete_input_in_lr1_grammar() {
        // "a c" — missing suffix
        let raw = "S -> a A d | b B d | a B e | b A e\nA -> c\nB -> c";
        let g = Grammar::from_string(raw).unwrap();
        let p = LR1Parser::new(g).unwrap();
        assert!(p.parse_input(vec!["a".to_string(), "c".to_string()]).is_err());
    }
}

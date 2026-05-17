#[cfg(test)]
mod tests {
    use crate::core::models::Grammar;
    use crate::parsers::slr1::SLR1Parser;
    use crate::parsers::lr0::LR0Parser;

    // ========================
    // Construction — grammar classification
    // ========================

    #[test]
    fn test_slr1_valid_simple_grammar_builds() {
        let g = Grammar::from_string("S -> a").unwrap();
        assert!(SLR1Parser::new(g).is_ok());
    }

    #[test]
    fn test_slr1_valid_two_symbol_production_builds() {
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        assert!(SLR1Parser::new(g).is_ok());
    }

    #[test]
    fn test_slr1_valid_epsilon_grammar_builds() {
        let g = Grammar::from_string("S -> A B\nA -> a | ϵ\nB -> b").unwrap();
        assert!(SLR1Parser::new(g).is_ok());
    }

    /// KEY HIERARCHY TEST: this grammar is NOT LR(0) (reduce-reduce on all terminals)
    /// but IS SLR(1) because FOLLOW(B) = {c} and FOLLOW(C) = {d} do not overlap.
    ///
    /// Grammar:  S → a B c | a C d
    ///           B → b
    ///           C → b
    ///
    /// State after "a b": {B → b ·, C → b ·}
    /// LR(0): reduce both for ALL terminals → conflict
    /// SLR(1): reduce B → b only on {c}, C → b only on {d} → no conflict
    #[test]
    fn test_slr1_accepts_grammar_rejected_by_lr0() {
        let raw = "S -> a B c | a C d\nB -> b\nC -> b";
        let g_lr0 = Grammar::from_string(raw).unwrap();
        let g_slr = Grammar::from_string(raw).unwrap();
        assert!(LR0Parser::new(g_lr0).is_err(),  "LR(0) must reject this grammar");
        assert!(SLR1Parser::new(g_slr).is_ok(),  "SLR(1) must accept this grammar");
    }

    /// Grammar NOT SLR(1): two nonterminals A and C both derive "a" and both are
    /// followed by "b" (FOLLOW overlap) — reduce-reduce conflict remains after FOLLOW filtering.
    ///
    /// Grammar:  S → A B | C D
    ///           A → a       FOLLOW(A) = {b}
    ///           B → b
    ///           C → a       FOLLOW(C) = {b}
    ///           D → b
    ///
    /// State after "a": {A → a ·, C → a ·}
    /// SLR(1): reduce A → a on {b}, reduce C → a on {b} → conflict on 'b'
    #[test]
    fn test_slr1_rejects_reduce_reduce_with_overlapping_follow() {
        let raw = "S -> A B | C D\nA -> a\nB -> b\nC -> a\nD -> b";
        let g = Grammar::from_string(raw).unwrap();
        let result = SLR1Parser::new(g);
        assert!(result.is_err(), "SLR(1) must detect reduce-reduce conflict");
        if let Err(msg) = result {
            assert!(msg.contains("SLR(1)"), "Error should mention SLR(1), got: {}", msg);
        }
    }

    // ========================
    // White-box: FIRST sets
    // ========================

    #[test]
    fn test_slr1_first_of_nonterminal_simple() {
        // B → b: FIRST(B) = {b} — verified via parsing (first is private)
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = SLR1Parser::new(g).unwrap();
        let result = p.parse_input(vec!["a".to_string(), "b".to_string()]);
        assert!(result.is_ok());
    }

    // ========================
    // White-box: FOLLOW sets
    // ========================

    #[test]
    fn test_slr1_follow_start_symbol_has_dollar() {
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = SLR1Parser::new(g).unwrap();
        // S' is the augmented start; "S'" in follow map
        assert!(p.follow.get("S'").unwrap().contains("$"));
    }

    #[test]
    fn test_slr1_follow_nonterminal_from_successor_terminal() {
        // S -> A B, A -> a | ε, B -> b: FOLLOW(A) = FIRST(B) = {b}
        let g = Grammar::from_string("S -> A B\nA -> a | ϵ\nB -> b").unwrap();
        let p = SLR1Parser::new(g).unwrap();
        let follow_a = p.follow.get("A").unwrap();
        assert!(follow_a.contains("b"), "FOLLOW(A) should contain 'b'");
    }

    #[test]
    fn test_slr1_follow_nonterminal_at_end_of_rhs_inherits_from_lhs() {
        // S -> a B: FOLLOW(B) must include FOLLOW(S) = {$}
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = SLR1Parser::new(g).unwrap();
        let follow_b = p.follow.get("B").unwrap();
        assert!(follow_b.contains("$"), "FOLLOW(B) should contain '$'");
    }

    #[test]
    fn test_slr1_follow_disjoint_in_slr1_but_not_lr0_grammar() {
        // S -> a B c | a C d: FOLLOW(B) = {c}, FOLLOW(C) = {d}
        let g = Grammar::from_string("S -> a B c | a C d\nB -> b\nC -> b").unwrap();
        let p = SLR1Parser::new(g).unwrap();
        let follow_b = p.follow.get("B").unwrap();
        let follow_c = p.follow.get("C").unwrap();
        assert!(follow_b.contains("c") && !follow_b.contains("d"),
            "FOLLOW(B) = {{c}}, got {:?}", follow_b);
        assert!(follow_c.contains("d") && !follow_c.contains("c"),
            "FOLLOW(C) = {{d}}, got {:?}", follow_c);
    }

    // ========================
    // Parse — accept
    // ========================

    #[test]
    fn test_slr1_parse_single_token_accept() {
        let g = Grammar::from_string("S -> a").unwrap();
        let p = SLR1Parser::new(g).unwrap();
        let result = p.parse_input(vec!["a".to_string()]);
        assert!(result.is_ok());
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_slr1_parse_multi_token_accept() {
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = SLR1Parser::new(g).unwrap();
        let result = p.parse_input(vec!["a".to_string(), "b".to_string()]);
        assert!(result.is_ok());
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_slr1_parse_epsilon_production_skip() {
        // S -> A B, A -> a | ε, B -> b: input ["b"] accepted via A → ε
        let g = Grammar::from_string("S -> A B\nA -> a | ϵ\nB -> b").unwrap();
        let p = SLR1Parser::new(g).unwrap();
        let result = p.parse_input(vec!["b".to_string()]);
        assert!(result.is_ok(), "Should accept 'b' via A → ε: {:?}", result);
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_slr1_parse_epsilon_production_with_token() {
        // S -> A B, A -> a | ε, B -> b: input ["a", "b"] via A → a
        let g = Grammar::from_string("S -> A B\nA -> a | ϵ\nB -> b").unwrap();
        let p = SLR1Parser::new(g).unwrap();
        let result = p.parse_input(vec!["a".to_string(), "b".to_string()]);
        assert!(result.is_ok());
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_slr1_parse_slr1_specific_grammar_first_alternative() {
        // S -> a B c | a C d, B -> b, C -> b: "a b c" via S → a B c
        let g = Grammar::from_string("S -> a B c | a C d\nB -> b\nC -> b").unwrap();
        let p = SLR1Parser::new(g).unwrap();
        let result = p.parse_input(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        assert!(result.is_ok(), "Expected accept for 'a b c': {:?}", result);
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_slr1_parse_slr1_specific_grammar_second_alternative() {
        // "a b d" via S → a C d
        let g = Grammar::from_string("S -> a B c | a C d\nB -> b\nC -> b").unwrap();
        let p = SLR1Parser::new(g).unwrap();
        let result = p.parse_input(vec!["a".to_string(), "b".to_string(), "d".to_string()]);
        assert!(result.is_ok(), "Expected accept for 'a b d': {:?}", result);
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_slr1_parse_recursive_grammar() {
        // S -> a S b | c: "a a c b b"
        let g = Grammar::from_string("S -> a S b | c").unwrap();
        let p = SLR1Parser::new(g).unwrap();
        let result = p.parse_input(vec![
            "a".to_string(), "a".to_string(), "c".to_string(),
            "b".to_string(), "b".to_string(),
        ]);
        assert!(result.is_ok(), "Expected accept: {:?}", result);
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    // ========================
    // Parse — reject
    // ========================

    #[test]
    fn test_slr1_parse_rejects_wrong_single_token() {
        let g = Grammar::from_string("S -> a").unwrap();
        let p = SLR1Parser::new(g).unwrap();
        assert!(p.parse_input(vec!["b".to_string()]).is_err());
    }

    #[test]
    fn test_slr1_parse_rejects_wrong_second_token() {
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = SLR1Parser::new(g).unwrap();
        assert!(p.parse_input(vec!["a".to_string(), "x".to_string()]).is_err());
    }

    #[test]
    fn test_slr1_parse_rejects_empty_for_non_nullable() {
        let g = Grammar::from_string("S -> a b").unwrap();
        let p = SLR1Parser::new(g).unwrap();
        assert!(p.parse_input(vec![]).is_err());
    }

    #[test]
    fn test_slr1_parse_rejects_wrong_suffix_in_slr1_grammar() {
        // S -> a B c | a C d: "a b" without suffix must fail
        let g = Grammar::from_string("S -> a B c | a C d\nB -> b\nC -> b").unwrap();
        let p = SLR1Parser::new(g).unwrap();
        assert!(p.parse_input(vec!["a".to_string(), "b".to_string()]).is_err());
    }

    #[test]
    fn test_slr1_parse_rejects_wrong_prefix() {
        // "b c" when grammar expects starting 'a'
        let g = Grammar::from_string("S -> a B c | a C d\nB -> b\nC -> b").unwrap();
        let p = SLR1Parser::new(g).unwrap();
        assert!(p.parse_input(vec!["b".to_string(), "c".to_string()]).is_err());
    }

    #[test]
    fn test_slr1_parse_rejects_mismatched_recursive_close() {
        // S -> a S b | c: "a c a b" is wrong
        let g = Grammar::from_string("S -> a S b | c").unwrap();
        let p = SLR1Parser::new(g).unwrap();
        assert!(p.parse_input(vec![
            "a".to_string(), "c".to_string(), "a".to_string(), "b".to_string()
        ]).is_err());
    }
}

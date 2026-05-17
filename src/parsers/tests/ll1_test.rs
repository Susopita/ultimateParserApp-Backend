#[cfg(test)]
mod tests {
    use crate::core::models::{Grammar, Symbol};
    use crate::parsers::ll1::LL1Parser;
    use crate::parsers::Parser;

    // ========================
    // Construction — grammar classification
    // ========================

    #[test]
    fn test_ll1_valid_simple_grammar_builds() {
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        assert!(LL1Parser::new(g).is_ok());
    }

    #[test]
    fn test_ll1_valid_epsilon_grammar_builds() {
        let g = Grammar::from_string("S -> a A\nA -> b | ϵ").unwrap();
        assert!(LL1Parser::new(g).is_ok());
    }

    #[test]
    fn test_ll1_valid_arithmetic_grammar_builds() {
        // Classic right-recursive arithmetic — definitively LL(1)
        let g = Grammar::from_string("E -> T E'\nE' -> + T E' | ε\nT -> id").unwrap();
        assert!(LL1Parser::new(g).is_ok());
    }

    #[test]
    fn test_ll1_valid_all_epsilon_chain_builds() {
        // S -> A, A -> ε — trivially LL(1)
        let g = Grammar::from_string("S -> A\nA -> ε").unwrap();
        assert!(LL1Parser::new(g).is_ok());
    }

    #[test]
    fn test_ll1_rejects_direct_left_recursion() {
        // E -> E + T | T: FIRST(E+T) and FIRST(T) both start with 'id' — conflict
        let g = Grammar::from_string("E -> E + T | T\nT -> id").unwrap();
        assert!(LL1Parser::new(g).is_err());
    }

    #[test]
    fn test_ll1_rejects_first_first_conflict() {
        // S -> a A | a B: two alternatives start with the same terminal 'a'
        let g = Grammar::from_string("S -> a A | a B\nA -> x\nB -> y").unwrap();
        let result = LL1Parser::new(g);
        assert!(result.is_err());
        if let Err(msg) = result {
            assert!(msg.contains("not LL(1)"), "Error should mention LL(1), got: {}", msg);
        }
    }

    #[test]
    fn test_ll1_rejects_first_follow_conflict() {
        // S -> A a, A -> a | ε
        // table[A, a] gets both A→a (FIRST) and A→ε (via FOLLOW(A)={a})
        let g = Grammar::from_string("S -> A a\nA -> a | ε").unwrap();
        assert!(LL1Parser::new(g).is_err());
    }

    // ========================
    // White-box: FIRST sets
    // ========================

    #[test]
    fn test_ll1_first_of_terminal_is_itself() {
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = LL1Parser::new(g).unwrap();
        let a = Symbol::Terminal("a".to_string());
        assert!(p.first.get(&a).unwrap().contains(&a));
    }

    #[test]
    fn test_ll1_first_of_nonterminal_simple() {
        // FIRST(B) = {b}
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = LL1Parser::new(g).unwrap();
        let b_nt = Symbol::NonTerminal("B".to_string());
        assert!(p.first.get(&b_nt).unwrap().contains(&Symbol::Terminal("b".to_string())));
    }

    #[test]
    fn test_ll1_first_includes_epsilon_for_nullable_nonterminal() {
        // A -> a | ε: FIRST(A) = {a, ε}
        let g = Grammar::from_string("S -> a A b\nA -> c | ε").unwrap();
        let p = LL1Parser::new(g).unwrap();
        let a_nt = Symbol::NonTerminal("A".to_string());
        let first_a = p.first.get(&a_nt).unwrap();
        assert!(first_a.contains(&Symbol::Epsilon));
        assert!(first_a.contains(&Symbol::Terminal("c".to_string())));
    }

    #[test]
    fn test_ll1_first_propagates_through_nullable_chain() {
        // S -> A B c, A -> ε, B -> b | ε
        // FIRST(S) must contain 'b' (via A→ε, B→b) and 'c' (via A→ε, B→ε)
        let g = Grammar::from_string("S -> A B c\nA -> ε\nB -> b | ε").unwrap();
        let p = LL1Parser::new(g).unwrap();
        let s_nt = Symbol::NonTerminal("S".to_string());
        let first_s = p.first.get(&s_nt).unwrap();
        assert!(first_s.contains(&Symbol::Terminal("b".to_string())));
        assert!(first_s.contains(&Symbol::Terminal("c".to_string())));
    }

    // ========================
    // White-box: FOLLOW sets
    // ========================

    #[test]
    fn test_ll1_follow_of_start_contains_dollar() {
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = LL1Parser::new(g).unwrap();
        let s_nt = Symbol::NonTerminal("S".to_string());
        assert!(p.follow.get(&s_nt).unwrap().contains(&Symbol::Terminal("$".to_string())));
    }

    #[test]
    fn test_ll1_follow_of_nonterminal_from_successor() {
        // S -> a A b: FOLLOW(A) must contain 'b'
        let g = Grammar::from_string("S -> a A b\nA -> c | ε").unwrap();
        let p = LL1Parser::new(g).unwrap();
        let a_nt = Symbol::NonTerminal("A".to_string());
        assert!(p.follow.get(&a_nt).unwrap().contains(&Symbol::Terminal("b".to_string())));
    }

    #[test]
    fn test_ll1_follow_propagates_from_production_lhs() {
        // S -> a B, B -> b C, C -> c | ε: FOLLOW(C) includes FOLLOW(B) = {$}
        let g = Grammar::from_string("S -> a B\nB -> b C\nC -> c | ε").unwrap();
        let p = LL1Parser::new(g).unwrap();
        let c_nt = Symbol::NonTerminal("C".to_string());
        assert!(p.follow.get(&c_nt).unwrap().contains(&Symbol::Terminal("$".to_string())));
    }

    #[test]
    fn test_ll1_follow_of_nonterminal_at_end_of_production() {
        // E -> T E': FOLLOW(E') = FOLLOW(E) = {$}
        let g = Grammar::from_string("E -> T E'\nE' -> + T E' | ε\nT -> id").unwrap();
        let p = LL1Parser::new(g).unwrap();
        let ep = Symbol::NonTerminal("E'".to_string());
        assert!(p.follow.get(&ep).unwrap().contains(&Symbol::Terminal("$".to_string())));
    }

    // ========================
    // White-box: parse table structure
    // ========================

    #[test]
    fn test_ll1_table_has_entry_for_start_and_first_token() {
        // S -> a B, B -> b: table[(S, a)] must exist
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = LL1Parser::new(g).unwrap();
        let s = Symbol::NonTerminal("S".to_string());
        let a = Symbol::Terminal("a".to_string());
        assert!(p.table.contains_key(&(s, a)));
    }

    #[test]
    fn test_ll1_table_entry_for_epsilon_uses_follow() {
        // S -> a A b, A -> c | ε: table[(A, b)] = [ε] because b ∈ FOLLOW(A)
        let g = Grammar::from_string("S -> a A b\nA -> c | ε").unwrap();
        let p = LL1Parser::new(g).unwrap();
        let a_nt = Symbol::NonTerminal("A".to_string());
        let b_t = Symbol::Terminal("b".to_string());
        let entry = p.table.get(&(a_nt, b_t)).unwrap();
        assert_eq!(*entry, vec![Symbol::Epsilon]);
    }

    // ========================
    // Parse — accept
    // ========================

    #[test]
    fn test_ll1_parse_simple_accept() {
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = LL1Parser::new(g).unwrap();
        let result = p.parse(vec!["a".to_string(), "b".to_string()]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().last().unwrap().action, "Success!");
    }

    #[test]
    fn test_ll1_parse_empty_input_for_fully_nullable_grammar() {
        // S -> A, A -> ε: empty input must succeed
        let g = Grammar::from_string("S -> A\nA -> ε").unwrap();
        let p = LL1Parser::new(g).unwrap();
        let result = p.parse(vec![]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().last().unwrap().action, "Success!");
    }

    #[test]
    fn test_ll1_parse_optional_nonterminal_skipped() {
        // S -> a A, A -> b | ε: input ["a"] accepted via A → ε
        let g = Grammar::from_string("S -> a A\nA -> b | ϵ").unwrap();
        let p = LL1Parser::new(g).unwrap();
        let result = p.parse(vec!["a".to_string()]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().last().unwrap().action, "Success!");
    }

    #[test]
    fn test_ll1_parse_optional_nonterminal_used() {
        // S -> a A, A -> b | ε: input ["a", "b"] accepted via A → b
        let g = Grammar::from_string("S -> a A\nA -> b | ϵ").unwrap();
        let p = LL1Parser::new(g).unwrap();
        let result = p.parse(vec!["a".to_string(), "b".to_string()]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().last().unwrap().action, "Success!");
    }

    #[test]
    fn test_ll1_parse_arithmetic_single_id() {
        let g = Grammar::from_string("E -> T E'\nE' -> + T E' | ε\nT -> id").unwrap();
        let p = LL1Parser::new(g).unwrap();
        let result = p.parse(vec!["id".to_string()]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().last().unwrap().action, "Success!");
    }

    #[test]
    fn test_ll1_parse_arithmetic_id_plus_id() {
        let g = Grammar::from_string("E -> T E'\nE' -> + T E' | ε\nT -> id").unwrap();
        let p = LL1Parser::new(g).unwrap();
        let result = p.parse(vec!["id".to_string(), "+".to_string(), "id".to_string()]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().last().unwrap().action, "Success!");
    }

    #[test]
    fn test_ll1_parse_arithmetic_id_plus_id_plus_id() {
        // Chained: id + id + id
        let g = Grammar::from_string("E -> T E'\nE' -> + T E' | ε\nT -> id").unwrap();
        let p = LL1Parser::new(g).unwrap();
        let result = p.parse(vec![
            "id".to_string(), "+".to_string(), "id".to_string(),
            "+".to_string(), "id".to_string(),
        ]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().last().unwrap().action, "Success!");
    }

    #[test]
    fn test_ll1_snapshots_steps_are_sequential() {
        // snapshot[i].step must equal i for a successful parse
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = LL1Parser::new(g).unwrap();
        let snaps = p.parse(vec!["a".to_string(), "b".to_string()]).unwrap();
        for (i, snap) in snaps.iter().enumerate() {
            assert_eq!(snap.step, i, "snapshot[{}].step should be {}", i, i);
        }
    }

    #[test]
    fn test_ll1_parse_recursive_grammar() {
        // S -> a S b | c — right recursive (IS LL(1))
        let g = Grammar::from_string("S -> a S b | c").unwrap();
        let p = LL1Parser::new(g).unwrap();
        // "a a c b b"
        let result = p.parse(vec!["a".to_string(), "a".to_string(), "c".to_string(),
                                   "b".to_string(), "b".to_string()]);
        assert!(result.is_ok(), "Expected accept but got: {:?}", result);
    }

    // ========================
    // Parse — reject
    // ========================

    #[test]
    fn test_ll1_parse_rejects_wrong_first_token() {
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = LL1Parser::new(g).unwrap();
        assert!(p.parse(vec!["x".to_string()]).is_err());
    }

    #[test]
    fn test_ll1_parse_rejects_wrong_second_token() {
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = LL1Parser::new(g).unwrap();
        assert!(p.parse(vec!["a".to_string(), "x".to_string()]).is_err());
    }

    #[test]
    fn test_ll1_parse_rejects_empty_for_non_nullable_grammar() {
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = LL1Parser::new(g).unwrap();
        assert!(p.parse(vec![]).is_err());
    }

    #[test]
    fn test_ll1_parse_rejects_arithmetic_missing_operand_after_plus() {
        // "id +" — missing second id
        let g = Grammar::from_string("E -> T E'\nE' -> + T E' | ε\nT -> id").unwrap();
        let p = LL1Parser::new(g).unwrap();
        assert!(p.parse(vec!["id".to_string(), "+".to_string()]).is_err());
    }

    #[test]
    fn test_ll1_parse_rejects_dangling_operator() {
        // "+ id" — operator before operand
        let g = Grammar::from_string("E -> T E'\nE' -> + T E' | ε\nT -> id").unwrap();
        let p = LL1Parser::new(g).unwrap();
        assert!(p.parse(vec!["+".to_string(), "id".to_string()]).is_err());
    }

    #[test]
    fn test_ll1_parse_rejects_recursive_wrong_close() {
        // S -> a S b | c: "a c a b" is wrong (mismatched brackets)
        let g = Grammar::from_string("S -> a S b | c").unwrap();
        let p = LL1Parser::new(g).unwrap();
        assert!(p.parse(vec!["a".to_string(), "c".to_string(), "a".to_string(),
                              "b".to_string()]).is_err());
    }

    #[test]
    fn test_ll1_parse_rejects_extra_tokens() {
        // "a b c" when grammar only expects "a b"
        let g = Grammar::from_string("S -> a b").unwrap();
        let g2 = Grammar::from_string("S -> a b").unwrap();
        let p = LL1Parser::new(g2).unwrap();
        // "a b" should accept, "a b c" should fail
        assert!(p.parse(vec!["a".to_string(), "b".to_string()]).is_ok());
        let p2 = LL1Parser::new(g).unwrap();
        // extra token 'c' — after matching 'b', stack has '$', input has 'c' → mismatch
        assert!(p2.parse(vec!["a".to_string(), "b".to_string(), "c".to_string()]).is_err());
    }
}

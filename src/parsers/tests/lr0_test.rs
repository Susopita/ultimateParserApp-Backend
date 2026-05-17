#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use crate::core::models::{Grammar, Symbol};
    use crate::parsers::lr0::{LR0Action, LR0Item, LR0Parser};

    // ========================
    // Construction — grammar classification
    // ========================

    #[test]
    fn test_lr0_valid_single_production_builds() {
        let g = Grammar::from_string("S -> a").unwrap();
        assert!(LR0Parser::new(g).is_ok());
    }

    #[test]
    fn test_lr0_valid_two_symbol_production_builds() {
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        assert!(LR0Parser::new(g).is_ok());
    }

    #[test]
    fn test_lr0_valid_epsilon_production_builds() {
        // S -> a b S | ε is LR(0) (shift-resolve wins over reduce)
        let g = Grammar::from_string("S -> a b S | ϵ").unwrap();
        assert!(LR0Parser::new(g).is_ok());
    }

    #[test]
    fn test_lr0_valid_recursive_grammar_builds() {
        // S -> a S b | c — LR(0)
        let g = Grammar::from_string("S -> a S b | c").unwrap();
        assert!(LR0Parser::new(g).is_ok());
    }

    #[test]
    fn test_lr0_rejects_reduce_reduce_conflict() {
        // S -> A | B, A -> a, B -> a: in the state {A→a·, B→a·},
        // LR(0) must reduce for ALL terminals — two different reductions → error
        let g = Grammar::from_string("S -> A | B\nA -> a\nB -> a").unwrap();
        assert!(LR0Parser::new(g).is_err());
        assert!(LR0Parser::new(Grammar::from_string("S -> A | B\nA -> a\nB -> a").unwrap())
            .err().unwrap()
            .contains("LR(0)"));
    }

    #[test]
    fn test_lr0_rejects_ambiguous_grammar() {
        // S -> a B | a C, B -> b, C -> b: after "ab", two complete items with same RHS
        let g = Grammar::from_string("S -> a B | a C\nB -> b\nC -> b").unwrap();
        assert!(LR0Parser::new(g).is_err());
    }

    // ========================
    // White-box: canonical collection
    // ========================

    #[test]
    fn test_lr0_state_count_single_production() {
        // S → a: 3 states: I0={S'→.S, S→.a}, I1={S'→S.}, I2={S→a.}
        let g = Grammar::from_string("S -> a").unwrap();
        let p = LR0Parser::new(g).unwrap();
        assert_eq!(p.states.len(), 3);
    }

    #[test]
    fn test_lr0_state_count_two_productions() {
        // S → a B, B → b: 5 states
        // I0={S'→.S, S→.aB}, I1={S'→S.}, I2={S→a.B, B→.b},
        // I3={S→aB.}, I4={B→b.}
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = LR0Parser::new(g).unwrap();
        assert_eq!(p.states.len(), 5);
    }

    #[test]
    fn test_lr0_initial_item_in_state_0() {
        // State 0 must contain [S' → · S]
        let g = Grammar::from_string("S -> a").unwrap();
        let p = LR0Parser::new(g).unwrap();
        let state0 = &p.states[0];
        let has_initial = state0.iter().any(|item| {
            item.production.left == Symbol::NonTerminal("S'".to_string())
                && item.dot_position == 0
        });
        assert!(has_initial, "State 0 must contain S' → · S");
    }

    #[test]
    fn test_lr0_closure_adds_reachable_items() {
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = LR0Parser::new(g).unwrap();

        let initial_item = LR0Item::new(p.augmented_grammar.productions[0].clone(), 0);
        let mut init_set = HashSet::new();
        init_set.insert(initial_item);
        let closure = p.closure(&init_set);

        // Must contain S' → · S and S → · a B
        assert!(closure.len() >= 2);
        let has_s_prod = closure.iter().any(|item| {
            matches!(&item.production.right[0], Symbol::Terminal(t) if t == "a")
                && item.dot_position == 0
        });
        assert!(has_s_prod, "Closure must contain S → · a B");
    }

    #[test]
    fn test_lr0_transitions_not_empty() {
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = LR0Parser::new(g).unwrap();
        assert!(!p.transitions.is_empty());
    }

    // ========================
    // White-box: ACTION and GOTO tables
    // ========================

    #[test]
    fn test_lr0_action_table_has_accept() {
        // For any grammar, the Accept action must be in the table
        let g = Grammar::from_string("S -> a").unwrap();
        let p = LR0Parser::new(g).unwrap();
        let has_accept = p.action_table.values().any(|a| {
            matches!(a, LR0Action::Accept)
        });
        assert!(has_accept, "ACTION table must contain at least one Accept entry");
    }

    #[test]
    fn test_lr0_action_table_has_shift_for_first_terminal() {
        // In state 0 of S → a, there must be a Shift action on 'a'
        let g = Grammar::from_string("S -> a").unwrap();
        let p = LR0Parser::new(g).unwrap();
        let shift_exists = p.action_table.iter().any(|((_, sym), action)| {
            sym == "a" && matches!(action, LR0Action::Shift(_))
        });
        assert!(shift_exists, "Should have Shift on terminal 'a'");
    }

    #[test]
    fn test_lr0_goto_table_has_entry_for_start_symbol() {
        // After reducing to S, GOTO(0, S) must lead to the accept state
        let g = Grammar::from_string("S -> a").unwrap();
        let p = LR0Parser::new(g).unwrap();
        let has_s_goto = p.goto_table.keys().any(|(_, sym)| sym == "S");
        assert!(has_s_goto, "GOTO table must have an entry for 'S'");
    }

    #[test]
    fn test_lr0_get_all_terminals_includes_dollar() {
        let g = Grammar::from_string("S -> a b").unwrap();
        let p = LR0Parser::new(g).unwrap();
        let terminals = p.get_all_terminals();
        assert!(terminals.contains(&"$".to_string()));
    }

    #[test]
    fn test_lr0_get_all_nonterminals_excludes_augmented() {
        // S' (augmented start) must NOT appear in user-facing non-terminals
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = LR0Parser::new(g).unwrap();
        let nts = p.get_all_non_terminals();
        assert!(!nts.contains(&"S'".to_string()));
        assert!(nts.contains(&"S".to_string()));
        assert!(nts.contains(&"B".to_string()));
    }

    // ========================
    // Parse — accept
    // ========================

    #[test]
    fn test_lr0_parse_single_token() {
        let g = Grammar::from_string("S -> a").unwrap();
        let p = LR0Parser::new(g).unwrap();
        let result = p.parse_input(vec!["a".to_string()]);
        assert!(result.is_ok());
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_lr0_parse_two_tokens() {
        let g = Grammar::from_string("S -> a b").unwrap();
        let p = LR0Parser::new(g).unwrap();
        let result = p.parse_input(vec!["a".to_string(), "b".to_string()]);
        assert!(result.is_ok());
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_lr0_parse_multi_token_with_nonterminal() {
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = LR0Parser::new(g).unwrap();
        let result = p.parse_input(vec!["a".to_string(), "b".to_string()]);
        assert!(result.is_ok());
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_lr0_parse_recursive_base_case() {
        // S -> a S b | c: "c" is the base case
        let g = Grammar::from_string("S -> a S b | c").unwrap();
        let p = LR0Parser::new(g).unwrap();
        let result = p.parse_input(vec!["c".to_string()]);
        assert!(result.is_ok(), "Expected accept for 'c': {:?}", result);
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_lr0_parse_recursive_one_level() {
        // S -> a S b | c: "a c b"
        let g = Grammar::from_string("S -> a S b | c").unwrap();
        let p = LR0Parser::new(g).unwrap();
        let result = p.parse_input(vec!["a".to_string(), "c".to_string(), "b".to_string()]);
        assert!(result.is_ok(), "Expected accept for 'a c b': {:?}", result);
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_lr0_parse_recursive_two_levels() {
        // S -> a S b | c: "a a c b b"
        let g = Grammar::from_string("S -> a S b | c").unwrap();
        let p = LR0Parser::new(g).unwrap();
        let result = p.parse_input(vec![
            "a".to_string(), "a".to_string(), "c".to_string(),
            "b".to_string(), "b".to_string(),
        ]);
        assert!(result.is_ok(), "Expected accept for 'a a c b b': {:?}", result);
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_lr0_parse_epsilon_base() {
        // S -> a b S | ε: empty input accepted
        let g = Grammar::from_string("S -> a b S | ϵ").unwrap();
        let p = LR0Parser::new(g).unwrap();
        let result = p.parse_input(vec![]);
        // LR(0) resolves shift-reduce by favoring shift, epsilon reduce on $ is fine
        assert!(result.is_ok(), "Expected accept for empty input: {:?}", result);
    }

    #[test]
    fn test_lr0_parse_epsilon_with_tokens() {
        // S -> a b S | ε: "a b" is accepted (S→ε at the end)
        let g = Grammar::from_string("S -> a b S | ϵ").unwrap();
        let p = LR0Parser::new(g).unwrap();
        let result = p.parse_input(vec!["a".to_string(), "b".to_string()]);
        assert!(result.is_ok(), "Expected accept for 'a b': {:?}", result);
    }

    #[test]
    fn test_lr0_parse_snapshots_contain_accept_action() {
        let g = Grammar::from_string("S -> a").unwrap();
        let p = LR0Parser::new(g).unwrap();
        let snaps = p.parse_input(vec!["a".to_string()]).unwrap();
        assert!(snaps.last().unwrap().action.contains("Accept"));
    }

    // ========================
    // Parse — reject
    // ========================

    #[test]
    fn test_lr0_parse_rejects_wrong_single_token() {
        let g = Grammar::from_string("S -> a").unwrap();
        let p = LR0Parser::new(g).unwrap();
        assert!(p.parse_input(vec!["b".to_string()]).is_err());
    }

    #[test]
    fn test_lr0_parse_rejects_missing_second_token() {
        // S → a b: input ["a"] must fail
        let g = Grammar::from_string("S -> a b").unwrap();
        let p = LR0Parser::new(g).unwrap();
        assert!(p.parse_input(vec!["a".to_string()]).is_err());
    }

    #[test]
    fn test_lr0_parse_rejects_wrong_middle_token() {
        // S → a B, B → b: input ["a", "x"] must fail
        let g = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let p = LR0Parser::new(g).unwrap();
        assert!(p.parse_input(vec!["a".to_string(), "x".to_string()]).is_err());
    }

    #[test]
    fn test_lr0_parse_rejects_mismatched_parentheses() {
        // S → a S b | c: "a c" missing closing 'b'
        let g = Grammar::from_string("S -> a S b | c").unwrap();
        let p = LR0Parser::new(g).unwrap();
        assert!(p.parse_input(vec!["a".to_string(), "c".to_string()]).is_err());
    }

    #[test]
    fn test_lr0_parse_rejects_empty_for_non_nullable_grammar() {
        let g = Grammar::from_string("S -> a b").unwrap();
        let p = LR0Parser::new(g).unwrap();
        assert!(p.parse_input(vec![]).is_err());
    }
}

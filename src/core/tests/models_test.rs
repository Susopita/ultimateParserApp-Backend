#[cfg(test)]
mod tests {
    use crate::core::models::{Grammar, Symbol};

    // ========================
    // Grammar::from_string — parsing
    // ========================

    #[test]
    fn test_grammar_parsing_simple() {
        let raw = "S -> A B | a";
        let grammar = Grammar::from_string(raw).unwrap();

        assert_eq!(grammar.start_symbol, Symbol::NonTerminal("S".to_string()));
        assert_eq!(grammar.productions.len(), 2);
        assert_eq!(
            grammar.productions[0].right,
            vec![Symbol::NonTerminal("A".to_string()), Symbol::NonTerminal("B".to_string())]
        );
        assert_eq!(grammar.productions[1].right, vec![Symbol::Terminal("a".to_string())]);
    }

    #[test]
    fn test_grammar_parsing_complex() {
        let raw = "S → A B\nA → a | ϵ\nB → b";
        let grammar = Grammar::from_string(raw).unwrap();

        assert_eq!(grammar.productions.len(), 4);

        let a_eps = grammar
            .productions
            .iter()
            .find(|p| p.left == Symbol::NonTerminal("A".to_string()) && p.right == vec![Symbol::Epsilon]);
        assert!(a_eps.is_some());
        assert!(!grammar.is_left_recursive());
    }

    #[test]
    fn test_grammar_unicode_arrow() {
        let grammar = Grammar::from_string("S → a b").unwrap();
        assert_eq!(grammar.productions.len(), 1);
        assert_eq!(
            grammar.productions[0].right,
            vec![Symbol::Terminal("a".to_string()), Symbol::Terminal("b".to_string())]
        );
    }

    #[test]
    fn test_grammar_epsilon_keyword() {
        let grammar = Grammar::from_string("A -> epsilon").unwrap();
        assert_eq!(grammar.productions[0].right, vec![Symbol::Epsilon]);
    }

    #[test]
    fn test_grammar_epsilon_unicode_u03b5() {
        // ε = U+03B5 (via parse_symbol path)
        let grammar = Grammar::from_string("A -> ε").unwrap();
        assert_eq!(grammar.productions[0].right, vec![Symbol::Epsilon]);
    }

    #[test]
    fn test_grammar_epsilon_unicode_u03f5() {
        // ϵ = U+03F5 (via early-check path)
        let grammar = Grammar::from_string("A -> ϵ").unwrap();
        assert_eq!(grammar.productions[0].right, vec![Symbol::Epsilon]);
    }

    #[test]
    fn test_grammar_multiple_alternatives_count() {
        // S -> a | b | c must produce exactly 3 productions
        let grammar = Grammar::from_string("S -> a | b | c").unwrap();
        assert_eq!(grammar.productions.len(), 3);
    }

    #[test]
    fn test_grammar_start_symbol_is_first_lhs() {
        let grammar = Grammar::from_string("A -> a\nB -> b\nC -> c").unwrap();
        assert_eq!(grammar.start_symbol, Symbol::NonTerminal("A".to_string()));
    }

    #[test]
    fn test_grammar_invalid_no_arrow_returns_error() {
        let result = Grammar::from_string("S a b");
        assert!(result.is_err());
    }

    #[test]
    fn test_grammar_empty_input_returns_error() {
        let result = Grammar::from_string("");
        assert!(result.is_err());
    }

    #[test]
    fn test_grammar_whitespace_only_returns_error() {
        let result = Grammar::from_string("   \n  \n  ");
        assert!(result.is_err());
    }

    #[test]
    fn test_grammar_terminal_as_lhs_returns_error() {
        // 'a' starts with lowercase → Terminal → must be rejected as LHS
        let result = Grammar::from_string("a -> b");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Non-Terminal"));
    }

    #[test]
    fn test_grammar_nonterminal_with_prime() {
        // E' starts with uppercase 'E' → NonTerminal
        let grammar = Grammar::from_string("E -> T E'\nE' -> + T E' | ε\nT -> id").unwrap();
        let ep = grammar
            .productions
            .iter()
            .find(|p| p.left == Symbol::NonTerminal("E'".to_string()));
        assert!(ep.is_some());
    }

    // ========================
    // Grammar::is_left_recursive
    // ========================

    #[test]
    fn test_left_recursion_direct() {
        // S -> S a | b  (direct left recursion)
        let grammar = Grammar::from_string("S -> S a | b").unwrap();
        assert!(grammar.is_left_recursive());
    }

    #[test]
    fn test_left_recursion_indirect() {
        // A -> B a, B -> A b | b  (indirect: A→B→A)
        let grammar = Grammar::from_string("A -> B a\nB -> A b | b").unwrap();
        assert!(grammar.is_left_recursive());
    }

    #[test]
    fn test_no_left_recursion_right_recursive() {
        // S -> a S | a  (right-recursive, NOT left-recursive)
        let grammar = Grammar::from_string("S -> a S | a").unwrap();
        assert!(!grammar.is_left_recursive());
    }

    #[test]
    fn test_no_left_recursion_simple_chain() {
        // E -> T E', E' -> + T E' | ε, T -> id — right-recursive, no left recursion
        let grammar = Grammar::from_string("E -> T E'\nE' -> + T E' | ε\nT -> id").unwrap();
        assert!(!grammar.is_left_recursive());
    }

    #[test]
    fn test_no_left_recursion_flat_grammar() {
        let grammar = Grammar::from_string("S -> a B c\nB -> b").unwrap();
        assert!(!grammar.is_left_recursive());
    }
}

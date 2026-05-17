#[cfg(test)]
mod tests {
    use crate::core::models::Grammar;
    use crate::parsers::recursive_descent::RecursiveDescentParser;

    #[test]
    fn test_rd_simple_accept() {
        let grammar = Grammar::from_string("S -> a").unwrap();
        let parser = RecursiveDescentParser::new(grammar).unwrap();
        let result = parser.parse_input(vec!["a".to_string()]);
        assert!(result.is_ok());
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_rd_multi_token() {
        let grammar = Grammar::from_string("S -> a B\nB -> b").unwrap();
        let parser = RecursiveDescentParser::new(grammar).unwrap();
        let result = parser.parse_input(vec!["a".to_string(), "b".to_string()]);
        assert!(result.is_ok());
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_rd_epsilon() {
        let grammar = Grammar::from_string("S -> A B\nA -> a | ϵ\nB -> b").unwrap();
        let parser = RecursiveDescentParser::new(grammar).unwrap();
        let result = parser.parse_input(vec!["b".to_string()]);
        assert!(result.is_ok(), "Should accept 'b' via A → ε: {:?}", result);
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_rd_backtracking() {
        // Grammar with alternatives — must backtrack from first to second
        let grammar = Grammar::from_string("S -> a b | a c\nC -> c").unwrap();
        let parser = RecursiveDescentParser::new(grammar).unwrap();
        let result = parser.parse_input(vec!["a".to_string(), "c".to_string()]);
        assert!(result.is_ok(), "Should accept 'a c' via backtracking: {:?}", result);
        let snapshots = result.unwrap();
        // Must contain at least one Backtrack snapshot
        let has_backtrack = snapshots.iter().any(|s| s.action.contains("Backtrack"));
        assert!(has_backtrack, "Backtracking should be visible in snapshots");
    }

    #[test]
    fn test_rd_error_input() {
        let grammar = Grammar::from_string("S -> a").unwrap();
        let parser = RecursiveDescentParser::new(grammar).unwrap();
        let result = parser.parse_input(vec!["b".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_rd_rejects_left_recursive() {
        let grammar = Grammar::from_string("S -> S a | a").unwrap();
        let result = RecursiveDescentParser::new(grammar);
        assert!(result.is_err(), "Should reject left-recursive grammars");
        assert!(result.unwrap_err().contains("left-recursive"));
    }
}

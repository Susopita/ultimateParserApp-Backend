#[cfg(test)]
mod tests {
    use crate::core::models::{Grammar, Symbol};

    #[test]
    fn test_grammar_parsing_simple() {
        let raw = "S -> A B | a";
        let grammar = Grammar::from_string(raw).unwrap();
        
        assert_eq!(grammar.start_symbol, Symbol::NonTerminal("S".to_string()));
        assert_eq!(grammar.productions.len(), 2);
        
        // S -> A B
        assert_eq!(grammar.productions[0].left, Symbol::NonTerminal("S".to_string()));
        assert_eq!(grammar.productions[0].right, vec![
            Symbol::NonTerminal("A".to_string()),
            Symbol::NonTerminal("B".to_string())
        ]);
        
        // S -> a
        assert_eq!(grammar.productions[1].right, vec![Symbol::Terminal("a".to_string())]);
    }

    #[test]
    fn test_grammar_parsing_complex() {
        let raw = "S → A B\nA → a | ϵ\nB → b";
        let grammar = Grammar::from_string(raw).unwrap();
        
        assert_eq!(grammar.productions.len(), 4);
        
        // Check A -> ϵ
        let a_epsilon = grammar.productions.iter().find(|p| {
            p.left == Symbol::NonTerminal("A".to_string()) && p.right == vec![Symbol::Epsilon]
        });
        assert!(a_epsilon.is_some());
        
        // Check recursion (S -> A B, A -> a, B -> b) - not recursive
        assert!(!grammar.is_left_recursive());
    }

    #[test]
    fn test_left_recursion_detection() {
        let raw = "S -> S a | b";
        let grammar = Grammar::from_string(raw).unwrap();
        assert!(grammar.is_left_recursive());
    }
}

#[cfg(test)]
mod tests {
    use crate::core::models::Grammar;
    use crate::parsers::lr1::LR1Parser;

    #[test]
    fn test_lr1_simple() {
        let raw = "S -> a";
        let grammar = Grammar::from_string(raw).unwrap();
        let parser = LR1Parser::new(grammar).unwrap();
        let result = parser.parse_input(vec!["a".to_string()]);
        assert!(result.is_ok());
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_lr1_multi_token() {
        let raw = "S -> a B\nB -> b";
        let grammar = Grammar::from_string(raw).unwrap();
        let parser = LR1Parser::new(grammar).unwrap();
        let result = parser.parse_input(vec!["a".to_string(), "b".to_string()]);
        assert!(result.is_ok());
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_lr1_with_epsilon() {
        let raw = "S -> A B\nA -> a | ϵ\nB -> b";
        let grammar = Grammar::from_string(raw).unwrap();
        let parser = LR1Parser::new(grammar).unwrap();
        let result = parser.parse_input(vec!["a".to_string(), "b".to_string()]);
        assert!(result.is_ok());
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_lr1_error_input() {
        let raw = "S -> a";
        let grammar = Grammar::from_string(raw).unwrap();
        let parser = LR1Parser::new(grammar).unwrap();
        let result = parser.parse_input(vec!["b".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_lr1_initial_item_has_dollar_lookahead() {
        let raw = "S -> a b";
        let grammar = Grammar::from_string(raw).unwrap();
        let parser = LR1Parser::new(grammar).unwrap();
        let state0 = &parser.states[0];
        let has_initial = state0.iter().any(|item| {
            item.production.left == crate::core::models::Symbol::NonTerminal("S'".to_string())
                && item.dot_position == 0
                && item.lookahead == "$"
        });
        assert!(has_initial, "State 0 must contain [S' → · S, $]");
    }
}

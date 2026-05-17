#[cfg(test)]
mod tests {
    use crate::core::models::Grammar;
    use crate::parsers::slr1::SLR1Parser;

    #[test]
    fn test_slr1_simple() {
        let raw = "S -> a";
        let grammar = Grammar::from_string(raw).unwrap();
        let parser = SLR1Parser::new(grammar).unwrap();
        let result = parser.parse_input(vec!["a".to_string()]);
        assert!(result.is_ok());
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_slr1_multi_token() {
        let raw = "S -> a B\nB -> b";
        let grammar = Grammar::from_string(raw).unwrap();
        let parser = SLR1Parser::new(grammar).unwrap();
        let result = parser.parse_input(vec!["a".to_string(), "b".to_string()]);
        assert!(result.is_ok());
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_slr1_reduces_conflict_that_lr0_cant() {
        let raw = "S -> A B\nA -> a | ϵ\nB -> b";
        let grammar = Grammar::from_string(raw).unwrap();
        let parser = SLR1Parser::new(grammar).unwrap();
        let result = parser.parse_input(vec!["a".to_string(), "b".to_string()]);
        assert!(result.is_ok());
        assert!(result.unwrap().last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_slr1_follow_set() {
        let raw = "S -> A B\nA -> a | ϵ\nB -> b";
        let grammar = Grammar::from_string(raw).unwrap();
        let parser = SLR1Parser::new(grammar).unwrap();
        let follow_a = parser.follow.get("A").unwrap();
        assert!(follow_a.contains("b"));
    }

    #[test]
    fn test_slr1_error_input() {
        let raw = "S -> a";
        let grammar = Grammar::from_string(raw).unwrap();
        let parser = SLR1Parser::new(grammar).unwrap();
        let result = parser.parse_input(vec!["b".to_string()]);
        assert!(result.is_err());
    }
}

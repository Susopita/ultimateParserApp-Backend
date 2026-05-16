#[cfg(test)]
mod tests {
    use crate::core::models::Grammar;
    use crate::parsers::ll1::LL1Parser;
    use crate::parsers::Parser;

    #[test]
    fn test_ll1_parsing_simple() {
        let raw = "S -> a A\nA -> b | ϵ";
        let grammar = Grammar::from_string(raw).unwrap();
        let parser = LL1Parser::new(grammar).unwrap();
        
        let input = vec!["a".to_string(), "b".to_string()];
        let result = parser.parse(input);
        assert!(result.is_ok());
        
        let snapshots = result.unwrap();
        assert!(snapshots.len() > 0);
        assert_eq!(snapshots.last().unwrap().action, "Success!");
    }

    #[test]
    fn test_ll1_error() {
        let raw = "S -> a A\nA -> b | ϵ";
        let grammar = Grammar::from_string(raw).unwrap();
        let parser = LL1Parser::new(grammar).unwrap();
        
        let input = vec!["c".to_string()];
        let result = parser.parse(input);
        assert!(result.is_err());
    }
}

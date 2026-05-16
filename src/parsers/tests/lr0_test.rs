#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use crate::core::models::Grammar;
    use crate::parsers::lr0::{LR0Parser, LR0Item};

    #[test]
    fn test_lr0_closure() {
        let raw = "S -> E\nE -> E + T | T\nT -> id";
        let grammar = Grammar::from_string(raw).unwrap();
        let parser = LR0Parser::new(grammar.clone());
        
        // Initial item: S -> .E
        let initial_item = LR0Item::new(grammar.productions[0].clone(), 0);
        let mut initial_set = HashSet::new();
        initial_set.insert(initial_item);
        
        let closure = parser.closure(&initial_set);
        
        // Closure should contain S -> .E, E -> .E + T, E -> .T, T -> .id
        assert!(closure.len() >= 4);
        
        let has_t_id = closure.iter().any(|item| {
            if let crate::core::models::Symbol::Terminal(val) = &item.production.right[0] {
                val == "id" && item.dot_position == 0
            } else {
                false
            }
        });
        assert!(has_t_id);
    }
}

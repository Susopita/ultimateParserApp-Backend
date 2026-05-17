#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use crate::core::models::Grammar;
    use crate::parsers::lr0::{LR0Parser, LR0Item};

    #[test]
    fn test_lr0_closure() {
        // Use a grammar that IS LR(0)
        let raw = "S -> a B\nB -> b";
        let grammar = Grammar::from_string(raw).unwrap();
        let parser = LR0Parser::new(grammar).unwrap();

        // Initial item: S' → .S (augmented grammar's first production)
        let initial_item = LR0Item::new(parser.augmented_grammar.productions[0].clone(), 0);
        let mut initial_set = HashSet::new();
        initial_set.insert(initial_item);

        let closure = parser.closure(&initial_set);

        // Closure should contain S' → .S, S → .a B
        assert!(closure.len() >= 2);

        let has_s_ab = closure.iter().any(|item| {
            if let crate::core::models::Symbol::Terminal(val) = &item.production.right[0] {
                val == "a" && item.dot_position == 0
            } else {
                false
            }
        });
        assert!(has_s_ab);
    }

    #[test]
    fn test_lr0_goto() {
        let raw = "S -> a B\nB -> b";
        let grammar = Grammar::from_string(raw).unwrap();
        let parser = LR0Parser::new(grammar).unwrap();

        // State 0 should exist and have transitions
        assert!(!parser.states.is_empty());
        assert!(!parser.transitions.is_empty());
    }

    #[test]
    fn test_lr0_canonical_collection() {
        // Simple LR(0) grammar: S → a
        let raw = "S -> a";
        let grammar = Grammar::from_string(raw).unwrap();
        let parser = LR0Parser::new(grammar).unwrap();

        // Should have states: I0={S'→.S, S→.a}, I1={S'→S.}, I2={S→a.}
        assert_eq!(parser.states.len(), 3);
    }

    #[test]
    fn test_lr0_parsing_simple() {
        let raw = "S -> a";
        let grammar = Grammar::from_string(raw).unwrap();
        let parser = LR0Parser::new(grammar).unwrap();

        let input = vec!["a".to_string()];
        let result = parser.parse_input(input);
        assert!(result.is_ok());

        let snapshots = result.unwrap();
        assert!(snapshots.last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_lr0_parsing_multi_token() {
        let raw = "S -> a B\nB -> b";
        let grammar = Grammar::from_string(raw).unwrap();
        let parser = LR0Parser::new(grammar).unwrap();

        let input = vec!["a".to_string(), "b".to_string()];
        let result = parser.parse_input(input);
        assert!(result.is_ok());

        let snapshots = result.unwrap();
        assert!(snapshots.last().unwrap().action.contains("Accept"));
    }

    #[test]
    fn test_lr0_parsing_error() {
        let raw = "S -> a";
        let grammar = Grammar::from_string(raw).unwrap();
        let parser = LR0Parser::new(grammar).unwrap();

        let input = vec!["b".to_string()];
        let result = parser.parse_input(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_lr0_reduce_reduce_conflict() {
        // This grammar has a reduce-reduce conflict under LR(0)
        let raw = "S -> A | B\nA -> a\nB -> a";
        let grammar = Grammar::from_string(raw).unwrap();
        let result = LR0Parser::new(grammar);
        // Grammar with S -> A | B, A -> a, B -> a is NOT LR(0) due to reduce-reduce conflict
        assert!(result.is_err());
    }

    #[test]
    fn test_lr0_epsilon() {
        let raw = "S -> a b S | ϵ";
        let grammar = Grammar::from_string(raw).unwrap();
        let parser = LR0Parser::new(grammar).unwrap();
        
        let input = vec!["a".to_string(), "b".to_string(), "a".to_string(), "b".to_string()];
        
        println!("States:");
        for (i, items) in parser.states.iter().enumerate() {
            println!("State {}:", i);
            for item in items {
                println!("  {}", item.to_display_string());
            }
        }
        
        println!("\nAction Table:");
        for ((state, symbol), action) in &parser.action_table {
            println!("({}, {}) -> {:?}", state, symbol, action.to_display_string());
        }
        
        println!("\nGoto Table:");
        for ((state, symbol), to_state) in &parser.goto_table {
            println!("({}, {}) -> {}", state, symbol, to_state);
        }
        
        match parser.parse_input(input) {
            Ok(snapshots) => {
                println!("Success!");
                for s in snapshots {
                    println!("{:?}", s);
                }
            }
            Err(e) => println!("Error: {}", e),
        }
    }
}

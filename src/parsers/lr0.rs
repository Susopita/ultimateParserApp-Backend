use std::collections::HashSet;
use crate::core::models::{Symbol, Production, Grammar};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LR0Item {
    pub production: Production,
    pub dot_position: usize,
}

impl LR0Item {
    pub fn new(production: Production, dot_position: usize) -> Self {
        Self { production, dot_position }
    }

    pub fn next_symbol(&self) -> Option<&Symbol> {
        self.production.right.get(self.dot_position)
    }
}

pub struct LR0Parser {
    pub grammar: Grammar,
}

impl LR0Parser {
    pub fn new(grammar: Grammar) -> Self {
        Self { grammar }
    }

    /// Computes the closure of a set of LR(0) items.
    pub fn closure(&self, items: &HashSet<LR0Item>) -> HashSet<LR0Item> {
        let mut closure = items.clone();
        let mut changed = true;

        while changed {
            changed = false;
            let mut to_add = HashSet::new();

            for item in &closure {
                if let Some(Symbol::NonTerminal(nt_name)) = item.next_symbol() {
                    // Find all productions for this NonTerminal
                    for prod in &self.grammar.productions {
                        if let Symbol::NonTerminal(name) = &prod.left {
                            if name == nt_name {
                                let new_item = LR0Item::new(prod.clone(), 0);
                                if !closure.contains(&new_item) {
                                    to_add.insert(new_item);
                                }
                            }
                        }
                    }
                }
            }

            if !to_add.is_empty() {
                for item in to_add {
                    closure.insert(item);
                }
                changed = true;
            }
        }

        closure
    }
}

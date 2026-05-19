use crate::core::models::{Grammar, Symbol, Production};

impl Grammar {
    /// Parses a raw grammar string into a Grammar struct.
    /// Format: S -> A B | a
    /// Symbols starting with Uppercase are NonTerminals.
    /// Others are Terminals. 'ϵ' or 'epsilon' are Epsilon.
    pub fn from_string(input: &str) -> Result<Self, String> {
        let mut productions = Vec::new();
        let mut start_symbol = None;
        let mut non_terminals = std::collections::HashSet::new();

        // First pass: identify all Non-Terminals (left-hand sides)
        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = if line.contains("->") {
                line.split("->").collect()
            } else if line.contains("→") {
                line.split("→").collect()
            } else {
                return Err(format!("Invalid production format: {}", line));
            };
            if parts.len() != 2 {
                return Err(format!("Invalid production format: {}", line));
            }
            let left_raw = parts[0].trim();
            non_terminals.insert(left_raw.to_string());
        }

        // Second pass: build productions
        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = if line.contains("->") {
                line.split("->").collect()
            } else {
                line.split("→").collect()
            };

            let left_raw = parts[0].trim();
            let left_symbol = Symbol::NonTerminal(left_raw.to_string());

            if start_symbol.is_none() {
                start_symbol = Some(left_symbol.clone());
            }

            let alternatives: Vec<&str> = parts[1].split('|').collect();
            for alt in alternatives {
                let alt = alt.trim();
                let mut right_symbols = Vec::new();

                if alt.is_empty() || alt == "ϵ" || alt == "ε" || alt == "epsilon" {
                    right_symbols.push(Symbol::Epsilon);
                } else {
                    for word in alt.split_whitespace() {
                        let sym = if word == "ϵ" || word == "ε" || word == "epsilon" {
                            Symbol::Epsilon
                        } else if non_terminals.contains(word) {
                            Symbol::NonTerminal(word.to_string())
                        } else {
                            Symbol::Terminal(word.to_string())
                        };
                        right_symbols.push(sym);
                    }
                }

                productions.push(Production {
                    left: left_symbol.clone(),
                    right: right_symbols,
                });
            }
        }

        let start_symbol = start_symbol.ok_or("Grammar must have at least one production")?;

        Ok(Grammar {
            productions,
            start_symbol,
        })
    }

    /// Checks if the grammar has any left recursion (direct or indirect).
    pub fn is_left_recursive(&self) -> bool {
        let non_terminals: Vec<Symbol> = self.productions.iter()
            .map(|p| p.left.clone())
            .filter(|s| matches!(s, Symbol::NonTerminal(_)))
            .collect();
        
        let mut unique_nt = Vec::new();
        for nt in non_terminals {
            if !unique_nt.contains(&nt) {
                unique_nt.push(nt);
            }
        }

        for prod in &self.productions {
            if !prod.right.is_empty() && prod.left == prod.right[0] {
                return true;
            }
        }
        
        for nt in &unique_nt {
            if self.has_cycle(nt, &mut Vec::new()) {
                return true;
            }
        }

        false
    }

    fn has_cycle(&self, current: &Symbol, visited: &mut Vec<Symbol>) -> bool {
        if visited.contains(current) {
            return true;
        }

        visited.push(current.clone());

        for prod in &self.productions {
            if &prod.left == current {
                if let Some(Symbol::NonTerminal(_)) = prod.right.first() {
                    if self.has_cycle(&prod.right[0], visited) {
                        return true;
                    }
                }
            }
        }

        visited.pop();
        false
    }
}

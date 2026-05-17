use crate::core::models::{Grammar, Symbol, Production};

impl Grammar {
    /// Parses a raw grammar string into a Grammar struct.
    /// Format: S -> A B | a
    /// Symbols starting with Uppercase are NonTerminals.
    /// Others are Terminals. 'ϵ' or 'epsilon' are Epsilon.
    pub fn from_string(input: &str) -> Result<Self, String> {
        let mut productions = Vec::new();
        let mut start_symbol = None;

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Split Left and Right by -> or →
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
            let left_symbol = Self::parse_symbol(left_raw);

            if !matches!(left_symbol, Symbol::NonTerminal(_)) {
                return Err(format!("Left side must be a Non-Terminal: {}", left_raw));
            }

            // Set start symbol if not set
            if start_symbol.is_none() {
                start_symbol = Some(left_symbol.clone());
            }

            // Split alternatives by |
            let alternatives: Vec<&str> = parts[1].split('|').collect();
            for alt in alternatives {
                let alt = alt.trim();
                let mut right_symbols = Vec::new();

                if alt.is_empty() || alt == "ϵ" || alt == "epsilon" {
                    right_symbols.push(Symbol::Epsilon);
                } else {
                    for word in alt.split_whitespace() {
                        right_symbols.push(Self::parse_symbol(word));
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

    fn parse_symbol(s: &str) -> Symbol {
        match s {
            "ϵ" | "ε" | "epsilon" => Symbol::Epsilon,
            _ if s.chars().next().map_or(false, |c| c.is_uppercase()) => {
                Symbol::NonTerminal(s.to_string())
            }
            _ => Symbol::Terminal(s.to_string()),
        }
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

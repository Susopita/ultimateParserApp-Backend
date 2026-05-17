use std::collections::{HashMap, HashSet};
use crate::core::models::{Grammar, Symbol, Production, LR0ParseSnapshot};

/// Represents an LR(0) item: a production with a dot position.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LR0Item {
    pub production: Production,
    pub dot_position: usize,
}

impl LR0Item {
    pub fn new(production: Production, dot_position: usize) -> Self {
        Self { production, dot_position }
    }

    /// Returns the symbol immediately after the dot, or None if the item is complete.
    pub fn next_symbol(&self) -> Option<&Symbol> {
        // ε-productions are immediately complete
        if self.production.right.len() == 1 && self.production.right[0] == Symbol::Epsilon {
            return None;
        }
        self.production.right.get(self.dot_position)
    }

    /// Returns true if the dot is at the end (ready to reduce).
    pub fn is_complete(&self) -> bool {
        if self.production.right.len() == 1 && self.production.right[0] == Symbol::Epsilon {
            return true;
        }
        self.dot_position >= self.production.right.len()
    }

    /// Formats the item as a readable string: "A → α · β"
    pub fn to_display_string(&self) -> String {
        let left = self.production.left.to_string();

        if self.production.right.len() == 1 && self.production.right[0] == Symbol::Epsilon {
            return format!("{} → ·", left);
        }

        let mut parts = Vec::new();
        for (i, sym) in self.production.right.iter().enumerate() {
            if i == self.dot_position {
                parts.push("·".to_string());
            }
            parts.push(sym.to_string());
        }
        if self.dot_position >= self.production.right.len() {
            parts.push("·".to_string());
        }

        format!("{} → {}", left, parts.join(" "))
    }
}

/// Represents an LR(0) action in the parsing table.
#[derive(Debug, Clone)]
pub enum LR0Action {
    Shift(usize),
    Reduce(usize, String), // (production_index, production_display_string)
    Accept,
}

impl LR0Action {
    /// Returns a compact display string like "s3", "r2", or "acc".
    pub fn to_display_string(&self) -> String {
        match self {
            LR0Action::Shift(s) => format!("s{}", s),
            LR0Action::Reduce(idx, _) => format!("r{}", idx),
            LR0Action::Accept => "acc".to_string(),
        }
    }
}

/// Full LR(0) parser with canonical collection, ACTION and GOTO tables.
pub struct LR0Parser {
    pub augmented_grammar: Grammar,
    pub states: Vec<HashSet<LR0Item>>,
    pub transitions: HashMap<(usize, String), usize>,
    pub action_table: HashMap<(usize, String), LR0Action>,
    pub goto_table: HashMap<(usize, String), usize>,
    pub production_list: Vec<Production>,
}

impl LR0Parser {
    /// Creates a new LR(0) parser by augmenting the grammar and building all tables.
    pub fn new(grammar: Grammar) -> Result<Self, String> {
        // Step 1: Augment the grammar with S' → S
        let augmented_start = Symbol::NonTerminal("S'".to_string());
        let augmented_production = Production {
            left: augmented_start.clone(),
            right: vec![grammar.start_symbol.clone()],
        };

        let mut augmented_productions = vec![augmented_production];
        augmented_productions.extend(grammar.productions.clone());

        let augmented_grammar = Grammar {
            productions: augmented_productions,
            start_symbol: augmented_start,
        };

        let production_list = augmented_grammar.productions.clone();

        let mut parser = LR0Parser {
            augmented_grammar,
            states: Vec::new(),
            transitions: HashMap::new(),
            action_table: HashMap::new(),
            goto_table: HashMap::new(),
            production_list,
        };

        // Step 2-4: Build canonical collection and tables
        parser.build_canonical_collection();
        parser.build_tables()?;

        Ok(parser)
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
                    for prod in &self.augmented_grammar.productions {
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

    /// Computes goto(I, X) = closure({[A → αX·β] | [A → α·Xβ] ∈ I})
    pub fn goto(&self, items: &HashSet<LR0Item>, symbol: &Symbol) -> HashSet<LR0Item> {
        let mut moved = HashSet::new();

        for item in items {
            if let Some(next) = item.next_symbol() {
                if next == symbol {
                    moved.insert(LR0Item::new(
                        item.production.clone(),
                        item.dot_position + 1,
                    ));
                }
            }
        }

        if moved.is_empty() {
            return moved;
        }

        self.closure(&moved)
    }

    /// Builds the canonical collection of LR(0) item sets (automaton states).
    fn build_canonical_collection(&mut self) {
        // I₀ = closure({S' → ·S})
        let initial_item = LR0Item::new(
            self.augmented_grammar.productions[0].clone(),
            0,
        );
        let mut initial_set = HashSet::new();
        initial_set.insert(initial_item);
        let i0 = self.closure(&initial_set);

        self.states.push(i0);

        let mut i = 0;
        while i < self.states.len() {
            let symbols = self.get_symbols_after_dot(i);

            for symbol in symbols {
                let state_clone = self.states[i].clone();
                let goto_set = self.goto(&state_clone, &symbol);

                if goto_set.is_empty() {
                    continue;
                }

                let existing = self.states.iter().position(|s| *s == goto_set);

                let target_id = match existing {
                    Some(id) => id,
                    None => {
                        self.states.push(goto_set);
                        self.states.len() - 1
                    }
                };

                let sym_str = symbol.to_string();
                self.transitions.insert((i, sym_str), target_id);
            }

            i += 1;
        }
    }

    /// Collects all distinct symbols appearing right after the dot in a given state.
    fn get_symbols_after_dot(&self, state_id: usize) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        let mut seen = HashSet::new();

        for item in &self.states[state_id] {
            if let Some(next) = item.next_symbol() {
                let key = next.to_string();
                if !seen.contains(&key) {
                    seen.insert(key);
                    symbols.push(next.clone());
                }
            }
        }

        symbols
    }

    /// Builds the ACTION and GOTO tables from the canonical collection.
    fn build_tables(&mut self) -> Result<(), String> {
        let augmented_start = self.augmented_grammar.start_symbol.clone();

        // Collect all terminal symbols present in the grammar
        let mut all_terminals: HashSet<String> = HashSet::new();
        for prod in &self.augmented_grammar.productions {
            for sym in &prod.right {
                if let Symbol::Terminal(t) = sym {
                    all_terminals.insert(t.clone());
                }
            }
        }
        all_terminals.insert("$".to_string());

        for state_id in 0..self.states.len() {
            let items: Vec<LR0Item> = self.states[state_id].iter().cloned().collect();

            for item in &items {
                if item.is_complete() {
                    if item.production.left == augmented_start {
                        // S' → S· → Accept on $
                        self.insert_action(state_id, "$".to_string(), LR0Action::Accept)?;
                    } else {
                        // Reduce by this production for ALL terminals (LR(0) property)
                        let prod_idx = self.production_list.iter()
                            .position(|p| *p == item.production)
                            .unwrap_or(0);
                        let prod_str = format_production(&item.production);

                        for t in &all_terminals {
                            let action = LR0Action::Reduce(prod_idx, prod_str.clone());
                            self.insert_action(state_id, t.clone(), action)?;
                        }
                    }
                } else if let Some(next) = item.next_symbol() {
                    match next {
                        Symbol::Terminal(t) => {
                            if let Some(&target) = self.transitions.get(&(state_id, t.clone())) {
                                let action = LR0Action::Shift(target);
                                self.insert_action(state_id, t.clone(), action)?;
                            }
                        }
                        Symbol::NonTerminal(nt) => {
                            if let Some(&target) = self.transitions.get(&(state_id, nt.clone())) {
                                self.goto_table.insert((state_id, nt.clone()), target);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }

    /// Inserts an action, detecting conflicts (shift-reduce or reduce-reduce).
    /// Resolves Shift-Reduce conflicts by favoring Shift (standard practice).
    fn insert_action(&mut self, state: usize, terminal: String, action: LR0Action) -> Result<(), String> {
        let key = (state, terminal.clone());
        if let Some(existing) = self.action_table.get(&key) {
            let existing_str = existing.to_display_string();
            let new_str = action.to_display_string();
            
            if existing_str != new_str {
                match (existing, &action) {
                    // Favor Shift over Reduce
                    (LR0Action::Shift(_), LR0Action::Reduce(_, _)) => {
                        // Keep the existing Shift action
                        return Ok(());
                    }
                    (LR0Action::Reduce(_, _), LR0Action::Shift(_)) => {
                        // Overwrite Reduce with Shift
                        self.action_table.insert(key, action);
                        return Ok(());
                    }
                    // Reduce-Reduce conflict is always an error
                    (LR0Action::Reduce(_, _), LR0Action::Reduce(_, _)) => {
                        return Err(format!(
                            "Grammar is not LR(0): Reduce-Reduce conflict in state {} on terminal '{}' ({} vs {})",
                            state, terminal, existing_str, new_str
                        ));
                    }
                    _ => {
                        return Err(format!(
                            "Grammar is not LR(0): Unknown conflict in state {} on terminal '{}' ({} vs {})",
                            state, terminal, existing_str, new_str
                        ));
                    }
                }
            }
        } else {
            self.action_table.insert(key, action);
        }
        Ok(())
    }

    /// Parses the input token sequence using the LR(0) tables and returns snapshots.
    pub fn parse_input(&self, mut input: Vec<String>) -> Result<Vec<LR0ParseSnapshot>, String> {
        input.push("$".to_string());

        let mut snapshots = Vec::new();
        let mut state_stack: Vec<usize> = vec![0];
        let mut symbol_stack: Vec<String> = Vec::new();
        let mut input_ptr = 0;
        let mut step = 0;

        loop {
            let current_state = *state_stack.last().unwrap();
            let current_input = input[input_ptr].clone();

            let action = self.action_table.get(&(current_state, current_input.clone()));

            // Record snapshot BEFORE applying the action
            snapshots.push(LR0ParseSnapshot {
                step,
                state_stack: state_stack.clone(),
                symbol_stack: symbol_stack.clone(),
                input_remaining: input[input_ptr..].to_vec(),
                action: String::new(),
            });

            match action {
                Some(LR0Action::Shift(next_state)) => {
                    snapshots.last_mut().unwrap().action = format!(
                        "Shift '{}' → push state {}", current_input, next_state
                    );
                    symbol_stack.push(current_input);
                    state_stack.push(*next_state);
                    input_ptr += 1;
                }
                Some(LR0Action::Reduce(prod_idx, prod_str)) => {
                    let production = &self.production_list[*prod_idx];
                    let rhs_len = if production.right.len() == 1
                        && production.right[0] == Symbol::Epsilon
                    {
                        0
                    } else {
                        production.right.len()
                    };

                    let left_str = production.left.to_string();

                    snapshots.last_mut().unwrap().action = format!(
                        "Reduce by {}", prod_str
                    );

                    for _ in 0..rhs_len {
                        state_stack.pop();
                        symbol_stack.pop();
                    }

                    let top_state = *state_stack.last().unwrap();
                    symbol_stack.push(left_str.clone());

                    if let Some(&goto_state) = self.goto_table.get(&(top_state, left_str.clone())) {
                        state_stack.push(goto_state);
                    } else {
                        return Err(format!(
                            "Error: No GOTO entry for state {} with '{}'", top_state, left_str
                        ));
                    }
                }
                Some(LR0Action::Accept) => {
                    snapshots.last_mut().unwrap().action = "Accept! ✓".to_string();
                    break;
                }
                None => {
                    let msg = format!(
                        "Syntax Error: No action for state {} on input '{}'",
                        current_state, current_input
                    );
                    snapshots.last_mut().unwrap().action = msg.clone();
                    return Err(msg);
                }
            }

            step += 1;
            if step > 1000 {
                return Err("Error: Maximum steps exceeded (possible infinite loop)".to_string());
            }
        }

        Ok(snapshots)
    }

    /// Returns all terminals used in the grammar (including $).
    pub fn get_all_terminals(&self) -> Vec<String> {
        let mut terminals: Vec<String> = self.augmented_grammar.productions.iter()
            .flat_map(|p| p.right.iter())
            .filter_map(|s| if let Symbol::Terminal(t) = s { Some(t.clone()) } else { None })
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        terminals.sort();
        terminals.push("$".to_string());
        terminals
    }

    /// Returns all non-terminals used in the grammar (excluding S').
    pub fn get_all_non_terminals(&self) -> Vec<String> {
        let mut nts: Vec<String> = self.augmented_grammar.productions.iter()
            .map(|p| &p.left)
            .filter_map(|s| if let Symbol::NonTerminal(nt) = s { Some(nt.clone()) } else { None })
            .filter(|nt| nt != "S'")
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        nts.sort();
        nts
    }
}

/// Formats a production as a readable string: "A → α"
fn format_production(prod: &Production) -> String {
    let left = prod.left.to_string();
    let right: Vec<String> = prod.right.iter().map(|s| s.to_string()).collect();
    format!("{} → {}", left, right.join(" "))
}

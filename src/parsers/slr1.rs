use std::collections::{HashMap, HashSet};
use crate::core::models::{Grammar, Symbol, Production, LR0ParseSnapshot};
use crate::parsers::lr0::{LR0Item, LR0Action};

pub struct SLR1Parser {
    pub augmented_grammar: Grammar,
    pub states: Vec<HashSet<LR0Item>>,
    pub transitions: HashMap<(usize, String), usize>,
    pub action_table: HashMap<(usize, String), LR0Action>,
    pub goto_table: HashMap<(usize, String), usize>,
    pub production_list: Vec<Production>,
    pub follow: HashMap<String, HashSet<String>>,
    first: HashMap<String, HashSet<String>>,
}

impl SLR1Parser {
    pub fn new(grammar: Grammar) -> Result<Self, String> {
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

        let mut parser = SLR1Parser {
            augmented_grammar,
            states: Vec::new(),
            transitions: HashMap::new(),
            action_table: HashMap::new(),
            goto_table: HashMap::new(),
            production_list,
            follow: HashMap::new(),
            first: HashMap::new(),
        };

        parser.build_canonical_collection();
        parser.compute_first();
        parser.compute_follow();
        parser.build_tables()?;

        Ok(parser)
    }

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

    fn build_canonical_collection(&mut self) {
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

    fn compute_first(&mut self) {
        // Initialize FIRST sets for all terminals and non-terminals
        for prod in &self.augmented_grammar.productions {
            // Non-terminal on the left
            if let Symbol::NonTerminal(nt) = &prod.left {
                self.first.entry(nt.clone()).or_insert_with(HashSet::new);
            }
            // Terminals in the right-hand side
            for sym in &prod.right {
                match sym {
                    Symbol::Terminal(t) => {
                        let mut set = HashSet::new();
                        set.insert(t.clone());
                        self.first.insert(t.clone(), set);
                    }
                    Symbol::NonTerminal(nt) => {
                        self.first.entry(nt.clone()).or_insert_with(HashSet::new);
                    }
                    Symbol::Epsilon => {}
                }
            }
        }

        // Fixed-point iteration
        let mut changed = true;
        while changed {
            changed = false;
            for prod in &self.augmented_grammar.productions.clone() {
                let nt = match &prod.left {
                    Symbol::NonTerminal(s) => s.clone(),
                    _ => continue,
                };

                // epsilon production
                if prod.right.len() == 1 && prod.right[0] == Symbol::Epsilon {
                    let set = self.first.entry(nt.clone()).or_insert_with(HashSet::new);
                    if set.insert("ε".to_string()) {
                        changed = true;
                    }
                    continue;
                }

                let additions = self.first_of_sequence(&prod.right);
                let set = self.first.entry(nt).or_insert_with(HashSet::new);
                for token in additions {
                    if set.insert(token) {
                        changed = true;
                    }
                }
            }
        }
    }

    fn first_of_sequence(&self, symbols: &[Symbol]) -> HashSet<String> {
        let mut result = HashSet::new();

        for sym in symbols {
            match sym {
                Symbol::Terminal(t) => {
                    result.insert(t.clone());
                    return result;
                }
                Symbol::Epsilon => {
                    result.insert("ε".to_string());
                    return result;
                }
                Symbol::NonTerminal(nt) => {
                    if let Some(first_nt) = self.first.get(nt) {
                        let has_epsilon = first_nt.contains("ε");
                        for token in first_nt {
                            if token != "ε" {
                                result.insert(token.clone());
                            }
                        }
                        if !has_epsilon {
                            return result;
                        }
                        // ε ∈ FIRST(nt), continue to next symbol
                    } else {
                        return result;
                    }
                }
            }
        }

        // All symbols derived ε
        result.insert("ε".to_string());
        result
    }

    fn compute_follow(&mut self) {
        // Initialize FOLLOW sets
        for prod in &self.augmented_grammar.productions {
            if let Symbol::NonTerminal(nt) = &prod.left {
                self.follow.entry(nt.clone()).or_insert_with(HashSet::new);
            }
        }

        // FOLLOW(S') = {"$"}
        if let Symbol::NonTerminal(start) = &self.augmented_grammar.start_symbol.clone() {
            self.follow
                .entry(start.clone())
                .or_insert_with(HashSet::new)
                .insert("$".to_string());
        }

        let mut changed = true;
        while changed {
            changed = false;
            for prod in &self.augmented_grammar.productions.clone() {
                let lhs = match &prod.left {
                    Symbol::NonTerminal(s) => s.clone(),
                    _ => continue,
                };

                // skip ε-productions
                if prod.right.len() == 1 && prod.right[0] == Symbol::Epsilon {
                    continue;
                }

                for (i, sym) in prod.right.iter().enumerate() {
                    let nt = match sym {
                        Symbol::NonTerminal(s) => s.clone(),
                        _ => continue,
                    };

                    let beta = &prod.right[i + 1..];
                    let first_beta = self.first_of_sequence(beta);
                    let beta_derives_epsilon = first_beta.contains("ε") || beta.is_empty();

                    // FOLLOW(A) += FIRST(β) \ {ε}
                    let additions: Vec<String> = first_beta
                        .iter()
                        .filter(|t| *t != "ε")
                        .cloned()
                        .collect();

                    let follow_nt = self.follow.entry(nt.clone()).or_insert_with(HashSet::new);
                    for token in &additions {
                        if follow_nt.insert(token.clone()) {
                            changed = true;
                        }
                    }

                    // if ε ∈ FIRST(β) or β is empty: FOLLOW(A) += FOLLOW(lhs)
                    if beta_derives_epsilon {
                        let follow_lhs: HashSet<String> = self
                            .follow
                            .get(&lhs)
                            .cloned()
                            .unwrap_or_default();

                        let follow_nt = self.follow.entry(nt).or_insert_with(HashSet::new);
                        for token in follow_lhs {
                            if follow_nt.insert(token) {
                                changed = true;
                            }
                        }
                    }
                }
            }
        }
    }

    fn build_tables(&mut self) -> Result<(), String> {
        let augmented_start = self.augmented_grammar.start_symbol.clone();

        for state_id in 0..self.states.len() {
            let items: Vec<LR0Item> = self.states[state_id].iter().cloned().collect();

            for item in &items {
                if item.is_complete() {
                    if item.production.left == augmented_start {
                        self.insert_action(state_id, "$".to_string(), LR0Action::Accept)?;
                    } else {
                        let lhs_name = match &item.production.left {
                            Symbol::NonTerminal(s) => s.clone(),
                            _ => continue,
                        };

                        let prod_idx = self
                            .production_list
                            .iter()
                            .position(|p| *p == item.production)
                            .unwrap_or(0);
                        let prod_str = format_production(&item.production);

                        let follow_set: HashSet<String> = self
                            .follow
                            .get(&lhs_name)
                            .cloned()
                            .unwrap_or_default();

                        for t in follow_set {
                            let action = LR0Action::Reduce(prod_idx, prod_str.clone());
                            self.insert_action(state_id, t, action)?;
                        }
                    }
                } else if let Some(next) = item.next_symbol() {
                    match next {
                        Symbol::Terminal(t) => {
                            if let Some(&target) =
                                self.transitions.get(&(state_id, t.clone()))
                            {
                                let action = LR0Action::Shift(target);
                                self.insert_action(state_id, t.clone(), action)?;
                            }
                        }
                        Symbol::NonTerminal(nt) => {
                            if let Some(&target) =
                                self.transitions.get(&(state_id, nt.clone()))
                            {
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

    fn insert_action(
        &mut self,
        state: usize,
        terminal: String,
        action: LR0Action,
    ) -> Result<(), String> {
        let key = (state, terminal.clone());
        if let Some(existing) = self.action_table.get(&key) {
            let existing_str = existing.to_display_string();
            let new_str = action.to_display_string();

            if existing_str != new_str {
                match (existing, &action) {
                    (LR0Action::Shift(_), LR0Action::Reduce(_, _)) => {
                        return Ok(());
                    }
                    (LR0Action::Reduce(_, _), LR0Action::Shift(_)) => {
                        self.action_table.insert(key, action);
                        return Ok(());
                    }
                    (LR0Action::Reduce(_, _), LR0Action::Reduce(_, _)) => {
                        return Err(format!(
                            "Grammar is not SLR(1): Reduce-Reduce conflict in state {} on terminal '{}' ({} vs {})",
                            state, terminal, existing_str, new_str
                        ));
                    }
                    _ => {
                        return Err(format!(
                            "Grammar is not SLR(1): Unknown conflict in state {} on terminal '{}' ({} vs {})",
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
                        "Shift '{}' → push state {}",
                        current_input, next_state
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

                    snapshots.last_mut().unwrap().action =
                        format!("Reduce by {}", prod_str);

                    for _ in 0..rhs_len {
                        state_stack.pop();
                        symbol_stack.pop();
                    }

                    let top_state = *state_stack.last().unwrap();
                    symbol_stack.push(left_str.clone());

                    if let Some(&goto_state) =
                        self.goto_table.get(&(top_state, left_str.clone()))
                    {
                        state_stack.push(goto_state);
                    } else {
                        return Err(format!(
                            "Error: No GOTO entry for state {} with '{}'",
                            top_state, left_str
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
                return Err(
                    "Error: Maximum steps exceeded (possible infinite loop)".to_string(),
                );
            }
        }

        Ok(snapshots)
    }

    pub fn get_all_terminals(&self) -> Vec<String> {
        let mut terminals: Vec<String> = self
            .augmented_grammar
            .productions
            .iter()
            .flat_map(|p| p.right.iter())
            .filter_map(|s| {
                if let Symbol::Terminal(t) = s {
                    Some(t.clone())
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        terminals.sort();
        terminals.push("$".to_string());
        terminals
    }

    pub fn get_all_non_terminals(&self) -> Vec<String> {
        let mut nts: Vec<String> = self
            .augmented_grammar
            .productions
            .iter()
            .map(|p| &p.left)
            .filter_map(|s| {
                if let Symbol::NonTerminal(nt) = s {
                    Some(nt.clone())
                } else {
                    None
                }
            })
            .filter(|nt| nt != "S'")
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        nts.sort();
        nts
    }
}

fn format_production(prod: &Production) -> String {
    let left = prod.left.to_string();
    let right: Vec<String> = prod.right.iter().map(|s| s.to_string()).collect();
    format!("{} → {}", left, right.join(" "))
}

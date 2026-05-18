use std::collections::{HashMap, HashSet};
use crate::core::models::{Grammar, Symbol, Production, LR0ParseSnapshot};

/// LR(1) item: [A → α · β, a] where `a` is the lookahead (terminal or "$").
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LR1Item {
    pub production: Production,
    pub dot_position: usize,
    pub lookahead: String,
}

impl LR1Item {
    pub fn new(production: Production, dot_position: usize, lookahead: String) -> Self {
        Self { production, dot_position, lookahead }
    }

    /// Returns the symbol immediately after the dot, or None if the item is complete.
    pub fn next_symbol(&self) -> Option<&Symbol> {
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

    /// Formats as "A → α · β, a"
    pub fn to_display_string(&self) -> String {
        let left = self.production.left.to_string();

        if self.production.right.len() == 1 && self.production.right[0] == Symbol::Epsilon {
            return format!("{} → ·, {}", left, self.lookahead);
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

        format!("{} → {}, {}", left, parts.join(" "), self.lookahead)
    }
}

/// LR(1) action in the parsing table.
#[derive(Debug, Clone)]
pub enum LR1Action {
    Shift(usize),
    Reduce(usize, String), // (production_index, production_display_string)
    Accept,
}

impl LR1Action {
    pub fn to_display_string(&self) -> String {
        match self {
            LR1Action::Shift(s) => format!("s{}", s),
            LR1Action::Reduce(idx, _) => format!("r{}", idx),
            LR1Action::Accept => "acc".to_string(),
        }
    }
}

/// Full LR(1) parser: canonical LR(1) automaton + ACTION/GOTO tables.
pub struct LR1Parser {
    pub augmented_grammar: Grammar,
    pub states: Vec<HashSet<LR1Item>>,
    pub transitions: HashMap<(usize, String), usize>,
    pub action_table: HashMap<(usize, String), LR1Action>,
    pub goto_table: HashMap<(usize, String), usize>,
    pub production_list: Vec<Production>,
    first: HashMap<String, HashSet<String>>,
}

impl LR1Parser {
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

        let mut parser = LR1Parser {
            augmented_grammar,
            states: Vec::new(),
            transitions: HashMap::new(),
            action_table: HashMap::new(),
            goto_table: HashMap::new(),
            production_list,
            first: HashMap::new(),
        };

        parser.compute_first();
        parser.build_canonical_collection();
        parser.build_tables()?;

        Ok(parser)
    }

    fn compute_first(&mut self) {
        for prod in &self.augmented_grammar.productions {
            if let Symbol::NonTerminal(nt) = &prod.left {
                self.first.entry(nt.clone()).or_insert_with(HashSet::new);
            }
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

        let mut changed = true;
        while changed {
            changed = false;
            for prod in &self.augmented_grammar.productions.clone() {
                let nt = match &prod.left {
                    Symbol::NonTerminal(s) => s.clone(),
                    _ => continue,
                };

                if prod.right.len() == 1 && prod.right[0] == Symbol::Epsilon {
                    if self.first.entry(nt).or_insert_with(HashSet::new).insert("ε".to_string()) {
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
                    } else {
                        return result;
                    }
                }
            }
        }

        result.insert("ε".to_string());
        result
    }

    /// Computes FIRST(β · lookahead): used in LR(1) closure for [A → α · B β, a].
    fn first_of_beta_lookahead(&self, beta: &[Symbol], lookahead: &str) -> HashSet<String> {
        let mut result = self.first_of_sequence(beta);
        if result.remove("ε") {
            result.insert(lookahead.to_string());
        }
        result
    }

    /// LR(1) closure: for [A → α · B β, a], adds [B → · γ, b] for each b ∈ FIRST(β a).
    pub fn closure(&self, items: &HashSet<LR1Item>) -> HashSet<LR1Item> {
        let mut result = items.clone();
        let mut changed = true;

        while changed {
            changed = false;
            let snapshot: Vec<LR1Item> = result.iter().cloned().collect();

            for item in &snapshot {
                if let Some(Symbol::NonTerminal(nt_name)) = item.next_symbol() {
                    let beta = &item.production.right[item.dot_position + 1..];
                    let lookaheads = self.first_of_beta_lookahead(beta, &item.lookahead);

                    for prod in &self.augmented_grammar.productions {
                        if let Symbol::NonTerminal(name) = &prod.left {
                            if name == nt_name {
                                for la in &lookaheads {
                                    let new_item = LR1Item::new(prod.clone(), 0, la.clone());
                                    if !result.contains(&new_item) {
                                        result.insert(new_item);
                                        changed = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        result
    }

    /// goto(I, X) = closure({ [A → αX · β, a] | [A → α · X β, a] ∈ I })
    pub fn goto(&self, items: &HashSet<LR1Item>, symbol: &Symbol) -> HashSet<LR1Item> {
        let mut moved = HashSet::new();

        for item in items {
            if let Some(next) = item.next_symbol() {
                if next == symbol {
                    moved.insert(LR1Item::new(
                        item.production.clone(),
                        item.dot_position + 1,
                        item.lookahead.clone(),
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
        // I₀ = closure({[S' → · S, $]})
        let initial_item = LR1Item::new(
            self.augmented_grammar.productions[0].clone(),
            0,
            "$".to_string(),
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

                self.transitions.insert((i, symbol.to_string()), target_id);
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

    fn build_tables(&mut self) -> Result<(), String> {
        let augmented_start = self.augmented_grammar.start_symbol.clone();

        for state_id in 0..self.states.len() {
            let items: Vec<LR1Item> = self.states[state_id].iter().cloned().collect();

            for item in &items {
                if item.is_complete() {
                    if item.production.left == augmented_start {
                        // S' → S · with lookahead $ → Accept
                        self.insert_action(state_id, "$".to_string(), LR1Action::Accept)?;
                    } else {
                        // LR(1): reduce only on the specific lookahead of this item
                        let prod_idx = self.production_list.iter()
                            .position(|p| *p == item.production)
                            .unwrap_or(0);
                        let prod_str = format_production(&item.production);
                        self.insert_action(
                            state_id,
                            item.lookahead.clone(),
                            LR1Action::Reduce(prod_idx, prod_str),
                        )?;
                    }
                } else if let Some(next) = item.next_symbol() {
                    match next {
                        Symbol::Terminal(t) => {
                            if let Some(&target) = self.transitions.get(&(state_id, t.clone())) {
                                self.insert_action(state_id, t.clone(), LR1Action::Shift(target))?;
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

    fn insert_action(&mut self, state: usize, terminal: String, action: LR1Action) -> Result<(), String> {
        let key = (state, terminal.clone());
        if let Some(existing) = self.action_table.get(&key) {
            let existing_str = existing.to_display_string();
            let new_str = action.to_display_string();

            if existing_str != new_str {
                match (existing, &action) {
                    (LR1Action::Shift(_), LR1Action::Reduce(_, _)) => return Ok(()),
                    (LR1Action::Reduce(_, _), LR1Action::Shift(_)) => {
                        self.action_table.insert(key, action);
                        return Ok(());
                    }
                    (LR1Action::Reduce(_, _), LR1Action::Reduce(_, _)) => {
                        return Err(format!(
                            "Grammar is not LR(1): Reduce-Reduce conflict in state {} on terminal '{}' ({} vs {})",
                            state, terminal, existing_str, new_str
                        ));
                    }
                    _ => {
                        return Err(format!(
                            "Grammar is not LR(1): Unknown conflict in state {} on terminal '{}' ({} vs {})",
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
                Some(LR1Action::Shift(next_state)) => {
                    snapshots.last_mut().unwrap().action = format!(
                        "Shift '{}' → push state {}", current_input, next_state
                    );
                    symbol_stack.push(current_input);
                    state_stack.push(*next_state);
                    input_ptr += 1;
                }
                Some(LR1Action::Reduce(prod_idx, prod_str)) => {
                    let production = &self.production_list[*prod_idx];
                    let rhs_len = if production.right.len() == 1
                        && production.right[0] == Symbol::Epsilon
                    {
                        0
                    } else {
                        production.right.len()
                    };

                    let left_str = production.left.to_string();
                    snapshots.last_mut().unwrap().action = format!("Reduce by {}", prod_str);

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
                Some(LR1Action::Accept) => {
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

    pub fn parse_input_with_tree(&self, input: Vec<String>) -> Result<(Vec<LR0ParseSnapshot>, crate::core::models::ParseTreeNode), String> {
        use crate::core::models::ParseTreeNode;
        let mut input = input;
        input.push("$".to_string());

        let mut snapshots = Vec::new();
        let mut state_stack: Vec<usize> = vec![0];
        let mut symbol_stack: Vec<String> = Vec::new();
        let mut node_stack: Vec<ParseTreeNode> = Vec::new();
        let mut input_ptr = 0;
        let mut step = 0;
        let mut nid = 0usize;

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
                Some(LR1Action::Shift(next_state)) => {
                    snapshots.last_mut().unwrap().action = format!("Shift '{}' → push state {}", current_input, next_state);
                    node_stack.push(ParseTreeNode { id: nid, label: current_input.clone(), node_type: "terminal".to_string(), children: vec![] });
                    nid += 1;
                    symbol_stack.push(current_input);
                    state_stack.push(*next_state);
                    input_ptr += 1;
                }
                Some(LR1Action::Reduce(prod_idx, prod_str)) => {
                    let production = &self.production_list[*prod_idx];
                    let rhs_len = if production.right.len() == 1 && production.right[0] == Symbol::Epsilon { 0 } else { production.right.len() };
                    let left_str = production.left.to_string();
                    snapshots.last_mut().unwrap().action = format!("Reduce by {}", prod_str);

                    let mut children: Vec<ParseTreeNode> = Vec::new();
                    for _ in 0..rhs_len {
                        state_stack.pop();
                        symbol_stack.pop();
                        if let Some(c) = node_stack.pop() { children.push(c); }
                    }
                    children.reverse();
                    if rhs_len == 0 {
                        children.push(ParseTreeNode { id: nid, label: "ϵ".to_string(), node_type: "epsilon".to_string(), children: vec![] });
                        nid += 1;
                    }
                    let parent = ParseTreeNode { id: nid, label: left_str.clone(), node_type: "non_terminal".to_string(), children };
                    nid += 1;

                    let top_state = *state_stack.last().unwrap();
                    symbol_stack.push(left_str.clone());
                    node_stack.push(parent);

                    if let Some(&goto_state) = self.goto_table.get(&(top_state, left_str.clone())) {
                        state_stack.push(goto_state);
                    } else {
                        return Err(format!("Error: No GOTO entry for state {} with '{}'", top_state, left_str));
                    }
                }
                Some(LR1Action::Accept) => {
                    snapshots.last_mut().unwrap().action = "Accept! ✓".to_string();
                    break;
                }
                None => {
                    let msg = format!("Syntax Error: No action for state {} on input '{}'", current_state, current_input);
                    snapshots.last_mut().unwrap().action = msg.clone();
                    return Err(msg);
                }
            }
            step += 1;
            if step > 1000 { return Err("Error: Maximum steps exceeded (possible infinite loop)".to_string()); }
        }

        let root = node_stack.into_iter().last()
            .unwrap_or(ParseTreeNode { id: 0, label: "?".to_string(), node_type: "non_terminal".to_string(), children: vec![] });
        Ok((snapshots, root))
    }

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

fn format_production(prod: &Production) -> String {
    let left = prod.left.to_string();
    let right: Vec<String> = prod.right.iter().map(|s| s.to_string()).collect();
    format!("{} → {}", left, right.join(" "))
}

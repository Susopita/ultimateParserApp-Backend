use std::collections::{HashMap, HashSet};
use crate::core::models::{Grammar, Symbol, ParseSnapshot};
use crate::parsers::Parser;

pub struct LL1Parser {
    pub grammar: Grammar,
    pub first: HashMap<Symbol, HashSet<Symbol>>,
    pub follow: HashMap<Symbol, HashSet<Symbol>>,
    pub table: HashMap<(Symbol, Symbol), Vec<Symbol>>, // (NonTerminal, Terminal) -> Right side symbols
}

impl LL1Parser {
    pub fn new(grammar: Grammar) -> Result<Self, String> {
        let mut parser = LL1Parser {
            grammar,
            first: HashMap::new(),
            follow: HashMap::new(),
            table: HashMap::new(),
        };

        parser.compute_first();
        parser.compute_follow();
        parser.compute_table()?;

        Ok(parser)
    }

    fn compute_first(&mut self) {
        // Initial First sets for Terminals and Epsilon
        for prod in &self.grammar.productions {
            self.first.entry(Symbol::Epsilon).or_default().insert(Symbol::Epsilon);
            for symbol in &prod.right {
                if let Symbol::Terminal(_) = symbol {
                    self.first.entry(symbol.clone()).or_default().insert(symbol.clone());
                }
            }
        }

        let mut changed = true;
        while changed {
            changed = false;
            let productions = self.grammar.productions.clone();

            for prod in &productions {
                let left = &prod.left;
                let rhs_first = self.get_sequence_first(&prod.right);
                
                let entry = self.first.entry(left.clone()).or_default();
                let old_size = entry.len();
                for sym in rhs_first {
                    entry.insert(sym);
                }
                if entry.len() > old_size {
                    changed = true;
                }
            }
        }
    }

    fn get_sequence_first(&self, sequence: &[Symbol]) -> HashSet<Symbol> {
        let mut result = HashSet::new();
        if sequence.is_empty() {
            result.insert(Symbol::Epsilon);
            return result;
        }

        for (_i, sym) in sequence.iter().enumerate() {
            if let Some(sym_first) = self.first.get(sym) {
                let mut has_epsilon = false;
                for f in sym_first {
                    if f == &Symbol::Epsilon {
                        has_epsilon = true;
                    } else {
                        result.insert(f.clone());
                    }
                }

                if !has_epsilon {
                    return result;
                }
            } else {
                // If it's a NonTerminal not yet in first set, we can't add anything yet
                return result;
            }
        }

        result.insert(Symbol::Epsilon);
        result
    }

    fn compute_follow(&mut self) {
        let start = self.grammar.start_symbol.clone();
        let dollar = Symbol::Terminal("$".to_string());
        
        self.follow.entry(start).or_default().insert(dollar.clone());

        let mut changed = true;
        while changed {
            changed = false;
            let productions = self.grammar.productions.clone();

            for prod in &productions {
                let left = &prod.left;
                
                for i in 0..prod.right.len() {
                    let b = &prod.right[i];
                    if let Symbol::NonTerminal(_) = b {
                        let beta = &prod.right[i+1..];
                        let first_beta = self.get_sequence_first(beta);
                        
                        let b_follow = self.follow.entry(b.clone()).or_default();
                        let old_size = b_follow.len();
                        
                        for f in &first_beta {
                            if f != &Symbol::Epsilon {
                                b_follow.insert(f.clone());
                            }
                        }
                        
                        if first_beta.contains(&Symbol::Epsilon) {
                            if let Some(left_follow) = self.follow.get(left).cloned() {
                                let b_follow = self.follow.get_mut(b).unwrap();
                                for f in left_follow {
                                    b_follow.insert(f);
                                }
                            }
                        }
                        
                        if self.follow.get(b).unwrap().len() > old_size {
                            changed = true;
                        }
                    }
                }
            }
        }
    }

    fn compute_table(&mut self) -> Result<(), String> {
        for prod in &self.grammar.productions {
            let a = prod.left.clone();
            let alpha = &prod.right;
            let first_alpha = self.get_sequence_first(alpha);

            for a_sym in first_alpha {
                if let Symbol::Terminal(_) = a_sym {
                    if let Some(existing) = self.table.insert((a.clone(), a_sym.clone()), alpha.clone()) {
                        if existing != *alpha {
                            return Err(format!("Grammar is not LL(1): Multiple entries for [{}, {}]", a, a_sym));
                        }
                    }
                } else if a_sym == Symbol::Epsilon {
                    if let Some(follow_a) = self.follow.get(&a) {
                        for b in follow_a {
                            if let Some(existing) = self.table.insert((a.clone(), b.clone()), alpha.clone()) {
                                if existing != *alpha {
                                    return Err(format!("Grammar is not LL(1): Multiple entries for [{}, {}]", a, b));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl LL1Parser {
    pub fn parse_with_tree(&self, input: Vec<String>) -> Result<(Vec<ParseSnapshot>, crate::core::models::ParseTreeNode), String> {
        use crate::core::models::ParseTreeNode;

        struct Arena {
            labels: Vec<String>,
            types: Vec<String>,
            children: Vec<Vec<usize>>,
        }
        impl Arena {
            fn new() -> Self { Arena { labels: vec![], types: vec![], children: vec![] } }
            fn add(&mut self, label: String, ty: &str) -> usize {
                let id = self.labels.len();
                self.labels.push(label);
                self.types.push(ty.to_string());
                self.children.push(vec![]);
                id
            }
            fn link(&mut self, parent: usize, child: usize) {
                self.children[parent].push(child);
            }
            fn build(&self, id: usize) -> ParseTreeNode {
                ParseTreeNode {
                    id,
                    label: self.labels[id].clone(),
                    node_type: self.types[id].clone(),
                    children: self.children[id].iter().map(|&c| self.build(c)).collect(),
                }
            }
        }

        let mut arena = Arena::new();
        let root_ty = "non_terminal";
        let root_id = arena.add(self.grammar.start_symbol.to_string(), root_ty);

        // Stack entries: (Symbol, node_id in arena)
        let dollar_id = arena.add("$".to_string(), "terminal");
        let mut stack: Vec<(Symbol, usize)> = vec![
            (Symbol::Terminal("$".to_string()), dollar_id),
            (self.grammar.start_symbol.clone(), root_id),
        ];

        let mut snapshots = Vec::new();
        let mut input = input;
        input.push("$".to_string());
        let mut input_ptr = 0;
        let mut step = 0;

        while !stack.is_empty() {
            let (top_sym, top_node_id) = stack.last().unwrap().clone();
            let current_input = input[input_ptr].clone();
            let current_sym = Symbol::Terminal(current_input.clone());

            snapshots.push(ParseSnapshot {
                step,
                stack: stack.iter().map(|(s, _)| s.clone()).collect(),
                input_remaining: input[input_ptr..].to_vec(),
                action: format!("Analyzing top: {}, input: {}", top_sym, current_input),
            });

            if top_sym == current_sym {
                if top_sym == Symbol::Terminal("$".to_string()) {
                    snapshots.last_mut().unwrap().action = "Success!".to_string();
                    break;
                }
                stack.pop();
                input_ptr += 1;
                step += 1;
                snapshots.last_mut().unwrap().action = format!("Match terminal: {}", current_input);
            } else if let Symbol::NonTerminal(_) = &top_sym {
                if let Some(rhs) = self.table.get(&(top_sym.clone(), current_sym.clone())) {
                    stack.pop();
                    let mut action_desc = format!("{} ->", top_sym);

                    if rhs.is_empty() || rhs[0] == Symbol::Epsilon {
                        action_desc.push_str(" ϵ");
                        let eps_id = arena.add("ϵ".to_string(), "epsilon");
                        arena.link(top_node_id, eps_id);
                    } else {
                        let mut child_entries: Vec<(Symbol, usize)> = Vec::new();
                        for s in rhs.iter() {
                            if s != &Symbol::Epsilon {
                                let ty = if matches!(s, Symbol::NonTerminal(_)) { "non_terminal" } else { "terminal" };
                                let cid = arena.add(s.to_string(), ty);
                                arena.link(top_node_id, cid);
                                child_entries.push((s.clone(), cid));
                                action_desc.push_str(&format!(" {}", s));
                            }
                        }
                        for entry in child_entries.into_iter().rev() {
                            stack.push(entry);
                        }
                    }

                    snapshots.last_mut().unwrap().action = action_desc;
                    step += 1;
                } else {
                    return Err(format!("Syntax Error: No rule for [{}, {}]", top_sym, current_input));
                }
            } else {
                return Err(format!("Syntax Error: Unexpected terminal {}", current_input));
            }
        }

        Ok((snapshots, arena.build(root_id)))
    }
}

impl Parser for LL1Parser {
    fn parse(&self, mut input: Vec<String>) -> Result<Vec<ParseSnapshot>, String> {
        let mut snapshots = Vec::new();
        let mut stack = vec![Symbol::Terminal("$".to_string()), self.grammar.start_symbol.clone()];
        input.push("$".to_string());
        
        let mut step = 0;
        let mut input_ptr = 0;

        while !stack.is_empty() {
            let top = stack.last().unwrap().clone();
            let current_input = input[input_ptr].clone();
            let current_sym = Symbol::Terminal(current_input.clone());

            snapshots.push(ParseSnapshot {
                step,
                stack: stack.clone(),
                input_remaining: input[input_ptr..].to_vec(),
                action: format!("Analyzing top: {}, input: {}", top, current_input),
            });

            if top == current_sym {
                if top == Symbol::Terminal("$".to_string()) {
                    snapshots.last_mut().unwrap().action = "Success!".to_string();
                    break;
                }
                stack.pop();
                input_ptr += 1;
                step += 1;
                snapshots.last_mut().unwrap().action = format!("Match terminal: {}", current_input);
            } else if let Symbol::NonTerminal(_) = top {
                if let Some(rhs) = self.table.get(&(top.clone(), current_sym.clone())) {
                    stack.pop();
                    let mut action_desc = format!("{} ->", top);
                    if rhs.is_empty() || rhs[0] == Symbol::Epsilon {
                        action_desc.push_str(" ϵ");
                    } else {
                        for s in rhs.iter().rev() {
                            if s != &Symbol::Epsilon {
                                stack.push(s.clone());
                            }
                        }
                        for s in rhs {
                            action_desc.push_str(&format!(" {}", s));
                        }
                    }
                    snapshots.last_mut().unwrap().action = action_desc;
                    step += 1;
                } else {
                    return Err(format!("Syntax Error: No rule for [{}, {}]", top, current_input));
                }
            } else {
                return Err(format!("Syntax Error: Unexpected terminal {}", current_input));
            }
        }

        Ok(snapshots)
    }
}

// Implement Display for Symbol to make actions readable
impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Symbol::Terminal(s) => write!(f, "{}", s),
            Symbol::NonTerminal(s) => write!(f, "{}", s),
            Symbol::Epsilon => write!(f, "ϵ"),
        }
    }
}

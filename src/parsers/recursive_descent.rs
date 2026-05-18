use crate::core::models::{Grammar, Symbol, Production, ParseSnapshot};

const MAX_DEPTH: usize = 200;

/// Recursive descent parser with backtracking.
/// Tries each production alternative in order; on failure, backtracks and tries the next.
/// Rejects left-recursive grammars at construction time.
#[derive(Debug)]
pub struct RecursiveDescentParser {
    pub grammar: Grammar,
}

impl RecursiveDescentParser {
    pub fn new(grammar: Grammar) -> Result<Self, String> {
        if grammar.is_left_recursive() {
            return Err(
                "Recursive Descent cannot parse left-recursive grammars. \
                 Remove or transform left recursion first."
                    .to_string(),
            );
        }
        Ok(Self { grammar })
    }

    /// Parses a token sequence and returns snapshots of each decision step.
    /// Snapshots only reflect the successful path plus explicit backtrack markers.
    pub fn parse_input(&self, input: Vec<String>) -> Result<Vec<ParseSnapshot>, String> {
        let mut tokens = input;
        tokens.push("$".to_string());

        let start = self.grammar.start_symbol.clone();
        let mut call_path: Vec<Symbol> = Vec::new();
        let mut snapshots: Vec<ParseSnapshot> = Vec::new();

        snapshots.push(ParseSnapshot {
            step: 0,
            stack: vec![start.clone()],
            input_remaining: tokens.clone(),
            action: format!("Start: expand {}", start),
        });

        match self.try_symbol(&start, &tokens, 0, &mut call_path, &mut snapshots, 0) {
            Some(pos) if tokens[pos] == "$" => {
                let step = snapshots.len();
                snapshots.push(ParseSnapshot {
                    step,
                    stack: Vec::new(),
                    input_remaining: vec!["$".to_string()],
                    action: "Accept! ✓".to_string(),
                });
                renumber(&mut snapshots);
                Ok(snapshots)
            }
            Some(pos) => {
                let msg = format!(
                    "Syntax Error: Unexpected '{}' — input not fully consumed",
                    tokens[pos]
                );
                Err(msg)
            }
            None => Err("Syntax Error: Input does not match grammar.".to_string()),
        }
    }

    fn try_symbol(
        &self,
        symbol: &Symbol,
        tokens: &[String],
        pos: usize,
        call_path: &mut Vec<Symbol>,
        snapshots: &mut Vec<ParseSnapshot>,
        depth: usize,
    ) -> Option<usize> {
        if depth > MAX_DEPTH {
            return None;
        }

        match symbol {
            Symbol::Terminal(t) => {
                if pos < tokens.len() && &tokens[pos] == t {
                    snapshots.push(ParseSnapshot {
                        step: snapshots.len(),
                        stack: call_path.clone(),
                        input_remaining: tokens[pos..].to_vec(),
                        action: format!("Match '{}'", t),
                    });
                    Some(pos + 1)
                } else {
                    None
                }
            }
            Symbol::Epsilon => {
                snapshots.push(ParseSnapshot {
                    step: snapshots.len(),
                    stack: call_path.clone(),
                    input_remaining: tokens[pos..].to_vec(),
                    action: "Match ε".to_string(),
                });
                Some(pos)
            }
            Symbol::NonTerminal(nt_name) => {
                self.try_nonterminal(nt_name, symbol, tokens, pos, call_path, snapshots, depth)
            }
        }
    }

    fn try_nonterminal(
        &self,
        nt_name: &str,
        symbol: &Symbol,
        tokens: &[String],
        pos: usize,
        call_path: &mut Vec<Symbol>,
        snapshots: &mut Vec<ParseSnapshot>,
        depth: usize,
    ) -> Option<usize> {
        let productions: Vec<&Production> = self.grammar.productions.iter()
            .filter(|p| p.left == *symbol)
            .collect();

        call_path.push(symbol.clone());
        let alt_count = productions.len();

        for (idx, prod) in productions.iter().enumerate() {
            let rhs_str = format_rhs(&prod.right);
            let saved = snapshots.len();

            snapshots.push(ParseSnapshot {
                step: snapshots.len(),
                stack: call_path.clone(),
                input_remaining: tokens[pos..].to_vec(),
                action: format!(
                    "Try {} → {} [{}/{}]",
                    nt_name,
                    rhs_str,
                    idx + 1,
                    alt_count
                ),
            });

            match self.try_rhs(&prod.right, tokens, pos, call_path, snapshots, depth + 1) {
                Some(new_pos) => {
                    call_path.pop();
                    return Some(new_pos);
                }
                None => {
                    // Discard snapshots from the failed branch, keep backtrack marker
                    snapshots.truncate(saved);
                    let is_last = idx == alt_count - 1;
                    snapshots.push(ParseSnapshot {
                        step: snapshots.len(),
                        stack: call_path.clone(),
                        input_remaining: tokens[pos..].to_vec(),
                        action: if is_last {
                            format!("Backtrack: {} → {} failed — all alternatives exhausted", nt_name, rhs_str)
                        } else {
                            format!("Backtrack: {} → {} failed — trying next alternative", nt_name, rhs_str)
                        },
                    });
                }
            }
        }

        call_path.pop();
        None
    }

    fn try_rhs(
        &self,
        rhs: &[Symbol],
        tokens: &[String],
        pos: usize,
        call_path: &mut Vec<Symbol>,
        snapshots: &mut Vec<ParseSnapshot>,
        depth: usize,
    ) -> Option<usize> {
        let mut current = pos;
        for symbol in rhs {
            match self.try_symbol(symbol, tokens, current, call_path, snapshots, depth) {
                Some(next) => current = next,
                None => return None,
            }
        }
        Some(current)
    }

    pub fn parse_input_with_tree(&self, input: Vec<String>) -> Result<(Vec<ParseSnapshot>, crate::core::models::ParseTreeNode), String> {
        let mut tokens = input;
        tokens.push("$".to_string());

        let start = self.grammar.start_symbol.clone();
        let mut call_path: Vec<Symbol> = Vec::new();
        let mut snapshots: Vec<ParseSnapshot> = Vec::new();
        let mut nid = 0usize;

        snapshots.push(ParseSnapshot {
            step: 0,
            stack: vec![start.clone()],
            input_remaining: tokens.clone(),
            action: format!("Start: expand {}", start),
        });

        match self.try_symbol_tree(&start, &tokens, 0, &mut call_path, &mut snapshots, 0, &mut nid) {
            Some((pos, tree)) if tokens[pos] == "$" => {
                let step = snapshots.len();
                snapshots.push(ParseSnapshot {
                    step,
                    stack: Vec::new(),
                    input_remaining: vec!["$".to_string()],
                    action: "Accept! ✓".to_string(),
                });
                renumber(&mut snapshots);
                Ok((snapshots, tree))
            }
            Some((pos, _)) => Err(format!(
                "Syntax Error: Unexpected '{}' — input not fully consumed", tokens[pos]
            )),
            None => Err("Syntax Error: Input does not match grammar.".to_string()),
        }
    }

    fn try_symbol_tree(
        &self,
        symbol: &Symbol,
        tokens: &[String],
        pos: usize,
        call_path: &mut Vec<Symbol>,
        snapshots: &mut Vec<ParseSnapshot>,
        depth: usize,
        nid: &mut usize,
    ) -> Option<(usize, crate::core::models::ParseTreeNode)> {
        use crate::core::models::ParseTreeNode;
        if depth > MAX_DEPTH { return None; }

        match symbol {
            Symbol::Terminal(t) => {
                if pos < tokens.len() && &tokens[pos] == t {
                    snapshots.push(ParseSnapshot {
                        step: snapshots.len(),
                        stack: call_path.clone(),
                        input_remaining: tokens[pos..].to_vec(),
                        action: format!("Match '{}'", t),
                    });
                    let node = ParseTreeNode { id: { let i = *nid; *nid += 1; i }, label: t.clone(), node_type: "terminal".to_string(), children: vec![] };
                    Some((pos + 1, node))
                } else {
                    None
                }
            }
            Symbol::Epsilon => {
                snapshots.push(ParseSnapshot {
                    step: snapshots.len(),
                    stack: call_path.clone(),
                    input_remaining: tokens[pos..].to_vec(),
                    action: "Match ε".to_string(),
                });
                let node = ParseTreeNode { id: { let i = *nid; *nid += 1; i }, label: "ϵ".to_string(), node_type: "epsilon".to_string(), children: vec![] };
                Some((pos, node))
            }
            Symbol::NonTerminal(nt_name) => {
                self.try_nonterminal_tree(nt_name, symbol, tokens, pos, call_path, snapshots, depth, nid)
            }
        }
    }

    fn try_nonterminal_tree(
        &self,
        nt_name: &str,
        symbol: &Symbol,
        tokens: &[String],
        pos: usize,
        call_path: &mut Vec<Symbol>,
        snapshots: &mut Vec<ParseSnapshot>,
        depth: usize,
        nid: &mut usize,
    ) -> Option<(usize, crate::core::models::ParseTreeNode)> {
        use crate::core::models::ParseTreeNode;
        let productions: Vec<&Production> = self.grammar.productions.iter()
            .filter(|p| p.left == *symbol)
            .collect();

        call_path.push(symbol.clone());
        let alt_count = productions.len();

        for (idx, prod) in productions.iter().enumerate() {
            let rhs_str = format_rhs(&prod.right);
            let saved = snapshots.len();

            snapshots.push(ParseSnapshot {
                step: snapshots.len(),
                stack: call_path.clone(),
                input_remaining: tokens[pos..].to_vec(),
                action: format!("Try {} → {} [{}/{}]", nt_name, rhs_str, idx + 1, alt_count),
            });

            match self.try_rhs_tree(&prod.right, tokens, pos, call_path, snapshots, depth + 1, nid) {
                Some((new_pos, children)) => {
                    let parent = ParseTreeNode {
                        id: { let i = *nid; *nid += 1; i },
                        label: nt_name.to_string(),
                        node_type: "non_terminal".to_string(),
                        children,
                    };
                    call_path.pop();
                    return Some((new_pos, parent));
                }
                None => {
                    snapshots.truncate(saved);
                    let is_last = idx == alt_count - 1;
                    snapshots.push(ParseSnapshot {
                        step: snapshots.len(),
                        stack: call_path.clone(),
                        input_remaining: tokens[pos..].to_vec(),
                        action: if is_last {
                            format!("Backtrack: {} → {} failed — all alternatives exhausted", nt_name, rhs_str)
                        } else {
                            format!("Backtrack: {} → {} failed — trying next alternative", nt_name, rhs_str)
                        },
                    });
                }
            }
        }

        call_path.pop();
        None
    }

    fn try_rhs_tree(
        &self,
        rhs: &[Symbol],
        tokens: &[String],
        pos: usize,
        call_path: &mut Vec<Symbol>,
        snapshots: &mut Vec<ParseSnapshot>,
        depth: usize,
        nid: &mut usize,
    ) -> Option<(usize, Vec<crate::core::models::ParseTreeNode>)> {
        let mut current = pos;
        let mut children = Vec::new();
        for symbol in rhs {
            match self.try_symbol_tree(symbol, tokens, current, call_path, snapshots, depth, nid) {
                Some((next, node)) => { current = next; children.push(node); }
                None => return None,
            }
        }
        Some((current, children))
    }
}

fn format_rhs(rhs: &[Symbol]) -> String {
    if rhs.len() == 1 && rhs[0] == Symbol::Epsilon {
        return "ε".to_string();
    }
    rhs.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(" ")
}

fn renumber(snapshots: &mut Vec<ParseSnapshot>) {
    for (i, s) in snapshots.iter_mut().enumerate() {
        s.step = i;
    }
}

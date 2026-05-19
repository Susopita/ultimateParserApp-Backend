mod ai;
pub use ai::ai_assist;

use axum::{Json, response::IntoResponse, http::StatusCode};
use serde::{Deserialize, Serialize};
use crate::core::{Grammar, Production, Symbol};

#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    pub raw_grammar: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_left_recursion: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_symbol: Option<Symbol>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub production_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub productions: Option<Vec<Production>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_sets: Option<std::collections::HashMap<String, Vec<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follow_sets: Option<std::collections::HashMap<String, Vec<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Handler to analyze grammar and check for left recursion from raw text
pub async fn analyze_grammar(Json(payload): Json<AnalyzeRequest>) -> impl IntoResponse {
    match Grammar::from_string(&payload.raw_grammar) {
        Ok(grammar) => {
            let is_recursive = grammar.is_left_recursive();

            let mut parser = crate::parsers::ll1::LL1Parser {
                grammar: grammar.clone(),
                first: std::collections::HashMap::new(),
                follow: std::collections::HashMap::new(),
                table: std::collections::HashMap::new(),
            };
            parser.compute_first();
            parser.compute_follow();

            let mut first_sets = std::collections::HashMap::new();
            for (sym, set) in &parser.first {
                if let Symbol::NonTerminal(_) = sym {
                    let mut vec: Vec<String> = set.iter().map(|s| s.to_string()).collect();
                    vec.sort();
                    first_sets.insert(sym.to_string(), vec);
                }
            }

            let mut follow_sets = std::collections::HashMap::new();
            for (sym, set) in &parser.follow {
                if let Symbol::NonTerminal(_) = sym {
                    let mut vec: Vec<String> = set.iter().map(|s| s.to_string()).collect();
                    vec.sort();
                    follow_sets.insert(sym.to_string(), vec);
                }
            }

            (StatusCode::OK, Json(AnalyzeResponse {
                status: "success".to_string(),
                has_left_recursion: Some(is_recursive),
                start_symbol: Some(grammar.start_symbol.clone()),
                production_count: Some(grammar.productions.len()),
                productions: Some(grammar.productions),
                first_sets: Some(first_sets),
                follow_sets: Some(follow_sets),
                message: None,
            }))
        },
        Err(e) => {
            (StatusCode::BAD_REQUEST, Json(AnalyzeResponse {
                status: "error".to_string(),
                has_left_recursion: None,
                start_symbol: None,
                production_count: None,
                productions: None,
                first_sets: None,
                follow_sets: None,
                message: Some(e),
            }))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ParseRequest {
    pub raw_grammar: String,
    pub input_string: String,
}

#[derive(Debug, Serialize)]
pub struct ParseResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshots: Option<Vec<crate::core::models::ParseSnapshot>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parsing_table: Option<std::collections::HashMap<String, std::collections::HashMap<String, String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_tree: Option<crate::core::models::ParseTreeNode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ast: Option<crate::core::models::ParseTreeNode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_tree_dot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ast_dot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Greedy tokenizer that uses grammar terminals to split input when there are no spaces.
fn tokenize_input(input: &str, grammar: &crate::core::models::Grammar) -> Vec<String> {
    if input.contains(' ') {
        return input.split_whitespace().map(|s| s.to_string()).collect();
    }

    // Collect unique terminals from the grammar
    let mut terminals: Vec<String> = Vec::new();
    for prod in &grammar.productions {
        for sym in &prod.right {
            if let crate::core::models::Symbol::Terminal(t) = sym {
                if !terminals.contains(t) && t != "ϵ" && t != "ε" && t != "epsilon" {
                    terminals.push(t.clone());
                }
            }
        }
    }

    // Sort terminals by length descending (longest prefix match)
    terminals.sort_by(|a, b| b.len().cmp(&a.len()));

    let mut tokens = Vec::new();
    let mut remaining = input;

    while !remaining.is_empty() {
        let mut matched = false;
        for t in &terminals {
            if remaining.starts_with(t) {
                tokens.push(t.clone());
                remaining = &remaining[t.len()..];
                matched = true;
                break;
            }
        }

        if !matched {
            // If no terminal matches, just consume one character (fallback)
            let c = remaining.chars().next().unwrap();
            tokens.push(c.to_string());
            remaining = &remaining[c.len_utf8()..];
        }
    }

    tokens
}

/// Handler to execute Recursive Descent parsing simulation
pub async fn parse_rd(Json(payload): Json<ParseRequest>) -> impl IntoResponse {
    let grammar = match Grammar::from_string(&payload.raw_grammar) {
        Ok(g) => g,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(ParseResponse {
            status: "error".to_string(),
            snapshots: None,
            parsing_table: None,
            parse_tree: None,
            ast: None,
            parse_tree_dot: None,
            ast_dot: None,
            message: Some(format!("Grammar Error: {}", e)),
        })),
    };

    let parser = match crate::parsers::recursive_descent::RecursiveDescentParser::new(grammar.clone()) {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(ParseResponse {
            status: "error".to_string(),
            snapshots: None,
            parsing_table: None,
            parse_tree: None,
            ast: None,
            parse_tree_dot: None,
            ast_dot: None,
            message: Some(format!("Parser Error: {}", e)),
        })),
    };

    let tokens = tokenize_input(&payload.input_string, &grammar);

    match parser.parse_input_with_tree(tokens) {
        Ok((snapshots, parse_tree)) => {
            let ast = parse_tree.to_ast();
            let parse_tree_dot = parse_tree.to_dot("parse_tree");
            let ast_dot = ast.to_dot("ast");
            (StatusCode::OK, Json(ParseResponse {
                status: "success".to_string(),
                snapshots: Some(snapshots),
                parsing_table: None,
                parse_tree: Some(parse_tree),
                ast: Some(ast),
                parse_tree_dot: Some(parse_tree_dot),
                ast_dot: Some(ast_dot),
                message: None,
            }))
        },
        Err(e) => (StatusCode::OK, Json(ParseResponse {
            status: "error".to_string(),
            snapshots: None,
            parsing_table: None,
            parse_tree: None,
            ast: None,
            parse_tree_dot: None,
            ast_dot: None,
            message: Some(e),
        })),
    }
}

/// Handler to execute LL(1) parsing simulation
pub async fn parse_ll1(Json(payload): Json<ParseRequest>) -> impl IntoResponse {
    let grammar = match Grammar::from_string(&payload.raw_grammar) {
        Ok(g) => g,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(ParseResponse {
            status: "error".to_string(),
            snapshots: None,
            parsing_table: None,
            parse_tree: None,
            ast: None,
            parse_tree_dot: None,
            ast_dot: None,
            message: Some(format!("Grammar Error: {}", e)),
        })),
    };

    let parser = match crate::parsers::ll1::LL1Parser::new(grammar.clone()) {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(ParseResponse {
            status: "error".to_string(),
            snapshots: None,
            parsing_table: None,
            parse_tree: None,
            ast: None,
            parse_tree_dot: None,
            ast_dot: None,
            message: Some(format!("LL(1) Table Error: {}", e)),
        })),
    };

    let tokens = tokenize_input(&payload.input_string, &grammar);

    // Build serializable LL(1) table (needed for both success and error paths)
    let mut serializable_table = std::collections::HashMap::new();
    for ((nt, t), rhs) in &parser.table {
        let nt_str = nt.to_string();
        let t_str = t.to_string();
        let rhs_str = rhs.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(" ");
        serializable_table
            .entry(nt_str)
            .or_insert_with(std::collections::HashMap::new)
            .insert(t_str, rhs_str);
    }

    match parser.parse_with_tree(tokens) {
        Ok((snapshots, parse_tree)) => {
            let ast = parse_tree.to_ast();
            let parse_tree_dot = parse_tree.to_dot("parse_tree");
            let ast_dot = ast.to_dot("ast");
            (StatusCode::OK, Json(ParseResponse {
                status: "success".to_string(),
                snapshots: Some(snapshots),
                parsing_table: Some(serializable_table),
                parse_tree: Some(parse_tree),
                ast: Some(ast),
                parse_tree_dot: Some(parse_tree_dot),
                ast_dot: Some(ast_dot),
                message: None,
            }))
        },
        Err(e) => {
            (StatusCode::OK, Json(ParseResponse {
                status: "error".to_string(),
                snapshots: None,
                parsing_table: Some(serializable_table),
                parse_tree: None,
                ast: None,
                parse_tree_dot: None,
                ast_dot: None,
                message: Some(e),
            }))
        }
    }
}

// ─── LR(0) Parsing Endpoint ─────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct LR0AutomatonStateResponse {
    pub id: usize,
    pub items: Vec<String>,
    pub is_accept: bool,
}

#[derive(Debug, Serialize)]
pub struct LR0TransitionResponse {
    pub from: usize,
    pub to: usize,
    pub symbol: String,
}

#[derive(Debug, Serialize)]
pub struct LR0AutomatonResponse {
    pub states: Vec<LR0AutomatonStateResponse>,
    pub transitions: Vec<LR0TransitionResponse>,
}

#[derive(Debug, Serialize)]
pub struct LR0ParseResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automaton: Option<LR0AutomatonResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_table: Option<std::collections::HashMap<String, std::collections::HashMap<String, String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goto_table: Option<std::collections::HashMap<String, std::collections::HashMap<String, String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminals: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_terminals: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshots: Option<Vec<crate::core::models::LR0ParseSnapshot>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_tree: Option<crate::core::models::ParseTreeNode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ast: Option<crate::core::models::ParseTreeNode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_tree_dot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ast_dot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

pub type SLR1ParseResponse = LR0ParseResponse;
pub type LR1ParseResponse = LR0ParseResponse;
pub type LALR1ParseResponse = LR0ParseResponse;

type TableMap = std::collections::HashMap<String, std::collections::HashMap<String, String>>;

fn build_automaton_response(
    state_items: Vec<Vec<String>>,
    state_accept: Vec<bool>,
    transitions: &std::collections::HashMap<(usize, String), usize>,
) -> LR0AutomatonResponse {
    let states = state_items
        .into_iter()
        .zip(state_accept)
        .enumerate()
        .map(|(id, (items, is_accept))| LR0AutomatonStateResponse { id, items, is_accept })
        .collect();
    let transitions = transitions
        .iter()
        .map(|((from, sym), to)| LR0TransitionResponse { from: *from, to: *to, symbol: sym.clone() })
        .collect();
    LR0AutomatonResponse { states, transitions }
}

fn serialize_action_table<A>(
    table: &std::collections::HashMap<(usize, String), A>,
    display: impl Fn(&A) -> String,
) -> TableMap {
    let mut map = TableMap::new();
    for ((state, terminal), action) in table {
        map.entry(state.to_string()).or_default().insert(terminal.clone(), display(action));
    }
    map
}

fn serialize_goto_table(table: &std::collections::HashMap<(usize, String), usize>) -> TableMap {
    let mut map = TableMap::new();
    for ((state, nt), target) in table {
        map.entry(state.to_string()).or_default().insert(nt.clone(), target.to_string());
    }
    map
}

/// Handler to execute LR(0) parsing simulation
pub async fn parse_lr0(Json(payload): Json<ParseRequest>) -> impl IntoResponse {
    let grammar = match Grammar::from_string(&payload.raw_grammar) {
        Ok(g) => g,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(LR0ParseResponse {
            status: "error".to_string(),
            automaton: None,
            action_table: None,
            goto_table: None,
            terminals: None,
            non_terminals: None,
            snapshots: None,
            parse_tree: None,
            ast: None,
            parse_tree_dot: None,
            ast_dot: None,
            message: Some(format!("Grammar Error: {}", e)),
        })),
    };

    let parser = match crate::parsers::lr0::LR0Parser::new(grammar.clone()) {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(LR0ParseResponse {
            status: "error".to_string(),
            automaton: None,
            action_table: None,
            goto_table: None,
            terminals: None,
            non_terminals: None,
            snapshots: None,
            parse_tree: None,
            ast: None,
            parse_tree_dot: None,
            ast_dot: None,
            message: Some(format!("LR(0) Table Error: {}", e)),
        })),
    };

    let automaton = build_automaton_response(
        parser.states.iter().map(|items| items.iter().map(|i| i.to_display_string()).collect()).collect(),
        parser.states.iter().map(|items| items.iter().any(|i| i.is_complete() && i.production.left == parser.augmented_grammar.start_symbol)).collect(),
        &parser.transitions,
    );
    let action_map = serialize_action_table(&parser.action_table, |a| a.to_display_string());
    let goto_map = serialize_goto_table(&parser.goto_table);
    let tokens = tokenize_input(&payload.input_string, &grammar);
    let all_terminals = parser.get_all_terminals();
    let all_non_terminals = parser.get_all_non_terminals();

    match parser.parse_input_with_tree(tokens) {
        Ok((snapshots, parse_tree)) => {
            let ast = parse_tree.to_ast();
            let parse_tree_dot = parse_tree.to_dot("parse_tree");
            let ast_dot = ast.to_dot("ast");
            (StatusCode::OK, Json(LR0ParseResponse {
                status: "success".to_string(),
                automaton: Some(automaton),
                action_table: Some(action_map),
                goto_table: Some(goto_map),
                terminals: Some(all_terminals),
                non_terminals: Some(all_non_terminals),
                snapshots: Some(snapshots),
                parse_tree: Some(parse_tree),
                ast: Some(ast),
                parse_tree_dot: Some(parse_tree_dot),
                ast_dot: Some(ast_dot),
                message: None,
            }))
        }
        Err(e) => {
            (StatusCode::OK, Json(LR0ParseResponse {
                status: "error".to_string(),
                automaton: Some(automaton),
                action_table: Some(action_map),
                goto_table: Some(goto_map),
                terminals: Some(all_terminals),
                non_terminals: Some(all_non_terminals),
                snapshots: None,
                parse_tree: None,
                ast: None,
                parse_tree_dot: None,
                ast_dot: None,
                message: Some(e),
            }))
        }
    }
}

/// Handler to execute LR(1) parsing simulation
pub async fn parse_lr1(Json(payload): Json<ParseRequest>) -> impl IntoResponse {
    let grammar = match Grammar::from_string(&payload.raw_grammar) {
        Ok(g) => g,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(LR1ParseResponse {
            status: "error".to_string(),
            automaton: None,
            action_table: None,
            goto_table: None,
            terminals: None,
            non_terminals: None,
            snapshots: None,
            parse_tree: None,
            ast: None,
            parse_tree_dot: None,
            ast_dot: None,
            message: Some(format!("Grammar Error: {}", e)),
        })),
    };

    let parser = match crate::parsers::lr1::LR1Parser::new(grammar.clone()) {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(LR1ParseResponse {
            status: "error".to_string(),
            automaton: None,
            action_table: None,
            goto_table: None,
            terminals: None,
            non_terminals: None,
            snapshots: None,
            parse_tree: None,
            ast: None,
            parse_tree_dot: None,
            ast_dot: None,
            message: Some(format!("LR(1) Table Error: {}", e)),
        })),
    };

    let automaton = build_automaton_response(
        parser.states.iter().map(|items| items.iter().map(|i| i.to_display_string()).collect()).collect(),
        parser.states.iter().map(|items| items.iter().any(|i| i.is_complete() && i.production.left == parser.augmented_grammar.start_symbol)).collect(),
        &parser.transitions,
    );
    let action_map = serialize_action_table(&parser.action_table, |a| a.to_display_string());
    let goto_map = serialize_goto_table(&parser.goto_table);
    let tokens = tokenize_input(&payload.input_string, &grammar);
    let all_terminals = parser.get_all_terminals();
    let all_non_terminals = parser.get_all_non_terminals();

    match parser.parse_input_with_tree(tokens) {
        Ok((snapshots, parse_tree)) => {
            let ast = parse_tree.to_ast();
            let parse_tree_dot = parse_tree.to_dot("parse_tree");
            let ast_dot = ast.to_dot("ast");
            (StatusCode::OK, Json(LR1ParseResponse {
                status: "success".to_string(),
                automaton: Some(automaton),
                action_table: Some(action_map),
                goto_table: Some(goto_map),
                terminals: Some(all_terminals),
                non_terminals: Some(all_non_terminals),
                snapshots: Some(snapshots),
                parse_tree: Some(parse_tree),
                ast: Some(ast),
                parse_tree_dot: Some(parse_tree_dot),
                ast_dot: Some(ast_dot),
                message: None,
            }))
        }
        Err(e) => {
            (StatusCode::OK, Json(LR1ParseResponse {
                status: "error".to_string(),
                automaton: Some(automaton),
                action_table: Some(action_map),
                goto_table: Some(goto_map),
                terminals: Some(all_terminals),
                non_terminals: Some(all_non_terminals),
                snapshots: None,
                parse_tree: None,
                ast: None,
                parse_tree_dot: None,
                ast_dot: None,
                message: Some(e),
            }))
        }
    }
}

/// Handler to execute LALR(1) parsing simulation
pub async fn parse_lalr1(Json(payload): Json<ParseRequest>) -> impl IntoResponse {
    let grammar = match Grammar::from_string(&payload.raw_grammar) {
        Ok(g) => g,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(LALR1ParseResponse {
            status: "error".to_string(),
            automaton: None,
            action_table: None,
            goto_table: None,
            terminals: None,
            non_terminals: None,
            snapshots: None,
            parse_tree: None,
            ast: None,
            parse_tree_dot: None,
            ast_dot: None,
            message: Some(format!("Grammar Error: {}", e)),
        })),
    };

    let parser = match crate::parsers::lalr1::LALR1Parser::new(grammar.clone()) {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(LALR1ParseResponse {
            status: "error".to_string(),
            automaton: None,
            action_table: None,
            goto_table: None,
            terminals: None,
            non_terminals: None,
            snapshots: None,
            parse_tree: None,
            ast: None,
            parse_tree_dot: None,
            ast_dot: None,
            message: Some(format!("LALR(1) Table Error: {}", e)),
        })),
    };

    let automaton = build_automaton_response(
        parser.states.iter().map(|items| items.iter().map(|i| i.to_display_string()).collect()).collect(),
        parser.states.iter().map(|items| items.iter().any(|i| i.is_complete() && i.production.left == parser.augmented_grammar.start_symbol)).collect(),
        &parser.transitions,
    );
    let action_map = serialize_action_table(&parser.action_table, |a| a.to_display_string());
    let goto_map = serialize_goto_table(&parser.goto_table);
    let tokens = tokenize_input(&payload.input_string, &grammar);
    let all_terminals = parser.get_all_terminals();
    let all_non_terminals = parser.get_all_non_terminals();

    match parser.parse_input_with_tree(tokens) {
        Ok((snapshots, parse_tree)) => {
            let ast = parse_tree.to_ast();
            let parse_tree_dot = parse_tree.to_dot("parse_tree");
            let ast_dot = ast.to_dot("ast");
            (StatusCode::OK, Json(LALR1ParseResponse {
                status: "success".to_string(),
                automaton: Some(automaton),
                action_table: Some(action_map),
                goto_table: Some(goto_map),
                terminals: Some(all_terminals),
                non_terminals: Some(all_non_terminals),
                snapshots: Some(snapshots),
                parse_tree: Some(parse_tree),
                ast: Some(ast),
                parse_tree_dot: Some(parse_tree_dot),
                ast_dot: Some(ast_dot),
                message: None,
            }))
        }
        Err(e) => {
            (StatusCode::OK, Json(LALR1ParseResponse {
                status: "error".to_string(),
                automaton: Some(automaton),
                action_table: Some(action_map),
                goto_table: Some(goto_map),
                terminals: Some(all_terminals),
                non_terminals: Some(all_non_terminals),
                snapshots: None,
                parse_tree: None,
                ast: None,
                parse_tree_dot: None,
                ast_dot: None,
                message: Some(e),
            }))
        }
    }
}

/// Handler to execute SLR(1) parsing simulation
pub async fn parse_slr1(Json(payload): Json<ParseRequest>) -> impl IntoResponse {
    let grammar = match Grammar::from_string(&payload.raw_grammar) {
        Ok(g) => g,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(SLR1ParseResponse {
            status: "error".to_string(),
            automaton: None,
            action_table: None,
            goto_table: None,
            terminals: None,
            non_terminals: None,
            snapshots: None,
            parse_tree: None,
            ast: None,
            parse_tree_dot: None,
            ast_dot: None,
            message: Some(format!("Grammar Error: {}", e)),
        })),
    };

    let parser = match crate::parsers::slr1::SLR1Parser::new(grammar.clone()) {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(SLR1ParseResponse {
            status: "error".to_string(),
            automaton: None,
            action_table: None,
            goto_table: None,
            terminals: None,
            non_terminals: None,
            snapshots: None,
            parse_tree: None,
            ast: None,
            parse_tree_dot: None,
            ast_dot: None,
            message: Some(format!("SLR(1) Table Error: {}", e)),
        })),
    };

    let automaton = build_automaton_response(
        parser.states.iter().map(|items| items.iter().map(|i| i.to_display_string()).collect()).collect(),
        parser.states.iter().map(|items| items.iter().any(|i| i.is_complete() && i.production.left == parser.augmented_grammar.start_symbol)).collect(),
        &parser.transitions,
    );
    let action_map = serialize_action_table(&parser.action_table, |a| a.to_display_string());
    let goto_map = serialize_goto_table(&parser.goto_table);
    let tokens = tokenize_input(&payload.input_string, &grammar);
    let all_terminals = parser.get_all_terminals();
    let all_non_terminals = parser.get_all_non_terminals();

    match parser.parse_input_with_tree(tokens) {
        Ok((snapshots, parse_tree)) => {
            let ast = parse_tree.to_ast();
            let parse_tree_dot = parse_tree.to_dot("parse_tree");
            let ast_dot = ast.to_dot("ast");
            (StatusCode::OK, Json(SLR1ParseResponse {
                status: "success".to_string(),
                automaton: Some(automaton),
                action_table: Some(action_map),
                goto_table: Some(goto_map),
                terminals: Some(all_terminals),
                non_terminals: Some(all_non_terminals),
                snapshots: Some(snapshots),
                parse_tree: Some(parse_tree),
                ast: Some(ast),
                parse_tree_dot: Some(parse_tree_dot),
                ast_dot: Some(ast_dot),
                message: None,
            }))
        }
        Err(e) => {
            (StatusCode::OK, Json(SLR1ParseResponse {
                status: "error".to_string(),
                automaton: Some(automaton),
                action_table: Some(action_map),
                goto_table: Some(goto_map),
                terminals: Some(all_terminals),
                non_terminals: Some(all_non_terminals),
                snapshots: None,
                parse_tree: None,
                ast: None,
                parse_tree_dot: None,
                ast_dot: None,
                message: Some(e),
            }))
        }
    }
}

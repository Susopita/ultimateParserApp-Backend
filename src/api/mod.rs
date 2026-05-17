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
    pub message: Option<String>,
}

/// Handler to analyze grammar and check for left recursion from raw text
pub async fn analyze_grammar(Json(payload): Json<AnalyzeRequest>) -> impl IntoResponse {
    match Grammar::from_string(&payload.raw_grammar) {
        Ok(grammar) => {
            let is_recursive = grammar.is_left_recursive();
            
            (StatusCode::OK, Json(AnalyzeResponse {
                status: "success".to_string(),
                has_left_recursion: Some(is_recursive),
                start_symbol: Some(grammar.start_symbol.clone()),
                production_count: Some(grammar.productions.len()),
                productions: Some(grammar.productions),
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

/// Handler to execute LL(1) parsing simulation
pub async fn parse_ll1(Json(payload): Json<ParseRequest>) -> impl IntoResponse {
    let grammar = match Grammar::from_string(&payload.raw_grammar) {
        Ok(g) => g,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(ParseResponse {
            status: "error".to_string(),
            snapshots: None,
            parsing_table: None,
            message: Some(format!("Grammar Error: {}", e)),
        })),
    };

    let parser = match crate::parsers::ll1::LL1Parser::new(grammar.clone()) {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(ParseResponse {
            status: "error".to_string(),
            snapshots: None,
            parsing_table: None,
            message: Some(format!("LL(1) Table Error: {}", e)),
        })),
    };

    let tokens = tokenize_input(&payload.input_string, &grammar);

    match crate::parsers::Parser::parse(&parser, tokens) {
        Ok(snapshots) => {
            // Convert internal table to a serializable format
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

            (StatusCode::OK, Json(ParseResponse {
                status: "success".to_string(),
                snapshots: Some(snapshots),
                parsing_table: Some(serializable_table),
                message: None,
            }))
        },
        Err(e) => {
            (StatusCode::OK, Json(ParseResponse {
                status: "error".to_string(),
                snapshots: None,
                parsing_table: None,
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
    pub message: Option<String>,
}

pub type SLR1ParseResponse = LR0ParseResponse;

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
            message: Some(format!("LR(0) Table Error: {}", e)),
        })),
    };

    // Build automaton response
    let automaton = {
        let states: Vec<LR0AutomatonStateResponse> = parser.states.iter().enumerate().map(|(id, items)| {
            let item_strings: Vec<String> = items.iter().map(|item| item.to_display_string()).collect();
            let is_accept = items.iter().any(|item| {
                item.is_complete() && item.production.left == parser.augmented_grammar.start_symbol
            });
            LR0AutomatonStateResponse { id, items: item_strings, is_accept }
        }).collect();

        let transitions: Vec<LR0TransitionResponse> = parser.transitions.iter().map(|((from, sym), to)| {
            LR0TransitionResponse { from: *from, to: *to, symbol: sym.clone() }
        }).collect();

        LR0AutomatonResponse { states, transitions }
    };

    // Build serializable ACTION table: state_id_str -> terminal -> action_str
    let mut action_map: std::collections::HashMap<String, std::collections::HashMap<String, String>> = std::collections::HashMap::new();
    for ((state_id, terminal), action) in &parser.action_table {
        action_map
            .entry(state_id.to_string())
            .or_default()
            .insert(terminal.clone(), action.to_display_string());
    }

    // Build serializable GOTO table: state_id_str -> non_terminal -> target_state_str
    let mut goto_map: std::collections::HashMap<String, std::collections::HashMap<String, String>> = std::collections::HashMap::new();
    for ((state_id, nt), target) in &parser.goto_table {
        goto_map
            .entry(state_id.to_string())
            .or_default()
            .insert(nt.clone(), target.to_string());
    }

    let tokens = tokenize_input(&payload.input_string, &grammar);

    // Run the parsing simulation
    match parser.parse_input(tokens) {
        Ok(snapshots) => {
            (StatusCode::OK, Json(LR0ParseResponse {
                status: "success".to_string(),
                automaton: Some(automaton),
                action_table: Some(action_map),
                goto_table: Some(goto_map),
                terminals: Some(parser.get_all_terminals()),
                non_terminals: Some(parser.get_all_non_terminals()),
                snapshots: Some(snapshots),
                message: None,
            }))
        }
        Err(e) => {
            (StatusCode::OK, Json(LR0ParseResponse {
                status: "error".to_string(),
                automaton: Some(automaton),
                action_table: Some(action_map),
                goto_table: Some(goto_map),
                terminals: Some(parser.get_all_terminals()),
                non_terminals: Some(parser.get_all_non_terminals()),
                snapshots: None,
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
            message: Some(format!("SLR(1) Table Error: {}", e)),
        })),
    };

    let automaton = {
        let states: Vec<LR0AutomatonStateResponse> = parser.states.iter().enumerate().map(|(id, items)| {
            let item_strings: Vec<String> = items.iter().map(|item| item.to_display_string()).collect();
            let is_accept = items.iter().any(|item| {
                item.is_complete() && item.production.left == parser.augmented_grammar.start_symbol
            });
            LR0AutomatonStateResponse { id, items: item_strings, is_accept }
        }).collect();

        let transitions: Vec<LR0TransitionResponse> = parser.transitions.iter().map(|((from, sym), to)| {
            LR0TransitionResponse { from: *from, to: *to, symbol: sym.clone() }
        }).collect();

        LR0AutomatonResponse { states, transitions }
    };

    let mut action_map: std::collections::HashMap<String, std::collections::HashMap<String, String>> = std::collections::HashMap::new();
    for ((state_id, terminal), action) in &parser.action_table {
        action_map
            .entry(state_id.to_string())
            .or_default()
            .insert(terminal.clone(), action.to_display_string());
    }

    let mut goto_map: std::collections::HashMap<String, std::collections::HashMap<String, String>> = std::collections::HashMap::new();
    for ((state_id, nt), target) in &parser.goto_table {
        goto_map
            .entry(state_id.to_string())
            .or_default()
            .insert(nt.clone(), target.to_string());
    }

    let tokens = tokenize_input(&payload.input_string, &grammar);

    match parser.parse_input(tokens) {
        Ok(snapshots) => {
            (StatusCode::OK, Json(SLR1ParseResponse {
                status: "success".to_string(),
                automaton: Some(automaton),
                action_table: Some(action_map),
                goto_table: Some(goto_map),
                terminals: Some(parser.get_all_terminals()),
                non_terminals: Some(parser.get_all_non_terminals()),
                snapshots: Some(snapshots),
                message: None,
            }))
        }
        Err(e) => {
            (StatusCode::OK, Json(SLR1ParseResponse {
                status: "error".to_string(),
                automaton: Some(automaton),
                action_table: Some(action_map),
                goto_table: Some(goto_map),
                terminals: Some(parser.get_all_terminals()),
                non_terminals: Some(parser.get_all_non_terminals()),
                snapshots: None,
                message: Some(e),
            }))
        }
    }
}


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

    let parser = match crate::parsers::ll1::LL1Parser::new(grammar) {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(ParseResponse {
            status: "error".to_string(),
            snapshots: None,
            parsing_table: None,
            message: Some(format!("LL(1) Table Error: {}", e)),
        })),
    };

    // Convert input string to tokens (simple whitespace split for now, or character based if needed)
    // The frontend usually sends tokens separated by space or just characters.
    // For Phase 3, we'll assume space-separated tokens or single characters if no spaces.
    let tokens: Vec<String> = if payload.input_string.contains(' ') {
        payload.input_string.split_whitespace().map(|s| s.to_string()).collect()
    } else {
        payload.input_string.chars().map(|c| c.to_string()).collect()
    };

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

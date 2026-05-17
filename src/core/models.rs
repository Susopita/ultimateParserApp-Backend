use serde::{Deserialize, Serialize};

/// Represents a grammar symbol: Terminal, Non-Terminal, or Epsilon (empty string).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Symbol {
    Terminal(String),
    NonTerminal(String),
    Epsilon,
}

/// Represents a production rule in the grammar: Left -> Right
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Production {
    pub left: Symbol,
    pub right: Vec<Symbol>,
}

/// Represents a Formal Grammar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grammar {
    pub productions: Vec<Production>,
    pub start_symbol: Symbol,
}

/// A snapshot of the parser state at a given step (used by LL(1)).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseSnapshot {
    pub step: usize,
    pub stack: Vec<Symbol>,
    pub input_remaining: Vec<String>,
    pub action: String,
}

/// A snapshot of the LR(0) parser state at a given step.
/// Uses separate state and symbol stacks for accurate representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LR0ParseSnapshot {
    pub step: usize,
    pub state_stack: Vec<usize>,
    pub symbol_stack: Vec<String>,
    pub input_remaining: Vec<String>,
    pub action: String,
}

/// Structure for serializing automata to D3.js compatible JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomatonJSON {
    pub nodes: Vec<NodeJSON>,
    pub links: Vec<LinkJSON>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeJSON {
    pub id: String,
    pub label: String,
    #[serde(rename = "isFinal")]
    pub is_final: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkJSON {
    pub source: String,
    pub target: String,
    pub label: String,
}

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

/// A node in the parse tree or AST.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseTreeNode {
    pub id: usize,
    pub label: String,
    /// "terminal" | "non_terminal" | "epsilon"
    pub node_type: String,
    pub children: Vec<ParseTreeNode>,
}

impl ParseTreeNode {
    /// Derives an AST from the parse tree by pruning:
    /// - epsilon nodes (label == "ϵ")
    /// - single-child non_terminal chains (chain rule elimination)
    pub fn to_ast(&self) -> ParseTreeNode {
        let pruned: Vec<ParseTreeNode> = self.children.iter()
            .filter(|c| c.node_type != "epsilon")
            .map(|c| c.to_ast())
            .collect();

        if self.node_type == "non_terminal" && pruned.len() == 1 {
            return pruned.into_iter().next().unwrap();
        }

        ParseTreeNode {
            id: self.id,
            label: self.label.clone(),
            node_type: self.node_type.clone(),
            children: pruned,
        }
    }

    /// Builds a petgraph directed graph, then serializes it to a styled DOT string
    /// suitable for rendering with Graphviz.
    pub fn to_dot(&self, graph_name: &str) -> String {
        use petgraph::Graph;
        use petgraph::graph::NodeIndex;
        use std::collections::HashMap;

        // ── Build petgraph representation ───────────────────────────────
        let mut pg: Graph<(usize, String, String), ()> = Graph::new();
        let mut id_map: HashMap<usize, NodeIndex> = HashMap::new();

        fn visit(
            node: &ParseTreeNode,
            pg: &mut Graph<(usize, String, String), ()>,
            id_map: &mut HashMap<usize, NodeIndex>,
        ) -> NodeIndex {
            let idx = pg.add_node((node.id, node.label.clone(), node.node_type.clone()));
            id_map.insert(node.id, idx);
            for child in &node.children {
                let child_idx = visit(child, pg, id_map);
                pg.add_edge(idx, child_idx, ());
            }
            idx
        }

        visit(self, &mut pg, &mut id_map);

        // ── Generate styled DOT from petgraph node/edge iterators ───────
        let mut dot = format!(
            "digraph {name} {{\n\
             \trankdir=TB;\n\
             \tbgcolor=\"transparent\";\n\
             \tnode [fontname=\"Courier,monospace\" fontsize=13 margin=\"0.12,0.06\" penwidth=1.5];\n\
             \tedge [color=\"#64748b\" arrowsize=0.75 penwidth=1.2];\n",
            name = graph_name
        );

        for ni in pg.node_indices() {
            let (id, label, node_type) = &pg[ni];
            let (shape, fillcolor, fontcolor, border) = match node_type.as_str() {
                "terminal" => ("box",     "#10b981", "#ffffff", "#059669"),
                "epsilon"  => ("ellipse", "#1e293b", "#64748b", "#334155"),
                _          => ("ellipse", "#f59e0b", "#0f172a", "#d97706"),
            };
            let safe = label.replace('\\', "\\\\").replace('"', "\\\"");
            dot.push_str(&format!(
                "\tn{id} [label=\"{safe}\" shape={shape} style=\"filled,rounded\" \
                 fillcolor=\"{fillcolor}\" fontcolor=\"{fontcolor}\" color=\"{border}\"];\n"
            ));
        }

        for ei in pg.edge_indices() {
            let (a, b) = pg.edge_endpoints(ei).unwrap();
            dot.push_str(&format!("\tn{} -> n{};\n", pg[a].0, pg[b].0));
        }

        dot.push('}');
        dot
    }
}


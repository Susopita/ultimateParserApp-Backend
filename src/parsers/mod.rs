pub mod ll1;
pub mod recursive_descent;
pub mod lr0;

#[cfg(test)]
mod tests;

use crate::core::models::ParseSnapshot;

/// Trait that all parsers must implement.
pub trait Parser {
    /// Parses an input sequence of tokens and returns a list of snapshots representing the steps.
    fn parse(&self, input: Vec<String>) -> Result<Vec<ParseSnapshot>, String>;
}

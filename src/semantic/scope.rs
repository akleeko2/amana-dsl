// src/semantic/scope.rs
use super::SemanticAnalyzer;
use super::suggestions::levenshtein_distance;
use crate::ast::DataType;
use std::collections::BTreeMap;

/// Represents a variable or function symbol registered in Amana's symbol table.
#[derive(Clone, Debug)]
pub struct Symbol {
    /// The name of the symbol.
    pub name: String,
    /// The resolved type of the symbol.
    pub data_type: DataType,
}

/// Represents a scoping level in Amana containing local variables and parent scope references.
pub struct Scope {
    /// Reference to the parent scope's index in the allocator vector, if any.
    pub parent: Option<usize>,
    /// Map of symbol names to their respective registered Symbol details.
    pub symbols: BTreeMap<String, Symbol>,
}

impl SemanticAnalyzer {
    /// Enters a new nested scope block.
    pub fn enter_scope(&mut self) {
        let new_id = self.scopes.len();
        self.scopes.push(Scope {
            parent: Some(self.current_scope),
            symbols: BTreeMap::new(),
        });
        self.current_scope = new_id;
    }

    /// Exits the current scope, returning to its parent.
    pub fn exit_scope(&mut self) {
        if let Some(parent) = self.scopes[self.current_scope].parent {
            self.current_scope = parent;
        }
    }

    /// Declares a new Symbol inside the current active Scope.
    pub fn declare_symbol(&mut self, name: &str, data_type: DataType) {
        self.scopes[self.current_scope].symbols.insert(
            name.to_string(),
            Symbol {
                name: name.to_string(),
                data_type,
            },
        );
    }

    /// Resolves a symbol by searching up the nested scopes hierarchy.
    pub fn resolve_symbol(&self, name: &str) -> Option<&Symbol> {
        let mut curr = self.current_scope;
        loop {
            if let Some(sym) = self.scopes[curr].symbols.get(name) {
                return Some(sym);
            }
            if let Some(parent) = self.scopes[curr].parent {
                curr = parent;
            } else {
                break;
            }
        }
        None
    }

    /// Retrieves all visible variable and function names within the current scope hierarchy.
    pub fn get_all_symbols_in_scope(&self) -> Vec<String> {
        let mut symbols = vec![
            "time".to_string(),
            "http".to_string(),
            "auth".to_string(),
            "env".to_string(),
        ];

        let mut curr = self.current_scope;
        loop {
            for name in self.scopes[curr].symbols.keys() {
                symbols.push(name.clone());
            }
            if let Some(parent) = self.scopes[curr].parent {
                curr = parent;
            } else {
                break;
            }
        }
        symbols
    }

    /// Finds and returns the closest matching symbol in scope if within Levenshtein threshold.
    pub fn suggest_similar_variable(&self, name: &str) -> Option<String> {
        let candidates = self.get_all_symbols_in_scope();
        let mut best_candidate = None;
        let mut min_distance = 3;

        for candidate in candidates {
            let dist = levenshtein_distance(name, &candidate);
            if dist < min_distance {
                min_distance = dist;
                best_candidate = Some(candidate);
            }
        }
        best_candidate
    }
}

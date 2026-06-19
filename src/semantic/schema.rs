// src/semantic/schema.rs
use crate::ast::{DataType, ModelDecl};
use std::collections::BTreeMap;

/// Represents the database schema for a registered Model.
pub struct TableSchema {
    /// The original model name.
    pub model_name: String,
    /// The physical database table name.
    pub table_name: String,
    /// Map of columns and their corresponding data types.
    pub columns: BTreeMap<String, DataType>,
}

/// Context structure maintaining all whitelisted database table structures.
pub struct SchemaContext {
    /// Mapping from lowercase table names to their table schema.
    pub tables: BTreeMap<String, TableSchema>,
}

impl SchemaContext {
    /// Creates a new SchemaContext mapping and registers implicit fields (e.g. ID).
    pub fn new(models: &[ModelDecl]) -> Self {
        let mut tables = BTreeMap::new();
        for m in models {
            let mut columns = BTreeMap::new();
            for f in &m.fields {
                columns.insert(f.name.to_lowercase(), f.data_type.clone());
            }
            // Add implicit primary key id field
            columns.insert("id".to_string(), DataType::Int);

            let table_name = m.name.to_lowercase();
            tables.insert(
                table_name.clone(),
                TableSchema {
                    model_name: m.name.clone(),
                    table_name,
                    columns,
                },
            );
        }
        Self { tables }
    }

    /// Verifies if a given table name is whitelisted in the application schema.
    pub fn is_whitelisted(&self, table_name: &str) -> bool {
        self.tables.contains_key(&table_name.to_lowercase())
    }
}

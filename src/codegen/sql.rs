// src/codegen/sql.rs
use crate::ast::{DataType, Expression};
use crate::semantic::ir::ModelIR;

const DEFAULT_QUERY_LIMIT: f64 = 100.0;

fn sql_literal(value: &str, data_type: &DataType) -> String {
    if value.eq_ignore_ascii_case("CURRENT_TIMESTAMP")
        || value.eq_ignore_ascii_case("CURRENT_DATE")
        || value.eq_ignore_ascii_case("CURRENT_TIME")
        || matches!(
            data_type,
            DataType::Int | DataType::Float | DataType::Money | DataType::Bool
        )
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "''"))
    }
}

fn is_text_type(data_type: &DataType) -> bool {
    matches!(
        data_type,
        DataType::Str
            | DataType::Email
            | DataType::Password
            | DataType::DateTime
            | DataType::Custom(_)
    )
}

pub fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn is_pagination_key(key: &str) -> bool {
    matches!(key.to_lowercase().as_str(), "limit" | "offset" | "page")
}

fn find_named_arg<'a>(
    query_args: &'a [(Option<String>, Expression)],
    key: &str,
) -> Option<&'a Expression> {
    query_args.iter().find_map(|(arg_key, expr)| {
        arg_key
            .as_ref()
            .filter(|candidate| candidate.eq_ignore_ascii_case(key))
            .map(|_| expr)
    })
}

fn append_pagination_clause(
    sql: &mut String,
    params: &mut Vec<Expression>,
    query_args: &[(Option<String>, Expression)],
) -> Result<(), String> {
    let limit = find_named_arg(query_args, "limit").cloned();
    let offset = find_named_arg(query_args, "offset").cloned();
    let page = find_named_arg(query_args, "page").cloned();

    if offset.is_some() && page.is_some() {
        return Err(
            "Query execution failed: use either 'offset' or 'page', not both.".to_string(),
        );
    }

    if let Some(limit_expr) = limit.clone() {
        sql.push_str(" LIMIT ?");
        params.push(limit_expr);
    } else {
        sql.push_str(&format!(" LIMIT {}", DEFAULT_QUERY_LIMIT as i64));
    }

    if let Some(offset_expr) = offset {
        sql.push_str(" OFFSET ?");
        params.push(offset_expr);
    } else if let Some(page_expr) = page {
        let limit_expr = limit.unwrap_or(Expression::Number(DEFAULT_QUERY_LIMIT));
        sql.push_str(" OFFSET ((? - 1) * ?)");
        params.push(page_expr);
        params.push(limit_expr);
    }

    Ok(())
}

/// Generates the SQL Data Definition Language (DDL) CREATE TABLE statement for a ModelIR.
/// It registers column data types, constraints, uniqueness, and foreign keys.
pub fn generate_table_ddl(model: &ModelIR) -> String {
    let mut columns_ddl = Vec::new();

    let has_explicit_primary_key = model.fields.iter().any(|f| f.is_primary_key);
    if !has_explicit_primary_key {
        columns_ddl.push("\"id\" INTEGER PRIMARY KEY AUTOINCREMENT".to_string());
    }

    for f in &model.fields {
        if f.name.to_lowercase() == "id" {
            continue; // Prevent duplicate ID definition
        }

        let type_str = match &f.data_type {
            DataType::Str
            | DataType::Email
            | DataType::Password
            | DataType::DateTime
            | DataType::Custom(_) => "TEXT",
            DataType::Int | DataType::Bool => "INTEGER",
            DataType::Float | DataType::Money => "REAL",
            _ => "TEXT",
        };

        let field_name = f.name.to_lowercase();
        let field_sql = quote_identifier(&field_name);
        let mut field_ddl = format!("{} {}", field_sql, type_str);
        if f.is_primary_key {
            field_ddl.push_str(" PRIMARY KEY");
            if matches!(f.data_type, DataType::Int) {
                field_ddl.push_str(" AUTOINCREMENT");
            }
        }
        if f.is_unique {
            field_ddl.push_str(" UNIQUE");
        }
        if f.is_required && !f.is_primary_key && f.default_value.is_none() {
            field_ddl.push_str(" NOT NULL");
        }
        if let Some(default_value) = &f.default_value {
            field_ddl.push_str(" DEFAULT ");
            field_ddl.push_str(&sql_literal(default_value, &f.data_type));
        }
        if f.min_value.is_some() || f.max_value.is_some() {
            let mut checks = Vec::new();
            if let Some(min) = f.min_value {
                if is_text_type(&f.data_type) {
                    checks.push(format!("length({}) >= {}", field_sql, min));
                } else {
                    checks.push(format!("{} >= {}", field_sql, min));
                }
            }
            if let Some(max) = f.max_value {
                if is_text_type(&f.data_type) {
                    checks.push(format!("length({}) <= {}", field_sql, max));
                } else {
                    checks.push(format!("{} <= {}", field_sql, max));
                }
            }
            field_ddl.push_str(&format!(" CHECK ({})", checks.join(" AND ")));
        }
        if let Some((target_model, target_field)) = &f.foreign_key {
            let delete_action = f.on_delete.as_deref().unwrap_or("CASCADE");
            field_ddl.push_str(&format!(
                " REFERENCES {}({}) ON DELETE {}",
                quote_identifier(&target_model.to_lowercase()),
                quote_identifier(&target_field.to_lowercase()),
                delete_action
            ));
        }
        columns_ddl.push(field_ddl);
    }

    format!(
        "CREATE TABLE IF NOT EXISTS {} (\n  {}\n);",
        quote_identifier(&model.name.to_lowercase()),
        columns_ddl.join(",\n  ")
    )
}

/// Generates a parameterized SQL SELECT query string and its bindings, checking model fields rules.
/// This prevents SQL injection attacks on dynamic query methods (all, find, filter, count).
pub fn generate_safe_query(
    models: &[ModelIR],
    model_name: &str,
    query_method: &str,
    query_args: &[(Option<String>, Expression)],
) -> Result<(String, Vec<Expression>), String> {
    let table_key = model_name.to_lowercase();
    let model = models
        .iter()
        .find(|m| m.table_name == table_key)
        .ok_or_else(|| {
            format!(
                "Security Exception: Access to table '{}' is restricted or table is undefined.",
                model_name
            )
        })?;

    let table_sql = quote_identifier(&model.table_name);
    let mut sql = format!("SELECT * FROM {}", table_sql);
    let mut params = Vec::new();

    match query_method {
        "all" => {
            for (key, _) in query_args {
                let Some(key) = key else {
                    return Err(
                        "Query execution failed: 'all' accepts only named pagination arguments (limit, offset, page).".to_string(),
                    );
                };
                if !is_pagination_key(key) {
                    return Err(format!(
                        "Query execution failed: 'all' does not accept filter argument '{}'. Use filter(...) for column filters.",
                        key
                    ));
                }
            }
            append_pagination_clause(&mut sql, &mut params, query_args)?;
        }
        "find" => {
            sql.push_str(" WHERE \"id\" = ? LIMIT 1");
            if let Some((_, expr)) = query_args.first() {
                params.push(expr.clone());
            } else {
                return Err(
                    "Query execution failed: 'find' method requires an identifier argument."
                        .to_string(),
                );
            }
        }
        "filter" => {
            let mut filter_clauses = Vec::new();
            for (col_opt, expr) in query_args {
                if let Some(col) = col_opt {
                    if is_pagination_key(col) {
                        continue;
                    }
                    let has_col = model
                        .fields
                        .iter()
                        .any(|f| f.name.to_lowercase() == col.to_lowercase())
                        || col.to_lowercase() == "id";
                    if !has_col {
                        return Err(format!(
                            "SQL Compilation Error: Column '{}' not found in model '{}'",
                            col, model_name
                        ));
                    }
                    filter_clauses.push(format!("{} = ?", quote_identifier(&col.to_lowercase())));
                    params.push(expr.clone());
                } else {
                    return Err("Query execution failed: 'filter' method requires keyword arguments (e.g. status: \"active\").".to_string());
                }
            }
            if !filter_clauses.is_empty() {
                sql.push_str(" WHERE ");
                sql.push_str(&filter_clauses.join(" AND "));
            }
            append_pagination_clause(&mut sql, &mut params, query_args)?;
        }
        "count" => {
            sql = format!("SELECT COUNT(*) AS count FROM {}", table_sql);
            let mut filter_clauses = Vec::new();
            for (col_opt, expr) in query_args {
                if let Some(col) = col_opt {
                    let has_col = model
                        .fields
                        .iter()
                        .any(|f| f.name.to_lowercase() == col.to_lowercase())
                        || col.to_lowercase() == "id";
                    if !has_col {
                        return Err(format!(
                            "SQL Compilation Error: Column '{}' not found in model '{}'",
                            col, model_name
                        ));
                    }
                    filter_clauses.push(format!("{} = ?", quote_identifier(&col.to_lowercase())));
                    params.push(expr.clone());
                }
            }
            if !filter_clauses.is_empty() {
                sql.push_str(" WHERE ");
                sql.push_str(&filter_clauses.join(" AND "));
            }
        }
        _ => {
            return Err(format!(
                "Unsupported query method '{}' for SQL Codegen.",
                query_method
            ));
        }
    }

    Ok((sql, params))
}

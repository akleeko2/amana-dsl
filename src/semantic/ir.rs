// src/semantic/ir.rs
use crate::ast::{DataType, DesignBlock, Expression, StateDecl, ViewElement, ComponentDecl};
use serde::{Deserialize, Serialize};

/// Represents a field in the database Model inside the Amana IR.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelFieldIR {
    /// Column name in physical database table.
    pub name: String,
    /// Amana data type.
    pub data_type: DataType,
    /// Is primary key.
    pub is_primary_key: bool,
    /// Is unique constraint.
    pub is_unique: bool,
    /// Is required / not nullable.
    pub is_required: bool,
    /// Minimum value constraint.
    pub min_value: Option<f64>,
    /// Maximum value constraint.
    pub max_value: Option<f64>,
    /// Default value.
    pub default_value: Option<String>,
    /// Foreign key reference.
    pub foreign_key: Option<(String, String)>,
    /// On delete behavior.
    pub on_delete: Option<String>,
}

/// Global design theme compiled into the IR.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThemeIR {
    pub settings: Vec<(String, String)>,
}

/// Represents a database Model structure compiled to Amana IR.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelIR {
    /// Original PascalCase model name.
    pub name: String,
    /// Lowercase physical database table name.
    pub table_name: String,
    /// List of model fields.
    pub fields: Vec<ModelFieldIR>,
}

/// Represents a server-side query fetch statement.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FetchIR {
    /// Target variable name to store the query result.
    pub var_name: String,
    /// Fetch target model name.
    pub model_name: String,
    /// Query method (all, find, filter, count).
    pub query_method: String,
    /// Query arguments.
    pub query_args: Vec<(Option<String>, Expression)>,
}

/// Represents security access routing parameters compiled to Amana IR.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuardIR {
    /// Abstract condition expression to authorize request.
    pub cond_expr: Expression,
    /// URL redirect path if forbidden access.
    pub deny_path: String,
    /// URL redirect path if user is unauthenticated.
    pub unauth_path: String,
}

/// Represents dynamic POST form submissions handled by the backend.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FormActionIR {
    /// Target database Model name.
    pub model_name: String,
    /// Form action method (create, update, delete).
    pub action: String,
    /// Form fields.
    pub fields: Vec<String>,
    /// Server-side default field bindings evaluated at submit time.
    pub defaults: Vec<(String, Expression)>,
    /// Server-side ownership/authorization filters evaluated at submit time.
    pub constraints: Vec<(String, Expression)>,
    /// URL redirect path on success.
    pub redirect_success: String,
}

/// Represents an active URL path mapping to a specific view template.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteIR {
    /// URL match pattern (e.g. /tasks).
    pub path: String,
    /// Target EJS view template name.
    pub view_name: String,
    /// Access control routing protection block.
    pub guard: Option<GuardIR>,
    /// List of server-side data fetches required before rendering.
    pub fetches: Vec<FetchIR>,
    /// List of registered form processing hooks.
    pub form_actions: Vec<FormActionIR>,
}

/// Represents an HTML template view compiled to EJS.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViewIR {
    /// PascalCase view name.
    pub name: String,
    /// View client state declarations.
    pub client_states: Vec<StateDecl>,
    /// Structured ViewElement layout tree.
    pub render_body: Option<ViewElement>,
    /// Compiled CSS styles layout block.
    pub styles: Option<String>,
    /// Page-level design grammar settings.
    pub canvas: Option<DesignBlock>,
}

/// Represents global application metadata and security configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppIR {
    /// App name.
    pub name: String,
    /// HTML head page title.
    pub title: String,
    /// Path to physical SQLite database file.
    pub db_path: String,
    /// Model used for authentication (e.g. User).
    pub auth_model: String,
    /// Capability whitelist allowed by the application.
    pub capabilities: Vec<String>,
}

/// Seed rows for generated applications.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeedIR {
    pub model_name: String,
    pub rows: Vec<Vec<(String, Expression)>>,
}

/// Version metadata for the generated Amana IR file.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IRVersion {
    /// Major version.
    pub major: u32,
    /// Minor version.
    pub minor: u32,
    /// Patch version.
    pub patch: u32,
    /// Allowed capability checklist.
    pub capabilities: Vec<String>,
}

/// Root node of the compiled Amana Intermediate Representation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AmanaIR {
    /// IR file format versions.
    pub ir_version: IRVersion,
    /// App configuration parameters.
    pub app: AppIR,
    /// Registered database tables and schema.
    pub models: Vec<ModelIR>,
    /// Design theme settings.
    pub theme: Option<ThemeIR>,
    /// Registered route handlers.
    pub routes: Vec<RouteIR>,
    /// Compiled EJS templates and styling blocks.
    pub views: Vec<ViewIR>,
    /// Seed data rows.
    pub seeds: Vec<SeedIR>,
    /// Compiled reusable components.
    pub components: Vec<ComponentDecl>,
}

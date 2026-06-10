// src/ast/mod.rs
use serde::{Deserialize, Serialize};

/// Represents data types supported by Amana's database fields and semantic analyzer.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DataType {
    Str,
    Int,
    Float,
    Bool,
    Email,
    Password,
    DateTime,
    Money,
    Custom(String),
    Model(String),
    List(Box<DataType>),
}

/// Represents a field definition inside an Amana database model.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelField {
    pub name: String,
    pub data_type: DataType,
    pub is_primary_key: bool,
    pub is_unique: bool,
    pub is_required: bool,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub default_value: Option<String>,
    pub foreign_key: Option<(String, String)>, // (Model, Field)
    pub on_delete: Option<String>,
}

/// Represents global design theme settings consumed by code generators.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThemeDecl {
    pub settings: Vec<(String, String)>,
}

/// Represents seed data for generated applications.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeedDecl {
    pub model_name: String,
    pub rows: Vec<Vec<(String, Expression)>>,
}

/// Represents a frontend design grammar block such as canvas, compose, visual, brand, art, responsive, or creative.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DesignBlock {
    pub kind: String,
    pub settings: Vec<(String, String)>,
}

/// Represents an access control permission rule (e.g. `permit Admin read Todo`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PermissionRule {
    pub role: String,
    pub action: String,
    pub resource: String,
}

/// Represents a database model declaration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelDecl {
    pub name: String,
    pub fields: Vec<ModelField>,
    pub permissions: Vec<PermissionRule>,
}

/// Represents a route declaration matching a URL path to an EJS view.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteDecl {
    pub path: String,
    pub view_name: String,
    pub guards: Vec<GuardStmt>,
    pub fetches: Vec<FetchStmt>,
}

/// Represents a guard statement for route protection
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuardStmt {
    pub condition: Expression,
    pub else_action: String, // e.g., "redirect /login"
}

/// Represents language expressions, literals, operations, member access, and calls.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Expression {
    Binary {
        left: Box<Expression>,
        op: String,
        right: Box<Expression>,
    },
    Unary {
        op: String,
        expr: Box<Expression>,
    },
    Ternary {
        cond: Box<Expression>,
        then_branch: Box<Expression>,
        else_branch: Box<Expression>,
    },
    Call {
        callee: Box<Expression>,
        args: Vec<Expression>,
    },
    MemberAccess {
        object: Box<Expression>,
        property: String,
    },
    Identifier(String),
    Number(f64),
    StringLiteral(String),
    Boolean(bool),
    Null,
}

/// Represents structural HTML elements, formatted text, control flow loops, forms, and charts.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ViewElement {
    Element {
        tag: String,
        classes: Vec<String>,
        attributes: Vec<(String, Expression)>,
        children: Vec<ViewElement>,
    },
    Text(String),
    FormattedText(Vec<Expression>),
    ForEach {
        item_var: String,
        list_expr: Expression,
        body: Vec<ViewElement>,
    },
    IfBlock {
        condition: Expression,
        then_branch: Vec<ViewElement>,
        else_branch: Option<Vec<ViewElement>>,
    },
    FormBlock {
        fields: Vec<String>,
        connect_action: String, // e.g. "Todo.create"
        redirect_success: String,
        defaults: Vec<(String, Expression)>,
        constraints: Vec<(String, Expression)>,
        ui: Option<String>,
        submit_label: Option<String>,
        field_options: Vec<FormFieldOptions>,
    },
    Chart {
        data_expr: String,
        chart_type: String,
        x_field: String,
        y_field: String,
    },
    DesignBlock(DesignBlock),
    SlotDecl {
        name: String,
        optional: bool,
    },
    ResourceGrid {
        resource_expr: Expression,
        item_component: String,
        item_arg_name: String,
        empty_element: Option<Vec<ViewElement>>,
        loading_element: Option<Vec<ViewElement>>,
        error_element: Option<Vec<ViewElement>>,
        filter_fields: Vec<String>,
        sort_fields: Vec<String>,
    },
    ResourceTable {
        resource_expr: Expression,
        item_component: String,
        item_arg_name: String,
        empty_element: Option<Vec<ViewElement>>,
        loading_element: Option<Vec<ViewElement>>,
        error_element: Option<Vec<ViewElement>>,
        filter_fields: Vec<String>,
        sort_fields: Vec<String>,
    },
}

/// Represents UI metadata for one generated form field.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FormFieldOptions {
    pub name: String,
    pub label: Option<String>,
    pub placeholder: Option<String>,
    pub input_type: Option<String>,
    pub help: Option<String>,
    pub required: Option<bool>,
}

/// Represents client state declarations (e.g. `state count = 0`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateDecl {
    pub name: String,
    pub initial_value: Expression,
    pub persist: String, // "memory", "cookie", "session", "local"
}

/// Represents a protected route access configuration with allow rules and redirects.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProtectedBlock {
    pub allow_expr: Expression,
    pub deny_path: String,
    pub unauth_path: String,
}

/// Represents server-side queries and standard library data fetching statements.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FetchStmt {
    pub var_name: String,
    pub model_name: String,
    pub query_method: String, // "all", "find", "filter", "count"
    pub query_args: Vec<(Option<String>, Expression)>, // e.g. [ (Some("id"), Expression) ]
}

/// Represents a view declaration block (including fetches, client states, render body, and styling).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViewDecl {
    pub name: String,
    pub protected: Option<ProtectedBlock>,
    pub server_fetches: Vec<FetchStmt>,
    pub client_states: Vec<StateDecl>,
    pub render_body: Option<ViewElement>,
    pub styles: Option<String>,
    pub canvas: Option<DesignBlock>,
}

/// Represents a custom component parameter definition.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ComponentParam {
    pub name: String,
    pub ty: Option<String>,
    pub default_value: Option<Expression>,
    pub required: bool,
}

/// Represents a custom component variant style rule.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StyleRule {
    pub selector: String,
    pub declarations: Vec<CssDecl>,
}

/// Represents a style declaration key-value pair.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CssDecl {
    pub property: String,
    pub value: String,
}

/// Represents responsive visual style rules.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ResponsiveRule {
    pub breakpoint: String,
    pub rules: Vec<StyleRule>,
}

/// Represents a custom component or global variant declaration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VariantDecl {
    pub target: String,
    pub name: String,
    pub base_rules: Vec<StyleRule>,
    pub hover_rules: Vec<StyleRule>,
    pub slot_rules: Vec<(String, Vec<StyleRule>)>,
    pub responsive_rules: Vec<ResponsiveRule>,
}

/// Represents a reusable component declaration with custom styling, parameters, and variants.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComponentDecl {
    pub name: String,
    pub params: Vec<ComponentParam>,
    pub render_body: Option<ViewElement>,
    pub styles: Option<String>,
    pub variants: Vec<VariantDecl>,
}

/// Represents global token configurations.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenConfigBlock {
    pub colors: Vec<(String, String)>,
    pub spacing: Vec<(String, String)>,
    pub radius: Vec<(String, String)>,
    pub shadows: Vec<(String, String)>,
}

/// Represents global application configurations (database path, auth model, capabilities).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub name: String,
    pub title: String,
    pub db_path: String,
    pub auth_model: String,
    pub capabilities: Vec<String>,
}

/// A node in the AST, representing top-level Amana declarations.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AmanaNode {
    App(AppConfig),
    Theme(ThemeDecl),
    Model(ModelDecl),
    Route(RouteDecl),
    View(ViewDecl),
    Component(ComponentDecl),
    Seed(SeedDecl),
    Variant(VariantDecl),
    Tokens(TokenConfigBlock),
}


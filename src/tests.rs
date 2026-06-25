// src/tests.rs
use crate::ast::*;
use crate::codegen::express::{generate_project, validate_custom_hooks};
use crate::codegen::sql::{generate_safe_query, generate_table_ddl};
use crate::lexer::{Lexer, TokenKind};
use crate::parser::Parser;
use crate::semantic::SemanticAnalyzer;
use crate::{
    CliCommand, IrSnapshotMode, IrSnapshotRequest, compile_ir, compile_resolved_ir,
    handle_ir_snapshot, inspect_design, parse_cli_args, resolve_program_from_file,
};
use std::env;
use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn test_lexer() {
    let source = "model User:\n    email: str [unique]";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    assert!(!tokens.is_empty());
    // Since "model" is matched as an identifier in this lexer (or rather, the lexer might emit model as an App/Model keyword)
    // Let's verify that the token kinds match the expected TokenKind
    assert_eq!(tokens[0].kind, TokenKind::Model);
    assert_eq!(tokens[1].kind, TokenKind::Identifier("User".to_string()));
}

#[test]
fn test_parser() {
    let source = "app CafeSystem:\n    title: \"Cafe\"\n    auth_model: User\n    db_path: \"cafe.db\"\n\nmodel User:\n    email: str";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    assert_eq!(ast.len(), 2);
    match &ast[0] {
        AmanaNode::App(config) => {
            assert_eq!(config.name, "CafeSystem");
            assert_eq!(config.title, "Cafe");
        }
        _ => panic!("Expected App config node"),
    }
}

#[test]
fn test_multifile_imports_are_resolved_before_ir_generation() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let dir = env::temp_dir().join(format!(
        "amana_import_test_{}_{}",
        std::process::id(),
        unique
    ));
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("models.amana"),
        r#"
model User:
    email: email unique required
    password: password required min 8
"#,
    )
    .unwrap();
    fs::write(
        dir.join("app.amana"),
        r#"
import "./models.amana"

app MultiFile:
    title: "Multi File"
    auth_model: User

route / -> view Home

view Home:
    render:
        div.page:
            p: "Ready"
"#,
    )
    .unwrap();

    let entry = dir.join("app.amana");
    let program = resolve_program_from_file(&entry.to_string_lossy(), None, false).unwrap();
    assert_eq!(program.files.len(), 2);
    let ir = compile_resolved_ir(&program, false).unwrap();
    assert!(ir.models.iter().any(|model| model.name == "User"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_css_production_tokens_compile_to_safe_css() {
    let source = r#"
app CssTokens:
    title: "CSS Tokens"
    auth_model: User

model User:
    email: email unique
    password: password

route / -> view Home

view Home:
    render:
        div.panel:
            p: "Panel"

    style:
        .panel:
            direction: rtl
            position: sticky
            layer: modal
            top: lg
            transition: fade 300ms ease
"#;

    let ir = compile_ir("css-tokens.amana", source, false).unwrap();
    let styles = ir.views[0].styles.as_ref().unwrap();
    assert!(styles.contains("direction: rtl;"));
    assert!(styles.contains("position: sticky;"));
    assert!(styles.contains("z-index: var(--layer-modal);"));
    assert!(styles.contains("top: var(--space-lg);"));
    assert!(styles.contains("transition: opacity 300ms ease;"));
}

#[test]
fn test_sql_codegen_quotes_reserved_identifiers() {
    let source = r#"
app Shop:
    title: "Shop"
    auth_model: User

model User:
    email: email unique
    password: password

model Order:
    order_number: str unique required
    status: str default "pending"
    total: money required

route / -> view Home

view Home:
    server:
        fetch orders = Order.filter(status: "pending")
    render:
        div.page:
            p: "Orders"
"#;

    let ir = compile_ir("reserved-sql.amana", source, false).unwrap();
    let order = ir
        .models
        .iter()
        .find(|model| model.name == "Order")
        .unwrap();
    let ddl = generate_table_ddl(order);
    assert!(ddl.contains("CREATE TABLE IF NOT EXISTS \"order\""));
    assert!(ddl.contains("\"order_number\" TEXT UNIQUE NOT NULL"));
    assert!(ddl.contains("\"status\" TEXT DEFAULT 'pending'"));

    let (query, _) = generate_safe_query(
        &ir.models,
        "Order",
        "filter",
        &[(
            Some("status".to_string()),
            Expression::StringLiteral("pending".to_string()),
        )],
    )
    .unwrap();
    assert_eq!(query, "SELECT * FROM \"order\" WHERE \"status\" = ? LIMIT 100");
}

#[test]
fn test_semantic_valid() {
    let fields = vec![ModelField {
        name: "email".to_string(),
        data_type: DataType::Str,
        is_primary_key: false,
        is_unique: true,
        is_required: false,
        min_value: None,
        max_value: None,
        default_value: None,
        foreign_key: None,
        on_delete: None,
    }];
    let models = vec![ModelDecl {
        name: "User".to_string(),
        fields,
        permissions: vec![],
    }];

    let mut analyzer = SemanticAnalyzer::new(&models, "User", &[], &[]);
    let view = ViewDecl {
        name: "Home".to_string(),
        protected: Some(ProtectedBlock {
            allow_expr: Expression::Binary {
                left: Box::new(Expression::MemberAccess {
                    object: Box::new(Expression::Identifier("User".to_string())),
                    property: "current".to_string(),
                }),
                op: "!=".to_string(),
                right: Box::new(Expression::Null),
            },
            deny_path: "/login".to_string(),
            unauth_path: "/login".to_string(),
        }),
        server_fetches: vec![],
        client_states: vec![],
        render_body: None,
        styles: None,
        canvas: None,
    };

    let result = analyzer.validate_view(&view);
    assert!(
        result.is_ok(),
        "Semantic validation failed: {:?}",
        result.err()
    );
}

#[test]
fn test_semantic_invalid_comparison() {
    let fields = vec![ModelField {
        name: "age".to_string(),
        data_type: DataType::Int,
        is_primary_key: false,
        is_unique: false,
        is_required: false,
        min_value: None,
        max_value: None,
        default_value: None,
        foreign_key: None,
        on_delete: None,
    }];
    let models = vec![ModelDecl {
        name: "User".to_string(),
        fields,
        permissions: vec![],
    }];

    let mut analyzer = SemanticAnalyzer::new(&models, "User", &[], &[]);
    let view = ViewDecl {
        name: "Home".to_string(),
        protected: Some(ProtectedBlock {
            allow_expr: Expression::Binary {
                left: Box::new(Expression::MemberAccess {
                    object: Box::new(Expression::MemberAccess {
                        object: Box::new(Expression::Identifier("User".to_string())),
                        property: "current".to_string(),
                    }),
                    property: "age".to_string(),
                }),
                op: "==".to_string(),
                right: Box::new(Expression::StringLiteral("young".to_string())),
            },
            deny_path: "/login".to_string(),
            unauth_path: "/login".to_string(),
        }),
        server_fetches: vec![],
        client_states: vec![],
        render_body: None,
        styles: None,
        canvas: None,
    };

    let result = analyzer.validate_view(&view);
    assert!(
        result.is_err(),
        "Expected type mismatch error when comparing Int to String"
    );
    let err_msg = result.err().unwrap();
    assert!(
        err_msg.contains("type mismatch")
            || err_msg.contains("Comparison")
            || err_msg.contains("not valid"),
        "Unexpected error: {}",
        err_msg
    );
}

#[test]
fn test_hook_validation_valid() {
    let code = r#"
        module.exports = {
            beforeAll(req, res, next) {
                console.log("Hook active");
                next();
            }
        };
    "#;
    assert!(validate_custom_hooks(code).is_ok());
}

#[test]
fn test_hook_validation_invalid_sig() {
    let code = r#"
        module.exports = {
            beforeAll(req, res) {
                res.send("Bad signature");
            }
        };
    "#;
    assert!(validate_custom_hooks(code).is_err());
}

#[test]
fn test_hook_validation_unrecognized() {
    let code = r#"
        module.exports = {
            beforeEach(req, res, next) {
                next();
            }
        };
    "#;
    assert!(validate_custom_hooks(code).is_err());
}

#[test]
fn test_codegen_determinism() {
    let app = crate::semantic::ir::AppIR {
        name: "TestApp".to_string(),
        title: "Test Application".to_string(),
        db_path: "test.db".to_string(),
        auth_model: "User".to_string(),
        capabilities: vec![],
    };

    let ir = crate::semantic::ir::AmanaIR {
        ir_version: crate::semantic::ir::IRVersion {
            major: 1,
            minor: 0,
            patch: 0,
            capabilities: vec![],
        },
        app,
        models: vec![],
        theme: None,
        routes: vec![],
        views: vec![],
        seeds: vec![],
        components: vec![],
    };

    let temp_dir = "./temp_test_dist";
    let _ = std::fs::remove_dir_all(temp_dir);

    generate_project(temp_dir, &ir).unwrap();
    let app_js_1 = std::fs::read_to_string(format!("{}/app.js", temp_dir)).unwrap();

    generate_project(temp_dir, &ir).unwrap();
    let app_js_2 = std::fs::read_to_string(format!("{}/app.js", temp_dir)).unwrap();

    assert_eq!(
        app_js_1, app_js_2,
        "Codegen output must be 100% byte-for-byte identical"
    );

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn test_capabilities_and_view_boundaries() {
    let models = vec![];

    // 1. time capability check: missing
    let mut analyzer_no_cap = SemanticAnalyzer::new(&models, "User", &[], &[]);
    let view_time = ViewDecl {
        name: "TimeView".to_string(),
        protected: None,
        server_fetches: vec![FetchStmt {
            var_name: "t".to_string(),
            model_name: "time".to_string(),
            query_method: "now".to_string(),
            query_args: vec![],
        }],
        client_states: vec![],
        render_body: None,
        styles: None,
        canvas: None,
    };
    assert!(analyzer_no_cap.validate_view(&view_time).is_err());
    assert!(
        analyzer_no_cap
            .validate_view(&view_time)
            .unwrap_err()
            .contains("Missing capability 'time'")
    );

    // 2. time capability check: granted
    let mut analyzer_with_cap = SemanticAnalyzer::new(&models, "User", &["time".to_string()], &[]);
    assert!(analyzer_with_cap.validate_view(&view_time).is_ok());

    // 3. view boundary check: calling std in render block must fail
    let view_invalid_render = ViewDecl {
        name: "RenderTimeView".to_string(),
        protected: None,
        server_fetches: vec![],
        client_states: vec![],
        render_body: Some(ViewElement::Element {
            tag: "p".to_string(),
            classes: vec![],
            attributes: vec![],
            children: vec![ViewElement::FormattedText(vec![Expression::Call {
                callee: Box::new(Expression::MemberAccess {
                    object: Box::new(Expression::Identifier("time".to_string())),
                    property: "now".to_string(),
                }),
                args: vec![],
            }])],
        }),
        styles: None,
        canvas: None,
    };
    // Even if time capability is granted, std calls are banned inside render blocks!
    assert!(
        analyzer_with_cap
            .validate_view(&view_invalid_render)
            .is_err()
    );
    assert!(
        analyzer_with_cap
            .validate_view(&view_invalid_render)
            .unwrap_err()
            .contains("not allowed in render blocks")
    );
}

#[test]
fn test_optimizer_constant_folding_and_dce() {
    use crate::semantic::optimizer::{fold_constants, optimize_ast};

    // Test constant folding: 1 + 2 -> 3
    let expr = Expression::Binary {
        left: Box::new(Expression::Number(1.0)),
        op: "+".to_string(),
        right: Box::new(Expression::Number(2.0)),
    };
    let folded = fold_constants(expr);
    assert_eq!(folded, Expression::Number(3.0));

    // Test constant folding: "Hello " + "World" -> "Hello World"
    let expr_str = Expression::Binary {
        left: Box::new(Expression::StringLiteral("Hello ".to_string())),
        op: "+".to_string(),
        right: Box::new(Expression::StringLiteral("World".to_string())),
    };
    let folded_str = fold_constants(expr_str);
    assert_eq!(
        folded_str,
        Expression::StringLiteral("Hello World".to_string())
    );

    // Test DCE: Model User is kept (referenced as auth_model), Model Unused is removed
    let app = AmanaNode::App(AppConfig {
        name: "TestApp".to_string(),
        title: "Test".to_string(),
        db_path: "test.db".to_string(),
        auth_model: "User".to_string(),
        capabilities: vec![],
    });
    let model_user = AmanaNode::Model(ModelDecl {
        name: "User".to_string(),
        fields: vec![],
        permissions: vec![],
    });
    let model_unused = AmanaNode::Model(ModelDecl {
        name: "Unused".to_string(),
        fields: vec![],
        permissions: vec![],
    });

    let nodes = vec![app, model_user, model_unused];
    let optimized = optimize_ast(nodes);

    // Should only contain App config and Model User
    assert_eq!(optimized.len(), 2);
    let has_unused = optimized.iter().any(|node| {
        if let AmanaNode::Model(m) = node {
            m.name == "Unused"
        } else {
            false
        }
    });
    assert!(!has_unused, "Unused model should have been eliminated");
}

#[test]
fn test_components_and_styling_dsl() {
    let source = r#"
component GlassCard:
    style:
        .card:
            background: glass
            radius: large
            padding: large
            border: smooth
            shadow: smooth
    render:
        div.card:
            h3: title
            p: body_text
            slot:

route / -> view Home

view Home:
    client:
        state count = 0
    render:
        div:
            GlassCard(title: "My Title", body_text: "My Body"):
                p(click: count = count + 1): f"Click me: {count}"
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    // Check lexer mapped component keyword
    let has_component_keyword = tokens
        .iter()
        .any(|t| matches!(t.kind, TokenKind::Component));
    assert!(has_component_keyword);

    // Parse
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    assert_eq!(ast.len(), 3);

    let comp_decl = match &ast[0] {
        AmanaNode::Component(c) => c,
        _ => panic!("Expected ComponentDecl"),
    };

    assert_eq!(comp_decl.name, "GlassCard");

    // Check styling DSL compiled to premium CSS variables
    let styles = comp_decl.styles.as_ref().unwrap();
    assert!(styles.contains("background: var(--glass-bg); backdrop-filter: var(--glass-blur);"));
    assert!(styles.contains("border-radius: var(--radius-large);"));
    assert!(styles.contains("padding: var(--padding-large);"));
    assert!(styles.contains("border: 1px solid var(--border-color);"));
    assert!(styles.contains("box-shadow: var(--shadow-md);"));

    // Validate semantic analyzer
    let mut analyzer = SemanticAnalyzer::new(&[], "User", &[], std::slice::from_ref(comp_decl));
    let view_decl = match &ast[2] {
        AmanaNode::View(v) => v,
        _ => panic!("Expected ViewDecl"),
    };
    if let Err(e) = analyzer.validate_view(view_decl) {
        panic!("Semantic validation failed: {}", e);
    }

    // Test IR Generation and Component Inlining
    let app_config = AppConfig {
        name: "TestApp".to_string(),
        title: "Test Application".to_string(),
        db_path: "test.db".to_string(),
        auth_model: "User".to_string(),
        capabilities: vec![],
    };

    let ir = crate::semantic::ir_gen::generate_ir(&analyzer, &ast, &app_config).unwrap();

    let home_view_ir = ir.views.iter().find(|v| v.name == "Home").unwrap();

    let ejs_body = crate::codegen::html::generate_ejs(
        home_view_ir.render_body.as_ref().unwrap(),
        &home_view_ir.client_states,
    );

    let mut ejs_template = ejs_body.clone();
    if !home_view_ir.client_states.is_empty() {
        let state_fields: Vec<String> = home_view_ir
            .client_states
            .iter()
            .map(|s| {
                format!(
                    "{}: {}",
                    s.name,
                    crate::codegen::html::compile_expression_to_js(&s.initial_value)
                )
            })
            .collect();
        ejs_template = format!(
            "<div x-data=\"{{ {} }}\">\n{}\n</div>",
            state_fields.join(", "),
            ejs_template
        );
    }

    // Check AlpineJS x-data wrapper around the view's template
    assert!(ejs_template.contains("<div x-data=\"{ count: 0 }\">"));

    // Check component inlined body is present inside the EJS template
    assert!(ejs_template.contains("class=\"card\""));
    assert!(ejs_template.contains("My Title"));
    assert!(ejs_template.contains("My Body"));

    // Check slot replacement with child is present inside the EJS template
    assert!(ejs_template.contains("slot-container"));

    // Check AlpineJS click event binding
    assert!(ejs_template.contains("x-on:click=\"(count = (count + 1))\""));

    // Check AlpineJS x-text client state interpolation
    assert!(ejs_template.contains("x-text=\"`Click me: ${count}`\""));
}

#[test]
fn test_levenshtein_suggestions() {
    let fields = vec![ModelField {
        name: "email".to_string(),
        data_type: DataType::Str,
        is_primary_key: false,
        is_unique: true,
        is_required: false,
        min_value: None,
        max_value: None,
        default_value: None,
        foreign_key: None,
        on_delete: None,
    }];
    let models = vec![ModelDecl {
        name: "User".to_string(),
        fields,
        permissions: vec![],
    }];

    let mut analyzer = SemanticAnalyzer::new(&models, "User", &[], &[]);
    analyzer.declare_symbol("count", DataType::Int);

    let expr_var = Expression::Identifier("cont".to_string());
    let res_var = analyzer.check_expression_type(&expr_var);
    assert!(res_var.is_err());
    let err_var = res_var.unwrap_err();
    assert!(err_var.contains("Undefined variable: 'cont'"));
    assert!(err_var.contains("Did you mean 'count'?"));

    let expr_member = Expression::MemberAccess {
        object: Box::new(Expression::Identifier("User".to_string())),
        property: "current".to_string(),
    };
    let expr_field = Expression::MemberAccess {
        object: Box::new(expr_member),
        property: "emil".to_string(),
    };
    let res_field = analyzer.check_expression_type(&expr_field);
    assert!(res_field.is_err());
    let err_field = res_field.unwrap_err();
    assert!(err_field.contains("Property 'emil' does not exist in model 'User'"));
    assert!(err_field.contains("Did you mean 'email'?"));
}

#[test]
fn test_env_variables() {
    let models = vec![];
    let analyzer = SemanticAnalyzer::new(&models, "User", &[], &[]);

    let expr_env1 = Expression::Call {
        callee: Box::new(Expression::Identifier("env".to_string())),
        args: vec![Expression::StringLiteral("DB_PATH".to_string())],
    };
    let res_env1 = analyzer.check_expression_type(&expr_env1);
    assert!(res_env1.is_ok());
    assert_eq!(res_env1.unwrap(), DataType::Str);

    let js_code1 = crate::codegen::html::compile_expression_to_js(&expr_env1);
    assert_eq!(js_code1, "(process.env[\"DB_PATH\"] || \"\")");

    let expr_env2 = Expression::Call {
        callee: Box::new(Expression::Identifier("env".to_string())),
        args: vec![
            Expression::StringLiteral("PORT".to_string()),
            Expression::StringLiteral("3000".to_string()),
        ],
    };
    let res_env2 = analyzer.check_expression_type(&expr_env2);
    assert!(res_env2.is_ok());
    assert_eq!(res_env2.unwrap(), DataType::Str);

    let js_code2 = crate::codegen::html::compile_expression_to_js(&expr_env2);
    assert_eq!(js_code2, "(process.env[\"PORT\"] || \"3000\")");

    let expr_invalid = Expression::Call {
        callee: Box::new(Expression::Identifier("env".to_string())),
        args: vec![Expression::Number(8080.0)],
    };
    let res_invalid = analyzer.check_expression_type(&expr_invalid);
    assert!(res_invalid.is_err());
    assert!(
        res_invalid
            .unwrap_err()
            .contains("Function 'env' arguments must be string")
    );
}

#[test]
fn test_type_coercion() {
    let models = vec![];
    let mut analyzer = SemanticAnalyzer::new(&models, "User", &[], &[]);
    analyzer.declare_symbol("age", DataType::Int);
    analyzer.declare_symbol("name", DataType::Str);

    let expr_concat = Expression::Binary {
        left: Box::new(Expression::Identifier("name".to_string())),
        op: "+".to_string(),
        right: Box::new(Expression::Identifier("age".to_string())),
    };
    let res_concat = analyzer.check_expression_type(&expr_concat);
    assert!(res_concat.is_ok());
    assert_eq!(res_concat.unwrap(), DataType::Str);

    let expr_assign = Expression::Binary {
        left: Box::new(Expression::Identifier("name".to_string())),
        op: "=".to_string(),
        right: Box::new(Expression::Identifier("age".to_string())),
    };
    let res_assign = analyzer.check_expression_type(&expr_assign);
    assert!(res_assign.is_ok());
    assert_eq!(res_assign.unwrap(), DataType::Str);

    analyzer.declare_symbol("salary", DataType::Money);
    analyzer.declare_symbol("bonus", DataType::Int);
    analyzer.declare_symbol("cond", DataType::Bool);

    let expr_ternary = Expression::Ternary {
        cond: Box::new(Expression::Identifier("cond".to_string())),
        then_branch: Box::new(Expression::Identifier("salary".to_string())),
        else_branch: Box::new(Expression::Identifier("bonus".to_string())),
    };
    let res_ternary = analyzer.check_expression_type(&expr_ternary);
    assert!(res_ternary.is_ok());
    assert_eq!(res_ternary.unwrap(), DataType::Money);
}

#[test]
fn test_crud_form_actions() {
    let source = r#"
model Todo:
    title: str
    completed: bool

route / -> view ManageTodos

view ManageTodos:
    render:
        div:
            form [id, title, completed]:
                connect Todo.update
                redirect success -> /
            form [id]:
                connect Todo.delete
                redirect success -> /
"#;
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    let model = match &ast[0] {
        AmanaNode::Model(m) => m,
        _ => panic!("Expected Model node"),
    };

    let view = match &ast[2] {
        AmanaNode::View(v) => v,
        _ => panic!("Expected View node"),
    };

    let mut analyzer = SemanticAnalyzer::new(std::slice::from_ref(model), "User", &[], &[]);
    assert!(analyzer.validate_view(view).is_ok());

    let app_config = AppConfig {
        name: "TestApp".to_string(),
        title: "Test".to_string(),
        db_path: "test.db".to_string(),
        auth_model: "User".to_string(),
        capabilities: vec![],
    };

    let ir = crate::semantic::ir_gen::generate_ir(&analyzer, &ast, &app_config).unwrap();
    let route_ir = ir
        .routes
        .iter()
        .find(|r| r.view_name == "ManageTodos")
        .unwrap();

    assert_eq!(route_ir.form_actions.len(), 2);

    let update_action = route_ir
        .form_actions
        .iter()
        .find(|a| a.action == "update")
        .unwrap();
    assert_eq!(update_action.model_name, "Todo");
    assert_eq!(update_action.fields, vec!["id", "title", "completed"]);
    assert_eq!(update_action.redirect_success, "/");

    let delete_action = route_ir
        .form_actions
        .iter()
        .find(|a| a.action == "delete")
        .unwrap();
    assert_eq!(delete_action.model_name, "Todo");
    assert_eq!(delete_action.fields, vec!["id"]);
    assert_eq!(delete_action.redirect_success, "/");
}

#[test]
fn test_unknown_form_action_is_rejected() {
    let model = ModelDecl {
        name: "Todo".to_string(),
        fields: vec![ModelField {
            name: "title".to_string(),
            data_type: DataType::Str,
            is_primary_key: false,
            is_unique: false,
            is_required: false,
            min_value: None,
            max_value: None,
            default_value: None,
            foreign_key: None,
            on_delete: None,
        }],
        permissions: vec![],
    };

    let view = ViewDecl {
        name: "BadForm".to_string(),
        protected: None,
        server_fetches: vec![],
        client_states: vec![],
        render_body: Some(ViewElement::FormBlock {
            fields: vec!["title".to_string()],
            connect_action: "Todo.publish".to_string(),
            redirect_success: "/".to_string(),
            defaults: vec![],
            constraints: vec![],
            ui: None,
            submit_label: None,
            field_options: vec![],
        }),
        styles: None,
        canvas: None,
    };

    let mut analyzer = SemanticAnalyzer::new(&[model], "User", &[], &[]);
    let result = analyzer.validate_view(&view);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unsupported form action"));
}

#[test]
fn test_missing_route_view_is_rejected() {
    let nodes = vec![AmanaNode::Route(RouteDecl {
        path: "/missing".to_string(),
        view_name: "MissingView".to_string(),
        guards: vec![],
        fetches: vec![],
    })];
    let app_config = AppConfig {
        name: "TestApp".to_string(),
        title: "Test".to_string(),
        db_path: "test.db".to_string(),
        auth_model: "User".to_string(),
        capabilities: vec![],
    };
    let analyzer = SemanticAnalyzer::new(&[], "User", &[], &[]);
    let result = crate::semantic::ir_gen::generate_ir(&analyzer, &nodes, &app_config);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("references missing view"));
}

#[test]
fn test_advanced_css_dsl_tokens() {
    let source = r#"
component Panel:
    style:
        .panel:
            layout: grid
            columns: 3
            gap: lg
            width: 100%
            color: text
            background: glass
            size: xl
    render:
        div.panel:
            slot:
"#;
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    let comp = match &ast[0] {
        AmanaNode::Component(c) => c,
        _ => panic!("Expected Component node"),
    };
    let styles = comp.styles.as_ref().unwrap();
    assert!(styles.contains("display: grid;"));
    assert!(styles.contains("grid-template-columns: repeat(3, minmax(0, 1fr));"));
    assert!(styles.contains("gap: var(--space-lg);"));
    assert!(styles.contains("width: 100%;"));
    assert!(styles.contains("color: var(--text-primary);"));
    assert!(styles.contains("backdrop-filter: var(--glass-blur);"));
    assert!(styles.contains("font-size: var(--text-xl);"));
}

#[test]
fn test_custom_design_tokens_reach_runtime_and_preview_guards() {
    let source = r##"
app CustomDesign:
    title: "Custom Design"
    auth_model: User

theme:
    mode: day
    primary: "#ff3d8b"
    accent: "#1f8fff"
    canvas: "#fff7fb"
    text: "#25111f"
    gradient_hero: "linear-gradient(135deg, #fff7fb, #ffe4f0)"

model User:
    email: email unique
    password: password

route / -> view Home

view Home:
    canvas:
        composition: asymmetric
        width: full

    render:
        section.hero:
            compose:
                layout: split-diagonal
            Container():
                Button(label: "Custom CTA"):
                    component:
                        variant: custom
                        shape: squircle
                    visual:
                        background: "#ff3d8b"
                        text: "#ffffff"
                        border: "#ffc4dd"
                        radius: "26px"
                    states:
                        hover:
                            bg: "#25111f"
                            text: "#ffffff"
"##;

    let ir = compile_ir("custom-design.amana", source, false).unwrap();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let out_dir = env::temp_dir().join(format!(
        "amana_custom_design_{}_{}",
        std::process::id(),
        unique
    ));
    let out_dir_str = out_dir.to_string_lossy().to_string();
    generate_project(&out_dir_str, &ir).unwrap();

    let engine_js = fs::read_to_string(out_dir.join("runtime/engine.js")).unwrap();
    assert!(engine_js.contains("function safeCssLiteral"));
    assert!(engine_js.contains("themeColor(settings.primary"));
    assert!(engine_js.contains("dg-component-variant-custom"));
    assert!(engine_js.contains("--custom-bg"));
    assert!(engine_js.contains("--state-hover-bg"));
    assert!(engine_js.contains("grid-column: 1 / -1"));
    assert!(engine_js.contains("@media (max-width: 1200px)"));

    let ir_json = fs::read_to_string(out_dir.join("amana_ir.json")).unwrap();
    assert!(ir_json.contains("#ff3d8b"));
    assert!(ir_json.contains("hover.bg"));

    let _ = fs::remove_dir_all(out_dir);
}

#[test]
fn test_theme_direction_reaches_generated_html_and_css_runtime() {
    let source = r#"
app DirectionApp:
    title: "Direction"
    auth_model: User

theme:
    mode: day
    direction: ltr
    language: en
    primary: emerald
    accent: rose

model User:
    email: email unique
    password: password

route / -> view Home

view Home:
    render:
        div.page:
            Navbar(brand: "Direction")
"#;

    let ir = compile_ir("direction.amana", source, false).unwrap();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let out_dir = env::temp_dir().join(format!(
        "amana_direction_runtime_{}_{}",
        std::process::id(),
        unique
    ));
    let out_dir_str = out_dir.to_string_lossy().to_string();
    generate_project(&out_dir_str, &ir).unwrap();

    let engine_js = fs::read_to_string(out_dir.join("runtime/engine.js")).unwrap();
    let home_ejs = fs::read_to_string(out_dir.join("views/home.ejs")).unwrap();
    assert!(engine_js.contains("function themeDirection"));
    assert!(engine_js.contains("--amana-direction: ${direction};"));
    assert!(home_ejs.contains("<html lang=\"en\" dir=\"ltr\">"));
    assert!(home_ejs.contains("--amana-direction: ltr;"));
    assert!(home_ejs.contains("html { direction: ltr; }"));

    let _ = fs::remove_dir_all(out_dir);
}

#[test]
fn test_reserved_loop_variable_is_aliased_in_generated_ejs() {
    let source = r#"
app ReservedLoop:
    title: "Reserved Loop"
    auth_model: User

model User:
    email: email unique
    password: password

model CaseStudy:
    title: str required

route / -> view Home

view Home:
    server:
        fetch cases = CaseStudy.all()

    render:
        Grid(min: "16rem"):
            for case in cases:
                Card(title: case.title)
"#;

    let ir = compile_ir("reserved-loop.amana", source, false).unwrap();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let out_dir = env::temp_dir().join(format!(
        "amana_reserved_loop_{}_{}",
        std::process::id(),
        unique
    ));
    let out_dir_str = out_dir.to_string_lossy().to_string();
    generate_project(&out_dir_str, &ir).unwrap();

    let home_ejs = fs::read_to_string(out_dir.join("views/home.ejs")).unwrap();
    assert!(!home_ejs.contains("for (let case of"));
    assert!(home_ejs.contains("for (let __amana_case of cases)"));
    assert!(home_ejs.contains("<%= __amana_case.title %>"));

    let _ = fs::remove_dir_all(out_dir);
}

#[test]
fn test_safe_queries_apply_default_and_explicit_pagination() {
    let source = r#"
app QueryApp:
    title: "Query"
    auth_model: User

model User:
    email: email unique
    password: password

model Project:
    name: str
    status: str

route / -> view Home

view Home:
    server:
        fetch projects = Project.all(limit: 20, page: 2)
        fetch active = Project.filter(status: "active", offset: 40)
    render:
        div:
            p: "Projects"
"#;

    let ir = compile_ir("query-pagination.amana", source, false).unwrap();
    let project = ir
        .models
        .iter()
        .find(|model| model.name == "Project")
        .unwrap();

    let (all_sql, all_params) = generate_safe_query(
        &ir.models,
        &project.name,
        "all",
        &[
            (Some("limit".to_string()), Expression::Number(20.0)),
            (Some("page".to_string()), Expression::Number(2.0)),
        ],
    )
    .unwrap();
    assert_eq!(
        all_sql,
        "SELECT * FROM \"project\" LIMIT ? OFFSET ((? - 1) * ?)"
    );
    assert_eq!(all_params.len(), 3);

    let (filter_sql, filter_params) = generate_safe_query(
        &ir.models,
        &project.name,
        "filter",
        &[
            (
                Some("status".to_string()),
                Expression::StringLiteral("active".to_string()),
            ),
            (Some("offset".to_string()), Expression::Number(40.0)),
        ],
    )
    .unwrap();
    assert_eq!(
        filter_sql,
        "SELECT * FROM \"project\" WHERE \"status\" = ? LIMIT 100 OFFSET ?"
    );
    assert_eq!(filter_params.len(), 2);
}

#[test]
fn test_route_params_fetches_and_runtime_path_conversion() {
    let source = r#"
app ParamsApp:
    title: "Params"
    auth_model: User

model User:
    email: email unique
    password: password

model Project:
    name: str

route /projects/[id] -> view ProjectPage

view ProjectPage:
    server:
        fetch project = Project.find(params.id)
    render:
        div:
            h1: project.name
"#;

    let ir = compile_ir("params.amana", source, false).unwrap();
    assert_eq!(ir.routes[0].path, "/projects/[id]");
    assert!(matches!(
        &ir.routes[0].fetches[0].query_args[0].1,
        Expression::MemberAccess { .. }
    ));

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let out_dir = env::temp_dir().join(format!(
        "amana_params_runtime_{}_{}",
        std::process::id(),
        unique
    ));
    let out_dir_str = out_dir.to_string_lossy().to_string();
    generate_project(&out_dir_str, &ir).unwrap();

    let engine_js = fs::read_to_string(out_dir.join("runtime/engine.js")).unwrap();
    assert!(engine_js.contains("function expressRoutePath"));
    assert!(engine_js.contains("router.get(expressRoutePath(r.path)"));
    assert!(engine_js.contains("if (id === 'params') return req.params || {};"));

    let _ = fs::remove_dir_all(out_dir);
}

#[test]
fn test_generated_runtime_production_hardening_hooks_are_present() {
    let source = r#"
app HardenedApp:
    title: "Hardened"
    auth_model: User
    capabilities:
        - auth
        - api.rest

model User:
    email: email unique
    password: password

model Project:
    name: str

seed Project:
    row:
        name: "Seeded"

route / -> view Home

view Home:
    render:
        div:
            p: "Home"
"#;

    let ir = compile_ir("hardened.amana", source, false).unwrap();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let out_dir = env::temp_dir().join(format!(
        "amana_hardened_runtime_{}_{}",
        std::process::id(),
        unique
    ));
    let out_dir_str = out_dir.to_string_lossy().to_string();
    generate_project(&out_dir_str, &ir).unwrap();

    let security_js = fs::read_to_string(out_dir.join("middleware/security.js")).unwrap();
    assert!(security_js.contains("const authLimiter"));
    assert!(security_js.contains("const apiLimiter"));
    assert!(security_js.contains("standardHeaders: true"));

    let engine_js = fs::read_to_string(out_dir.join("runtime/engine.js")).unwrap();
    assert!(engine_js.contains("AMANA_RUN_SEEDS"));
    assert!(engine_js.contains("shouldRunSeeds()"));
    assert!(engine_js.contains("AMANA_ALLOW_PUBLIC_REST"));
    assert!(engine_js.contains("router.use('/api', apiLimiter);"));
    assert!(engine_js.contains("AMANA_FORCE_HTTPS"));
    assert!(engine_js.contains("hsts: isProduction"));
    assert!(engine_js.contains("SESSION_SECRET must be at least 32 characters"));
    assert!(engine_js.contains("routeErrorResponse"));

    let _ = fs::remove_dir_all(out_dir);
}

#[test]
fn test_theme_google_font_names_are_configurable() {
    let source = r#"
app FontApp:
    title: "Fonts"
    auth_model: User

theme:
    font_provider: google
    font_family: "Noto Sans Arabic"
    heading_font_family: "Space Grotesk"
    arabic_font_family: "Tajawal"

model User:
    email: email unique
    password: password

route / -> view Home

view Home:
    render:
        div:
            h1: "Fonts"
"#;

    let ir = compile_ir("fonts.amana", source, false).unwrap();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let out_dir = env::temp_dir().join(format!(
        "amana_fonts_runtime_{}_{}",
        std::process::id(),
        unique
    ));
    let out_dir_str = out_dir.to_string_lossy().to_string();
    generate_project(&out_dir_str, &ir).unwrap();

    let home_ejs = fs::read_to_string(out_dir.join("views/home.ejs")).unwrap();
    assert!(home_ejs.contains("family=Space+Grotesk"));
    assert!(home_ejs.contains("family=Noto+Sans+Arabic"));
    assert!(home_ejs.contains("'Noto Sans Arabic'"));
    assert!(home_ejs.contains("'Space Grotesk'"));

    let engine_js = fs::read_to_string(out_dir.join("runtime/engine.js")).unwrap();
    assert!(engine_js.contains("safeFontName"));
    assert!(engine_js.contains("heading_font_family"));

    let _ = fs::remove_dir_all(out_dir);
}

#[test]
fn test_theme_css_render_forms_and_seeds_v2_reach_node_runtime() {
    if Command::new("node").arg("--version").status().is_err() {
        eprintln!("Skipping Node.js render v2 smoke because node is not available.");
        return;
    }

    let source = r#"
app VibeApp:
    title: "Vibe App"
    auth_model: User
    db_path: "vibe.db"
    capabilities:
        - auth
        - api.rest

theme:
    mode: dark
    primary: indigo
    accent: cyan
    radius: soft
    surface: glass
    density: comfortable

model User:
    name: str required min 2 max 80
    email: email unique required
    password: password required min 8

model PricingPlan:
    name: str required min 2 max 80
    price: money required min 0
    description: str required

seed PricingPlan:
    row:
        name: "Starter"
        price: 0
        description: "For launch"
    row:
        name: "Pro"
        price: 29
        description: "For teams"

route / -> view Home
route /signup -> view Signup

view Home:
    render:
        Container():
            Navbar(brand: "Amana")
            Hero(title: "Build complete apps", subtitle: "Theme, responsive layout, forms, and seeds."):
                Button(label: "Start", href: "/signup")
            Grid(min: "18rem"):
                FeatureCard(title: "CSS DSL", description: "Short design tokens.")
                PricingCard(title: "Starter", price: "$0", description: "Ready seed data.")
            Footer()
    style:
        .card:
            surface: elevated
            radius: xl
            shadow: floating
            glow: primary
            gradient: hero
            border: subtle
            columns: responsive 18rem
            hover: lift

view Signup:
    render:
        Section(title: "Create account"):
            form [name, email, password]:
                connect User.register
                ui: card
                submit: "Create account"
                redirect success -> /
                field name:
                    label: "Name"
                    placeholder: "Your name"
                    required: true
                field email:
                    label: "Email"
                    placeholder: "you@example.com"
                    type: email
                field password:
                    label: "Password"
                    type: password
                    help: "Use at least 8 characters."
"#;

    let ir = compile_ir("vibe.amana", source, false).unwrap();
    let theme = ir.theme.as_ref().expect("theme should reach IR");
    assert!(
        theme
            .settings
            .iter()
            .any(|(key, value)| key == "surface" && value == "glass")
    );
    assert_eq!(ir.seeds.len(), 1);
    assert_eq!(ir.seeds[0].rows.len(), 2);

    let pricing = ir
        .models
        .iter()
        .find(|model| model.name == "PricingPlan")
        .unwrap();
    assert!(
        pricing
            .fields
            .iter()
            .any(|field| field.name == "name" && field.is_required)
    );
    assert!(
        pricing
            .fields
            .iter()
            .any(|field| field.name == "price" && field.min_value == Some(0.0))
    );
    let user_model = ir.models.iter().find(|model| model.name == "User").unwrap();
    let user_ddl = crate::codegen::sql::generate_table_ddl(user_model);
    assert!(user_ddl.contains("length(\"name\") >= 2"));
    assert!(user_ddl.contains("length(\"password\") >= 8"));
    let pricing_ddl = crate::codegen::sql::generate_table_ddl(pricing);
    assert!(pricing_ddl.contains("\"price\" >= 0"));

    let home = ir.views.iter().find(|view| view.name == "Home").unwrap();
    let styles = home.styles.as_ref().unwrap();
    assert!(styles.contains("background: var(--surface-elevated);"));
    assert!(styles.contains("border-radius: var(--radius-xl);"));
    assert!(styles.contains("box-shadow: var(--shadow-floating);"));
    assert!(styles.contains("background: var(--gradient-hero);"));
    assert!(styles.contains("grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr));"));
    assert!(styles.contains(".card:hover"));

    let signup = ir.views.iter().find(|view| view.name == "Signup").unwrap();
    let signup_json = serde_json::to_string(&signup.render_body).unwrap();
    assert!(signup_json.contains("submit_label"));
    assert!(signup_json.contains("Create account"));
    assert!(signup_json.contains("field_options"));

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let out_dir = env::temp_dir().join(format!("amana_v2_{}_{}", std::process::id(), unique));
    let out_dir_str = out_dir.to_string_lossy().to_string();
    generate_project(&out_dir_str, &ir).unwrap();

    let engine_js = fs::read_to_string(out_dir.join("runtime/engine.js")).unwrap();
    assert!(engine_js.contains("function themeCss"));
    assert!(engine_js.contains("function renderStandardComponent"));
    assert!(engine_js.contains("async seedData"));
    assert!(engine_js.contains("validateRuntimeFieldValue"));
    assert!(engine_js.contains("amana-hero"));
    assert!(engine_js.contains("amana-form-card"));
    assert!(engine_js.contains("Submit"));

    let ir_json = fs::read_to_string(out_dir.join("amana_ir.json")).unwrap();
    assert!(ir_json.contains("\"theme\""));
    assert!(ir_json.contains("\"seeds\""));

    let status = Command::new("node")
        .arg("--check")
        .arg(out_dir.join("runtime/engine.js"))
        .status()
        .unwrap();
    assert!(status.success(), "node --check failed for render v2 engine");

    let _ = fs::remove_dir_all(out_dir);
}

#[test]
fn test_frontend_design_grammar_reaches_ir_and_runtime() {
    if Command::new("node").arg("--version").status().is_err() {
        eprintln!("Skipping frontend design grammar smoke because node is not available.");
        return;
    }

    let source = r#"
app GrammarApp:
    title: "Grammar"
    auth_model: User

model User:
    email: email unique
    password: password

route / -> view Home

view Home:
    canvas:
        composition: asymmetric
        rhythm: dramatic
        width: wide
        responsive:
            mobile: stacked

    render:
        Container():
            Hero(title: "AI composes the page", subtitle: "Amana provides grammar, not templates."):
                compose:
                    layout: split-diagonal
                    focal: product-preview
                    balance: text-heavy
                visual:
                    surface: glass layered
                    gradient: mesh cyan indigo
                    border: glow subtle
                    shape: diagonal-cut
                    depth: 4
                type:
                    scale: dramatic
                    align: start
                    contrast: high
                motion:
                    entrance: stagger-up
                    hover: lift-glow
                    speed: 620ms
                creative:
                    freedom: high
                    uniqueness: strong
                    signature: "asymmetric grammar hero"
                Button(label: "Explore", href: "/")
"#;

    let ir = compile_ir("grammar.amana", source, false).unwrap();
    let home = ir.views.iter().find(|view| view.name == "Home").unwrap();
    let canvas = home.canvas.as_ref().expect("canvas should reach IR");
    assert_eq!(canvas.kind, "canvas");
    assert!(
        canvas
            .settings
            .iter()
            .any(|(key, value)| key == "responsive.mobile" && value == "stacked")
    );

    let body_json = serde_json::to_string(&home.render_body).unwrap();
    assert!(body_json.contains("\"kind\":\"compose\""));
    assert!(body_json.contains("split-diagonal"));
    assert!(body_json.contains("\"kind\":\"creative\""));
    assert!(body_json.contains("asymmetric grammar hero"));

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let out_dir = env::temp_dir().join(format!("amana_design_{}_{}", std::process::id(), unique));
    let out_dir_str = out_dir.to_string_lossy().to_string();
    generate_project(&out_dir_str, &ir).unwrap();

    let engine_js = fs::read_to_string(out_dir.join("runtime/engine.js")).unwrap();
    assert!(engine_js.contains("function canvasAttributes"));
    assert!(engine_js.contains("dg-layout-split-diagonal"));
    assert!(engine_js.contains("dg-gradient-mesh-cyan-indigo"));
    assert!(engine_js.contains("data-ai-signature"));
    assert!(engine_js.contains("<body${bodyAttrs}>"));

    let status = Command::new("node")
        .arg("--check")
        .arg(out_dir.join("runtime/engine.js"))
        .status()
        .unwrap();
    assert!(
        status.success(),
        "node --check failed for design grammar engine"
    );

    let _ = fs::remove_dir_all(out_dir);
}

#[test]
fn test_frontend_design_grammar_v2_controls_reach_runtime() {
    if Command::new("node").arg("--version").status().is_err() {
        eprintln!("Skipping frontend design grammar v2 smoke because node is not available.");
        return;
    }

    let source = r#"
app GrammarV2App:
    title: "Grammar V2"
    auth_model: User

model User:
    email: email unique
    password: password

route / -> view Home

view Home:
    canvas:
        composition: bento
        rhythm: spacious
        width: wide
        flow: immersive
        content_width: 1280px
        responsive:
            mobile: stacked

    render:
        Section(title: "Controlled creativity", subtitle: "The AI chooses direction, Amana keeps it structured."):
            compose:
                layout: bento
                focus_path: radial
                density: spacious
                columns: 3
            visual:
                palette: neon lab
                texture: noise
                frame: browser
                texture_opacity: 0.08
            brand:
                voice: premium
                personality: precise
                colorway: cyber cyan
                trust: high
            art:
                direction: technical-blueprint
                motif: orbit
                lighting: rim
                texture: noise
            responsive:
                mobile: stacked
                collapse: stack
                columns: 2
            interaction:
                feedback: tactile
                affordance: obvious
                focus_strength: 4px
            a11y:
                contrast: enhanced
                focus: strong
                reduce_motion: auto
            creative:
                freedom: high
                uniqueness: strong
                avoid_repetition: true
                reference: "premium technical dashboard"
                signature: "technical neon bento dashboard"
            Kpi(label: "Latency", value: "24ms", trend: "-18%")
            FeatureCard(title: "Composable visual direction", description: "No template lock-in.")
            Button(label: "Inspect", href: "/", size: "lg", icon: "->")
"#;

    let ir = compile_ir("grammar-v2.amana", source, false).unwrap();
    let home = ir.views.iter().find(|view| view.name == "Home").unwrap();
    let body_json = serde_json::to_string(&home.render_body).unwrap();
    assert!(body_json.contains("\"kind\":\"brand\""));
    assert!(body_json.contains("\"kind\":\"art\""));
    assert!(body_json.contains("\"kind\":\"responsive\""));
    assert!(body_json.contains("technical neon bento dashboard"));
    assert!(body_json.contains("Kpi"));

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let out_dir =
        env::temp_dir().join(format!("amana_design_v2_{}_{}", std::process::id(), unique));
    let out_dir_str = out_dir.to_string_lossy().to_string();
    generate_project(&out_dir_str, &ir).unwrap();

    let engine_js = fs::read_to_string(out_dir.join("runtime/engine.js")).unwrap();
    assert!(engine_js.contains("dg-layout-bento"));
    assert!(engine_js.contains("dg-art-technical-blueprint"));
    assert!(engine_js.contains("dg-colorway-cyber-cyan"));
    assert!(engine_js.contains("data-ai-brand-voice"));
    assert!(engine_js.contains("data-ai-art-direction"));
    assert!(engine_js.contains("amana-kpi"));

    let status = Command::new("node")
        .arg("--check")
        .arg(out_dir.join("runtime/engine.js"))
        .status()
        .unwrap();
    assert!(
        status.success(),
        "node --check failed for design grammar v2 engine"
    );

    let _ = fs::remove_dir_all(out_dir);
}

#[test]
fn test_inspect_design_report_scores_ai_controls() {
    let source = r#"
app ReportApp:
    title: "Report"
    auth_model: User

model User:
    email: email unique
    password: password

route / -> view Home

view Home:
    canvas:
        composition: bento
        rhythm: spacious
        responsive:
            mobile: stacked

    render:
        Hero(title: "Report"):
            compose:
                layout: bento
            visual:
                surface: glass layered
            brand:
                voice: premium
            art:
                direction: cinematic-product
            responsive:
                mobile: stacked
            creative:
                freedom: high
                uniqueness: strong
                signature: "report hero"
            Button(label: "Go", href: "/")
            Kpi(label: "Score", value: "92")
"#;

    let ir = compile_ir("report.amana", source, false).unwrap();
    let report = inspect_design("report.amana", &ir);
    assert_eq!(report.stage, "inspect-design");
    assert!(report.score >= 70);
    assert!(report.views[0].design_blocks.contains(&"brand".to_string()));
    assert!(report.views[0].design_blocks.contains(&"art".to_string()));
    assert!(
        report.views[0]
            .ai_controls
            .iter()
            .any(|control| control.contains("creative.signature=report hero"))
    );
}

#[test]
fn test_invalid_seed_is_rejected_during_check() {
    let source = r#"
app SeedApp:
    auth_model: User

model User:
    email: email unique required
    password: password required min 8

model PricingPlan:
    name: str required
    price: money required min 0

seed PricingPlan:
    name: "Starter"
    unknown: "bad"
"#;
    let err = compile_ir("bad-seed.amana", source, false).unwrap_err();
    assert_eq!(err.stage, "semantic");
    assert!(err.message.contains("Seed field 'unknown' does not exist"));
}

#[test]
fn test_cli_subcommands_parse() {
    let args = vec![
        "amana".to_string(),
        "check".to_string(),
        "app.amana".to_string(),
    ];
    assert_eq!(
        parse_cli_args(&args).unwrap(),
        CliCommand::Check {
            source_file: "app.amana".to_string(),
            json: false,
            ir_snapshot: None,
        }
    );

    let args = vec![
        "amana".to_string(),
        "build".to_string(),
        "app.amana".to_string(),
        "dist_app".to_string(),
    ];
    assert_eq!(
        parse_cli_args(&args).unwrap(),
        CliCommand::Build {
            source_file: "app.amana".to_string(),
            output_dir: "dist_app".to_string(),
            json: false,
            ir_snapshot: None,
        }
    );

    let args = vec![
        "amana".to_string(),
        "dev".to_string(),
        "app.amana".to_string(),
        "dist_app".to_string(),
        "--no-install".to_string(),
    ];
    assert_eq!(
        parse_cli_args(&args).unwrap(),
        CliCommand::Dev {
            source_file: "app.amana".to_string(),
            output_dir: "dist_app".to_string(),
            install: false,
            watch: true,
        }
    );

    let args = vec![
        "amana".to_string(),
        "dev".to_string(),
        "app.amana".to_string(),
        "dist_app".to_string(),
        "--no-watch".to_string(),
    ];
    assert_eq!(
        parse_cli_args(&args).unwrap(),
        CliCommand::Dev {
            source_file: "app.amana".to_string(),
            output_dir: "dist_app".to_string(),
            install: true,
            watch: false,
        }
    );

    let args = vec![
        "amana".to_string(),
        "check".to_string(),
        "app.amana".to_string(),
        "--json".to_string(),
    ];
    assert_eq!(
        parse_cli_args(&args).unwrap(),
        CliCommand::Check {
            source_file: "app.amana".to_string(),
            json: true,
            ir_snapshot: None,
        }
    );

    let args = vec![
        "amana".to_string(),
        "fmt".to_string(),
        "app.amana".to_string(),
        "--check".to_string(),
    ];
    assert_eq!(
        parse_cli_args(&args).unwrap(),
        CliCommand::Fmt {
            source_file: "app.amana".to_string(),
            check: true,
            json: false,
            all_graph: false,
        }
    );

    let args = vec![
        "amana".to_string(),
        "fmt".to_string(),
        "app.amana".to_string(),
        "--all".to_string(),
        "--json".to_string(),
    ];
    assert_eq!(
        parse_cli_args(&args).unwrap(),
        CliCommand::Fmt {
            source_file: "app.amana".to_string(),
            check: false,
            json: true,
            all_graph: true,
        }
    );

    let args = vec![
        "amana".to_string(),
        "inspect-design".to_string(),
        "app.amana".to_string(),
        "--json".to_string(),
    ];
    assert_eq!(
        parse_cli_args(&args).unwrap(),
        CliCommand::InspectDesign {
            source_file: "app.amana".to_string(),
            json: true,
        }
    );
}

#[test]
fn test_component_call_without_colon_parses() {
    let source = r#"
app UiApp:
    title: "UI"

component NavBar:
    render:
        nav.navbar:
            a(href: "/"): "Home"

route / -> view Home

view Home:
    render:
        div.page:
            NavBar()
"#;
    let ir = compile_ir("ui.amana", source, false).unwrap();
    let home = ir.views.iter().find(|view| view.name == "Home").unwrap();
    let body = serde_json::to_string(&home.render_body).unwrap();
    assert!(body.contains("navbar"));
}

#[test]
fn test_component_empty_colon_returns_json_ready_diagnostic() {
    let source = r#"
app UiApp:
    title: "UI"

component NavBar:
    render:
        nav.navbar:
            a(href: "/"): "Home"

route / -> view Home

view Home:
    render:
        div.page:
            NavBar():
"#;
    let err = compile_ir("bad-ui.amana", source, false).unwrap_err();
    let diagnostic = err.to_json_diagnostic();
    assert_eq!(diagnostic.stage, "parser");
    assert_eq!(
        diagnostic.message,
        "Component calls without children can be written as NavBar()."
    );
    assert_eq!(
        diagnostic.suggestion.as_deref(),
        Some("Use NavBar(): only when the component has children.")
    );
    assert!(diagnostic.line.unwrap() > 0);
    assert!(diagnostic.column.unwrap() > 0);
}

#[test]
fn test_formatter_foundation_normalizes_self_closing_components() {
    let source = "view Home:\n  render:\n    div.page:\n      NavBar():\n      Card():\n        p: \"Body\"\n";
    let formatted = crate::formatter::format_source(source);
    assert!(formatted.contains("            NavBar()\n"));
    assert!(formatted.contains("            Card():\n"));
    assert!(!formatted.contains("NavBar():"));
}

#[test]
fn test_ir_snapshot_write_and_verify() {
    let source = r#"
app SnapshotApp:
    title: "Snapshot"

model User:
    email: email unique
    password: password

route / -> view Home

view Home:
    render:
        div:
            h1: "Snapshot"
"#;
    let ir = compile_ir("snapshot.amana", source, false).unwrap();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let snapshot_path = env::temp_dir().join(format!("amana_snapshot_{}.json", unique));

    let write = IrSnapshotRequest {
        mode: IrSnapshotMode::Write,
        path: snapshot_path.clone(),
    };
    handle_ir_snapshot(&ir, Some(&write)).unwrap();
    assert!(snapshot_path.exists());

    let verify = IrSnapshotRequest {
        mode: IrSnapshotMode::Verify,
        path: snapshot_path.clone(),
    };
    assert!(handle_ir_snapshot(&ir, Some(&verify)).is_ok());

    let _ = fs::remove_file(snapshot_path);
}

#[test]
fn test_form_defaults_and_ownership_constraints_reach_ir() {
    let source = r#"
app OwnedApp:
    title: "Owned"
    auth_model: User
    db_path: "owned.db"
    capabilities:
        - auth

model User:
    name: str
    email: email unique
    password: password

model Project:
    name: str
    description: str
    owner_id: int foreign_key User(id) on_delete CASCADE

route /projects -> view Projects

view Projects:
    protected:
        allow: User.current != null
        deny: -> /login
        unauthenticated: -> /login
    server:
        fetch projects = Project.filter(owner_id: User.current.id)
    render:
        div:
            form [name, description]:
                connect Project.create
                default owner_id = User.current.id
                redirect success -> /projects
            for p in projects:
                form [id, name, description]:
                    connect Project.update
                    where owner_id = User.current.id
                    redirect success -> /projects
                form [id]:
                    connect Project.delete
                    where owner_id = User.current.id
                    redirect success -> /projects
"#;
    let ir = compile_ir("owned.amana", source, false).unwrap();
    let route = ir.routes.iter().find(|r| r.path == "/projects").unwrap();

    let create = route
        .form_actions
        .iter()
        .find(|action| action.action == "create")
        .unwrap();
    assert_eq!(create.defaults.len(), 1);
    assert_eq!(create.defaults[0].0, "owner_id");

    let update = route
        .form_actions
        .iter()
        .find(|action| action.action == "update")
        .unwrap();
    assert_eq!(update.constraints.len(), 1);
    assert_eq!(update.constraints[0].0, "owner_id");

    let delete = route
        .form_actions
        .iter()
        .find(|action| action.action == "delete")
        .unwrap();
    assert_eq!(delete.constraints.len(), 1);
    assert_eq!(delete.constraints[0].0, "owner_id");
}

#[test]
fn test_current_user_form_defaults_require_protected_view() {
    let source = r#"
app OwnedApp:
    auth_model: User

model User:
    email: email unique
    password: password

model Project:
    name: str
    owner_id: int foreign_key User(id) on_delete CASCADE

route /projects -> view Projects

view Projects:
    render:
        div:
            form [name]:
                connect Project.create
                default owner_id = User.current.id
                redirect success -> /projects
"#;
    let err = compile_ir("bad-owned.amana", source, false).unwrap_err();
    assert!(err.message.contains("must be inside a protected view"));
}

#[test]
fn test_generated_node_app_e2e_smoke() {
    if Command::new("node").arg("--version").status().is_err() {
        eprintln!("Skipping Node.js e2e smoke because node is not available.");
        return;
    }

    let source = r#"
app SmokeApp:
    title: "Smoke"
    auth_model: User
    db_path: "smoke.db"
    capabilities:
        - auth
        - api.rest

model User:
    name: str
    email: email unique
    password: password

model Project:
    name: str
    owner_id: int foreign_key User(id) on_delete CASCADE

route / -> view Home
route /projects -> view Projects

view Home:
    render:
        div:
            h1: "Smoke"

view Projects:
    protected:
        allow: User.current != null
        deny: -> /
        unauthenticated: -> /
    server:
        fetch projects = Project.filter(owner_id: User.current.id)
    render:
        div:
            form [name]:
                connect Project.create
                default owner_id = User.current.id
                redirect success -> /projects
            for p in projects:
                form [id, name]:
                    connect Project.update
                    where owner_id = User.current.id
                    redirect success -> /projects
"#;
    let ir = compile_ir("smoke.amana", source, false).unwrap();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let out_dir = env::temp_dir().join(format!("amana_e2e_{}_{}", std::process::id(), unique));
    let out_dir_str = out_dir.to_string_lossy().to_string();
    generate_project(&out_dir_str, &ir).unwrap();

    for rel_path in [
        "app.js",
        "runtime/engine.js",
        "middleware/security.js",
        "middleware/hooks-worker.js",
    ] {
        let status = Command::new("node")
            .arg("--check")
            .arg(out_dir.join(rel_path))
            .status()
            .unwrap();
        assert!(status.success(), "node --check failed for {}", rel_path);
    }

    let engine_js = fs::read_to_string(out_dir.join("runtime/engine.js")).unwrap();
    assert!(engine_js.contains("resolvedDefaults"));
    assert!(engine_js.contains("resolvedConstraints"));
    assert!(engine_js.contains("Record not found or action is not authorized."));

    let _ = fs::remove_dir_all(out_dir);
}

#[test]
fn test_all_standard_components_rendered() {
    let components = vec![
        "Button", "Card", "Container", "Section", "Grid", "Stack", "Navbar", "Hero", "FeatureCard",
        "PricingCard", "FormField", "Alert", "Footer", "Modal", "Tabs", "Badge", "Kpi", "Stat",
        "LogoCloud", "TestimonialCard", "Timeline", "TimelineItem", "EmptyState", "Split",
        "Cluster", "Sidebar", "Icon",
    ];

    for tag in components {
        let element = crate::ast::ViewElement::Element {
            tag: tag.to_string(),
            classes: vec![],
            attributes: vec![],
            children: vec![],
        };
        let rendered = crate::codegen::html::generate_ejs(&element, &[]);
        assert!(
            !rendered.starts_with(&format!("<{}", tag)),
            "Component {} failed to compile and fell back to standard tag: {}",
            tag,
            rendered
        );
    }
}

#[test]
fn test_modal_production_features_and_grid_stretch() {
    use crate::ast::{Expression, ViewElement};
    use crate::codegen::html::generate_ejs;

    // 1. Test Grid stretch options
    let grid_stretched = ViewElement::Element {
        tag: "Grid".to_string(),
        classes: vec![],
        attributes: vec![("stretch".to_string(), Expression::Boolean(true))],
        children: vec![],
    };
    let grid_rendered = generate_ejs(&grid_stretched, &[]);
    assert!(grid_rendered.contains("amana-grid-stretch"), "Stretched grid should contain amana-grid-stretch class");

    // 2. Test Modal production features (escaped titles, monotonic title IDs, focus trap, scroll lock, ARIA tags, overlay click, ESC key)
    let modal = ViewElement::Element {
        tag: "Modal".to_string(),
        classes: vec![],
        attributes: vec![
            ("open".to_string(), Expression::Identifier("modal_open".to_string())),
            ("title".to_string(), Expression::StringLiteral("My Cool Title".to_string())),
            ("closable".to_string(), Expression::Boolean(true)),
        ],
        children: vec![],
    };
    let modal_rendered = generate_ejs(&modal, &[]);
    assert!(modal_rendered.contains("role=\"dialog\""), "Modal should have dialog role");
    assert!(modal_rendered.contains("aria-modal=\"true\""), "Modal should have aria-modal");
    assert!(modal_rendered.contains("aria-labelledby=\"amana-modal-title-"), "Modal should have monotonic aria-labelledby ID");
    assert!(modal_rendered.contains("My Cool Title"), "Modal should render title");
    assert!(modal_rendered.contains("@keydown.escape.window=\"modal_open = false\""), "Modal should close on ESC");
    assert!(modal_rendered.contains("@keydown.tab="), "Modal should have keyboard focus trap");
    assert!(modal_rendered.contains("if (modal_open)"), "Modal should lock page scroll");
}

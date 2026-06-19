param(
    [ValidateSet("all", "lexer", "parser", "semantic", "codegen", "runtime", "docs", "theme", "design", "forms", "queries", "components")]
    [string]$Area = "all",
    [switch]$Inventory
)

$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")

function Invoke-Ripgrep {
    param(
        [string]$Pattern,
        [string[]]$Paths
    )

    Push-Location $Root
    try {
        rg -n --glob "!target/**" --glob "!dist/**" --glob "!test_dist/**" --glob "!amana-live-compiler/node_modules/**" $Pattern @Paths
    }
    finally {
        Pop-Location
    }
}

$Areas = [ordered]@{
    lexer = @{
        Pattern = "TokenKind|read_identifier_or_keyword|read_operator_or_symbol|read_string|read_multiline_string|read_formatted_string|read_number|Indent|Dedent|HashColor"
        Paths = @("src/lexer", "src/ast")
    }
    parser = @{
        Pattern = "parse_app|parse_theme|parse_model|parse_field|parse_seed|parse_route|parse_view|parse_component|parse_view_element|parse_expression|parse_tokens_decl|parse_variant_node|parse_design_block_body"
        Paths = @("src/parser", "src/ast")
    }
    semantic = @{
        Pattern = "validate_theme|validate_design_block|validate_view|validate_seed|validate_variant|check_expression_type|types_compatible|Missing capability|current_view_is_protected|standard library"
        Paths = @("src/semantic", "src/main.rs")
    }
    codegen = @{
        Pattern = "generate_project|generate_ejs|render_standard_component|compile_expression_to_js|generate_table_ddl|generate_safe_query|files_to_write|CodegenBackend|ExpressNodeBackend"
        Paths = @("src/codegen", "src/semantic/ir.rs")
    }
    runtime = @{
        Pattern = "compileApiRoutes|compileRouteTable|compileForms|evalAmanaExpression|runSeeds|SESSION_SECRET|AMANA_RUN_SEEDS|api.rest|time|network.outbound|auth|csrf"
        Paths = @("src/codegen/express/static_files")
    }
    theme = @{
        Pattern = "validate_theme|theme_css|theme_direction|theme_language|font_provider|font_family|arabic_font_family|gradient_|radius|density|direction"
        Paths = @("src/semantic/mod.rs", "src/codegen/express/theme.rs", "src/codegen/express/static_files/engine.rs", "src/parser/css.rs")
    }
    design = @{
        Pattern = "DesignBlock|is_design_block_name|validate_design_block|LAYOUT_VALUES|SURFACE_VALUES|HOVER_VALUES|ENTRANCE_VALUES|GRADIENT_VALUES|DENSITY_VALUES|SHADOW_VALUES|inspect_design|DesignWarning"
        Paths = @("src/parser", "src/semantic", "src/main.rs", "src/codegen/express/views.rs")
    }
    forms = @{
        Pattern = "FormBlock|FormFieldOptions|parse_fetch_stmt|parse_state_decl|connect_action|redirect_success|allowed_actions|extract_form_actions|form-submit|resolvedDefaults|resolvedConstraints"
        Paths = @("src/parser", "src/semantic", "src/codegen")
    }
    queries = @{
        Pattern = "FetchStmt|FetchIR|generate_safe_query|query_method|append_pagination_clause|limit|offset|page|filter|count|find|all"
        Paths = @("src/parser", "src/semantic", "src/codegen")
    }
    components = @{
        Pattern = "ComponentDecl|ComponentParam|render_standard_component|standard_components|SlotDecl|replace_slots|inline_components|ResourceGrid|ResourceTable|variant"
        Paths = @("src/parser", "src/semantic", "src/codegen")
    }
    docs = @{
        Pattern = "language-inventory|Amana Language|CLI|Component|Theme|Runtime|Security|TODO|examples/"
        Paths = @("doc", "README.md")
    }
}

if ($Inventory) {
    Push-Location $Root
    try {
        python scripts/language_inventory.py --write
    }
    finally {
        Pop-Location
    }
}

if ($Area -eq "all") {
    foreach ($entry in $Areas.GetEnumerator()) {
        Write-Host "`n## $($entry.Key)"
        Invoke-Ripgrep -Pattern $entry.Value.Pattern -Paths $entry.Value.Paths
    }
}
else {
    $selected = $Areas[$Area]
    Invoke-Ripgrep -Pattern $selected.Pattern -Paths $selected.Paths
}

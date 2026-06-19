#!/usr/bin/env python3
"""Generate a source-derived inventory of the Amana language surface."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "src"
DEFAULT_OUTPUT = ROOT / "doc" / "language-inventory.generated.md"


def read(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8", errors="replace")


def quoted_strings(text: str) -> list[str]:
    return re.findall(r'"([^"]+)"', text)


def unique(items: list[str]) -> list[str]:
    seen: set[str] = set()
    out: list[str] = []
    for item in items:
        if item not in seen:
            seen.add(item)
            out.append(item)
    return out


def between(text: str, start: str, end: str | None = None) -> str:
    start_idx = text.find(start)
    if start_idx < 0:
        return ""
    if end is None:
        return text[start_idx:]
    end_idx = text.find(end, start_idx)
    if end_idx < 0:
        return text[start_idx:]
    return text[start_idx:end_idx]


def brace_block(text: str, marker: str) -> str:
    idx = text.find(marker)
    if idx < 0:
        return ""
    open_idx = text.find("{", idx)
    if open_idx < 0:
        return ""
    depth = 0
    for pos in range(open_idx, len(text)):
        ch = text[pos]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return text[open_idx + 1 : pos]
    return ""


def extract_token_kind_variants(lexer: str) -> list[str]:
    body = between(lexer, "pub enum TokenKind", "pub struct Token")
    variants: list[str] = []
    for raw_line in body.splitlines():
        line = raw_line.split("//", 1)[0].strip()
        if not line or line in {"{", "}"}:
            continue
        match = re.match(r"([A-Z][A-Za-z0-9_]*)", line)
        if match:
            variants.append(match.group(1))
    return unique(variants)


def extract_lexer_words(lexer: str) -> dict[str, list[str]]:
    body = brace_block(lexer, "fn read_identifier_or_keyword")
    pairs = re.findall(r'"([^"]+)"\s*=>\s*TokenKind::([A-Za-z]+)', body)
    type_kinds = {"Str", "Int", "Float", "Bool", "Email", "Password", "DateTime", "Money"}
    literal_kinds = {"Boolean", "Null"}
    operator_kinds = {"And", "Or", "Not"}
    result = {"keywords": [], "types": [], "literals": [], "word_operators": []}
    for word, kind in pairs:
        if kind in type_kinds:
            result["types"].append(word)
        elif kind in literal_kinds:
            result["literals"].append(word)
        elif kind in operator_kinds:
            result["word_operators"].append(word)
        else:
            result["keywords"].append(word)
    return {key: unique(value) for key, value in result.items()}


def extract_theme_keys(semantic: str) -> list[str]:
    section = between(semantic, "fn validate_theme", "fn validate_design_block")
    match = re.search(r"allowed_keys\s*=\s*\[(.*?)\];", section, flags=re.S)
    return unique(quoted_strings(match.group(1))) if match else []


def extract_design_blocks(semantic: str) -> dict[str, list[str]]:
    section = between(semantic, "fn validate_design_block", "for (key, value) in &block.settings")
    blocks: dict[str, list[str]] = {}
    for match in re.finditer(r'"([A-Za-z0-9_-]+)"\s*=>\s*&\[(.*?)\],', section, flags=re.S):
        blocks[match.group(1)] = unique(quoted_strings(match.group(2)))
    return dict(sorted(blocks.items()))


def extract_closed_design_values(semantic: str) -> dict[str, list[str]]:
    values: dict[str, list[str]] = {}
    for match in re.finditer(r"const\s+([A-Z_]+):\s*&\[&str\]\s*=\s*&\[(.*?)\];", semantic, re.S):
        const_name = match.group(1)
        if const_name == "ALLOWED_PROPERTIES":
            continue
        values[const_name.lower().replace("_values", "")] = unique(
            quoted_strings(match.group(2))
        )
    return dict(sorted(values.items()))


def extract_allowed_css_properties(semantic: str) -> list[str]:
    match = re.search(r"ALLOWED_PROPERTIES:\s*&\[&str\]\s*=\s*&\[(.*?)\];", semantic, re.S)
    return unique(quoted_strings(match.group(1))) if match else []


def extract_css_token_values(css: str, fn_name: str) -> list[str]:
    body = brace_block(css, f"fn {fn_name}")
    values: list[str] = []
    for match in re.finditer(r'((?:"[^"]+"\s*(?:\|\s*)?)+)\s*=>\s*Some', body):
        values.extend(quoted_strings(match.group(1)))
    return unique(values)


def extract_standard_components(html: str) -> list[str]:
    section = between(html, "fn render_standard_component", "/// Generates standard EJS")
    components: list[str] = []
    for line in section.splitlines():
        stripped = line.strip()
        if not re.match(r'^"[^"]+"(?:\s*\|\s*"[^"]+")*\s*=>', stripped):
            continue
        components.extend(quoted_strings(stripped.split("=>", 1)[0]))
    return unique(components)


def extract_event_attributes(html: str) -> list[str]:
    match = re.search(r"let event_keys\s*=\s*\[(.*?)\];", html, re.S)
    return unique(quoted_strings(match.group(1))) if match else []


def extract_generated_files(express: str) -> list[str]:
    return unique(re.findall(r'files_to_write\.insert\(\s*"([^"]+)"', express))


def extract_package_info(package_rs: str) -> dict[str, list[str]]:
    scripts_section = between(package_rs, '"scripts": {', '},')
    deps_section = between(package_rs, '"dependencies": {', '},')
    dev_deps_section = between(package_rs, '"devDependencies": {', "}")
    key_pattern = re.compile(r'"([^"]+)"\s*:')
    return {
        "scripts": [key for key in unique(key_pattern.findall(scripts_section)) if key != "scripts"],
        "dependencies": [
            key for key in unique(key_pattern.findall(deps_section)) if key != "dependencies"
        ],
        "dev_dependencies": [
            key
            for key in unique(key_pattern.findall(dev_deps_section))
            if key != "devDependencies"
        ],
    }


def extract_cli(main: str) -> dict[str, list[str]]:
    usage = between(main, "fn print_usage", "type BuildProjectError")
    commands = re.findall(r"amana\s+([a-z-]+)", usage)
    aliases = re.findall(r'"([^"]+)"\s*\|\s*"([^"]+)"\s*=>', between(main, "fn parse_cli_args", "fn parse_inspect_design_args"))
    flat_aliases = [item for pair in aliases for item in pair]
    return {
        "commands": unique([cmd for cmd in commands if cmd not in {"<source-file.amana>"}]),
        "aliases": unique(flat_aliases),
    }


def extract_query_methods(sql: str) -> list[str]:
    section = between(sql, "match query_method", "_ =>")
    return unique(re.findall(r'"([a-z]+)"\s*=>\s*{', section))


def extract_form_actions(text: str) -> list[str]:
    actions = []
    for match in re.finditer(r"allowed_actions\s*=\s*\[(.*?)\];", text, re.S):
        actions.extend(quoted_strings(match.group(1)))
    return unique(actions)


def collect_inventory() -> dict[str, Any]:
    lexer = read("src/lexer/mod.rs")
    parser_top = read("src/parser/top_level.rs")
    parser_views = read("src/parser/views.rs")
    parser_design = read("src/parser/design.rs")
    parser_css = read("src/parser/css.rs")
    parser_styles = read("src/parser/styles.rs")
    semantic = read("src/semantic/mod.rs")
    semantic_views = read("src/semantic/views.rs")
    semantic_ir = read("src/semantic/ir.rs")
    semantic_ir_gen = read("src/semantic/ir_gen.rs")
    html = read("src/codegen/html.rs")
    sql = read("src/codegen/sql.rs")
    express = read("src/codegen/express.rs")
    engine = read("src/codegen/express/static_files/engine.rs")
    package_rs = read("src/codegen/express/static_files/package.rs")
    main = read("src/main.rs")

    top_nodes = unique(re.findall(r"AmanaNode::([A-Za-z]+)", between(parser_top, "pub fn parse", "fn parse_theme")))
    view_blocks = ["protected", "server", "client", "render", "style", "canvas"]
    component_blocks = ["render", "style", "variants"]
    form_settings = ["connect", "redirect", "default", "where", "ui", "submit", "field"]
    form_field_options = ["label", "placeholder", "type", "help", "required"]
    resource_options = ["item", "empty", "loading", "error", "filters", "sort"]
    state_persistence = ["memory", "cookie", "session", "local"]
    std_fetch_methods = {
        "time": ["now"] if "stdLib.time.now" in engine or "fetch.query_method === 'now'" in engine else [],
        "http": [method for method in ["get", "post"] if f"stdLib.http.{method}" in engine],
        "auth": [method for method in ["verify", "hash"] if f"stdLib.auth.{method}" in engine],
    }

    return {
        "source_files": [
            str(path.relative_to(ROOT)).replace("\\", "/")
            for path in sorted(SRC.rglob("*.rs"))
            if "tests.rs" not in str(path)
        ],
        "cli": extract_cli(main),
        "top_level_nodes": top_nodes,
        "imports": {
            "syntax": 'import "./relative-file.amana"',
            "implementation": "src/main.rs: strip_import_lines, resolve_source_file",
        },
        "lexer": {
            "token_kinds": extract_token_kind_variants(lexer),
            **extract_lexer_words(lexer),
            "symbols": ["+", "-", "*", "/", "==", "!=", ">", "<", ">=", "<=", "=", "?", ":", ".", ",", "->", "%", "(", ")", "[", "]"],
            "strings": ['"text"', '"""multi-line"""', 'f"Hello {name}"'],
        },
        "parser": {
            "view_blocks": view_blocks,
            "component_blocks": component_blocks,
            "design_block_names": unique(quoted_strings(brace_block(parser_design, "is_design_block_name"))),
            "form_settings": form_settings,
            "form_field_options": form_field_options,
            "resource_options": resource_options,
            "state_persistence": state_persistence,
        },
        "theme": {
            "allowed_keys": extract_theme_keys(semantic),
            "closed_values": {
                "mode": ["dark", "night", "day", "light"],
                "direction": ["ltr", "rtl"],
                "radius": ["none", "sharp", "soft", "round", "pill"],
                "density": ["compact", "comfortable", "spacious"],
                "font_provider": ["system", "google"],
            },
        },
        "design": {
            "allowed_keys_by_block": extract_design_blocks(semantic),
            "closed_values": extract_closed_design_values(semantic),
        },
        "css": {
            "allowed_properties": extract_allowed_css_properties(semantic),
            "spacing_tokens": extract_css_token_values(parser_css, "spacing_token"),
            "size_tokens": extract_css_token_values(parser_css, "size_token"),
            "color_tokens": extract_css_token_values(parser_css, "color_token"),
        },
        "semantic": {
            "query_methods": extract_query_methods(sql),
            "form_actions": extract_form_actions(semantic_views + semantic_ir_gen),
            "standard_libraries": {
                "capabilities": ["time", "network.outbound", "auth"],
                "server_fetch_methods": std_fetch_methods,
                "global_expressions": ["env", "params", "query", "body", "csrfToken"],
            },
            "variant_targets": unique(quoted_strings(between(semantic, "let standard_components = [", "if !standard_components"))),
        },
        "codegen": {
            "backend": "express-node",
            "standard_components": extract_standard_components(html),
            "event_attributes": extract_event_attributes(html),
            "generated_files": unique(["views/<view>.ejs"] + extract_generated_files(express)),
            "package": extract_package_info(package_rs),
        },
        "ir": {
            "version": {"major": 1, "minor": 0, "patch": 0},
            "capabilities": unique(quoted_strings(between(semantic_ir_gen, "capabilities: vec![", "],"))),
            "structs": unique(re.findall(r"pub struct ([A-Za-z0-9_]+)", semantic_ir)),
        },
        "feature_status": {
            "tokens": "Implemented. Top-level tokens blocks are parsed into AST, preserved in IR, and emitted into generated theme CSS.",
            "permit": "Implemented. Model permit rules are parsed into ModelDecl.permissions, preserved in IR, and enforced in generated Express REST routes, form mutations, and server fetch filtering.",
            "chart": "Implemented. Chart(data, type, x, y) has parser, AST, semantic, EJS/runtime support.",
            "ternary": "Implemented. Ternary expressions parse as condition ? then_value : else_value and flow through semantic/codegen/runtime.",
            "persist": "Implemented. memory/local/session/cookie are parsed as PersistMode and non-memory modes emit browser persistence behavior.",
            "resources": "Implemented. ResourceGrid/Table lifecycle, filters, and sort are emitted into generated EJS over server-fetched rows.",
            "variants": "Implemented. Variants are parsed, validated, preserved in IR, and emitted as target-specific generated CSS for base, hover, slot, and responsive rules.",
        },
    }


def md_list(items: list[str]) -> str:
    if not items:
        return "- None detected.\n"
    return "".join(f"- `{item}`\n" for item in items)


def md_inline(items: list[str]) -> str:
    return ", ".join(f"`{item}`" for item in items) if items else "`none`"


def render_markdown(inv: dict[str, Any]) -> str:
    lines: list[str] = []
    lines.append("# Amana Language Inventory (Generated)\n")
    lines.append("This file is generated from compiler implementation files, excluding tests. Do not edit it by hand; run `python scripts/language_inventory.py --write`.\n")

    lines.append("## Scanned Source Files\n")
    lines.append(md_list(inv["source_files"]))

    lines.append("## CLI Surface\n")
    lines.append(f"- Commands: {md_inline(inv['cli']['commands'])}\n")
    lines.append(f"- Aliases: {md_inline(inv['cli']['aliases'])}\n")

    lines.append("## Top-Level Language Nodes\n")
    lines.append(md_list(inv["top_level_nodes"]))
    lines.append(f"- Import preprocessor syntax: `{inv['imports']['syntax']}`\n")

    lines.append("## Lexer\n")
    lines.append(f"- Keywords: {md_inline(inv['lexer']['keywords'])}\n")
    lines.append(f"- Data types: {md_inline(inv['lexer']['types'])}\n")
    lines.append(f"- Literals: {md_inline(inv['lexer']['literals'])}\n")
    lines.append(f"- Word operators: {md_inline(inv['lexer']['word_operators'])}\n")
    lines.append(f"- Symbols: {md_inline(inv['lexer']['symbols'])}\n")
    lines.append(f"- String forms: {md_inline(inv['lexer']['strings'])}\n")

    lines.append("## Parser Blocks\n")
    lines.append(f"- View blocks: {md_inline(inv['parser']['view_blocks'])}\n")
    lines.append(f"- Component blocks: {md_inline(inv['parser']['component_blocks'])}\n")
    lines.append(f"- Design blocks: {md_inline(inv['parser']['design_block_names'])}\n")
    lines.append(f"- Form settings: {md_inline(inv['parser']['form_settings'])}\n")
    lines.append(f"- Form field options: {md_inline(inv['parser']['form_field_options'])}\n")
    lines.append(f"- Resource options: {md_inline(inv['parser']['resource_options'])}\n")
    lines.append(f"- State persistence values parsed by syntax: {md_inline(inv['parser']['state_persistence'])}\n")

    lines.append("## Theme Keys\n")
    lines.append(md_list(inv["theme"]["allowed_keys"]))
    lines.append("### Closed Theme Values\n")
    for key, values in inv["theme"]["closed_values"].items():
        lines.append(f"- `{key}`: {md_inline(values)}\n")

    lines.append("## Design Grammar\n")
    for block, keys in inv["design"]["allowed_keys_by_block"].items():
        lines.append(f"### `{block}`\n")
        lines.append(f"{md_inline(keys)}\n")
    lines.append("### Closed Design Values\n")
    for key, values in inv["design"]["closed_values"].items():
        lines.append(f"- `{key}`: {md_inline(values)}\n")

    lines.append("## CSS DSL\n")
    lines.append(f"- Allowed properties: {md_inline(inv['css']['allowed_properties'])}\n")
    lines.append(f"- Spacing tokens: {md_inline(inv['css']['spacing_tokens'])}\n")
    lines.append(f"- Size tokens: {md_inline(inv['css']['size_tokens'])}\n")
    lines.append(f"- Color tokens: {md_inline(inv['css']['color_tokens'])}\n")

    lines.append("## Semantic Surface\n")
    lines.append(f"- Query methods: {md_inline(inv['semantic']['query_methods'])}\n")
    lines.append(f"- Form actions: {md_inline(inv['semantic']['form_actions'])}\n")
    lines.append(f"- Standard library capabilities: {md_inline(inv['semantic']['standard_libraries']['capabilities'])}\n")
    for lib, methods in inv["semantic"]["standard_libraries"]["server_fetch_methods"].items():
        lines.append(f"- `{lib}` fetch methods: {md_inline(methods)}\n")
    lines.append(f"- Runtime/global expression names: {md_inline(inv['semantic']['standard_libraries']['global_expressions'])}\n")

    lines.append("## Codegen Surface\n")
    lines.append(f"- Backend: `{inv['codegen']['backend']}`\n")
    lines.append(f"- Standard components: {md_inline(inv['codegen']['standard_components'])}\n")
    lines.append(f"- Alpine/event attributes: {md_inline(inv['codegen']['event_attributes'])}\n")
    lines.append("- Generated files:\n")
    lines.append(md_list(inv["codegen"]["generated_files"]))
    lines.append(f"- Node package scripts: {md_inline(inv['codegen']['package']['scripts'])}\n")
    lines.append(f"- Runtime dependencies: {md_inline(inv['codegen']['package']['dependencies'])}\n")
    lines.append(f"- Dev dependencies: {md_inline(inv['codegen']['package']['dev_dependencies'])}\n")

    lines.append("## IR\n")
    version = inv["ir"]["version"]
    lines.append(f"- IR version: `{version['major']}.{version['minor']}.{version['patch']}`\n")
    lines.append(f"- IR target capabilities: {md_inline(inv['ir']['capabilities'])}\n")
    lines.append(f"- IR structs: {md_inline(inv['ir']['structs'])}\n")

    lines.append("## Feature Status Notes\n")
    for key, value in inv["feature_status"].items():
        lines.append(f"- `{key}`: {value}\n")

    lines.append("## Search Recipes\n")
    lines.append("- `scripts/search-language.ps1 -Area lexer`\n")
    lines.append("- `scripts/search-language.ps1 -Area parser`\n")
    lines.append("- `scripts/search-language.ps1 -Area semantic`\n")
    lines.append("- `scripts/search-language.ps1 -Area codegen`\n")
    lines.append("- `scripts/search-language.ps1 -Area runtime`\n")
    lines.append("- `python scripts/language_inventory.py --write`\n")

    return "\n".join(lines).replace("\n\n\n", "\n\n")


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate Amana language inventory from source.")
    parser.add_argument("--write", nargs="?", const=str(DEFAULT_OUTPUT), help="Write Markdown output. Defaults to doc/language-inventory.generated.md.")
    parser.add_argument("--check", action="store_true", help="Fail if the generated Markdown differs from the output file.")
    parser.add_argument("--json", action="store_true", help="Print raw inventory JSON.")
    args = parser.parse_args()

    inventory = collect_inventory()
    if args.json:
        print(json.dumps(inventory, indent=2, ensure_ascii=False))
        return 0

    markdown = render_markdown(inventory)
    output = Path(args.write) if args.write else DEFAULT_OUTPUT
    if not output.is_absolute():
        output = ROOT / output

    if args.check:
        existing = output.read_text(encoding="utf-8") if output.exists() else ""
        if existing != markdown:
            print(f"{output} is out of date. Run: python scripts/language_inventory.py --write", file=sys.stderr)
            return 1
        print(f"{output} is up to date.")
        return 0

    if args.write:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(markdown, encoding="utf-8")
        print(f"Wrote {output}")
    else:
        print(markdown)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

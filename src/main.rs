// src/main.rs
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub mod ast;
pub mod codegen;
pub mod formatter;
pub mod lexer;
pub mod parser;
pub mod semantic;

#[cfg(test)]
pub mod tests;

use ast::{AmanaNode, AppConfig, DesignBlock, ViewElement};
use lexer::Lexer;
use parser::Parser;
use semantic::SemanticAnalyzer;
use semantic::ir::AmanaIR;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliCommand {
    Check {
        source_file: String,
        json: bool,
        ir_snapshot: Option<IrSnapshotRequest>,
    },
    Build {
        source_file: String,
        output_dir: String,
        json: bool,
        ir_snapshot: Option<IrSnapshotRequest>,
    },
    Dev {
        source_file: String,
        output_dir: String,
        install: bool,
        watch: bool,
    },
    Fmt {
        source_file: String,
        check: bool,
        json: bool,
        all_graph: bool,
    },
    InspectDesign {
        source_file: String,
        json: bool,
    },
    Lsp,
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum IrSnapshotMode {
    Write,
    Verify,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IrSnapshotRequest {
    mode: IrSnapshotMode,
    path: PathBuf,
}

#[derive(Debug, Clone)]
struct CompileFailure {
    stage: &'static str,
    message: String,
    line: Option<usize>,
    column: Option<usize>,
    suggestion: Option<String>,
    file_path: Option<String>,
}

#[derive(Debug, Serialize)]
struct JsonDiagnostic {
    ok: bool,
    stage: String,
    line: Option<usize>,
    column: Option<usize>,
    message: String,
    suggestion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_path: Option<String>,
}

#[derive(Debug, Serialize)]
struct JsonSuccess {
    ok: bool,
    stage: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ir_snapshot: Option<String>,
}

#[derive(Debug, Serialize)]
struct DesignReport {
    ok: bool,
    stage: &'static str,
    source_file: String,
    score: u8,
    summary: String,
    views: Vec<DesignViewReport>,
    suggestions: Vec<String>,
}

#[derive(Debug, Serialize)]
struct DesignViewReport {
    name: String,
    canvas: bool,
    component_count: usize,
    design_block_count: usize,
    design_blocks: Vec<String>,
    standard_components: Vec<String>,
    ai_controls: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone)]
struct SourceUnit {
    path: PathBuf,
    source: String,
}

#[derive(Debug, Clone)]
struct ResolvedProgram {
    entry: PathBuf,
    files: Vec<SourceUnit>,
    nodes: Vec<AmanaNode>,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let command = match parse_cli_args(&args) {
        Ok(command) => command,
        Err(message) => {
            eprintln!("{}", message);
            print_usage();
            process::exit(1);
        }
    };

    match command {
        CliCommand::Help => {
            print_usage();
        }
        CliCommand::Check {
            source_file,
            json,
            ir_snapshot,
        } => {
            let program = match resolve_program_from_file(&source_file, None, !json) {
                Ok(program) => program,
                Err(err) => exit_with_compile_error(err.with_file_path(source_file.clone()), None, json),
            };
            let source = program.entry_source();
            match compile_resolved_ir(&program, !json) {
                Ok(ir) => {
                    let snapshot_path = match handle_ir_snapshot(&ir, ir_snapshot.as_ref()) {
                        Ok(path) => path,
                        Err(err) => exit_with_compile_error(err.with_file_path(source_file.clone()), Some(&source), json),
                    };
                    if json {
                        print_json_success(JsonSuccess {
                            ok: true,
                            stage: "check",
                            message: "Check completed successfully.".to_string(),
                            output_dir: None,
                            ir_snapshot: snapshot_path,
                        });
                    } else {
                        println!("{} Check completed successfully.", color_green("[Amana]"));
                    }
                }
                Err(err) => exit_with_compile_error(err.with_file_path(source_file.clone()), Some(&source), json),
            }
        }
        CliCommand::Build {
            source_file,
            output_dir,
            json,
            ir_snapshot,
        } => match build_project(&source_file, &output_dir, !json, ir_snapshot.as_ref()) {
            Ok(snapshot_path) => {
                if json {
                    print_json_success(JsonSuccess {
                        ok: true,
                        stage: "build",
                        message: "Build completed successfully.".to_string(),
                        output_dir: Some(output_dir),
                        ir_snapshot: snapshot_path,
                    });
                } else {
                    println!("{} Build completed successfully.", color_green("[Amana]"));
                }
            }
            Err(err) => {
                let (failure, source) = *err;
                exit_with_compile_error(failure.with_file_path(source_file.clone()), source.as_deref(), json);
            }
        },
        CliCommand::Dev {
            source_file,
            output_dir,
            install,
        watch,
        } => {
            if let Err(err) = build_project(&source_file, &output_dir, true, None) {
                let (failure, source) = *err;
                exit_with_compile_error(failure.with_file_path(source_file.clone()), source.as_deref(), false);
            }
            if install && let Err(message) = run_npm_install(&output_dir) {
                eprintln!("{} npm install failed:\n{}", color_red("[Amana]"), message);
                process::exit(1);
            }
            if watch {
                start_dev_rebuild_watcher(source_file.clone(), output_dir.clone());
            }
            if let Err(message) = run_npm_dev(&output_dir) {
                eprintln!("{} npm run dev failed:\n{}", color_red("[Amana]"), message);
                process::exit(1);
            }
        }
        CliCommand::Fmt {
            source_file,
            check,
            json,
            all_graph,
        } => match run_formatter(&source_file, check, all_graph) {
            Ok(outcome) => {
                if json {
                    print_json_success(JsonSuccess {
                        ok: true,
                        stage: "formatter",
                        message: if outcome.changed {
                            format!(
                                "Formatting changes were applied to {} file(s).",
                                outcome.files_checked
                            )
                        } else {
                            format!(
                                "{} file(s) already formatted.",
                                outcome.files_checked
                            )
                        },
                        output_dir: None,
                        ir_snapshot: None,
                    });
                } else if outcome.changed {
                    println!(
                        "{} Formatting changes were applied to {} file(s).",
                        color_green("[Amana]"),
                        outcome.files_checked
                    );
                } else {
                    println!(
                        "{} {} file(s) already formatted.",
                        color_green("[Amana]"),
                        outcome.files_checked
                    );
                }
            }
            Err(err) => exit_with_compile_error(err.with_file_path(source_file.clone()), None, json),
        },
        CliCommand::InspectDesign { source_file, json } => {
            let program = match resolve_program_from_file(&source_file, None, !json) {
                Ok(program) => program,
                Err(err) => exit_with_compile_error(err.with_file_path(source_file.clone()), None, json),
            };
            let source = program.entry_source();
            match compile_resolved_ir(&program, !json) {
                Ok(ir) => {
                    let report = inspect_design(&source_file, &ir);
                    if json {
                        print_design_report_json(&report);
                    } else {
                        print_design_report_text(&report);
                    }
                }
                Err(err) => exit_with_compile_error(err.with_file_path(source_file.clone()), Some(&source), json),
            }
        }
        CliCommand::Lsp => {
            if let Err(message) = run_lsp_server() {
                eprintln!("[Amana LSP] {}", message);
                process::exit(1);
            }
        }
    }
}

fn parse_cli_args(args: &[String]) -> Result<CliCommand, String> {
    if args.len() < 2 {
        return Ok(CliCommand::Help);
    }

    match args[1].as_str() {
        "-h" | "--help" | "help" => Ok(CliCommand::Help),
        "check" => parse_check_args(args),
        "build" => parse_build_args(args),
        "dev" => parse_dev_args(args),
        "fmt" => parse_fmt_args(args),
        "inspect-design" | "design-report" => parse_inspect_design_args(args),
        "lsp" | "language-server" => Ok(CliCommand::Lsp),
        source_file => {
            if args.len() > 3 {
                return Err(
                    "Legacy usage accepts only: amana <source-file.amana> [output-dir]".to_string(),
                );
            }
            Ok(CliCommand::Build {
                source_file: source_file.to_string(),
                output_dir: args.get(2).cloned().unwrap_or_else(|| "./dist".to_string()),
                json: false,
                ir_snapshot: None,
            })
        }
    }
}

fn parse_inspect_design_args(args: &[String]) -> Result<CliCommand, String> {
    let mut source_file = None;
    let mut json = false;
    for arg in args.iter().skip(2) {
        match arg.as_str() {
            "--json" => json = true,
            "--design" => {} // accept design flag
            _ if arg.starts_with("--") => {
                return Err(format!("Unknown inspect-design flag '{}'.", arg));
            }
            _ => {
                if source_file.is_some() {
                    return Err(
                        "Usage: amana inspect-design <source-file.amana> [--json]".to_string()
                    );
                }
                source_file = Some(arg.clone());
            }
        }
    }
    Ok(CliCommand::InspectDesign {
        source_file: source_file.ok_or_else(|| {
            "Usage: amana inspect-design <source-file.amana> [--json]".to_string()
        })?,
        json,
    })
}

fn parse_check_args(args: &[String]) -> Result<CliCommand, String> {
    let mut source_file: Option<String> = None;
    let mut json = false;
    let mut ir_snapshot = None;
    let mut i = 2;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--json" => {
                json = true;
                i += 1;
            }
            "--snapshot-ir" => {
                let source = source_file
                    .as_deref()
                    .ok_or_else(|| "Place <source-file.amana> before --snapshot-ir.".to_string())?;
                let path = consume_optional_path(args, &mut i)
                    .unwrap_or_else(|| default_ir_snapshot_path(source));
                ir_snapshot = Some(IrSnapshotRequest {
                    mode: IrSnapshotMode::Write,
                    path,
                });
            }
            "--verify-ir-snapshot" => {
                let source = source_file.as_deref().ok_or_else(|| {
                    "Place <source-file.amana> before --verify-ir-snapshot.".to_string()
                })?;
                let path = consume_optional_path(args, &mut i)
                    .unwrap_or_else(|| default_ir_snapshot_path(source));
                ir_snapshot = Some(IrSnapshotRequest {
                    mode: IrSnapshotMode::Verify,
                    path,
                });
            }
            _ if arg.starts_with("--snapshot-ir=") => {
                ir_snapshot = Some(IrSnapshotRequest {
                    mode: IrSnapshotMode::Write,
                    path: PathBuf::from(arg.trim_start_matches("--snapshot-ir=")),
                });
                i += 1;
            }
            _ if arg.starts_with("--verify-ir-snapshot=") => {
                ir_snapshot = Some(IrSnapshotRequest {
                    mode: IrSnapshotMode::Verify,
                    path: PathBuf::from(arg.trim_start_matches("--verify-ir-snapshot=")),
                });
                i += 1;
            }
            _ if arg.starts_with("--") => return Err(format!("Unknown check flag '{}'.", arg)),
            _ => {
                if source_file.is_some() {
                    return Err("Usage: amana check <source-file.amana> [--json] [--snapshot-ir [path]] [--verify-ir-snapshot [path]]".to_string());
                }
                source_file = Some(arg.clone());
                i += 1;
            }
        }
    }
    Ok(CliCommand::Check {
        source_file: source_file
            .ok_or_else(|| "Usage: amana check <source-file.amana>".to_string())?,
        json,
        ir_snapshot,
    })
}

fn parse_build_args(args: &[String]) -> Result<CliCommand, String> {
    let mut source_file: Option<String> = None;
    let mut output_dir: Option<String> = None;
    let mut json = false;
    let mut ir_snapshot = None;
    let mut i = 2;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--json" => {
                json = true;
                i += 1;
            }
            "--snapshot-ir" => {
                let source = source_file
                    .as_deref()
                    .ok_or_else(|| "Place <source-file.amana> before --snapshot-ir.".to_string())?;
                let path = consume_optional_path(args, &mut i)
                    .unwrap_or_else(|| default_ir_snapshot_path(source));
                ir_snapshot = Some(IrSnapshotRequest {
                    mode: IrSnapshotMode::Write,
                    path,
                });
            }
            "--verify-ir-snapshot" => {
                let source = source_file.as_deref().ok_or_else(|| {
                    "Place <source-file.amana> before --verify-ir-snapshot.".to_string()
                })?;
                let path = consume_optional_path(args, &mut i)
                    .unwrap_or_else(|| default_ir_snapshot_path(source));
                ir_snapshot = Some(IrSnapshotRequest {
                    mode: IrSnapshotMode::Verify,
                    path,
                });
            }
            _ if arg.starts_with("--snapshot-ir=") => {
                ir_snapshot = Some(IrSnapshotRequest {
                    mode: IrSnapshotMode::Write,
                    path: PathBuf::from(arg.trim_start_matches("--snapshot-ir=")),
                });
                i += 1;
            }
            _ if arg.starts_with("--verify-ir-snapshot=") => {
                ir_snapshot = Some(IrSnapshotRequest {
                    mode: IrSnapshotMode::Verify,
                    path: PathBuf::from(arg.trim_start_matches("--verify-ir-snapshot=")),
                });
                i += 1;
            }
            _ if arg.starts_with("--") => return Err(format!("Unknown build flag '{}'.", arg)),
            _ => {
                if source_file.is_none() {
                    source_file = Some(arg.clone());
                } else if output_dir.is_none() {
                    output_dir = Some(arg.clone());
                } else {
                    return Err("Usage: amana build <source-file.amana> [output-dir] [--json] [--snapshot-ir [path]] [--verify-ir-snapshot [path]]".to_string());
                }
                i += 1;
            }
        }
    }
    Ok(CliCommand::Build {
        source_file: source_file
            .ok_or_else(|| "Usage: amana build <source-file.amana> [output-dir]".to_string())?,
        output_dir: output_dir.unwrap_or_else(|| "./dist".to_string()),
        json,
        ir_snapshot,
    })
}

fn parse_dev_args(args: &[String]) -> Result<CliCommand, String> {
    if args.len() < 3 || args.len() > 6 {
        return Err(
            "Usage: amana dev <source-file.amana> [output-dir] [--no-install] [--no-watch]"
                .to_string(),
        );
    }
    let mut source_file = None;
    let mut output_dir = "./dist".to_string();
    let mut install = true;
    let mut watch = true;
    for arg in args.iter().skip(2) {
        match arg.as_str() {
            "--no-install" => install = false,
            "--no-watch" => watch = false,
            _ if arg.starts_with("--") => return Err(format!("Unknown dev flag '{}'.", arg)),
            _ if source_file.is_none() => source_file = Some(arg.clone()),
            _ => output_dir = arg.clone(),
        }
    }
    Ok(CliCommand::Dev {
        source_file: source_file
            .ok_or_else(|| "Usage: amana dev <source-file.amana> [output-dir]".to_string())?,
        output_dir,
        install,
        watch,
    })
}

fn parse_fmt_args(args: &[String]) -> Result<CliCommand, String> {
    let mut source_file = None;
    let mut check = false;
    let mut json = false;
    let mut all_graph = false;
    for arg in args.iter().skip(2) {
        match arg.as_str() {
            "--check" => check = true,
            "--json" => json = true,
            "--all" => all_graph = true,
            _ if arg.starts_with("--") => return Err(format!("Unknown fmt flag '{}'.", arg)),
            _ => {
                if source_file.is_some() {
                    return Err(
                        "Usage: amana fmt <source-file.amana> [--check] [--json] [--all]".to_string()
                    );
                }
                source_file = Some(arg.clone());
            }
        }
    }
    Ok(CliCommand::Fmt {
        source_file: source_file
            .ok_or_else(|| "Usage: amana fmt <source-file.amana> [--check] [--all]".to_string())?,
        check,
        json,
        all_graph,
    })
}

fn consume_optional_path(args: &[String], i: &mut usize) -> Option<PathBuf> {
    *i += 1;
    if *i < args.len() && !args[*i].starts_with("--") {
        let path = PathBuf::from(&args[*i]);
        *i += 1;
        Some(path)
    } else {
        None
    }
}

fn print_usage() {
    println!("Amana Compiler v1.0.0");
    println!("Usage:");
    println!(
        "  amana check <source-file.amana> [--json] [--snapshot-ir [path]] [--verify-ir-snapshot [path]]"
    );
    println!("  amana build <source-file.amana> [output-dir] [--json] [--snapshot-ir [path]]");
    println!("  amana fmt <source-file.amana> [--check] [--json] [--all]");
    println!("  amana inspect-design <source-file.amana> [--json]");
    println!("  amana dev <source-file.amana> [output-dir] [--no-install] [--no-watch]");
    println!("  amana lsp                                  # language server over stdio");
    println!("  amana <source-file.amana> [output-dir]   # legacy build mode");
}

type BuildProjectError = Box<(CompileFailure, Option<String>)>;

fn build_project(
    source_file: &str,
    output_dir: &str,
    verbose: bool,
    ir_snapshot: Option<&IrSnapshotRequest>,
) -> Result<Option<String>, BuildProjectError> {
    let program =
        resolve_program_from_file(source_file, None, verbose).map_err(|err| Box::new((err, None)))?;
    let source = program.entry_source();
    let ir = compile_resolved_ir(&program, verbose).map_err(|err| Box::new((err, Some(source.clone()))))?;
    let snapshot_path =
        handle_ir_snapshot(&ir, ir_snapshot).map_err(|err| Box::new((err, Some(source.clone()))))?;

    if verbose {
        println!(
            "{} Step 4: Generating web application code into: {} ...",
            color_cyan("[Amana]"),
            output_dir
        );
    }
    codegen::generate_project(output_dir, &ir)
        .map_err(|message| Box::new((CompileFailure::new("codegen", message), Some(source))))?;
    Ok(snapshot_path)
}

impl ResolvedProgram {
    fn entry_source(&self) -> String {
        self.files
            .iter()
            .find(|unit| unit.path == self.entry)
            .map(|unit| unit.source.clone())
            .unwrap_or_default()
    }
}

fn resolve_program_from_file(
    source_file: &str,
    overlay: Option<(PathBuf, String)>,
    verbose: bool,
) -> Result<ResolvedProgram, CompileFailure> {
    let entry = normalize_source_path(Path::new(source_file));
    if verbose {
        println!(
            "{} Resolving source graph from: {} ...",
            color_cyan("[Amana]"),
            entry.display()
        );
    }
    let mut files = Vec::new();
    let mut nodes = Vec::new();
    let mut visited = BTreeSet::new();
    let mut stack = Vec::new();
    let overlay = overlay.map(|(path, source)| (normalize_source_path(&path), source));
    resolve_source_file(
        &entry,
        overlay.as_ref(),
        &mut visited,
        &mut stack,
        &mut files,
        &mut nodes,
    )?;
    Ok(ResolvedProgram {
        entry,
        files,
        nodes,
    })
}

fn normalize_source_path(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    fs::canonicalize(&absolute).unwrap_or(absolute)
}

fn resolve_source_file(
    path: &Path,
    overlay: Option<&(PathBuf, String)>,
    visited: &mut BTreeSet<PathBuf>,
    stack: &mut Vec<PathBuf>,
    files: &mut Vec<SourceUnit>,
    nodes: &mut Vec<AmanaNode>,
) -> Result<(), CompileFailure> {
    let normalized = normalize_source_path(path);
    if visited.contains(&normalized) {
        return Ok(());
    }
    if stack.contains(&normalized) {
        let chain = stack
            .iter()
            .chain(std::iter::once(&normalized))
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(" -> ");
        return Err(CompileFailure::new(
            "imports",
            format!("Circular Amana import detected: {}", chain),
        ).with_file_path(normalized.to_string_lossy().to_string()));
    }

    stack.push(normalized.clone());
    let source = if let Some((overlay_path, overlay_source)) = overlay {
        if *overlay_path == normalized {
            overlay_source.clone()
        } else {
            fs::read_to_string(&normalized).map_err(|err| {
                CompileFailure::new(
                    "file",
                    format!(
                        "Error reading source file '{}': {}",
                        normalized.display(),
                        err
                    ),
                ).with_file_path(normalized.to_string_lossy().to_string())
            })?
        }
    } else {
        fs::read_to_string(&normalized).map_err(|err| {
            CompileFailure::new(
                "file",
                format!(
                    "Error reading source file '{}': {}",
                    normalized.display(),
                    err
                ),
            ).with_file_path(normalized.to_string_lossy().to_string())
        })?
    };

    let (source_without_imports, imports) = strip_import_lines(&source)
        .map_err(|err| err.with_file_path(normalized.to_string_lossy().to_string()))?;
    let parent = normalized.parent().unwrap_or_else(|| Path::new("."));
    for import in imports {
        let import_path = Path::new(&import.path);
        let resolved = if import_path.is_absolute() {
            normalize_source_path(import_path)
        } else {
            normalize_source_path(&parent.join(import_path))
        };
        if !resolved.exists() {
            return Err(CompileFailure::new(
                "imports",
                format!(
                    "Import '{}' from '{}' does not exist at line {}:{}",
                    import.path,
                    normalized.display(),
                    import.line,
                    import.column
                ),
            ).with_file_path(normalized.to_string_lossy().to_string()));
        }
        resolve_source_file(&resolved, overlay, visited, stack, files, nodes)?;
    }

    let parsed = parse_nodes_for_file(&normalized, &source_without_imports)
        .map_err(|err| err.with_file_path(normalized.to_string_lossy().to_string()))?;
    nodes.extend(parsed);
    files.push(SourceUnit {
        path: normalized.clone(),
        source,
    });
    visited.insert(normalized);
    stack.pop();
    Ok(())
}

#[derive(Debug, Clone)]
struct ImportDirective {
    path: String,
    line: usize,
    column: usize,
}

fn strip_import_lines(source: &str) -> Result<(String, Vec<ImportDirective>), CompileFailure> {
    let mut cleaned = String::new();
    let mut imports = Vec::new();
    for (idx, line) in source.lines().enumerate() {
        let line_no = idx + 1;
        let leading = line.len() - line.trim_start().len();
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("import ") {
            let rest = rest.trim();
            let Some(path) = quoted_directive_value(rest) else {
                return Err(CompileFailure::new(
                    "imports",
                    format!(
                        "Invalid import syntax. Use import \"./file.amana\" at line {}:{}",
                        line_no,
                        leading + 1
                    ),
                ));
            };
            imports.push(ImportDirective {
                path,
                line: line_no,
                column: leading + 1,
            });
            cleaned.push('\n');
        } else {
            cleaned.push_str(line);
            cleaned.push('\n');
        }
    }
    Ok((cleaned, imports))
}

fn quoted_directive_value(rest: &str) -> Option<String> {
    let value = rest.strip_prefix('"')?;
    let end = value.find('"')?;
    let path = &value[..end];
    let tail = value[end + 1..].trim();
    if path.is_empty() || (!tail.is_empty() && !tail.starts_with('#')) {
        return None;
    }
    Some(path.replace('\\', "/"))
}

fn parse_nodes_for_file(_path: &Path, source_code: &str) -> Result<Vec<AmanaNode>, CompileFailure> {
    let mut lexer = Lexer::new(source_code);
    let tokens = lexer
        .tokenize()
        .map_err(|message| CompileFailure::new("lexer", message))?;
    let mut parser = Parser::new(tokens);
    parser
        .parse()
        .map_err(|message| CompileFailure::new("parser", message))
}

#[cfg(test)]
fn compile_ir(
    source_file: &str,
    source_code: &str,
    verbose: bool,
) -> Result<AmanaIR, CompileFailure> {
    if verbose {
        println!("{} Step 1: Tokenizing source code...", color_cyan("[Amana]"));
    }
    if verbose {
        println!("{} Step 2: Parsing token stream into AST...", color_cyan("[Amana]"));
    }
    let nodes = parse_nodes_for_file(Path::new(source_file), source_code)?;
    compile_nodes_ir(source_file, nodes, verbose)
}

fn compile_resolved_ir(
    program: &ResolvedProgram,
    verbose: bool,
) -> Result<AmanaIR, CompileFailure> {
    compile_nodes_ir(
        &program.entry.to_string_lossy(),
        program.nodes.clone(),
        verbose,
    )
}

fn compile_nodes_ir(
    source_file: &str,
    nodes: Vec<AmanaNode>,
    verbose: bool,
) -> Result<AmanaIR, CompileFailure> {
    let (config, models, views, components) = collect_program_parts(&nodes);

    if verbose {
        println!("{} Step 3: Performing semantic analysis and type checking...", color_cyan("[Amana]"));
    }
    let mut analyzer = SemanticAnalyzer::new(
        &models,
        &config.auth_model,
        &config.capabilities,
        &components,
    );
    for view in &views {
        analyzer.validate_view(view).map_err(|message| {
            CompileFailure::new(
                "semantic",
                format!(
                    "In view '{}' from '{}': {}",
                    view.name, source_file, message
                ),
            )
        })?;
    }
    for node in &nodes {
        if let AmanaNode::Variant(var) = node {
            analyzer.validate_variant(var).map_err(|message| {
                CompileFailure::new(
                    "semantic",
                    format!(
                        "In global variant '{}' for target '{}' from '{}': {}",
                        var.name, var.target, source_file, message
                    ),
                )
            })?;
        }
    }
    for comp in &components {
        for var in &comp.variants {
            analyzer.validate_variant(var).map_err(|message| {
                CompileFailure::new(
                    "semantic",
                    format!(
                        "In local variant '{}' for component '{}' from '{}': {}",
                        var.name, comp.name, source_file, message
                    ),
                )
            })?;
        }
    }
    for node in &nodes {
        if let AmanaNode::Seed(seed) = node {
            analyzer.validate_seed(seed).map_err(|message| {
                CompileFailure::new(
                    "semantic",
                    format!(
                        "In seed '{}' from '{}': {}",
                        seed.model_name, source_file, message
                    ),
                )
            })?;
        }
    }

    if verbose {
        println!("{} Optimizing AST nodes (constant folding & DCE)...", color_cyan("[Amana]"));
    }
    let optimized_nodes = semantic::optimizer::optimize_ast(nodes);

    if verbose {
        println!("{} Generating Intermediate Representation (IR)...", color_cyan("[Amana]"));
    }
    semantic::ir_gen::generate_ir(&analyzer, &optimized_nodes, &config)
        .map_err(|message| CompileFailure::new("ir", message))
}

fn collect_program_parts(
    nodes: &[AmanaNode],
) -> (
    AppConfig,
    Vec<ast::ModelDecl>,
    Vec<ast::ViewDecl>,
    Vec<ast::ComponentDecl>,
) {
    let mut app_config = None;
    let mut models = Vec::new();
    let mut views = Vec::new();
    let mut components = Vec::new();

    for node in nodes {
        match node {
            AmanaNode::App(config) => {
                app_config = Some(config.clone());
            }
            AmanaNode::Model(model) => {
                models.push(model.clone());
            }
            AmanaNode::View(view) => {
                views.push(view.clone());
            }
            AmanaNode::Component(comp) => {
                components.push(comp.clone());
            }
            _ => {}
        }
    }

    let config = app_config.unwrap_or_else(|| AppConfig {
        name: "AmanaApp".to_string(),
        title: "Amana Generated App".to_string(),
        db_path: "app.db".to_string(),
        auth_model: "User".to_string(),
        capabilities: vec![],
    });

    (config, models, views, components)
}

fn handle_ir_snapshot(
    ir: &AmanaIR,
    request: Option<&IrSnapshotRequest>,
) -> Result<Option<String>, CompileFailure> {
    let Some(request) = request else {
        return Ok(None);
    };
    let json = canonical_ir_json(ir)?;
    match request.mode {
        IrSnapshotMode::Write => {
            if let Some(parent) = request.path.parent()
                && !parent.as_os_str().is_empty()
            {
                fs::create_dir_all(parent).map_err(|err| {
                    CompileFailure::new(
                        "ir",
                        format!("Failed to create IR snapshot directory: {}", err),
                    )
                })?;
            }
            fs::write(&request.path, json).map_err(|err| {
                CompileFailure::new("ir", format!("Failed to write IR snapshot: {}", err))
            })?;
            Ok(Some(request.path.to_string_lossy().to_string()))
        }
        IrSnapshotMode::Verify => {
            let expected = fs::read_to_string(&request.path).map_err(|err| {
                CompileFailure::new("ir", format!("Failed to read IR snapshot: {}", err))
            })?;
            if expected.trim_end() == json.trim_end() {
                Ok(Some(request.path.to_string_lossy().to_string()))
            } else {
                Err(CompileFailure::new(
                    "ir",
                    format!(
                        "IR snapshot mismatch for '{}'. Run with --snapshot-ir to update it.",
                        request.path.to_string_lossy()
                    ),
                ))
            }
        }
    }
}

fn canonical_ir_json(ir: &AmanaIR) -> Result<String, CompileFailure> {
    serde_json::to_string_pretty(ir)
        .map(|json| format!("{}\n", json.trim_end()))
        .map_err(|err| CompileFailure::new("ir", format!("Failed to serialize IR: {}", err)))
}

fn default_ir_snapshot_path(source_file: &str) -> PathBuf {
    let source = Path::new(source_file);
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("amana");
    let parent = source.parent().unwrap_or_else(|| Path::new("."));
    parent
        .join(".amana_snapshots")
        .join(format!("{}.ir.json", stem))
}

fn inspect_design(source_file: &str, ir: &AmanaIR) -> DesignReport {
    let mut views = Vec::new();
    let mut total_score = 0usize;
    let mut suggestions = std::collections::BTreeSet::new();

    for view in &ir.views {
        let report = inspect_design_view(view);
        total_score += design_view_score(&report);
        for warning in &report.warnings {
            suggestions.insert(format!("{}: {}", report.name, warning));
        }
        views.push(report);
    }

    let score = if views.is_empty() {
        0
    } else {
        (total_score / views.len()).min(100) as u8
    };

    if score < 70 {
        suggestions.insert(
            "Add canvas, brand, art, responsive, and creative blocks to make AI-generated designs less repetitive."
                .to_string(),
        );
    }
    if !views
        .iter()
        .any(|view| view.design_blocks.iter().any(|block| block == "art"))
    {
        suggestions.insert(
            "Add an art block with direction, motif, lighting, or texture for stronger visual identity."
                .to_string(),
        );
    }
    if !views
        .iter()
        .any(|view| view.design_blocks.iter().any(|block| block == "brand"))
    {
        suggestions.insert(
            "Add a brand block with voice, personality, trust, or colorway so AI can vary tone safely."
                .to_string(),
        );
    }

    DesignReport {
        ok: true,
        stage: "inspect-design",
        source_file: source_file.to_string(),
        score,
        summary: if score >= 85 {
            "Design grammar is strong and AI-controllable.".to_string()
        } else if score >= 65 {
            "Design grammar is usable but should add more art direction and responsive controls."
                .to_string()
        } else {
            "Design grammar is under-specified and likely to produce repetitive layouts."
                .to_string()
        },
        views,
        suggestions: suggestions.into_iter().collect(),
    }
}

fn inspect_design_view(view: &semantic::ir::ViewIR) -> DesignViewReport {
    let mut blocks = Vec::new();
    let mut components = std::collections::BTreeSet::new();
    let mut ai_controls = std::collections::BTreeSet::new();

    if let Some(canvas) = &view.canvas {
        collect_design_block(canvas, &mut blocks, &mut ai_controls);
    }
    if let Some(body) = &view.render_body {
        collect_design_element(body, &mut blocks, &mut components, &mut ai_controls);
    }

    let design_blocks: Vec<String> = blocks
        .iter()
        .map(|block| block.kind.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let standard_components: Vec<String> = components.into_iter().collect();

    let mut warnings = Vec::new();
    if view.canvas.is_none() {
        warnings.push("Missing canvas block for page-level composition.".to_string());
    }
    if !design_blocks.iter().any(|block| block == "creative") {
        warnings
            .push("Missing creative block for AI freedom, uniqueness, and signature.".to_string());
    }
    if !design_blocks.iter().any(|block| block == "responsive") {
        warnings.push("Missing responsive block for mobile/tablet behavior.".to_string());
    }
    if !design_blocks.iter().any(|block| block == "brand") {
        warnings.push("Missing brand block for voice/personality/colorway control.".to_string());
    }
    if !design_blocks.iter().any(|block| block == "art") {
        warnings.push("Missing art block for direction, motif, lighting, or texture.".to_string());
    }
    if standard_components.len() < 3 {
        warnings.push(
            "Use at least three standard components to reduce raw div-heavy output.".to_string(),
        );
    }

    // Rule 1: Consecutive layout repetition check
    let mut layouts_used = Vec::new();
    fn collect_layouts(element: &ViewElement, layouts: &mut Vec<String>) {
        match element {
            ViewElement::Element { children, .. } => {
                for child in children {
                    collect_layouts(child, layouts);
                }
            }
            ViewElement::ForEach { body, .. } => {
                for child in body {
                    collect_layouts(child, layouts);
                }
            }
            ViewElement::IfBlock { then_branch, else_branch, .. } => {
                for child in then_branch {
                    collect_layouts(child, layouts);
                }
                if let Some(nodes) = else_branch {
                    for child in nodes {
                        collect_layouts(child, layouts);
                    }
                }
            }
            ViewElement::DesignBlock(block) => {
                if block.kind == "compose" {
                    if let Some((_, layout_val)) = block.settings.iter().find(|(k, _)| k == "layout") {
                        layouts.push(layout_val.clone());
                    }
                }
            }
            _ => {}
        }
    }
    if let Some(body) = &view.render_body {
        collect_layouts(body, &mut layouts_used);
    }
    
    let mut consecutive_layout_count = 1;
    let mut prev_layout = "";
    for layout in &layouts_used {
        if layout == prev_layout {
            consecutive_layout_count += 1;
            if consecutive_layout_count > 3 {
                warnings.push(format!(
                    "Layout '{}' is repeated consecutively 4+ times. Consider introducing Bento or Split grids for diversity.",
                    layout
                ));
            }
        } else {
            consecutive_layout_count = 1;
        }
        prev_layout = layout;
    }

    // Rule 2: Variant diversity check
    let mut total_cards = 0;
    let mut has_variant = false;
    fn count_cards(element: &ViewElement, total_cards: &mut usize, has_variant: &mut bool) {
        match element {
            ViewElement::Element { tag, attributes, children, .. } => {
                if tag == "Card" || tag == "FeatureCard" || tag == "PricingCard" {
                    *total_cards += 1;
                    if attributes.iter().any(|(k, _)| k == "variant") {
                        *has_variant = true;
                    }
                }
                for child in children {
                    count_cards(child, total_cards, has_variant);
                }
            }
            ViewElement::ForEach { body, .. } => {
                for child in body {
                    count_cards(child, total_cards, has_variant);
                }
            }
            ViewElement::IfBlock { then_branch, else_branch, .. } => {
                for child in then_branch {
                    count_cards(child, total_cards, has_variant);
                }
                if let Some(nodes) = else_branch {
                    for child in nodes {
                        count_cards(child, total_cards, has_variant);
                    }
                }
            }
            _ => {}
        }
    }
    if let Some(body) = &view.render_body {
        count_cards(body, &mut total_cards, &mut has_variant);
    }
    if total_cards > 4 && !has_variant {
        warnings.push("All cards on the page use default variants. Consider using 'glass', 'minimal', or 'luxury' variants to create hierarchy.".to_string());
    }

    DesignViewReport {
        name: view.name.clone(),
        canvas: view.canvas.is_some(),
        component_count: standard_components.len(),
        design_block_count: blocks.len(),
        design_blocks,
        standard_components,
        ai_controls: ai_controls.into_iter().collect(),
        warnings,
    }
}

fn collect_design_element(
    element: &ViewElement,
    blocks: &mut Vec<DesignBlock>,
    components: &mut std::collections::BTreeSet<String>,
    ai_controls: &mut std::collections::BTreeSet<String>,
) {
    match element {
        ViewElement::Element { tag, children, .. } => {
            if tag
                .chars()
                .next()
                .is_some_and(|first| first.is_ascii_uppercase())
            {
                components.insert(tag.clone());
            }
            for child in children {
                collect_design_element(child, blocks, components, ai_controls);
            }
        }
        ViewElement::ForEach { body, .. } => {
            for child in body {
                collect_design_element(child, blocks, components, ai_controls);
            }
        }
        ViewElement::IfBlock {
            then_branch,
            else_branch,
            ..
        } => {
            for child in then_branch {
                collect_design_element(child, blocks, components, ai_controls);
            }
            if let Some(else_branch) = else_branch {
                for child in else_branch {
                    collect_design_element(child, blocks, components, ai_controls);
                }
            }
        }
        ViewElement::DesignBlock(block) => {
            collect_design_block(block, blocks, ai_controls);
        }
        ViewElement::SlotDecl { .. } => {}
        ViewElement::ResourceGrid { empty_element, loading_element, error_element, .. }
        | ViewElement::ResourceTable { empty_element, loading_element, error_element, .. } => {
            if let Some(nodes) = empty_element {
                for child in nodes {
                    collect_design_element(child, blocks, components, ai_controls);
                }
            }
            if let Some(nodes) = loading_element {
                for child in nodes {
                    collect_design_element(child, blocks, components, ai_controls);
                }
            }
            if let Some(nodes) = error_element {
                for child in nodes {
                    collect_design_element(child, blocks, components, ai_controls);
                }
            }
        }
        _ => {}
    }
}

fn collect_design_block(
    block: &DesignBlock,
    blocks: &mut Vec<DesignBlock>,
    ai_controls: &mut std::collections::BTreeSet<String>,
) {
    for (key, value) in &block.settings {
        if matches!(
            block.kind.as_str(),
            "creative" | "brand" | "art" | "responsive" | "interaction" | "a11y"
        ) {
            ai_controls.insert(format!("{}.{}={}", block.kind, key, value));
        }
    }
    blocks.push(block.clone());
}

fn design_view_score(report: &DesignViewReport) -> usize {
    let mut score = 0usize;
    if report.canvas {
        score += 12;
    }
    for block in &report.design_blocks {
        score += match block.as_str() {
            "compose" => 10,
            "visual" => 10,
            "type" => 8,
            "motion" => 8,
            "creative" => 10,
            "brand" => 10,
            "art" => 10,
            "responsive" => 8,
            "interaction" => 6,
            "a11y" => 6,
            _ => 2,
        };
    }
    score += (report.standard_components.len() * 3).min(12);
    score += (report.ai_controls.len() * 2).min(14);
    score.saturating_sub(report.warnings.len() * 4).min(100)
}

fn print_design_report_json(report: &DesignReport) {
    match serde_json::to_string_pretty(report) {
        Ok(json) => println!("{}", json),
        Err(err) => {
            eprintln!("Failed to serialize design report JSON: {}", err);
            process::exit(1);
        }
    }
}

fn print_design_report_text(report: &DesignReport) {
    println!("[Amana] Design score: {}/100", report.score);
    println!("[Amana] {}", report.summary);
    for view in &report.views {
        println!(
            "[Amana] View {}: {} design blocks, {} components",
            view.name, view.design_block_count, view.component_count
        );
        for warning in &view.warnings {
            println!("  - {}", warning);
        }
    }
}

fn run_lsp_server() -> Result<(), String> {
    let mut server = LspServer {
        documents: BTreeMap::new(),
    };
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();

    while let Some(raw) = read_lsp_message(&mut input)? {
        let message: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|err| format!("Invalid JSON-RPC message: {}", err))?;
        let method = message
            .get("method")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let id = message.get("id").cloned();
        match method {
            "initialize" => {
                if let Some(id) = id {
                    send_lsp_response(
                        &mut output,
                        id,
                        serde_json::json!({
                            "capabilities": {
                                "textDocumentSync": {
                                    "openClose": true,
                                    "change": 1,
                                    "save": { "includeText": true }
                                },
                                "completionProvider": {
                                    "triggerCharacters": [" ", ":", "(", "."]
                                },
                                "documentFormattingProvider": true
                            },
                            "serverInfo": {
                                "name": "amana-lsp",
                                "version": "0.1.0"
                            }
                        }),
                    )?;
                }
            }
            "initialized" => {}
            "shutdown" => {
                if let Some(id) = id {
                    send_lsp_response(&mut output, id, serde_json::Value::Null)?;
                }
            }
            "exit" => break,
            "textDocument/didOpen" => {
                if let Some((uri, text)) = lsp_text_document(&message, "textDocument") {
                    server.documents.insert(uri.clone(), text);
                    server.publish_diagnostics(&mut output, &uri)?;
                }
            }
            "textDocument/didChange" => {
                if let Some(uri) = lsp_document_uri(&message, "textDocument")
                    && let Some(text) = message
                        .get("params")
                        .and_then(|params| params.get("contentChanges"))
                        .and_then(|changes| changes.as_array())
                        .and_then(|changes| changes.first())
                        .and_then(|change| change.get("text"))
                        .and_then(|text| text.as_str())
                {
                    server.documents.insert(uri.clone(), text.to_string());
                    server.publish_diagnostics(&mut output, &uri)?;
                }
            }
            "textDocument/didSave" => {
                if let Some(uri) = lsp_document_uri(&message, "textDocument") {
                    if let Some(text) = message
                        .get("params")
                        .and_then(|params| params.get("text"))
                        .and_then(|text| text.as_str())
                    {
                        server.documents.insert(uri.clone(), text.to_string());
                    }
                    server.publish_diagnostics(&mut output, &uri)?;
                }
            }
            "textDocument/completion" => {
                if let Some(id) = id {
                    send_lsp_response(&mut output, id, lsp_completion_items())?;
                }
            }
            "textDocument/formatting" => {
                if let Some(id) = id {
                    let uri = lsp_document_uri(&message, "textDocument");
                    let edit = uri
                        .as_ref()
                        .and_then(|uri| server.documents.get(uri))
                        .map(|source| {
                            let line_count = source.lines().count().max(1);
                            serde_json::json!([{
                                "range": {
                                    "start": { "line": 0, "character": 0 },
                                    "end": { "line": line_count + 1, "character": 0 }
                                },
                                "newText": formatter::format_source(source)
                            }])
                        })
                        .unwrap_or_else(|| serde_json::json!([]));
                    send_lsp_response(&mut output, id, edit)?;
                }
            }
            _ => {
                if let Some(id) = id {
                    send_lsp_error(&mut output, id, -32601, "Method not supported by amana-lsp")?;
                }
            }
        }
    }
    Ok(())
}

struct LspServer {
    documents: BTreeMap<String, String>,
}

impl LspServer {
    fn publish_diagnostics<W: Write>(&self, output: &mut W, uri: &str) -> Result<(), String> {
        let diagnostics = self.document_diagnostics(uri);
        send_lsp_notification(
            output,
            "textDocument/publishDiagnostics",
            serde_json::json!({
                "uri": uri,
                "diagnostics": diagnostics
            }),
        )
    }

    fn document_diagnostics(&self, uri: &str) -> serde_json::Value {
        let Some(path) = uri_to_path(uri) else {
            return serde_json::json!([lsp_diagnostic(
                "lsp",
                "Only file:// Amana document URIs are supported.",
                Some(1),
                Some(1),
                None
            )]);
        };
        let Some(source) = self.documents.get(uri) else {
            return serde_json::json!([]);
        };
        let source_file = path.to_string_lossy().to_string();
        let result = resolve_program_from_file(&source_file, Some((path, source.clone())), false)
            .and_then(|program| compile_resolved_ir(&program, false));
        match result {
            Ok(_) => serde_json::json!([]),
            Err(err) => serde_json::json!([lsp_diagnostic(
                err.stage,
                &err.message,
                err.line,
                err.column,
                err.suggestion.as_deref()
            )]),
        }
    }
}

fn read_lsp_message<R: Read>(input: &mut R) -> Result<Option<String>, String> {
    let mut headers = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        match input.read(&mut byte) {
            Ok(0) if headers.is_empty() => return Ok(None),
            Ok(0) => return Err("Unexpected EOF while reading LSP headers.".to_string()),
            Ok(_) => {
                headers.push(byte[0]);
                if headers.ends_with(b"\r\n\r\n") || headers.ends_with(b"\n\n") {
                    break;
                }
            }
            Err(err) => return Err(format!("Failed to read LSP headers: {}", err)),
        }
    }
    let header_text = String::from_utf8_lossy(&headers);
    let content_length = header_text
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            (key.trim().eq_ignore_ascii_case("content-length"))
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .ok_or_else(|| "Missing Content-Length in LSP message.".to_string())?;
    let mut body = vec![0u8; content_length];
    input
        .read_exact(&mut body)
        .map_err(|err| format!("Failed to read LSP body: {}", err))?;
    String::from_utf8(body)
        .map(Some)
        .map_err(|err| err.to_string())
}

fn write_lsp_message<W: Write>(output: &mut W, value: serde_json::Value) -> Result<(), String> {
    let body = serde_json::to_string(&value).map_err(|err| err.to_string())?;
    write!(output, "Content-Length: {}\r\n\r\n{}", body.len(), body)
        .map_err(|err| err.to_string())?;
    output.flush().map_err(|err| err.to_string())
}

fn send_lsp_response<W: Write>(
    output: &mut W,
    id: serde_json::Value,
    result: serde_json::Value,
) -> Result<(), String> {
    write_lsp_message(
        output,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        }),
    )
}

fn send_lsp_error<W: Write>(
    output: &mut W,
    id: serde_json::Value,
    code: i32,
    message: &str,
) -> Result<(), String> {
    write_lsp_message(
        output,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": code,
                "message": message
            }
        }),
    )
}

fn send_lsp_notification<W: Write>(
    output: &mut W,
    method: &str,
    params: serde_json::Value,
) -> Result<(), String> {
    write_lsp_message(
        output,
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }),
    )
}

fn lsp_text_document(message: &serde_json::Value, field: &str) -> Option<(String, String)> {
    let doc = message.get("params")?.get(field)?;
    let uri = doc.get("uri")?.as_str()?.to_string();
    let text = doc.get("text")?.as_str()?.to_string();
    Some((uri, text))
}

fn lsp_document_uri(message: &serde_json::Value, field: &str) -> Option<String> {
    message
        .get("params")?
        .get(field)?
        .get("uri")?
        .as_str()
        .map(|uri| uri.to_string())
}

fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let raw = uri.strip_prefix("file://")?;
    let decoded = percent_decode(raw);
    let path = if cfg!(windows) && decoded.starts_with('/') && decoded.get(2..3) == Some(":") {
        decoded[1..].to_string()
    } else {
        decoded
    };
    Some(PathBuf::from(
        path.replace('/', std::path::MAIN_SEPARATOR_STR),
    ))
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut idx = 0;
    while idx < bytes.len() {
        if bytes[idx] == b'%'
            && idx + 2 < bytes.len()
            && let Ok(hex) = std::str::from_utf8(&bytes[idx + 1..idx + 3])
            && let Ok(byte) = u8::from_str_radix(hex, 16)
        {
            out.push(byte);
            idx += 3;
            continue;
        }
        out.push(bytes[idx]);
        idx += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn lsp_diagnostic(
    stage: &str,
    message: &str,
    line: Option<usize>,
    column: Option<usize>,
    suggestion: Option<&str>,
) -> serde_json::Value {
    let start_line = line.unwrap_or(1).saturating_sub(1);
    let start_col = column.unwrap_or(1).saturating_sub(1);
    let mut full_message = message.to_string();
    if let Some(suggestion) = suggestion {
        full_message.push_str("\nSuggestion: ");
        full_message.push_str(suggestion);
    }
    serde_json::json!({
        "range": {
            "start": { "line": start_line, "character": start_col },
            "end": { "line": start_line, "character": start_col + 1 }
        },
        "severity": 1,
        "source": format!("amana/{}", stage),
        "message": full_message
    })
}

fn lsp_completion_items() -> serde_json::Value {
    let items = [
        ("app", 14, "Define application metadata."),
        ("theme", 14, "Define design theme tokens."),
        ("import", 14, "Import another Amana file."),
        ("model", 14, "Define a database model."),
        ("view", 14, "Define a server-rendered view."),
        (
            "component",
            14,
            "Define component design settings or reusable component.",
        ),
        ("route", 14, "Map a path to a view."),
        ("form", 14, "Create a server-backed form."),
        ("canvas", 14, "Describe page-level design grammar."),
        ("compose", 14, "Control layout composition."),
        (
            "visual",
            14,
            "Control surface, color, border, and gradients.",
        ),
        (
            "responsive",
            14,
            "Set desktop/laptop/tablet/mobile behavior.",
        ),
        ("direction", 10, "Theme direction: rtl or ltr."),
        ("rtl", 10, "Right-to-left direction token."),
        ("sticky", 10, "CSS position token."),
        ("fixed", 10, "CSS position token."),
        ("layer", 10, "z-index layer token."),
    ];
    serde_json::json!(
        items
            .iter()
            .map(|(label, kind, detail)| {
                serde_json::json!({
                    "label": label,
                    "kind": kind,
                    "detail": detail
                })
            })
            .collect::<Vec<_>>()
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FormatterOutcome {
    changed: bool,
    files_checked: usize,
}

fn run_formatter(
    source_file: &str,
    check: bool,
    all_graph: bool,
) -> Result<FormatterOutcome, CompileFailure> {
    if !all_graph {
        let changed = run_formatter_file(Path::new(source_file), check)?;
        return Ok(FormatterOutcome {
            changed,
            files_checked: 1,
        });
    }

    let entry = normalize_source_path(Path::new(source_file));
    let mut files = BTreeSet::new();
    collect_source_graph_paths(&entry, &mut files)
        .map_err(|err| CompileFailure::new("formatter", err))?;

    let mut changed_any = false;
    for path in &files {
        changed_any |= run_formatter_file(path, check)?;
    }

    Ok(FormatterOutcome {
        changed: changed_any,
        files_checked: files.len(),
    })
}

fn run_formatter_file(source_file: &Path, check: bool) -> Result<bool, CompileFailure> {
    let source = fs::read_to_string(source_file).map_err(|err| {
        CompileFailure::new("formatter", format!("Error reading source file: {}", err))
            .with_file_path(source_file.to_string_lossy().to_string())
    })?;
    let formatted = formatter::format_source(&source);
    let changed = formatted != source;
    if check && changed {
        return Err(CompileFailure::new(
            "formatter",
            format!(
                "File '{}' is not formatted. Run 'amana fmt {}'.",
                source_file.display(),
                source_file.display()
            ),
        )
        .with_file_path(source_file.to_string_lossy().to_string()));
    }
    if changed {
        fs::write(source_file, formatted).map_err(|err| {
            CompileFailure::new(
                "formatter",
                format!("Error writing formatted file: {}", err),
            )
            .with_file_path(source_file.to_string_lossy().to_string())
        })?;
    }
    Ok(changed)
}

fn run_npm_install(output_dir: &str) -> Result<(), String> {
    println!(
        "[Amana] Installing Node.js dependencies in {} ...",
        output_dir
    );
    let status = Command::new("npm")
        .arg("install")
        .current_dir(output_dir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|err| err.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("npm install exited with status {}", status))
    }
}

fn run_npm_dev(output_dir: &str) -> Result<(), String> {
    println!("[Amana] Starting development server in {} ...", output_dir);
    let status = Command::new("npm")
        .args(["run", "dev"])
        .current_dir(output_dir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|err| err.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("npm run dev exited with status {}", status))
    }
}

fn check_generated_runtime(output_dir: &str) -> Result<(), String> {
    let runtime_path = Path::new(output_dir).join("runtime").join("engine.js");
    if !runtime_path.exists() {
        return Err(format!(
            "Generated runtime file '{}' was not found after rebuild.",
            runtime_path.display()
        ));
    }
    let status = Command::new("node")
        .arg("--check")
        .arg(&runtime_path)
        .current_dir(output_dir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|err| err.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "node --check exited with status {} for '{}'",
            status,
            runtime_path.display()
        ))
    }
}

fn start_dev_rebuild_watcher(source_file: String, output_dir: String) {
    println!("[Amana] Watching Amana source graph for rebuilds ...");
    if let Err(err) = thread::Builder::new()
        .name("amana-dev-watch".to_string())
        .spawn(move || {
            let mut last_seen = source_graph_modified_at(&source_file).unwrap_or(UNIX_EPOCH);
            let mut last_build_at = UNIX_EPOCH;
            loop {
                thread::sleep(Duration::from_millis(300));
                let Ok(current) = source_graph_modified_at(&source_file) else {
                    continue;
                };
                if current <= last_seen {
                    continue;
                }
                last_seen = current;
                let now = SystemTime::now();
                if now
                    .duration_since(last_build_at)
                    .unwrap_or_else(|_| Duration::from_secs(0))
                    < Duration::from_millis(900)
                {
                    continue;
                }
                last_build_at = now;
                println!("[Amana] Source changed. Rebuilding {} ...", source_file);
                match build_project(&source_file, &output_dir, false, None) {
                    Ok(_) => match check_generated_runtime(&output_dir) {
                        Ok(_) => println!("[Amana] Rebuild completed. Node runtime will reload."),
                        Err(runtime_err) => eprintln!(
                            "[Amana dev Warning] Rebuild completed but runtime syntax check failed: {}",
                            runtime_err
                        ),
                    },
                    Err(err) => {
                        let (failure, _) = *err;
                        eprintln!("[Amana {} Error] {}", failure.stage, failure.message);
                    }
                }
            }
        })
    {
        eprintln!("[Amana] Failed to start dev watcher: {}", err);
    }
}

fn source_graph_modified_at(source_file: &str) -> Result<SystemTime, String> {
    let entry = normalize_source_path(Path::new(source_file));
    let mut files = BTreeSet::new();
    collect_source_graph_paths(&entry, &mut files)?;
    files
        .iter()
        .filter_map(|path| {
            fs::metadata(path)
                .ok()
                .and_then(|meta| meta.modified().ok())
        })
        .max()
        .ok_or_else(|| format!("No Amana source files found from '{}'.", source_file))
}

fn collect_source_graph_paths(path: &Path, files: &mut BTreeSet<PathBuf>) -> Result<(), String> {
    let normalized = normalize_source_path(path);
    if !files.insert(normalized.clone()) {
        return Ok(());
    }
    let source = fs::read_to_string(&normalized).map_err(|err| {
        format!(
            "Error reading source file '{}': {}",
            normalized.display(),
            err
        )
    })?;
    let (_, imports) = strip_import_lines(&source).map_err(|err| err.message)?;
    let parent = normalized.parent().unwrap_or_else(|| Path::new("."));
    for import in imports {
        let import_path = Path::new(&import.path);
        let resolved = if import_path.is_absolute() {
            normalize_source_path(import_path)
        } else {
            normalize_source_path(&parent.join(import_path))
        };
        collect_source_graph_paths(&resolved, files)?;
    }
    Ok(())
}

fn use_colors() -> bool {
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }
    use std::io::IsTerminal;
    std::io::stderr().is_terminal()
}

fn color_red(text: &str) -> String {
    if use_colors() {
        format!("\x1b[1;31m{}\x1b[0m", text)
    } else {
        text.to_string()
    }
}

fn color_green(text: &str) -> String {
    if use_colors() {
        format!("\x1b[1;32m{}\x1b[0m", text)
    } else {
        text.to_string()
    }
}

fn color_blue(text: &str) -> String {
    if use_colors() {
        format!("\x1b[1;34m{}\x1b[0m", text)
    } else {
        text.to_string()
    }
}

fn color_cyan(text: &str) -> String {
    if use_colors() {
        format!("\x1b[1;36m{}\x1b[0m", text)
    } else {
        text.to_string()
    }
}

fn color_bold(text: &str) -> String {
    if use_colors() {
        format!("\x1b[1m{}\x1b[0m", text)
    } else {
        text.to_string()
    }
}

fn exit_with_compile_error(err: CompileFailure, source: Option<&str>, json: bool) -> ! {
    if json {
        print_json_diagnostic(err.to_json_diagnostic());
    } else {
        eprintln!("{} {}{}{}", color_red("error"), color_bold("[Amana "), color_red(err.stage), color_bold(" Error]"));
        eprintln!("{}", color_bold(&err.message));
        if let Some(source) = source
            && let (Some(line), Some(column)) = (err.line, err.column)
        {
            eprintln!("{}", source_excerpt(source, line, column, err.file_path.as_deref()));
        }
        if let Some(suggestion) = &err.suggestion {
            eprintln!("  {} {}", color_cyan("help:"), suggestion);
        }
    }
    process::exit(1);
}

impl CompileFailure {
    fn new(stage: &'static str, message: String) -> Self {
        let (line, column) = extract_line_col(&message).unwrap_or((0, 0));
        let clean_message = clean_location_suffix(&message, line, column);
        let suggestion = infer_suggestion(&clean_message);
        Self {
            stage,
            message: clean_message,
            line: (line > 0).then_some(line),
            column: (column > 0).then_some(column),
            suggestion,
            file_path: None,
        }
    }

    fn with_file_path(mut self, path: String) -> Self {
        self.file_path = Some(path);
        self
    }

    fn to_json_diagnostic(&self) -> JsonDiagnostic {
        JsonDiagnostic {
            ok: false,
            stage: self.stage.to_string(),
            line: self.line,
            column: self.column,
            message: self.message.clone(),
            suggestion: self.suggestion.clone(),
            file_path: self.file_path.clone(),
        }
    }
}

fn print_json_diagnostic(diagnostic: JsonDiagnostic) {
    match serde_json::to_string_pretty(&diagnostic) {
        Ok(json) => println!("{}", json),
        Err(err) => {
            eprintln!("Failed to serialize diagnostic JSON: {}", err);
            process::exit(1);
        }
    }
}

fn print_json_success(success: JsonSuccess) {
    match serde_json::to_string_pretty(&success) {
        Ok(json) => println!("{}", json),
        Err(err) => {
            eprintln!("Failed to serialize success JSON: {}", err);
            process::exit(1);
        }
    }
}

fn extract_line_col(message: &str) -> Option<(usize, usize)> {
    let line_idx = message.find("line ")?;
    let rest = &message[line_idx + "line ".len()..];
    let mut digits = String::new();
    for ch in rest.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else {
            break;
        }
    }
    let line = digits.parse::<usize>().ok()?;
    let after_line = &rest[digits.len()..];
    let column = if let Some(colon_rest) = after_line.strip_prefix(':') {
        let mut col_digits = String::new();
        for ch in colon_rest.chars() {
            if ch.is_ascii_digit() {
                col_digits.push(ch);
            } else {
                break;
            }
        }
        col_digits.parse::<usize>().unwrap_or(1)
    } else {
        1
    };
    Some((line, column))
}

fn clean_location_suffix(message: &str, line: usize, column: usize) -> String {
    if line == 0 {
        return message.to_string();
    }
    let suffix_with_col = format!(" at line {}:{}", line, column.max(1));
    if let Some(stripped) = message.strip_suffix(&suffix_with_col) {
        return stripped.trim_end().to_string();
    }
    let suffix = format!(" at line {}", line);
    if let Some(stripped) = message.strip_suffix(&suffix) {
        return stripped.trim_end().to_string();
    }
    message.to_string()
}

fn infer_suggestion(message: &str) -> Option<String> {
    let prefix = "Component calls without children can be written as ";
    if let Some(rest) = message.strip_prefix(prefix) {
        let call = rest.trim_end_matches('.').trim();
        let component = call.trim_end_matches("()");
        if !component.is_empty() {
            return Some(format!(
                "Use {}(): only when the component has children.",
                component
            ));
        }
    }
    None
}

fn source_excerpt(source: &str, line: usize, column: usize, file_path: Option<&str>) -> String {
    let lines: Vec<&str> = source.lines().collect();
    if line == 0 || line > lines.len() {
        return String::new();
    }
    let mut excerpt = String::new();
    
    let display_path = file_path.unwrap_or("source.amana");
    excerpt.push_str(&format!("  {} {}:{}:{}\n", color_blue("-->"), display_path, line, column));
    
    let start_line = line.saturating_sub(2);
    let end_line = (line + 1).min(lines.len());
    let max_ln_width = end_line.to_string().len();
    
    excerpt.push_str(&format!("   {} \n", color_blue("|")));
    for (i, line_text) in lines.iter().enumerate().take(end_line).skip(start_line) {
        let curr_line_no = i + 1;
        if curr_line_no == line {
            excerpt.push_str(&format!(
                " {} {} {}\n",
                color_red(">"),
                color_blue(&format!("{:>width$} |", curr_line_no, width = max_ln_width)),
                line_text
            ));
            let caret = if use_colors() {
                "\x1b[1;31m^\x1b[0m".to_string()
            } else {
                "^".to_string()
            };
            excerpt.push_str(&format!(
                "   {} {}{}\n",
                color_blue(&format!("{:>width$} |", "", width = max_ln_width)),
                " ".repeat(column.saturating_sub(1)),
                caret
            ));
        } else {
            excerpt.push_str(&format!(
                "   {} {}\n",
                color_blue(&format!("{:>width$} |", curr_line_no, width = max_ln_width)),
                line_text
            ));
        }
    }
    excerpt.push_str(&format!("   {}", color_blue("|")));
    excerpt
}

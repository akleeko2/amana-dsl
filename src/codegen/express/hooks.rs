// src/codegen/express/hooks.rs

// Validates developer-provided custom hook modules before code generation.
fn clean_comments(content: &str) -> String {
    let mut clean = String::new();
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let chars: Vec<char> = content.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if in_line_comment {
            if chars[i] == '\n' {
                in_line_comment = false;
                clean.push('\n');
            }
        } else if in_block_comment {
            if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '/' {
                in_block_comment = false;
                i += 1;
            }
        } else {
            if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
                in_line_comment = true;
                i += 1;
            } else if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' {
                in_block_comment = true;
                i += 1;
            } else {
                clean.push(chars[i]);
            }
        }
        i += 1;
    }
    clean
}

fn extract_exports(clean: &str) -> Option<String> {
    if let Some(idx) = clean.find("module.exports")
        && let Some(start_brace) = clean[idx..].find('{')
    {
        let body_start = idx + start_brace + 1;
        let mut depth = 1;
        let mut end_idx = body_start;
        let chars: Vec<char> = clean[body_start..].chars().collect();
        for (offset, c) in chars.iter().enumerate() {
            if *c == '{' {
                depth += 1;
            } else if *c == '}' {
                depth -= 1;
                if depth == 0 {
                    end_idx = body_start + offset;
                    break;
                }
            }
        }
        if depth == 0 {
            return Some(clean[body_start..end_idx].to_string());
        }
    }
    None
}

fn validate_exports_body(body: &str) -> Result<(), String> {
    let chars: Vec<char> = body.chars().collect();
    let mut i = 0;
    let mut depth = 0;
    let mut in_string: Option<char> = None;

    while i < chars.len() {
        let c = chars[i];

        if let Some(quote) = in_string {
            if c == quote && (i == 0 || chars[i - 1] != '\\') {
                in_string = None;
            }
        } else {
            if c == '\'' || c == '"' || c == '`' {
                in_string = Some(c);
            } else if c == '{' {
                depth += 1;
            } else if c == '}' {
                depth -= 1;
            } else if depth == 0 && (c == ':' || c == '(') {
                let mut j = i;
                while j > 0 && chars[j - 1].is_whitespace() {
                    j -= 1;
                }
                let mut ident = String::new();
                while j > 0
                    && (chars[j - 1].is_alphanumeric()
                        || chars[j - 1] == '_'
                        || chars[j - 1] == '$')
                {
                    ident.insert(0, chars[j - 1]);
                    j -= 1;
                }

                let mut is_dot = false;
                let mut k = j;
                while k > 0 && chars[k - 1].is_whitespace() {
                    k -= 1;
                }
                if k > 0 && chars[k - 1] == '.' {
                    is_dot = true;
                }

                if !ident.is_empty() && !is_dot {
                    if ident == "beforeAll" {
                        let start_paren = if chars[i] == '(' {
                            Some(i)
                        } else {
                            let mut k = i + 1;
                            let mut found = None;
                            while k < chars.len() {
                                if chars[k] == '(' {
                                    found = Some(k);
                                    break;
                                }
                                if !chars[k].is_whitespace()
                                    && chars[k] != 'a'
                                    && chars[k] != 's'
                                    && chars[k] != 'y'
                                    && chars[k] != 'n'
                                    && chars[k] != 'c'
                                    && chars[k] != 'f'
                                    && chars[k] != 'u'
                                    && chars[k] != 'n'
                                    && chars[k] != 'c'
                                    && chars[k] != 't'
                                    && chars[k] != 'i'
                                    && chars[k] != 'o'
                                    && chars[k] != 'n'
                                {
                                    break;
                                }
                                k += 1;
                            }
                            found
                        };

                        if let Some(sp) = start_paren {
                            let mut ep = sp + 1;
                            let mut paren_depth = 1;
                            while ep < chars.len() {
                                if chars[ep] == '(' {
                                    paren_depth += 1;
                                } else if chars[ep] == ')' {
                                    paren_depth -= 1;
                                    if paren_depth == 0 {
                                        break;
                                    }
                                }
                                ep += 1;
                            }
                            if ep < chars.len() {
                                let args_str: String = chars[sp + 1..ep].iter().collect();
                                let args: Vec<&str> = args_str
                                    .split(',')
                                    .map(|s| s.trim())
                                    .filter(|s| !s.is_empty())
                                    .collect();
                                if args.len() != 3 {
                                    return Err(format!(
                                        "Custom hook signature mismatch: 'beforeAll' must accept exactly 3 parameters (req, res, next). Found {} parameters: {:?}",
                                        args.len(),
                                        args
                                    ));
                                }
                            }
                        }
                    } else if ident != "function" && ident != "async" {
                        return Err(format!(
                            "Custom hook contract violation: Unrecognized hook '{}'. Only 'beforeAll' is allowed.",
                            ident
                        ));
                    }
                }
            }
        }
        i += 1;
    }
    Ok(())
}

/// Validates developer-defined custom hooks file (`hooks.js`) to ensure signatures and hook names conform to Amana specifications.
pub fn validate_custom_hooks(content: &str) -> Result<(), String> {
    let clean = clean_comments(content);
    if let Some(exports_body) = extract_exports(&clean) {
        validate_exports_body(&exports_body)?;
    }
    Ok(())
}

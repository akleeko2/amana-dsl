pub fn format_source(source: &str) -> String {
    let normalized = source.replace("\r\n", "\n").replace('\t', "    ");
    let indent_unit = detect_indent_unit(&normalized);
    let mut lines: Vec<String> = normalized
        .lines()
        .map(|line| normalize_line_indent(line.trim_end(), indent_unit))
        .collect();

    normalize_self_closing_component_calls(&mut lines);
    collapse_blank_lines(&lines)
}

fn normalize_line_indent(line: &str, indent_unit: usize) -> String {
    if line.trim().is_empty() {
        return String::new();
    }
    let leading_spaces = line.chars().take_while(|c| *c == ' ').count();
    let indent_level = leading_spaces.div_ceil(indent_unit.max(1));
    format!("{}{}", " ".repeat(indent_level * 4), line.trim_start())
}

fn detect_indent_unit(source: &str) -> usize {
    source
        .lines()
        .filter_map(|line| {
            let spaces = line.chars().take_while(|c| *c == ' ').count();
            (spaces > 0 && !line.trim().is_empty()).then_some(spaces)
        })
        .reduce(gcd)
        .unwrap_or(4)
        .clamp(1, 4)
}

fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

fn normalize_self_closing_component_calls(lines: &mut [String]) {
    for idx in 0..lines.len() {
        let trimmed = lines[idx].trim_start();
        if !is_component_call_with_empty_colon(trimmed) {
            continue;
        }
        let current_indent = leading_spaces(&lines[idx]);
        let has_children = lines
            .iter()
            .skip(idx + 1)
            .find(|line| !line.trim().is_empty())
            .is_some_and(|next| leading_spaces(next) > current_indent);
        if !has_children {
            lines[idx] = lines[idx].trim_end_matches(':').to_string();
        }
    }
}

fn is_component_call_with_empty_colon(trimmed: &str) -> bool {
    if !trimmed.ends_with("):") {
        return false;
    }
    let Some(first_char) = trimmed.chars().next() else {
        return false;
    };
    first_char.is_ascii_uppercase() && trimmed.contains('(')
}

fn leading_spaces(line: &str) -> usize {
    line.chars().take_while(|c| *c == ' ').count()
}

fn collapse_blank_lines(lines: &[String]) -> String {
    let mut out = String::new();
    let mut blank_count = 0;
    for line in lines {
        if line.trim().is_empty() {
            blank_count += 1;
            if blank_count <= 2 {
                out.push('\n');
            }
            continue;
        }
        blank_count = 0;
        out.push_str(line);
        out.push('\n');
    }
    while out.starts_with('\n') {
        out.remove(0);
    }
    if out.is_empty() { String::new() } else { out }
}

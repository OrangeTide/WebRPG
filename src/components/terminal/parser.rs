//! Command-line parsing for the COMMAND.COM terminal.

/// Parse a command line into (command_name, args_string).
/// Respects quoted strings but keeps parsing simple.
pub(super) fn parse_command_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (cmd, rest) = match trimmed.find(|c: char| c.is_whitespace()) {
        Some(pos) => (&trimmed[..pos], trimmed[pos..].trim_start()),
        None => (trimmed, ""),
    };
    Some((cmd.to_uppercase(), rest.to_string()))
}

/// Parse args respecting `--` stop and `-` switches.
/// Returns (switches, positional_args).
pub(super) fn parse_args(args_str: &str) -> (Vec<String>, Vec<String>) {
    let mut switches = Vec::new();
    let mut positional = Vec::new();
    let mut stop_switches = false;

    for token in shell_split(args_str) {
        if stop_switches {
            positional.push(token);
        } else if token == "--" {
            stop_switches = true;
        } else if token.starts_with('-') && token.len() > 1 {
            switches.push(token);
        } else {
            positional.push(token);
        }
    }
    (switches, positional)
}

/// Simple shell-style token splitting (handles double quotes, no escaping).
fn shell_split(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let chars = input.chars();

    for c in chars {
        match c {
            '"' => in_quotes = !in_quotes,
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

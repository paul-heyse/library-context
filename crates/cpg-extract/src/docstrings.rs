//! Docstrings as Pyrefly finds them, and where a documented parameter's description lies in a
//! docstring literal's source: the recognizer behind parameter documentation (slice 2.1).
use pyrefly_python::docstring::Docstring;
use ruff_python_ast::Stmt;
use ruff_text_size::TextRange;

/// The body's docstring and its statement's range, where Pyrefly finds one
/// (`Docstring::range_from_stmts`, H1 C9).
pub(crate) fn docstring(body: &[Stmt]) -> Option<(String, TextRange)> {
    let range = Docstring::range_from_stmts(body)?;
    let Some(Stmt::Expr(e)) = body.first() else {
        return None;
    };
    let text = e.value.as_string_literal_expr()?.value.to_str().to_owned();
    Some((text, range))
}

/// Where a parameter's description lies in a docstring literal's source (slice 2.1 review F2,
/// F3).
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Located {
    /// The entry's byte span, whose lines are Pyrefly's text.
    Exact(usize, usize),
    /// The entry's byte span and its text, of which Pyrefly's text is a line prefix: its Google
    /// parser splits an entry at a deeper-indented continuation line holding a colon.
    Extended(usize, usize, String),
}

/// The string body of a literal's source: after its prefix and opening quotes, before its closing
/// ones.
fn literal_body(literal: &str) -> (usize, usize) {
    let prefix = literal
        .bytes()
        .take_while(|b| matches!(b, b'r' | b'R' | b'u' | b'U' | b'b' | b'B' | b'f' | b'F'))
        .count();
    let rest = &literal[prefix..];
    let quote = ["\"\"\"", "'''", "\"", "'"]
        .into_iter()
        .find(|q| rest.starts_with(q) && rest.len() >= 2 * q.len() && rest.ends_with(q))
        .map_or(0, str::len);
    (prefix + quote, literal.len() - quote)
}

fn leading_spaces(line: &str) -> usize {
    line.bytes().take_while(|b| *b == b' ').count()
}

/// The offset of the first non-blank byte at or after `from` in `line`.
fn skip_blank(line: &str, from: usize) -> usize {
    from + (line[from..].len() - line[from..].trim_start().len())
}

/// Where a documented parameter's entry begins on `line`: the byte offset of its description, if
/// the line is the entry's header (Sphinx `:param [type] name:`, or Google `name:` / `name
/// (type):`), read as Pyrefly's parsers read it.
fn entry_header(line: &str, name: &str) -> Option<usize> {
    let indent = line.len() - line.trim_start().len();
    let trimmed = &line[indent..];
    if let Some(rest) = trimmed.strip_prefix(":param") {
        let (part, _) = rest.split_once(':')?;
        let token = part
            .split_whitespace()
            .last()?
            .trim_matches(',')
            .trim_start_matches('*');
        let colon = indent + ":param".len() + part.len() + 1;
        return (token == name).then(|| skip_blank(line, colon));
    }
    let (header, _) = trimmed.split_once(':')?;
    let token = header
        .split_whitespace()
        .next()?
        .split('(')
        .next()?
        .trim()
        .trim_start_matches('*');
    let colon = indent + header.len() + 1;
    (token == name).then(|| skip_blank(line, colon))
}

/// Locate a parameter's description, as Pyrefly parsed it, in a docstring literal's source (slice
/// 2.1 review F2, F3). The entry is anchored on its header line, never on a substring of the
/// name, and runs over the following lines indented deeper than the header, up to a blank line
/// (or, in Sphinx style, a `:` field). It is accepted only when its trimmed lines equal Pyrefly's
/// (`Exact`), or begin with them (`Extended`: Pyrefly cut it short). `None` when no header, or
/// more than one, qualifies: the description is then a boundary, never guessed.
pub(crate) fn locate_description(literal: &str, name: &str, text: &str) -> Option<Located> {
    let wanted: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if wanted.is_empty() {
        return None;
    }
    let (open, close) = literal_body(literal);
    let body = &literal[open..close];
    let mut lines: Vec<(usize, &str)> = Vec::new();
    let mut at = 0;
    for line in body.split('\n') {
        lines.push((open + at, line.trim_end_matches('\r')));
        at += line.len() + 1;
    }
    let mut found = Vec::new();
    for (k, &(base, line)) in lines.iter().enumerate() {
        let Some(desc) = entry_header(line, name) else {
            continue;
        };
        let sphinx = line.trim_start().starts_with(":param");
        let indent = leading_spaces(line);
        // (start, end, trimmed text) of each non-empty piece of the entry.
        let mut pieces: Vec<(usize, usize, &str)> = Vec::new();
        let first = line[desc..].trim_end();
        if !first.is_empty() {
            pieces.push((base + desc, base + desc + first.len(), first));
        }
        for &(b, next) in &lines[k + 1..] {
            let trimmed = next.trim();
            if trimmed.is_empty()
                || leading_spaces(next) <= indent
                || (sphinx && trimmed.starts_with(':'))
            {
                break;
            }
            let from = b + (next.len() - next.trim_start().len());
            pieces.push((from, from + trimmed.len(), trimmed));
        }
        let texts: Vec<&str> = pieces.iter().map(|p| p.2).collect();
        if texts.len() < wanted.len() || texts[..wanted.len()] != wanted[..] {
            continue;
        }
        let (start, end) = (pieces[0].0, pieces[pieces.len() - 1].1);
        found.push(if texts.len() == wanted.len() {
            Located::Exact(start, end)
        } else {
            Located::Extended(start, end, texts.join("\n"))
        });
    }
    if found.len() == 1 { found.pop() } else { None }
}

#[cfg(test)]
mod docstring_tests {
    use super::{Located, locate_description};

    fn at(literal: &str, located: Option<Located>) -> String {
        match located.expect("located") {
            Located::Exact(a, z) => literal[a..z].to_owned(),
            Located::Extended(a, z, text) => format!("{}|{text}", &literal[a..z]),
        }
    }

    #[test]
    fn a_description_span_covers_its_lines_verbatim() {
        let literal = "\"\"\"Do it.\n\n    Args:\n        name: The name; it must\n            not be empty.\n        size: How many.\n    \"\"\"";
        assert_eq!(
            at(
                literal,
                locate_description(literal, "name", "The name; it must\nnot be empty.")
            ),
            "The name; it must\n            not be empty."
        );
        assert_eq!(
            at(literal, locate_description(literal, "size", "How many.")),
            "How many."
        );
        assert_eq!(locate_description(literal, "size", "Not there."), None);
    }

    /// Review F3's probe: a name that is a substring of another's (`out` in `timeout`) anchors on
    /// its own entry's header, never on the other's line.
    #[test]
    fn a_description_is_anchored_on_its_own_header() {
        let literal = "\"\"\"Wait.\n\n    Args:\n        timeout: Optional.\n        out: Optional.\n    \"\"\"";
        let located = locate_description(literal, "out", "Optional.").unwrap();
        let Located::Exact(a, _) = located else {
            panic!("{located:?}")
        };
        assert!(literal[..a].ends_with("    out: "), "{:?}", &literal[..a]);
        // Two headers qualifying is ambiguous: no span.
        let twice = "\"\"\"Args:\n    out: Optional.\n    out: Optional.\n\"\"\"";
        assert_eq!(locate_description(twice, "out", "Optional."), None);
    }

    /// Review F2: Pyrefly's Google parser ends an entry at a deeper continuation line holding a
    /// colon; the entry is extended to its end by indentation.
    #[test]
    fn a_description_cut_at_a_colon_line_is_extended() {
        let literal = "\"\"\"Proxy.\n\n    Args:\n        consent: Consent screen behavior.\n            - True: always ask\n            - \"remember\": ask once\n        other: Else.\n    \"\"\"";
        let docs = pyrefly_python::docstring::parse_parameter_documentation(literal);
        assert_eq!(docs["consent"], "Consent screen behavior.");
        assert_eq!(
            at(
                literal,
                locate_description(literal, "consent", &docs["consent"])
            ),
            "Consent screen behavior.\n            - True: always ask\n            - \"remember\": ask once\
             |Consent screen behavior.\n- True: always ask\n- \"remember\": ask once"
        );
    }

    #[test]
    fn a_sphinx_description_is_located() {
        let literal = "'''Do it.\n\n:param int size: How many\n    slots.\n:returns: nothing\n'''";
        let docs = pyrefly_python::docstring::parse_parameter_documentation(literal);
        assert_eq!(
            at(literal, locate_description(literal, "size", &docs["size"])),
            "How many\n    slots."
        );
    }
}

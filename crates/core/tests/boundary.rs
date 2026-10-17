//! Boundary tests: they read the repository's own files and fail when the
//! architecture is broken. Each constant below is labelled with the category
//! it belongs to in BUILDER-SPEC.md.
//!
//! - Builder requirement: the core performs no input or output; its
//!   dependencies are allowlisted; the shim decides nothing; the page keeps no
//!   copy of a rule and names no other origin.
//! - Reference default: the core uses no floats, hash maps, clocks or
//!   randomness; the page JavaScript holds no numeric literal other than 0 and
//!   1; shaders under data/shaders/ hold no numeric literal other than 0 and
//!   1; the page carries a Content-Security-Policy; the page neither navigates
//!   away on its own nor behaves differently under automation. Change these
//!   only by recording the change in SECOND-ORDER.md and BUILDER-SPEC.md.
//!
//! The page scans read every file under web/ at any depth, except a
//! directory named `vendor`, whose files are pinned by SHA256SUMS instead
//! (scripts/build.sh checks them).
//!
//! These tests do file I/O themselves. That is fine: they are tests, not the
//! core library, and only `crates/core/src` is held to the core rules.
//!
//! When a rule here seems to block necessary work, stop and say so in
//! SECOND-ORDER.md as a worklist row. Do not loosen the list to make it pass.

use std::fs;
use std::path::{Path, PathBuf};

/// Project decision, enforced as a Builder requirement: the crates the core
/// may use. Add to it only with the person's approval, recorded in PLAN.md.
const ALLOWED_DEPENDENCIES: &[&str] = &["serde", "serde_json"];

/// Builder requirement: names that mean the core is doing input or output.
const IO_IDENTIFIERS: &[&str] = &[
    "fs",
    "io",
    "net",
    "env",
    "process",
    "thread",
    "File",
    "stdin",
    "stdout",
    "stderr",
    "println",
    "print",
    "eprintln",
    "eprint",
    "dbg",
    "web_sys",
    "js_sys",
    "wasm_bindgen",
];

/// Reference default: names that make the same input give different results
/// on different runs or machines.
const NONDETERMINISM_IDENTIFIERS: &[(&str, &str)] = &[
    ("f32", "float"),
    ("f64", "float"),
    ("as_f64", "float"),
    ("from_f64", "float"),
    ("is_f64", "float"),
    ("HashMap", "hash map"),
    ("HashSet", "hash map"),
    ("hash_map", "hash map"),
    ("hash_set", "hash map"),
    ("RandomState", "hash map"),
    ("DefaultHasher", "hash map"),
    ("SystemTime", "clock"),
    ("Instant", "clock"),
    ("UNIX_EPOCH", "clock"),
    ("time", "clock"),
    ("chrono", "clock"),
    ("rand", "randomness"),
    ("thread_rng", "randomness"),
    ("getrandom", "randomness"),
];

fn forbidden_in_core(token: &str) -> Option<&'static str> {
    if IO_IDENTIFIERS.contains(&token) {
        return Some("input/output");
    }
    NONDETERMINISM_IDENTIFIERS
        .iter()
        .find(|(w, _)| *w == token)
        .map(|(_, why)| *why)
}

/// Builder requirement: words that mean the shim is deciding, not forwarding.
const SHIM_DECISION_WORDS: &[&str] = &["if", "else", "match", "for", "while", "loop"];

/// Builder requirement (network APIs, code from strings) and reference default
/// (arithmetic): names that mean the page is computing a rule or reaching
/// beyond the same-origin `fetch` of its own files.
const PAGE_FORBIDDEN_IDENTIFIERS: &[(&str, &str)] = &[
    ("Math", "arithmetic belongs in the core"),
    ("parseInt", "number parsing belongs in the core"),
    ("parseFloat", "number parsing belongs in the core"),
    ("Number", "number parsing belongs in the core"),
    ("toFixed", "rounding belongs in the core"),
    (
        "XMLHttpRequest",
        "no network requests beyond the page's own files",
    ),
    ("WebSocket", "no network requests beyond the page's own files"),
    ("EventSource", "no network requests beyond the page's own files"),
    (
        "RTCPeerConnection",
        "no network requests beyond the page's own files",
    ),
    ("sendBeacon", "no network requests beyond the page's own files"),
    ("importScripts", "no code from another file at run time"),
    ("eval", "no code from strings"),
    ("Function", "no code from strings"),
    (
        "webdriver",
        "the page must behave the same under the browser check",
    ),
];

/// Builder requirement (R7): ways a page can send the reader, and data, to
/// another address. A meta CSP cannot stop navigation, so the scan does. The
/// scan reads code with strings blanked, so `location.href` may still be read.
const PAGE_NAVIGATION: &[&str] = &[
    "location.href =",
    "location.href=",
    "location.assign",
    "location.replace",
    "location =",
    "location=",
    "window.open",
];

/// Absolute URLs that are names, not addresses: the page may write them.
const ALLOWED_ABSOLUTE_URLS: &[&str] = &["http://www.w3.org/2000/svg"];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn rust_files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let mut entries: Vec<PathBuf> = fs::read_dir(&d)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", d.display()))
            .map(|e| e.expect("dir entry").path())
            .collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().map(|x| x == "rs").unwrap_or(false) {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// Every dependency name declared in a Cargo.toml text, in any section whose
/// name ends in `dependencies` (normal, dev, build, target-specific), in both
/// `[dependencies]` + `name = ...` form and `[dependencies.name]` form.
fn declared_dependencies(toml: &str) -> Vec<String> {
    let mut deps = Vec::new();
    let mut in_deps_table = false;
    for raw in toml.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            let header = line.trim_matches(|c| c == '[' || c == ']').trim();
            let parts: Vec<&str> = header.split('.').map(|p| p.trim()).collect();
            in_deps_table = false;
            if let Some(pos) = parts.iter().position(|p| p.ends_with("dependencies")) {
                if pos + 1 < parts.len() {
                    deps.push(parts[pos + 1].trim_matches('"').to_string());
                } else {
                    in_deps_table = true;
                }
            }
            continue;
        }
        if in_deps_table {
            if let Some((key, _)) = line.split_once('=') {
                deps.push(key.trim().trim_matches('"').to_string());
            }
        }
    }
    deps
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Replaces comments and the contents of string and char literals with
/// spaces, keeping newlines so line numbers stay right.
fn strip_comments_and_strings(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let blank = |c: char| if c == '\n' { '\n' } else { ' ' };
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        // line comment
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                out.push(' ');
                i += 1;
            }
            continue;
        }
        // block comment (nested)
        if c == '/' && next == Some('*') {
            let mut depth = 0;
            while i < chars.len() {
                if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    out.push_str("  ");
                    i += 2;
                } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    out.push_str("  ");
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    out.push(blank(chars[i]));
                    i += 1;
                }
            }
            continue;
        }
        // raw string r"..." / r#"..."# (also br"...")
        let prev = if i > 0 { Some(chars[i - 1]) } else { None };
        let prev2 = if i > 1 { Some(chars[i - 2]) } else { None };
        let starts_token = match prev {
            None => true,
            Some(p) if !is_ident_char(p) => true,
            Some('b') => prev2.map(|q| !is_ident_char(q)).unwrap_or(true),
            _ => false,
        };
        if c == 'r' && starts_token && (next == Some('"') || next == Some('#')) {
            let mut j = i + 1;
            let mut hashes = 0;
            while chars.get(j) == Some(&'#') {
                hashes += 1;
                j += 1;
            }
            if chars.get(j) == Some(&'"') {
                out.push(' ');
                for _ in 0..hashes {
                    out.push(' ');
                }
                out.push(' ');
                j += 1;
                loop {
                    if j >= chars.len() {
                        break;
                    }
                    if chars[j] == '"' && (0..hashes).all(|k| chars.get(j + 1 + k) == Some(&'#')) {
                        for _ in 0..=hashes {
                            out.push(' ');
                        }
                        j += 1 + hashes;
                        break;
                    }
                    out.push(blank(chars[j]));
                    j += 1;
                }
                i = j;
                continue;
            }
        }
        // normal string
        if c == '"' {
            out.push(' ');
            i += 1;
            while i < chars.len() {
                if chars[i] == '\\' {
                    out.push(' ');
                    if i + 1 < chars.len() {
                        out.push(blank(chars[i + 1]));
                    }
                    i += 2;
                    continue;
                }
                if chars[i] == '"' {
                    out.push(' ');
                    i += 1;
                    break;
                }
                out.push(blank(chars[i]));
                i += 1;
            }
            continue;
        }
        // char literal (not a lifetime)
        if c == '\'' {
            if next == Some('\\') {
                // skip the backslash and the escaped character, then find the close
                let mut j = i + 3;
                while j < chars.len() && chars[j] != '\'' {
                    j += 1;
                }
                for _ in i..=j.min(chars.len() - 1) {
                    out.push(' ');
                }
                i = j + 1;
                continue;
            }
            if chars.get(i + 2) == Some(&'\'') {
                out.push_str("   ");
                i += 3;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// One problem found by a scan: line number (1-based), the token, and why.
#[derive(Debug, Clone, PartialEq)]
struct Violation {
    line: usize,
    token: String,
    why: String,
}

fn is_float_literal(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    if lower.starts_with("0x") || lower.starts_with("0o") || lower.starts_with("0b") {
        return false;
    }
    if lower.ends_with("f32") || lower.ends_with("f64") {
        return true;
    }
    let b: Vec<char> = lower.chars().collect();
    for k in 1..b.len() {
        let before_is_digit = b[k - 1].is_ascii_digit();
        let after = b.get(k + 1).copied();
        if b[k] == '.' && before_is_digit && after.map(|a| a.is_ascii_digit()).unwrap_or(false) {
            return true;
        }
        if b[k] == 'e'
            && before_is_digit
            && after
                .map(|a| a.is_ascii_digit() || a == '+' || a == '-')
                .unwrap_or(true)
        {
            return true;
        }
    }
    // a trailing dot like `1.` is a float literal too (but `1..` is a range)
    lower.ends_with('.') && !lower.ends_with("..")
}

/// Scans stripped core code for forbidden identifiers and float literals.
fn scan_core_code(src: &str) -> Vec<Violation> {
    let code = strip_comments_and_strings(src);
    let mut found = Vec::new();
    for (n, line) in code.lines().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            let prev = if i > 0 { Some(chars[i - 1]) } else { None };
            if c.is_ascii_digit() && !prev.map(|p| is_ident_char(p) || p == '.').unwrap_or(false) {
                let start = i;
                while i < chars.len() && (is_ident_char(chars[i]) || chars[i] == '.') {
                    i += 1;
                }
                let token: String = chars[start..i].iter().collect();
                if is_float_literal(&token) {
                    found.push(Violation {
                        line: n + 1,
                        token,
                        why: "float".into(),
                    });
                }
                continue;
            }
            if is_ident_char(c) && !c.is_ascii_digit() {
                let start = i;
                while i < chars.len() && is_ident_char(chars[i]) {
                    i += 1;
                }
                let token: String = chars[start..i].iter().collect();
                if let Some(why) = forbidden_in_core(&token) {
                    found.push(Violation {
                        line: n + 1,
                        token,
                        why: why.to_string(),
                    });
                }
                continue;
            }
            i += 1;
        }
    }
    found
}

/// Scans stripped shim code for decision keywords and the `?` operator.
fn scan_shim_code(src: &str) -> Vec<Violation> {
    let code = strip_comments_and_strings(src);
    let mut found = Vec::new();
    for (n, line) in code.lines().enumerate() {
        for word in line.split(|c: char| !is_ident_char(c)) {
            if SHIM_DECISION_WORDS.contains(&word) {
                found.push(Violation {
                    line: n + 1,
                    token: word.to_string(),
                    why: "decision".into(),
                });
            }
        }
        if line.contains('?') {
            found.push(Violation {
                line: n + 1,
                token: "?".into(),
                why: "decision".into(),
            });
        }
    }
    found
}

fn report(path: &Path, vs: &[Violation]) -> Vec<String> {
    vs.iter()
        .map(|v| format!("{}:{}: `{}` ({})", path.display(), v.line, v.token, v.why))
        .collect()
}

/// Replaces JS comments and the contents of string and template literals
/// with spaces, keeping newlines. (Template `${}` parts count as string text;
/// the page does not use template literals.)
fn strip_js(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let blank = |c: char| if c == '\n' { '\n' } else { ' ' };
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                out.push(' ');
                i += 1;
            }
            continue;
        }
        if c == '/' && next == Some('*') {
            out.push_str("  ");
            i += 2;
            while i < chars.len() && !(chars[i] == '*' && chars.get(i + 1) == Some(&'/')) {
                out.push(blank(chars[i]));
                i += 1;
            }
            out.push_str("  ");
            i += 2;
            continue;
        }
        if c == '"' || c == '\'' || c == '`' {
            out.push(' ');
            i += 1;
            while i < chars.len() && chars[i] != c {
                if chars[i] == '\\' {
                    out.push(' ');
                    i += 1;
                }
                if i < chars.len() {
                    out.push(blank(chars[i]));
                    i += 1;
                }
            }
            out.push(' ');
            i += 1;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Scans page JavaScript for rule-like code: numeric literals other than 0
/// and 1, and the forbidden names above.
fn scan_page_js(src: &str) -> Vec<Violation> {
    let code = strip_js(src);
    let mut found = Vec::new();
    for (n, line) in code.lines().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            let prev = if i > 0 { Some(chars[i - 1]) } else { None };
            let after_ident = prev
                .map(|p| is_ident_char(p) || p == '$' || p == '.')
                .unwrap_or(false);
            if c.is_ascii_digit() && !after_ident {
                let start = i;
                while i < chars.len() && (is_ident_char(chars[i]) || chars[i] == '.') {
                    i += 1;
                }
                let token: String = chars[start..i].iter().collect();
                if token != "0" && token != "1" {
                    found.push(Violation {
                        line: n + 1,
                        token,
                        why: "number in the page".into(),
                    });
                }
                continue;
            }
            if is_ident_char(c) || c == '$' {
                let start = i;
                while i < chars.len() && (is_ident_char(chars[i]) || chars[i] == '$') {
                    i += 1;
                }
                let token: String = chars[start..i].iter().collect();
                if let Some((_, why)) = PAGE_FORBIDDEN_IDENTIFIERS.iter().find(|(w, _)| *w == token) {
                    found.push(Violation {
                        line: n + 1,
                        token,
                        why: (*why).to_string(),
                    });
                }
                continue;
            }
            i += 1;
        }
    }
    found
}

/// Finds code that navigates the page, with strings and comments blanked.
fn scan_page_navigation(src: &str) -> Vec<Violation> {
    let code = strip_js(src);
    let mut found = Vec::new();
    for (n, line) in code.lines().enumerate() {
        for pattern in PAGE_NAVIGATION {
            let mut from = 0;
            while let Some(at) = line[from..].find(pattern) {
                let start = from + at;
                let end = start + pattern.len();
                // `location ==` is a comparison, not an assignment
                let comparison = pattern.ends_with('=') && line[end..].starts_with('=');
                if !comparison {
                    found.push(Violation {
                        line: n + 1,
                        token: pattern.trim_end_matches([' ', '=']).to_string(),
                        why: "the page navigates; R7".into(),
                    });
                }
                from = end;
            }
        }
    }
    found
}

/// A line of page source with the allowed absolute URLs removed.
fn without_allowed_urls(line: &str) -> String {
    let mut out = line.to_string();
    for url in ALLOWED_ABSOLUTE_URLS {
        out = out.replace(url, "");
    }
    out
}

/// Every file under `dir` at any depth, skipping directories named `vendor`.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(read) = fs::read_dir(&d) else {
            continue;
        };
        for entry in read {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                if path.file_name().map(|n| n != "vendor").unwrap_or(true) {
                    stack.push(path);
                }
            } else {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

fn web_files() -> Vec<PathBuf> {
    files_under(&repo_root().join("web"))
}

/// Scans a shader for numeric literals other than 0 and 1 (and 0.0, 1.0).
/// A number a rule depends on reaches the shader as a uniform from the core.
fn scan_shader(src: &str) -> Vec<Violation> {
    let mut found = Vec::new();
    for (n, raw) in src.lines().enumerate() {
        let line = raw.split("//").next().unwrap_or("");
        if line.trim_start().starts_with("#version") {
            continue;
        }
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            let prev = if i > 0 { Some(chars[i - 1]) } else { None };
            let after_ident = prev.map(is_ident_char).unwrap_or(false);
            let starts_number = c.is_ascii_digit()
                || (c == '.' && chars.get(i + 1).map(|d| d.is_ascii_digit()).unwrap_or(false));
            if starts_number && !after_ident {
                let start = i;
                while i < chars.len() && (is_ident_char(chars[i]) || chars[i] == '.') {
                    i += 1;
                }
                let token: String = chars[start..i].iter().collect();
                let plain = token.trim_end_matches(['f', 'u']);
                if !["0", "1", "0.0", "1.0", "0.", "1."].contains(&plain) {
                    found.push(Violation {
                        line: n + 1,
                        token,
                        why: "number in a shader; pass it as a uniform from the core".into(),
                    });
                }
                continue;
            }
            if is_ident_char(c) {
                while i < chars.len() && is_ident_char(chars[i]) {
                    i += 1;
                }
                continue;
            }
            i += 1;
        }
    }
    found
}

#[test]
fn core_dependencies_are_on_the_allowlist() {
    let path = repo_root().join("crates/core/Cargo.toml");
    let toml = fs::read_to_string(&path).expect("read core Cargo.toml");
    let deps = declared_dependencies(&toml);
    let outside: Vec<&String> = deps
        .iter()
        .filter(|d| !ALLOWED_DEPENDENCIES.contains(&d.as_str()))
        .collect();
    assert!(
        outside.is_empty(),
        "{}: dependencies outside the allowlist {:?}: {:?}",
        path.display(),
        ALLOWED_DEPENDENCIES,
        outside
    );
    assert!(
        !repo_root().join("crates/core/build.rs").exists(),
        "the core must not have a build script"
    );
}

#[test]
fn core_source_has_no_io_and_no_nondeterminism() {
    let src_dir = repo_root().join("crates/core/src");
    let files = rust_files_under(&src_dir);
    assert!(
        !files.is_empty(),
        "no source files found under {}",
        src_dir.display()
    );
    let mut problems = Vec::new();
    for f in &files {
        let text = fs::read_to_string(f).expect("read source");
        problems.extend(report(f, &scan_core_code(&text)));
    }
    assert!(
        problems.is_empty(),
        "core rule violations:\n{}",
        problems.join("\n")
    );
}

#[test]
fn shim_decides_nothing() {
    let src_dir = repo_root().join("crates/shim/src");
    let files = rust_files_under(&src_dir);
    assert!(!files.is_empty(), "no shim source found");
    let mut problems = Vec::new();
    for f in &files {
        let text = fs::read_to_string(f).expect("read shim source");
        problems.extend(report(f, &scan_shim_code(&text)));
    }
    assert!(
        problems.is_empty(),
        "the shim makes decisions:\n{}",
        problems.join("\n")
    );
}

#[test]
fn page_keeps_no_rule_and_names_no_other_origin() {
    let mut problems = Vec::new();
    let mut js_files = 0;
    for path in web_files() {
        let text = fs::read_to_string(&path).expect("read page file");
        for (n, line) in text.lines().enumerate() {
            if without_allowed_urls(line).contains("://") {
                problems.push(format!(
                    "{}:{}: absolute URL (another origin?)",
                    path.display(),
                    n + 1
                ));
            }
        }
        if path.extension().map(|x| x == "js").unwrap_or(false) {
            js_files += 1;
            problems.extend(report(&path, &scan_page_js(&text)));
            problems.extend(report(&path, &scan_page_navigation(&text)));
        }
    }
    assert!(js_files > 0, "no page JavaScript found in web/");
    assert!(problems.is_empty(), "page problems:\n{}", problems.join("\n"));
}

/// Reference default: shaders keep no tuning numbers of their own.
#[test]
fn shaders_hold_no_numbers_of_their_own() {
    let mut problems = Vec::new();
    for path in files_under(&repo_root().join("data/shaders")) {
        let ext = path.extension().and_then(|x| x.to_str()).unwrap_or("");
        if ["vert", "frag", "glsl"].contains(&ext) {
            let text = fs::read_to_string(&path).expect("read shader");
            problems.extend(report(&path, &scan_shader(&text)));
        }
    }
    assert!(problems.is_empty(), "shader problems:\n{}", problems.join("\n"));
}

/// Reference default: every HTML page carries a Content-Security-Policy that
/// allows WebAssembly, keeps connections on the origin, and blocks form posts.
#[test]
fn every_page_sets_the_content_security_policy() {
    let mut pages = 0;
    for path in web_files() {
        if path.extension().map(|x| x == "html").unwrap_or(false) {
            pages += 1;
            let text = fs::read_to_string(&path).expect("read html");
            let csp = csp_of(&text)
                .unwrap_or_else(|| panic!("{}: no Content-Security-Policy meta element", path.display()));
            for needed in [
                "default-src 'self'",
                "connect-src 'self'",
                "form-action 'none'",
                "'wasm-unsafe-eval'",
            ] {
                assert!(
                    csp.contains(needed),
                    "{}: CSP lacks {needed}: {csp}",
                    path.display()
                );
            }
        }
    }
    assert!(pages > 0, "no HTML page found in web/");
}

/// The content of the first `<meta http-equiv="Content-Security-Policy" content="...">`.
fn csp_of(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let at = lower.find("http-equiv=\"content-security-policy\"")?;
    let tag_start = lower[..at].rfind('<')?;
    let tag_end = at + lower[at..].find('>')?;
    let tag = &html[tag_start..tag_end];
    let c = tag.find("content=\"")? + "content=\"".len();
    let rest = &tag[c..];
    Some(rest[..rest.find('"')?].to_string())
}

// ---- The scanners are tests too: each must catch what it claims to. ----

#[test]
fn dependency_reader_sees_every_form() {
    let toml = "[package]\nname = \"x\"\n\n[dependencies]\nserde = \"1\"\n\n[dev-dependencies]\nregex = \"1\"\n\n[build-dependencies.cc]\nversion = \"1\"\n\n[target.'cfg(unix)'.dependencies]\nlibc = \"0.2\"\n";
    assert_eq!(declared_dependencies(toml), vec!["serde", "regex", "cc", "libc"]);
}

#[test]
fn core_scanner_catches_each_kind() {
    let cases = [
        ("let x = 0.5;", "float"),
        ("let y: f64 = 2;", "float"),
        ("let z = 1e3;", "float"),
        ("let w = 3f32;", "float"),
        ("let m = HashMap::new();", "hash map"),
        ("use std::collections::hash_map::Entry;", "hash map"),
        ("let t = std::time::SystemTime::now();", "clock"),
        ("let i = Instant::now();", "clock"),
        ("let s = std::fs::read_to_string(p);", "input/output"),
        ("println!(\"hi\");", "input/output"),
        ("use std::{fs, io};", "input/output"),
        ("use web_sys::window;", "input/output"),
    ];
    for (code, why) in cases {
        let found = scan_core_code(code);
        assert!(
            found.iter().any(|v| v.why == why),
            "expected a `{why}` violation in {code:?}, got {found:?}"
        );
    }
}

#[test]
fn core_scanner_ignores_comments_strings_hex_and_ranges() {
    let code = r##"
// f64 HashMap 0.5 in a line comment
/* SystemTime 1.5 /* nested f32 */ still comment */
let a = "0.5 f64 HashMap";
let b = r#"{"rate": 0.25, "HashMap": "x"}"#;
let c = 0xcbf29ce484222325u64;
for i in 0..10 { let _ = pair.0; }
let d = 'x';
let e = '\'';
fn f<'a>(s: &'a str) -> &'a str { s }
let g = 5.max(3);
let h = 100u32 * 7;
"##;
    let found = scan_core_code(code);
    assert!(found.is_empty(), "false positives: {found:?}");
}

#[test]
fn shim_scanner_catches_decisions() {
    assert!(
        !scan_shim_code("pub fn f(x: &str) -> String { if x.is_empty() { a() } else { b() } }").is_empty()
    );
    assert!(!scan_shim_code("pub fn f(x: &str) -> Result<String, E> { Ok(g(x)?) }").is_empty());
    assert!(!scan_shim_code("pub fn f(x: u32) -> u32 { match x { 0 => 1, _ => x } }").is_empty());
    assert!(
        scan_shim_code("// if this were a decision\npub fn f(x: &str) -> String { core::api::f(x) }")
            .is_empty()
    );
}

#[test]
fn page_scanner_catches_rules_and_ignores_text() {
    assert!(!scan_page_js("if (pct < 70) show();").is_empty());
    assert!(!scan_page_js("const p = Math.floor(a / b);").is_empty());
    assert!(!scan_page_js("navigator.sendBeacon(u, d);").is_empty());
    assert!(!scan_page_js("new WebSocket(u);").is_empty());
    assert!(scan_page_js("// 70 in a comment\nconst s = \"70% of 9\"; x[0] = y[1];").is_empty());
    assert!(!scan_page_js("if (!navigator.webdriver) lock();").is_empty());
}

#[test]
fn navigation_scanner_catches_navigation_and_ignores_reads() {
    assert!(!scan_page_navigation("window.location.href = ui.navigate;").is_empty());
    assert!(!scan_page_navigation("location.assign(next);").is_empty());
    assert!(!scan_page_navigation("window.open(link);").is_empty());
    assert!(scan_page_navigation("const here = location.href;").is_empty());
    assert!(scan_page_navigation("if (location == other) x();").is_empty());
    assert!(scan_page_navigation("// location.href = x in a comment").is_empty());
}

#[test]
fn shader_scanner_catches_tuning_numbers() {
    assert!(!scan_shader("float leaf = base.g > base.r * 1.15 ? 1.0 : 0.0;").is_empty());
    assert!(!scan_shader("vec3 c = mix(a, b, 0.82);").is_empty());
    assert!(!scan_shader("float h = .5;").is_empty());
    assert!(scan_shader("#version 300 es\nprecision highp float;\nout vec4 o; // 0.5 here\nvoid main() { o = vec4(u_colour, 1.0); }").is_empty());
}

#[test]
fn the_svg_namespace_is_a_name_not_an_address() {
    let line = "const ns = \"http://www.w3.org/2000/svg\";";
    assert!(!without_allowed_urls(line).contains("://"));
    assert!(without_allowed_urls("fetch(\"https://example.invalid\")").contains("://"));
}

#[test]
fn csp_reader_finds_the_policy_and_reports_its_absence() {
    let html = "<meta charset=\"utf-8\">\n<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'self'; form-action 'none'\">";
    assert_eq!(
        csp_of(html).as_deref(),
        Some("default-src 'self'; form-action 'none'")
    );
    assert_eq!(csp_of("<meta charset=\"utf-8\">"), None);
}

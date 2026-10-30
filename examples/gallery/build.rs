use std::fmt::Write as _;

fn main() {
    let config = slint_build::CompilerConfiguration::new().with_library_paths(
        std::collections::HashMap::from([("slintplot".into(), slintplot::library_path())]),
    );
    slint_build::compile_with_config("ui/gallery.slint", config).unwrap();

    // The code panel's code, taken from the samples in ui/gallery.slint.
    println!("cargo:rerun-if-changed=ui/gallery.slint");
    let source = std::fs::read_to_string("ui/gallery.slint").unwrap();
    let out = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("sample_code.rs");
    std::fs::write(out, sample_code(&source)).unwrap();
}

/// A Rust table of (sample id, code) for every `if root.sample == "<id>":`
/// element whose element is a chart, or a layout of charts: the element,
/// de-indented, after an import line of the SlintPlot names it uses.
fn sample_code(source: &str) -> String {
    let imports = slintplot_imports(source);
    let lines: Vec<&str> = source.lines().collect();
    let mut table = String::from(
        "/// The Slint code of each sample, from ui/gallery.slint (see build.rs).\n\
         const SAMPLE_CODE: &[(&str, &str)] = &[\n",
    );
    for (start, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("if root.sample == ") else {
            continue;
        };
        let Some((condition, element)) = rest.split_once(": ") else {
            continue;
        };
        let Some(name) = element.strip_suffix(" {") else {
            continue;
        };
        if name != "GridLayout" && !imports.iter().any(|i| i == name) {
            continue;
        }
        let ids: Vec<&str> = condition
            .split(" || ")
            .map(|c| c.trim_start_matches("root.sample == ").trim_matches('"'))
            .collect();
        let end = block_end(&lines, start);
        let indent = line.len() - trimmed.len();
        let mut body = format!("{element}\n");
        for l in &lines[start + 1..=end] {
            body.push_str(l.get(indent..).unwrap_or(l.trim_start()));
            body.push('\n');
        }
        let code = with_header(&body, &imports);
        for id in ids {
            writeln!(table, "    ({id:?}, {code:?}),").unwrap();
        }
    }
    table.push_str("];\n");
    table
}

/// The names in the `import { ... } from "@slintplot";` statement.
fn slintplot_imports(source: &str) -> Vec<String> {
    let end = source.find("} from \"@slintplot\";").unwrap();
    let start = source[..end].rfind("import {").unwrap() + "import {".len();
    source[start..end]
        .split(',')
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .collect()
}

/// The line of the brace that closes the element opened on line `start`;
/// braces in strings and comments don't count.
fn block_end(lines: &[&str], start: usize) -> usize {
    let mut depth = 0;
    for (i, line) in lines.iter().enumerate().skip(start) {
        let mut in_string = false;
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '\\' if in_string => {
                    chars.next();
                }
                '"' => in_string = !in_string,
                '/' if !in_string && chars.peek() == Some(&'/') => break,
                '{' if !in_string => depth += 1,
                '}' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        return i;
                    }
                }
                _ => {}
            }
        }
    }
    panic!("unbalanced braces after line {}", start + 1);
}

/// `body` after the import line of the names it uses, and a comment on the
/// gallery properties it reads.
fn with_header(body: &str, imports: &[String]) -> String {
    let words: Vec<&str> = body
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let mut used: Vec<&str> = imports
        .iter()
        .map(String::as_str)
        .filter(|i| words.contains(i))
        .collect();
    used.sort_unstable();
    let mut header = format!("import {{ {} }} from \"@slintplot\";\n\n", used.join(", "));
    if header.len() > 90 {
        header = format!(
            "import {{\n    {},\n}} from \"@slintplot\";\n\n",
            used.join(",\n    ")
        );
    }

    let mut properties: Vec<String> = Vec::new();
    for (i, _) in body.match_indices("root.") {
        let name: String = body[i + 5..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        let name = format!("root.{name}");
        if name != "root.status" && !properties.contains(&name) {
            properties.push(name);
        }
    }
    let mut text = String::new();
    if !properties.is_empty() {
        let list = match properties.as_slice() {
            [one] => one.clone(),
            [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
            [] => unreachable!(),
        };
        let verb = if properties.len() == 1 {
            "is a property"
        } else {
            "are properties"
        };
        text = format!(
            "{list} {verb} of the gallery window, mostly set from Rust: see \
             examples/gallery/src/main.rs."
        );
    }
    if body.contains("root.status") {
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str("Clicks write what was clicked to root.status, the gallery's footer text.");
    }
    if !text.is_empty() {
        header.push_str(&wrap_comment(&text, 78));
    }
    header + body.trim_end()
}

/// `text` as `//` comment lines of at most `width` characters.
fn wrap_comment(text: &str, width: usize) -> String {
    let mut out = String::new();
    let mut line = String::from("//");
    for word in text.split(' ') {
        if line.len() + 1 + word.len() > width && line.len() > 2 {
            out.push_str(&line);
            out.push('\n');
            line = String::from("//");
        }
        line.push(' ');
        line.push_str(word);
    }
    out.push_str(&line);
    out.push('\n');
    out
}

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
/// de-indented. Also the names imported from SlintPlot, for the code's
/// import line (see src/code.rs).
fn sample_code(source: &str) -> String {
    let imports = slintplot_imports(source);
    let lines: Vec<&str> = source.lines().collect();
    let mut table = format!(
        "/// The names ui/gallery.slint imports from SlintPlot.\n\
         const SLINTPLOT_NAMES: &[&str] = &{imports:?};\n\n\
         /// The Slint code of each sample, from ui/gallery.slint (see build.rs).\n\
         const SAMPLE_CODE: &[(&str, &str)] = &[\n"
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
        let code = body.trim_end();
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

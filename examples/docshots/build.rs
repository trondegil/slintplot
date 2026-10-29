//! Turns every `<pre data-shot="name">` code example in docs/index.html into a
//! component, so the screenshot next to it shows exactly that code.

use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::PathBuf;

const PAGE: &str = "../../docs/index.html";

fn main() {
    println!("cargo:rerun-if-changed={PAGE}");
    let page = std::fs::read_to_string(PAGE).unwrap();
    let library = std::fs::read_to_string(slintplot::library_path()).unwrap();

    let mut shots = Vec::new();
    for part in page.split("<pre data-shot=\"").skip(1) {
        let (name, rest) = part.split_once('"').unwrap();
        let code = rest.split_once("<code>").unwrap().1;
        let code = code.split_once("</code>").unwrap().0;
        let code = code
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&");
        shots.push((name.to_string(), code));
    }

    // Everything the examples import or use from the library.
    let mut names = BTreeSet::from(["ChartEngine".to_string()]);
    for (_, code) in &shots {
        for word in code.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
            if word.starts_with(char::is_uppercase) && is_exported(&library, word) {
                names.insert(word.to_string());
            }
        }
    }

    let mut slint = String::new();
    writeln!(
        slint,
        "import {{ {} }} from \"@slintplot\";",
        names.into_iter().collect::<Vec<_>>().join(", ")
    )
    .unwrap();
    writeln!(slint, "import {{ Palette }} from \"std-widgets.slint\";").unwrap();
    writeln!(slint, "export {{ ChartEngine }}").unwrap();
    for (i, (name, code)) in shots.iter().enumerate() {
        let body: String = code
            .lines()
            .filter(|line| !line.starts_with("import "))
            .map(|line| format!("    {line}\n"))
            .collect();
        // The one app property an example reads: a year of daily values.
        let properties = if body.contains("root.daily-counts") {
            "    in property <[float]> daily-counts;\n"
        } else {
            ""
        };
        assert!(
            !body.replace("root.daily-counts", "").contains("root."),
            "the {name} example reads an app property docshots doesn't provide"
        );
        writeln!(
            slint,
            "// {name}\ncomponent Shot{i} inherits VerticalLayout {{\n{properties}{body}}}"
        )
        .unwrap();
    }
    writeln!(slint, "export component Shots inherits Window {{").unwrap();
    writeln!(slint, "    in property <int> shot;\n    in property <bool> dark;\n    in property <[float]> daily-counts;").unwrap();
    writeln!(slint, "    changed dark => {{ Palette.color-scheme = self.dark ? ColorScheme.dark : ColorScheme.light; }}").unwrap();
    // The background and padding of the gallery's chart card.
    writeln!(slint, "    background: self.dark ? #232326 : #ffffff;\n    VerticalLayout {{\n        padding: 16px;").unwrap();
    for (i, (name, _)) in shots.iter().enumerate() {
        let data = if shots[i].1.contains("root.daily-counts") {
            " daily-counts: root.daily-counts; "
        } else {
            " "
        };
        writeln!(
            slint,
            "        if root.shot == {i}: Shot{i} {{{data}}} // {name}"
        )
        .unwrap();
    }
    writeln!(slint, "    }}\n}}").unwrap();

    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let names: Vec<String> = shots.iter().map(|(name, _)| format!("{name:?}")).collect();
    std::fs::write(
        out.join("shot_names.rs"),
        format!("pub const SHOTS: &[&str] = &[{}];\n", names.join(", ")),
    )
    .unwrap();
    let file = out.join("shots.slint");
    std::fs::write(&file, slint).unwrap();

    let config = slint_build::CompilerConfiguration::new().with_library_paths(
        std::collections::HashMap::from([("slintplot".into(), slintplot::library_path())]),
    );
    slint_build::compile_with_config(file, config).unwrap();
}

/// Whether the library's entry point exports `name`.
fn is_exported(library: &str, name: &str) -> bool {
    library
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .any(|word| word == name)
}

//! The code panel's files: a sample's Slint code from ui/gallery.slint (see
//! build.rs), in a window that declares the gallery properties it reads, with
//! the values they have now, and the rest of a Rust project that runs it.

use slint::{Color, Model, ModelRc, SharedString, VecModel};

use crate::{
    ChartCandle, ChartGanttTask, CodeFile, ChartDataset, ChartLegendPosition, ChartSeriesType, ChartSlice,
    ChartSunburstItem, ChartTreemapItem, ChartWaterfallStep, ChartXY, Gallery, SAMPLE_CODE,
    SLINTPLOT_NAMES,
};

/// Lines are wrapped at this width.
const WIDTH: usize = 100;
/// Lists longer than this show their first values and a comment.
const MAX_ITEMS: usize = 1000;
/// List items this short fill the lines; longer ones get a line each.
const SHORT: usize = 40;

/// The code panel's tabs for the sample `id`: its Slint code with the
/// gallery's current data, and the Rust project around it.
pub fn sample_files(ui: &Gallery, id: &str) -> ModelRc<CodeFile> {
    let slint = sample_code(ui, id);
    let files = [
        ("ui/app.slint", slint.clone()),
        ("src/main.rs", MAIN_RS.to_string()),
        ("build.rs", BUILD_RS.to_string()),
        ("Cargo.toml", cargo_toml(id)),
        ("How to run", how_to_run(slint.contains("debug(self.status)"))),
    ];
    let files = files.map(|(name, code)| CodeFile { name: name.into(), code: code.into() });
    ModelRc::new(VecModel::from(files.to_vec()))
}

const MAIN_RS: &str = "\
slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let app = Example::new()?;
    // Connects the charts to SlintPlot's Rust code.
    slintplot::register!(app);
    app.run()
}
";

const BUILD_RS: &str = "\
// Compiles ui/app.slint, with SlintPlot's charts importable as \"@slintplot\".
fn main() {
    let config = slint_build::CompilerConfiguration::new().with_library_paths(
        std::collections::HashMap::from([(\"slintplot\".into(), slintplot::library_path())]),
    );
    slint_build::compile_with_config(\"ui/app.slint\", config).unwrap();
}
";

/// The project's Cargo.toml, with SlintPlot of the gallery's version: its
/// major and minor number.
fn cargo_toml(id: &str) -> String {
    let version = env!("CARGO_PKG_VERSION").rsplit_once('.').map_or("", |(minor, _)| minor);
    format!(
        "[package]\n\
         name = \"{id}-example\"\n\
         version = \"0.1.0\"\n\
         edition = \"2021\"\n\n\
         [dependencies]\n\
         slint = \"1.18\"\n\
         slintplot = \"{version}\"\n\n\
         [build-dependencies]\n\
         slint-build = \"1.18\"\n\
         slintplot = \"{version}\"\n"
    )
}

/// The steps from the other tabs to a running chart.
fn how_to_run(prints_clicks: bool) -> String {
    let mut text = String::from(
        "\
This chart runs as a small Rust app made of the files in the other tabs.

1. Make a new project:

       cargo new my-chart
       cd my-chart

2. Replace Cargo.toml with the Cargo.toml tab.
3. Save the build.rs tab as build.rs, next to Cargo.toml.
4. Save the ui/app.slint tab as ui/app.slint.
5. Replace src/main.rs with the src/main.rs tab.
6. Run it:

       cargo run
",
    );
    if prints_clicks {
        text.push_str("\nClicks on the chart are printed in the terminal.\n");
    }
    text.push_str(
        "\n\
On Linux, Slint needs the fontconfig and xkbcommon development packages first\n\
(libfontconfig-dev and libxkbcommon-dev on Debian and Ubuntu).

More in Getting started: https://slintplot.dev/#getting-started
",
    );
    text
}

/// The Slint code of the sample `id`, with the gallery's current data.
fn sample_code(ui: &Gallery, id: &str) -> String {
    let Some(body) = template(id) else {
        return String::new();
    };
    let mut properties = String::new();
    for name in properties_read(body) {
        if let Some(callback) = callback_declaration(&name) {
            properties.push_str(callback);
            continue;
        }
        let (ty, value) = property(ui, &name);
        let start = format!("    property <{ty}> {name}: ");
        properties.push_str(&format!("{start}{};\n", value.render(start.len())));
    }
    if body.contains("root.status") {
        properties.push_str("    // What was clicked, written by the chart's callbacks.\n");
        properties.push_str("    property <string> status;\n");
        properties.push_str("    changed status => {\n        debug(self.status);\n    }\n");
    }
    let mut window = String::from("export component Example inherits Window {\n");
    window.push_str("    preferred-width: 720px;\n    preferred-height: 480px;\n");
    if !properties.is_empty() {
        window.push_str(&properties);
    }
    // In a layout, the chart fills the window.
    window.push_str("\n    VerticalLayout {\n        padding: 16px;\n\n");
    for line in body.lines() {
        if !line.is_empty() {
            window.push_str("        ");
        }
        window.push_str(line);
        window.push('\n');
    }
    window.push_str("    }\n}\n");
    // ChartEngine is exported so that slintplot::register! can connect it.
    let code = format!("export {{ ChartEngine }}\n\n{window}");
    format!("{}\n{code}", import_line(&code))
}

/// The sample's code from ui/gallery.slint, if it has any.
fn template(id: &str) -> Option<&'static str> {
    SAMPLE_CODE
        .iter()
        .find(|(sample, _)| *sample == id)
        .map(|(_, code)| *code)
}

/// The gallery properties `body` reads, `root.status` aside, in order.
pub fn properties_read(body: &str) -> Vec<String> {
    let mut names = Vec::new();
    for (i, _) in body.match_indices("root.") {
        let name: String = body[i + 5..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        if name != "status" && !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

/// The words of `code`.
fn words(code: &str) -> Vec<&str> {
    code.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect()
}

/// The import statement of the SlintPlot names `code` uses.
fn import_line(code: &str) -> String {
    let words = words(code);
    // The gallery doesn't import the enum the dataset values may use.
    let mut used: Vec<&str> = SLINTPLOT_NAMES
        .iter()
        .chain(&["ChartSeriesType"])
        .copied()
        .filter(|name| words.contains(name))
        .collect();
    used.dedup();
    used.sort_unstable();
    let line = format!("import {{ {} }} from \"@slintplot\";\n", used.join(", "));
    if line.len() <= WIDTH {
        return line;
    }
    format!("import {{\n    {},\n}} from \"@slintplot\";\n", used.join(",\n    "))
}

/// The declaration of the gallery callback `name`, if it is one, so that the
/// code that calls it compiles.
fn callback_declaration(name: &str) -> Option<&'static str> {
    match name {
        "edit-task" => Some(
            "    // This is simplified code: in the gallery, clicking a task opens a dialog\n\
             \x20   // that edits it. The full example code, with the dialog (TaskEditor), is on\n\
             \x20   // GitHub: https://github.com/trondegil/slintplot/blob/main/examples/gallery/ui/gallery.slint\n\
             \x20   callback edit-task(index: int);\n",
        ),
        _ => None,
    }
}

/// The type of the gallery property `name`, and its value now.
fn property(ui: &Gallery, name: &str) -> (&'static str, Lit) {
    let value = match name {
        // The gallery's layout for phones, below 720 pixels wide.
        "narrow" => false.lit(),
        "labels" => list(ui.get_labels()),
        "row-labels" => list(ui.get_row_labels()),
        "datasets" => list(ui.get_datasets()),
        "slices" => list(ui.get_slices()),
        "channels" => list(ui.get_channels()),
        "grid" => list(ui.get_grid()),
        "growth" => list(ui.get_growth()),
        "bullet-values" => list(ui.get_bullet_values()),
        "tree" => list(ui.get_tree()),
        "sun" => list(ui.get_sun()),
        "steps" => list(ui.get_steps()),
        "candles" => list(ui.get_candles()),
        "gantt-tasks" => list(ui.get_gantt_tasks()),
        "gauge-value" => ui.get_gauge_value().lit(),
        "view-start" => ui.get_view_start().lit(),
        "view-end" => ui.get_view_end().lit(),
        "ohlc" => ui.get_ohlc().lit(),
        "skip-gaps" => ui.get_skip_gaps().lit(),
        "decimate" => ui.get_decimate().lit(),
        "calendar-colors" => ui.get_calendar_colors().lit(),
        "legend-position" => ui.get_legend_position().lit(),
        "sample-title" => ui.get_sample_title().lit(),
        "language-title" => ui.get_language_title().lit(),
        "language-y-title" => ui.get_language_y_title().lit(),
        "language-y2-title" => ui.get_language_y2_title().lit(),
        _ => panic!("the code panel doesn't know the gallery property {name}"),
    };
    (property_type(name).unwrap(), value)
}

/// The Slint type of the gallery property `name`.
pub fn property_type(name: &str) -> Option<&'static str> {
    Some(match name {
        "narrow" | "ohlc" | "skip-gaps" | "decimate" => "bool",
        "labels" | "row-labels" => "[string]",
        "datasets" => "[ChartDataset]",
        "slices" | "channels" => "[ChartSlice]",
        "grid" | "growth" | "bullet-values" => "[float]",
        "tree" => "[ChartTreemapItem]",
        "sun" => "[ChartSunburstItem]",
        "steps" => "[ChartWaterfallStep]",
        "candles" => "[ChartCandle]",
        "gantt-tasks" => "[ChartGanttTask]",
        "gauge-value" | "view-start" | "view-end" => "float",
        "calendar-colors" => "int",
        "legend-position" => "ChartLegendPosition",
        "sample-title" | "language-title" | "language-y-title" | "language-y2-title" => "string",
        _ => return None,
    })
}

/// A Slint value, to be written within a width.
enum Lit {
    Atom(String),
    /// The items shown, and how many more there are.
    List(Vec<Lit>, usize),
    Struct(Vec<(&'static str, Lit)>),
}

impl Lit {
    /// The value on one line.
    fn flat(&self) -> String {
        match self {
            Lit::Atom(atom) => atom.clone(),
            Lit::List(items, _) => {
                let items: Vec<String> = items.iter().map(Lit::flat).collect();
                format!("[{}]", items.join(", "))
            }
            Lit::Struct(fields) => {
                let fields: Vec<String> = fields
                    .iter()
                    .map(|(name, value)| format!("{name}: {}", value.flat()))
                    .collect();
                format!("{{ {} }}", fields.join(", "))
            }
        }
    }

    /// The value starting `column` characters in, on one line if it fits,
    /// else over lines indented by `indent`.
    fn render_at(&self, indent: usize, column: usize) -> String {
        let flat = self.flat();
        let more = matches!(self, Lit::List(_, more) if *more > 0);
        if column + flat.len() <= WIDTH && !more {
            return flat;
        }
        let inner = " ".repeat(indent + 4);
        let close = " ".repeat(indent);
        match self {
            Lit::Atom(_) => flat,
            Lit::List(items, more) => {
                let mut out = String::from("[\n");
                let flats: Vec<String> = items.iter().map(Lit::flat).collect();
                if flats.iter().all(|item| item.len() <= SHORT) {
                    // Values fill the lines.
                    let mut line = inner.clone();
                    for item in &flats {
                        let item = format!("{item},");
                        if line.len() + 1 + item.len() > WIDTH && !line.trim().is_empty() {
                            out.push_str(line.trim_end());
                            out.push('\n');
                            line = inner.clone();
                        } else if !line.trim().is_empty() {
                            line.push(' ');
                        }
                        line.push_str(&item);
                    }
                    if !line.trim().is_empty() {
                        out.push_str(&line);
                        out.push('\n');
                    }
                } else {
                    for item in items {
                        out.push_str(&format!("{inner}{},\n", item.render_at(indent + 4, inner.len())));
                    }
                }
                if *more > 0 {
                    out.push_str(&format!(
                        "{inner}// ... and {more} more, made in examples/gallery/src/main.rs\n"
                    ));
                }
                out + &close + "]"
            }
            Lit::Struct(fields) => {
                let mut out = String::from("{\n");
                for (name, value) in fields {
                    let start = inner.len() + name.len() + 2;
                    out.push_str(&format!("{inner}{name}: {},\n", value.render_at(indent + 4, start)));
                }
                out + &close + "}"
            }
        }
    }

    /// The value as a property's, which starts `column` characters in.
    fn render(&self, column: usize) -> String {
        self.render_at(4, column)
    }
}

/// A Slint value of a Rust one.
trait ToLit {
    fn lit(&self) -> Lit;
}

/// The items of `model`, up to `MAX_ITEMS`.
fn list<T: ToLit + Clone + 'static>(model: ModelRc<T>) -> Lit {
    let shown = model.row_count().min(MAX_ITEMS);
    let items = (0..shown).filter_map(|i| model.row_data(i)).map(|item| item.lit());
    Lit::List(items.collect(), model.row_count() - shown)
}

impl ToLit for f32 {
    fn lit(&self) -> Lit {
        // Slint has no NaN literal, but 0 / 0 is NaN.
        Lit::Atom(if self.is_nan() {
            "0 / 0".into()
        } else if self.is_infinite() {
            format!("{}1 / 0", if *self < 0.0 { "-" } else { "" })
        } else {
            // + 0.0 turns -0 into 0.
            format!("{}", self + 0.0)
        })
    }
}

impl ToLit for i32 {
    fn lit(&self) -> Lit {
        Lit::Atom(self.to_string())
    }
}

impl ToLit for bool {
    fn lit(&self) -> Lit {
        Lit::Atom(self.to_string())
    }
}

impl ToLit for SharedString {
    fn lit(&self) -> Lit {
        let mut out = String::from("\"");
        for c in self.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                c => out.push(c),
            }
        }
        out.push('"');
        Lit::Atom(out)
    }
}

impl ToLit for Color {
    fn lit(&self) -> Lit {
        let (r, g, b, a) = (self.red(), self.green(), self.blue(), self.alpha());
        Lit::Atom(if a == 255 {
            format!("#{r:02x}{g:02x}{b:02x}")
        } else {
            format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
        })
    }
}

impl<T: ToLit + Clone + 'static> ToLit for ModelRc<T> {
    fn lit(&self) -> Lit {
        list(self.clone())
    }
}

impl ToLit for ChartSeriesType {
    fn lit(&self) -> Lit {
        Lit::Atom(match self {
            ChartSeriesType::Line => "ChartSeriesType.line".into(),
            ChartSeriesType::Bar => "ChartSeriesType.bar".into(),
            _ => "ChartSeriesType.auto".into(),
        })
    }
}

impl ToLit for ChartLegendPosition {
    fn lit(&self) -> Lit {
        Lit::Atom(match self {
            ChartLegendPosition::Bottom => "ChartLegendPosition.bottom".into(),
            ChartLegendPosition::Left => "ChartLegendPosition.left".into(),
            ChartLegendPosition::Right => "ChartLegendPosition.right".into(),
            _ => "ChartLegendPosition.top".into(),
        })
    }
}

/// Gantt task times in whole days are written as days of 86400 seconds.
impl ToLit for ChartGanttTask {
    fn lit(&self) -> Lit {
        let time = |seconds: f32| {
            let days = seconds / 86400.0;
            if seconds != 0.0 && days.fract() == 0.0 {
                Lit::Atom(format!("{days} * 86400"))
            } else {
                seconds.lit()
            }
        };
        let mut fields = vec![("label", self.label.lit())];
        if !self.group.is_empty() {
            fields.push(("group", self.group.lit()));
        }
        fields.push(("start", time(self.start)));
        fields.push(("end", time(self.end)));
        if self.progress != 0.0 {
            fields.push(("progress", self.progress.lit()));
        }
        if self.color.alpha() != 0 {
            fields.push(("color", self.color.lit()));
        }
        Lit::Struct(fields)
    }
}

/// Struct literals with the fields that differ from their defaults.
macro_rules! struct_lit {
    ($($ty:ident { $($field:ident: $name:literal),* })*) => {$(
        impl ToLit for $ty {
            fn lit(&self) -> Lit {
                let default = $ty::default();
                let mut fields = Vec::new();
                $(
                    let value = self.$field.lit();
                    if value.flat() != default.$field.lit().flat() {
                        fields.push(($name, value));
                    }
                )*
                Lit::Struct(fields)
            }
        }
    )*};
}

struct_lit! {
    ChartXY { x: "x", y: "y", r: "r" }
    ChartDataset {
        label: "label", series_type: "series-type", values: "values", points: "points",
        color: "color", hidden: "hidden", fill: "fill", tension: "tension",
        stepped: "stepped", right_axis: "right-axis"
    }
    ChartSlice { label: "label", value: "value", color: "color", hidden: "hidden" }
    ChartTreemapItem { label: "label", value: "value", group: "group", color: "color" }
    ChartSunburstItem {
        label: "label", value: "value", parent: "parent", color: "color", hidden: "hidden"
    }
    ChartWaterfallStep { label: "label", value: "value", total: "total" }

    ChartCandle {
        x: "x", open: "open", high: "high", low: "low", close: "close", volume: "volume"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sample_has_code_and_known_properties() {
        for sample in crate::SAMPLES {
            let body = template(sample.id).unwrap_or_else(|| panic!("no code for {}", sample.id));
            for name in properties_read(body) {
                assert!(
                    property_type(&name).is_some() || callback_declaration(&name).is_some(),
                    "{}: root.{name}",
                    sample.id
                );
            }
        }
    }

    #[test]
    fn cargo_toml_takes_slintplot_of_the_gallerys_version() {
        let version = env!("CARGO_PKG_VERSION");
        let minor = &version[..version.rfind('.').unwrap()];
        assert!(cargo_toml("bar").contains(&format!("slintplot = \"{minor}\"\n")));
    }

    #[test]
    fn values_are_slint_literals() {
        assert_eq!(f32::NAN.lit().flat(), "0 / 0");
        assert_eq!(2.5f32.lit().flat(), "2.5");
        assert_eq!(SharedString::from("a \"b\"").lit().flat(), "\"a \\\"b\\\"\"");
        let slice = ChartSlice { label: "Web".into(), value: 12.0, ..Default::default() };
        assert_eq!(slice.lit().flat(), "{ label: \"Web\", value: 12 }");
    }
}

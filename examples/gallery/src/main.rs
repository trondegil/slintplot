//! A sample browser for SlintPlot.
//!
//! `cargo run -p gallery` opens it; `cargo run -p gallery -- --snapshot <dir>`
//! renders every sample to PNG files instead.

#[cfg(not(target_arch = "wasm32"))]
mod dock_icon;
#[cfg(not(target_arch = "wasm32"))]
mod snapshot;
mod stream_model;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use stream_model::StreamModel;

slint::include_modules!();

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
/// The languages of the "languages" sample's picker, in its order.
const LANGUAGES: [ChartLanguage; 14] = [
    ChartLanguage::English,
    ChartLanguage::Danish,
    ChartLanguage::German,
    ChartLanguage::Spanish,
    ChartLanguage::French,
    ChartLanguage::Italian,
    ChartLanguage::Dutch,
    ChartLanguage::Norwegian,
    ChartLanguage::Polish,
    ChartLanguage::Portuguese,
    ChartLanguage::Finnish,
    ChartLanguage::Swedish,
    ChartLanguage::Japanese,
    ChartLanguage::Chinese,
];
/// The "languages" sample's own texts in each of `LANGUAGES`: the title, and
/// the names of the two datasets, which are also the axis titles.
const LANGUAGE_TEXTS: [(&str, &str, &str); 14] = [
    (
        "Visitors and conversion rate",
        "Visitors",
        "Conversion rate",
    ),
    (
        "Besøgende og konverteringsrate",
        "Besøgende",
        "Konverteringsrate",
    ),
    (
        "Besucher und Konversionsrate",
        "Besucher",
        "Konversionsrate",
    ),
    (
        "Visitantes y tasa de conversión",
        "Visitantes",
        "Tasa de conversión",
    ),
    (
        "Visiteurs et taux de conversion",
        "Visiteurs",
        "Taux de conversion",
    ),
    (
        "Visitatori e tasso di conversione",
        "Visitatori",
        "Tasso di conversione",
    ),
    ("Bezoekers en conversieratio", "Bezoekers", "Conversieratio"),
    (
        "Besøkende og konverteringsrate",
        "Besøkende",
        "Konverteringsrate",
    ),
    (
        "Odwiedzający i współczynnik konwersji",
        "Odwiedzający",
        "Współczynnik konwersji",
    ),
    (
        "Visitantes e taxa de conversão",
        "Visitantes",
        "Taxa de conversão",
    ),
    ("Kävijät ja konversioaste", "Kävijät", "Konversioaste"),
    (
        "Besökare och konverteringsgrad",
        "Besökare",
        "Konverteringsgrad",
    ),
    ("訪問者数とコンバージョン率", "訪問者数", "コンバージョン率"),
    ("访客数与转化率", "访客数", "转化率"),
];
const SKILLS: [&str; 7] = [
    "Eating",
    "Drinking",
    "Sleeping",
    "Designing",
    "Coding",
    "Cycling",
    "Running",
];
const SLICE_NAMES: [&str; 8] = [
    "Red", "Orange", "Green", "Blue", "Purple", "Teal", "Pink", "Grey",
];

/// The data actions most samples offer.
const DATA_ACTIONS: &[&str] = &[
    "Randomize",
    "Add dataset",
    "Add data",
    "Remove dataset",
    "Remove data",
];

/// The live data sample's buttons: pause, and the update rates.
const LIVE_ACTIONS: &[&str] = &[
    "Pause / resume",
    "10 updates/s",
    "30 updates/s",
    "60 updates/s",
];

/// Seconds the live data sample keeps.
const LIVE_SECONDS: u32 = 60;

/// Seconds since an arbitrary start, to measure update rates. The browser
/// has no `Instant`.
fn now() -> f64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        START
            .get_or_init(std::time::Instant::now)
            .elapsed()
            .as_secs_f64()
    }
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() / 1000.0
    }
}

/// The live data sample: three sensors whose values stream into ring buffers.
struct Live {
    /// Updates per second.
    rate: u32,
    paused: bool,
    /// Seconds after the time origin of the newest values.
    time: f32,
    last: [f32; 3],
    models: Vec<Rc<StreamModel<ChartXY>>>,
    /// Updates since `since`, for the measured rate.
    updates: u32,
    since: f64,
}

/// How a sample's data looks.
#[derive(Clone, Copy, PartialEq)]
enum Shape {
    /// Values per month, as in most samples.
    Months { datasets: usize, min: f32, max: f32 },
    /// One value per skill.
    Skills,
    /// Pie, doughnut and polar area slices.
    Slices,
    /// Scatter points.
    Points,
    /// Bubbles with a radius.
    Bubbles,
    /// A long random walk.
    Series { count: usize },
    /// Monthly values growing over several orders of magnitude.
    Exponential,
    /// Points every `step` seconds after the time origin.
    TimeSeries { count: usize, step: f32 },
    /// Activity by weekday and hour.
    Matrix,
    /// A symmetric correlation matrix, from -1 to 1.
    Correlation,
    /// One value per day for a year.
    Calendar,
    /// Survey answers per group, shown as shares of 100%.
    Survey,
    /// Grouped items for a treemap.
    Treemap,
    /// The population table as a hierarchy: continents, regions, countries.
    Sunburst,
    /// Website visitors through the steps to a purchase.
    Funnel,
    /// A profit and loss statement.
    Waterfall,
    /// Life expectancy per country in two years.
    Dumbbell,
    /// One value on a gauge.
    Gauge,
    /// Measures against their targets.
    Bullet,
    /// Sensors streaming new values.
    Live,
}

/// Population in millions (2025 estimates), by continent.
const POPULATION: [(&str, &str, f32); 18] = [
    ("Asia", "India", 1464.0),
    ("Asia", "China", 1416.0),
    ("Asia", "Indonesia", 285.0),
    ("Asia", "Pakistan", 255.0),
    ("Asia", "Bangladesh", 175.0),
    ("Asia", "Japan", 123.0),
    ("Asia", "Philippines", 116.0),
    ("Africa", "Nigeria", 237.0),
    ("Africa", "Ethiopia", 135.0),
    ("Africa", "Egypt", 118.0),
    ("Africa", "DR Congo", 112.0),
    ("Americas", "United States", 347.0),
    ("Americas", "Brazil", 212.0),
    ("Americas", "Mexico", 132.0),
    ("Europe", "Russia", 144.0),
    ("Europe", "Germany", 84.0),
    ("Europe", "United Kingdom", 70.0),
    ("Europe", "France", 66.0),
];

/// The region of each country of `POPULATION`, for the sunburst.
const REGIONS: [&str; 18] = [
    "South Asia",
    "East Asia",
    "Southeast Asia",
    "South Asia",
    "South Asia",
    "East Asia",
    "Southeast Asia",
    "West Africa",
    "East Africa",
    "North Africa",
    "Central Africa",
    "North America",
    "South America",
    "North America",
    "Eastern Europe",
    "Western Europe",
    "Western Europe",
    "Western Europe",
];

/// Website visitors through the steps to a purchase.
/// The survey's answers, and their colors: a fade from dark blue (every
/// day) to light blue (never), brightest first in dark mode. Both ramps were
/// checked for even steps and contrast against the background.
const ANSWER_ORDER: [&str; 5] = ["Every day", "Most days", "Some days", "Rarely", "Never"];
const SURVEY_LIGHT: [u32; 5] = [0x172f73, 0x1d4ed8, 0x3b7ef0, 0x5f9cf2, 0x86b6ef];
const SURVEY_DARK: [u32; 5] = [0xdbeafe, 0x93c5fd, 0x60a5fa, 0x3b82f6, 0x1d4ed8];

const FUNNEL: [(&str, f32); 5] = [
    ("Visited the site", 12_400.0),
    ("Viewed a product", 7_350.0),
    ("Added to cart", 3_120.0),
    ("Started checkout", 1_840.0),
    ("Purchased", 1_150.0),
];

/// A profit and loss statement, in thousands; `true` marks the totals.
const PROFIT_AND_LOSS: [(&str, f32, bool); 9] = [
    ("Revenue", 420.0, false),
    ("Cost of sales", -185.0, false),
    ("Gross profit", 0.0, true),
    ("Marketing", -48.0, false),
    ("R&D", -62.0, false),
    ("Admin", -31.0, false),
    ("Operating profit", 0.0, true),
    ("Tax", -22.0, false),
    ("Net profit", 0.0, true),
];

/// Life expectancy at birth in years, in 2000 and 2023 (rounded estimates).
const LIFE_EXPECTANCY: [(&str, f32, f32); 8] = [
    ("Japan", 81.1, 84.7),
    ("Norway", 78.7, 83.3),
    ("China", 71.4, 78.6),
    ("United States", 76.6, 78.4),
    ("Brazil", 70.1, 75.8),
    ("India", 62.7, 72.0),
    ("South Africa", 56.0, 65.0),
    ("Nigeria", 46.3, 54.5),
];

struct Sample {
    id: &'static str,
    category: &'static str,
    title: &'static str,
    description: &'static str,
    shape: Shape,
    actions: &'static [&'static str],
}

const fn sample(
    id: &'static str,
    category: &'static str,
    title: &'static str,
    description: &'static str,
    shape: Shape,
    actions: &'static [&'static str],
) -> Sample {
    Sample {
        id,
        category,
        title,
        description,
        shape,
        actions,
    }
}

const MONTHLY: Shape = Shape::Months {
    datasets: 2,
    min: -100.0,
    max: 100.0,
};

const SAMPLES: &[Sample] = &[
    sample(
        "bar-vertical",
        "Bar charts",
        "Vertical bar chart",
        "Grouped bars with a shared value axis. Hover a month to compare the datasets.",
        MONTHLY,
        DATA_ACTIONS,
    ),
    sample(
        "bar-horizontal",
        "Bar charts",
        "Horizontal bar chart",
        "HorizontalBarChart: the categories run down the left side. The legend is on the right, or above the chart on narrow screens.",
        MONTHLY,
        DATA_ACTIONS,
    ),
    sample(
        "bar-stacked",
        "Bar charts",
        "Stacked bar chart",
        "stacked: true. Positive and negative values stack separately.",
        Shape::Months {
            datasets: 3,
            min: -100.0,
            max: 100.0,
        },
        DATA_ACTIONS,
    ),
    sample(
        "bar-border-radius",
        "Bar charts",
        "Bar chart border radius",
        "bar-radius and bar-border-width round the bars and give them a solid outline.",
        MONTHLY,
        DATA_ACTIONS,
    ),
    sample(
        "line",
        "Line charts",
        "Line chart",
        "Smooth lines with point markers (tension: 0.4).",
        MONTHLY,
        DATA_ACTIONS,
    ),
    sample(
        "line-interpolation",
        "Line charts",
        "Interpolation modes",
        "The same values drawn with tension 0 (straight segments) and 0.4 (smooth curves).",
        MONTHLY,
        &["Randomize"],
    ),
    sample(
        "line-stepped",
        "Line charts",
        "Stepped line chart",
        "stepped: true draws the values as steps.",
        Shape::Months {
            datasets: 1,
            min: -100.0,
            max: 100.0,
        },
        &["Randomize", "Steps on", "Steps off"],
    ),
    sample(
        "line-points",
        "Line charts",
        "Line chart without points",
        "show-points: false. Hovering still marks the values of every line.",
        MONTHLY,
        DATA_ACTIONS,
    ),
    sample(
        "bubble",
        "Other charts",
        "Bubble chart",
        "Points with a radius in pixels. Scroll to zoom.",
        Shape::Bubbles,
        DATA_ACTIONS,
    ),
    sample(
        "combo-bar-line",
        "Other charts",
        "Combo bar and line",
        "A bar chart with one dataset drawn as a line (series-type: line).",
        Shape::Months {
            datasets: 3,
            min: -100.0,
            max: 100.0,
        },
        DATA_ACTIONS,
    ),
    sample(
        "doughnut",
        "Other charts",
        "Doughnut chart",
        "Hovered slices move outwards. Click a legend entry to hide a slice.",
        Shape::Slices,
        DATA_ACTIONS,
    ),
    sample(
        "pie",
        "Other charts",
        "Pie chart",
        "Hovered slices move outwards. Click a legend entry to hide a slice.",
        Shape::Slices,
        DATA_ACTIONS,
    ),
    sample(
        "polar-area",
        "Other charts",
        "Polar area chart",
        "Equal angles; the radius shows the value.",
        Shape::Slices,
        DATA_ACTIONS,
    ),
    sample(
        "radar",
        "Other charts",
        "Radar chart",
        "One spoke per label. Hover a spoke to compare the datasets.",
        Shape::Skills,
        DATA_ACTIONS,
    ),
    sample(
        "scatter",
        "Other charts",
        "Scatter chart",
        "Numeric x and y axes. Hover finds the nearest point; scroll to zoom.",
        Shape::Points,
        DATA_ACTIONS,
    ),
    sample(
        "stacked-bar-line",
        "Other charts",
        "Stacked bar and line",
        "Stacked bars with a line on top. Lines and bars stack separately.",
        Shape::Months {
            datasets: 3,
            min: 0.0,
            max: 100.0,
        },
        DATA_ACTIONS,
    ),
    sample(
        "area-line",
        "Area charts",
        "Line chart with fill",
        "fill: true fills the area under a line.",
        MONTHLY,
        DATA_ACTIONS,
    ),
    sample(
        "area-stacked",
        "Area charts",
        "Stacked area chart",
        "Stacked lines with fill: each area fills down to the one below.",
        Shape::Months {
            datasets: 3,
            min: 0.0,
            max: 100.0,
        },
        DATA_ACTIONS,
    ),
    sample(
        "area-radar",
        "Area charts",
        "Radar chart with fill",
        "Filled radar polygons.",
        Shape::Skills,
        DATA_ACTIONS,
    ),
    sample(
        "legend-position",
        "Legend",
        "Legend position",
        "legend-position places the legend on any side of the chart.",
        MONTHLY,
        &[
            "Position: top",
            "Position: right",
            "Position: bottom",
            "Position: left",
        ],
    ),
    sample(
        "axes-format",
        "Axes",
        "Titles, formats and data labels",
        "x-title, y-title, value-prefix and value-decimals, with show-values writing each value on its bar. Click the chart and use the arrow keys.",
        Shape::Months {
            datasets: 2,
            min: 1000.0,
            max: 9000.0,
        },
        DATA_ACTIONS,
    ),
    sample(
        "axes-fixed",
        "Axes",
        "Fixed range",
        "fixed-range with min-value and max-value: the axis stays put when the data changes.",
        MONTHLY,
        DATA_ACTIONS,
    ),
    sample(
        "axes-log",
        "Axes",
        "Logarithmic axis",
        "logarithmic: true. Values from 1 to 10,000 on a log scale.",
        Shape::Exponential,
        &["Randomize"],
    ),
    sample(
        "axes-two",
        "Axes",
        "Two value axes",
        "right-axis puts a dataset on a second value axis, with its own y2-title and y2-value-suffix. Its ticks line up with the left axis' grid.",
        Shape::Months {
            datasets: 2,
            min: 0.0,
            max: 1.0,
        },
        &["Randomize"],
    ),
    sample(
        "survey",
        "Bar charts",
        "100% stacked bars",
        "stacked-percent: each row adds up to 100%, with the shares written in the bars. Ordered answers get one hue, from dark blue (every day) to light blue (never).",
        Shape::Survey,
        &["Randomize"],
    ),
    sample(
        "time-hourly",
        "Time series",
        "Hourly time series",
        "TimeSeriesChart: x values are seconds after time-origin. Two weeks of hourly values; scroll to zoom into hours.",
        Shape::TimeSeries {
            count: 24 * 14,
            step: 3600.0,
        },
        &["Randomize"],
    ),
    sample(
        "time-week",
        "Time series",
        "One week, every 15 minutes",
        "A week of electricity use, Monday to Sunday: the axis shows days, and zooming in shows hours and minutes.",
        Shape::TimeSeries {
            count: 7 * 96,
            step: 900.0,
        },
        &["Week", "Day", "6 hours", "1 hour", "Randomize"],
    ),
    sample(
        "time-daily",
        "Time series",
        "Daily time series",
        "A year of daily values; the axis shows months, and zooming in shows days.",
        Shape::TimeSeries {
            count: 365,
            step: 86_400.0,
        },
        &["Randomize"],
    ),
    sample(
        "live",
        "Time series",
        "Live data",
        "Three sensors stream new values into ring buffers (StreamModel in the gallery's source), keeping the last minute. Hover still works while it streams.",
        Shape::Live,
        LIVE_ACTIONS,
    ),
    sample(
        "heatmap",
        "Heatmaps",
        "Heatmap",
        "HeatmapChart: activity by weekday and hour. Darker cells have higher values.",
        Shape::Matrix,
        &["Randomize"],
    ),
    sample(
        "heatmap-diverging",
        "Heatmaps",
        "Diverging heatmap",
        "A correlation matrix: diverging around 0, with a fixed range of -1 to 1 and values in the cells.",
        Shape::Correlation,
        &["Randomize"],
    ),
    sample(
        "calendar",
        "Heatmaps",
        "Calendar heatmap",
        "CalendarHeatmap: a year of daily values, like GitHub's contribution graph. It follows the theme; pick other colors with Colors.",
        Shape::Calendar,
        &["Randomize"],
    ),
    sample(
        "treemap",
        "Treemaps and sunbursts",
        "Treemap",
        "TreemapChart: areas proportional to the values, grouped by continent. Click a legend entry to hide a continent.",
        Shape::Treemap,
        &["Randomize"],
    ),
    sample(
        "sunburst",
        "Treemaps and sunbursts",
        "Sunburst",
        "SunburstChart: continents, regions and countries as rings. Click a continent or region to zoom in, and the center to zoom out.",
        Shape::Sunburst,
        &["Randomize"],
    ),
    sample(
        "funnel",
        "Business charts",
        "Funnel",
        "FunnelChart: one curved funnel fading down from the theme color, with each stage's share of the first and conversion badges.",
        Shape::Funnel,
        &["Randomize"],
    ),
    sample(
        "waterfall",
        "Business charts",
        "Waterfall",
        "WaterfallChart: increases, decreases and totals of a profit and loss statement.",
        Shape::Waterfall,
        &["Randomize"],
    ),
    sample(
        "dumbbell",
        "Business charts",
        "Dumbbell",
        "DumbbellChart: two values per category, here life expectancy in 2000 and 2023.",
        Shape::Dumbbell,
        &["Randomize"],
    ),
    sample(
        "gauge",
        "Business charts",
        "Gauges",
        "GaugeChart: a filled arc with rounded ends, the same with a ring of bands, and a needle with ticks over the bands.",
        Shape::Gauge,
        &["Randomize"],
    ),
    sample(
        "bullet",
        "Business charts",
        "Bullet charts",
        "BulletChart: a KPI panel with an axis per row and gray bands, and columns on a shared axis over colored bands.",
        Shape::Bullet,
        &["Randomize"],
    ),
    sample(
        "pie-labels",
        "Other charts",
        "Pie chart with data labels",
        "show-percentages writes each slice's share on it.",
        Shape::Slices,
        DATA_ACTIONS,
    ),
    sample(
        "zoom",
        "Advanced",
        "Zoom and pan",
        "zoom-enabled: true. Scroll to zoom around the mouse, drag to pan, double-click to reset.",
        Shape::Series { count: 500 },
        &["Randomize", "Reset zoom"],
    ),
    sample(
        "decimation",
        "Advanced",
        "Data decimation",
        "100,000 points per dataset. decimate: true draws one span per pixel column; zoom in to see the details.",
        Shape::Series { count: 100_000 },
        &["Decimation on", "Decimation off", "Randomize"],
    ),
    sample(
        "languages",
        "Advanced",
        "Languages",
        "ChartTheme.language: numbers, percentages, month names and the charts' own texts in 14 languages. The chart's title and dataset names are translated by the app.",
        Shape::Months {
            datasets: 2,
            min: 0.0,
            max: 1.0,
        },
        &["Randomize"],
    ),
];

/// The random data's seed, so the samples start the same every time.
const SEED: u64 = 0x2545_f491_4f6c_dd1d;

/// Small xorshift generator, so the example needs no extra dependencies.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }

    fn range(&mut self, min: f32, max: f32) -> f32 {
        (min + self.next() * (max - min)).round()
    }
}

fn strings<'a>(items: impl IntoIterator<Item = &'a str>) -> ModelRc<SharedString> {
    ModelRc::new(VecModel::from(
        items
            .into_iter()
            .map(SharedString::from)
            .collect::<Vec<_>>(),
    ))
}

fn floats(values: Vec<f32>) -> ModelRc<f32> {
    ModelRc::new(VecModel::from(values))
}

/// The data of the sample on screen, and how to change it.
struct State {
    ui: slint::Weak<Gallery>,
    rng: Rng,
    sample: &'static Sample,
    labels: Rc<VecModel<SharedString>>,
    datasets: Rc<VecModel<ChartDataset>>,
    slices: Rc<VecModel<ChartSlice>>,
    /// Heatmap rows, and heatmap or calendar values.
    row_labels: Rc<VecModel<SharedString>>,
    grid: Rc<VecModel<f32>>,
    tree: Rc<VecModel<ChartTreemapItem>>,
    sun: Rc<VecModel<ChartSunburstItem>>,
    steps: Rc<VecModel<ChartWaterfallStep>>,
    /// Whether Randomize was pressed, for samples that start with real data.
    randomized: bool,
    live: Live,
}

impl State {
    fn values(&mut self, count: usize) -> Vec<f32> {
        let (min, max) = match self.sample.shape {
            Shape::Months { min, max, .. } => (min, max),
            _ => (10.0, 100.0),
        };
        (0..count).map(|_| self.rng.range(min, max)).collect()
    }

    fn walk(&mut self, count: usize, offset: f32) -> Vec<f32> {
        let mut v = offset;
        (0..count)
            .map(|_| {
                v += (self.rng.next() - 0.5) * 8.0 + (offset - v) * 0.01;
                v
            })
            .collect()
    }

    fn points(&mut self, count: usize, bubbles: bool) -> Vec<ChartXY> {
        (0..count)
            .map(|_| ChartXY {
                x: self.rng.range(-100.0, 100.0),
                y: self.rng.range(-100.0, 100.0),
                r: if bubbles {
                    self.rng.range(4.0, 20.0)
                } else {
                    0.0
                },
            })
            .collect()
    }

    /// Plausible temperatures every `step` seconds, starting at midnight in
    /// mid October: a yearly and a daily cycle around a city's mean, plus
    /// weather that drifts a few degrees and returns.
    fn temperatures(&mut self, count: usize, step: f32, city: usize) -> Vec<f32> {
        use std::f32::consts::TAU;
        // Oslo, then Bergen (milder winters, cooler summers), then others.
        let (mean, yearly) = [(6.5, 10.0), (8.0, 6.5)]
            .get(city)
            .copied()
            .unwrap_or((7.0, 8.0));
        let mut weather = 0.0f32;
        (0..count)
            .map(|i| {
                let seconds = i as f32 * step;
                let day = seconds / 86_400.0;
                // Warmest in mid July (day 196 of the year), coldest in mid
                // January; the samples start in mid October (about day 290).
                let season = yearly * (TAU * (day + 290.0 - 196.0) / 365.0).cos();
                // Warmest in the afternoon.
                let daily = if step < 86_400.0 {
                    -3.0 * (TAU * (day.fract() - 0.15)).cos()
                } else {
                    0.0
                };
                weather = weather * 0.97
                    + (self.rng.next() - 0.5) * if step < 86_400.0 { 0.6 } else { 2.5 };
                ((mean + season + daily + weather) * 10.0).round() / 10.0
            })
            .collect()
    }

    /// Electricity use in kW every `step` seconds, starting on a Monday at
    /// midnight. An office (`building` 0) is busy on weekdays from 8 to 17;
    /// a home (1) peaks in the morning and the evening, later at weekends.
    fn power(&mut self, count: usize, step: f32, building: usize) -> Vec<f32> {
        let bump = |hour: f32, at: f32, width: f32| (-((hour - at) / width).powi(2)).exp();
        (0..count)
            .map(|i| {
                let seconds = i as f32 * step;
                let day = (seconds / 86_400.0) as usize;
                let hour = seconds % 86_400.0 / 3600.0;
                let weekend = day >= 5;
                let noise = (self.rng.next() - 0.5) * 2.0;
                let kw = if building == 0 {
                    let open = !weekend && (8.0..17.0).contains(&hour);
                    // Servers and ventilation run all the time; a dip at lunch.
                    6.0 + if open {
                        28.0 - 6.0 * bump(hour, 12.0, 0.6)
                    } else {
                        0.0
                    } + noise * 0.6
                } else {
                    let morning = if weekend { 9.5 } else { 7.0 };
                    1.5 + 4.0 * bump(hour, morning, 1.0)
                        + 6.0 * bump(hour, 18.5, 1.6)
                        + if weekend {
                            2.5 * bump(hour, 13.0, 2.5)
                        } else {
                            0.0
                        }
                        + noise * 0.4
                };
                (kw.max(0.0) * 10.0).round() / 10.0
            })
            .collect()
    }

    /// The dataset at `index` of the current sample, with its style.
    fn dataset(&mut self, index: usize) -> ChartDataset {
        let mut ds = ChartDataset {
            label: format!("Dataset {}", index + 1).into(),
            tension: 0.4,
            ..Default::default()
        };
        match self.sample.shape {
            Shape::Months { .. } | Shape::Skills => {
                ds.values = floats(self.values(self.labels.row_count()));
            }
            Shape::Points => ds.points = ModelRc::new(VecModel::from(self.points(30, false))),
            Shape::Bubbles => ds.points = ModelRc::new(VecModel::from(self.points(12, true))),
            Shape::Series { count } => {
                ds.values = floats(self.walk(count, 40.0 * index as f32));
                ds.tension = 0.0;
            }
            Shape::Exponential => {
                let growth = 1.0 + index as f32;
                let values = (0..self.labels.row_count())
                    .map(|i| {
                        (10f32.powf(i as f32 * 0.6 / growth + self.rng.next() * 0.4))
                            .round()
                            .max(1.0)
                    })
                    .collect();
                ds.values = floats(values);
            }
            Shape::TimeSeries { count, step } if self.sample.id == "time-week" => {
                let points: Vec<ChartXY> = self
                    .power(count, step, index)
                    .into_iter()
                    .enumerate()
                    .map(|(i, y)| ChartXY {
                        x: i as f32 * step,
                        y,
                        r: 0.0,
                    })
                    .collect();
                ds.points = ModelRc::new(VecModel::from(points));
                ds.fill = true;
                ds.label = (if index == 0 { "Office" } else { "Home" }).into();
            }
            Shape::TimeSeries { count, step } => {
                let points: Vec<ChartXY> = self
                    .temperatures(count, step, index)
                    .into_iter()
                    .enumerate()
                    .map(|(i, y)| ChartXY {
                        x: i as f32 * step,
                        y,
                        r: 0.0,
                    })
                    .collect();
                ds.points = ModelRc::new(VecModel::from(points));
                ds.tension = 0.3;
                ds.label = (if index == 0 { "Oslo" } else { "Bergen" }).into();
            }
            Shape::Survey => {
                ds.color = self.survey_color(index);
                // Made-up shares per department, varied a little by Randomize.
                const SHARES: [[f32; 5]; 6] = [
                    [24.0, 30.0, 28.0, 12.0, 6.0],
                    [18.0, 27.0, 31.0, 15.0, 9.0],
                    [5.0, 12.0, 34.0, 28.0, 21.0],
                    [10.0, 21.0, 35.0, 22.0, 12.0],
                    [8.0, 14.0, 23.0, 25.0, 30.0],
                    [12.0, 19.0, 33.0, 23.0, 13.0],
                ];
                ds.label = ANSWER_ORDER[index % 5].into();
                let values = SHARES
                    .iter()
                    .map(|row| (row[index % 5] * (0.8 + 0.4 * self.rng.next())).round())
                    .collect();
                ds.values = floats(values);
            }
            Shape::Dumbbell => {
                ds.label = (if index == 0 { "2000" } else { "2023" }).into();
                let values = LIFE_EXPECTANCY
                    .iter()
                    .map(|&(_, before, after)| {
                        let v = if index == 0 { before } else { after };
                        if self.randomized {
                            (v + (self.rng.next() - 0.5) * 4.0).round()
                        } else {
                            v
                        }
                    })
                    .collect();
                ds.values = floats(values);
            }
            _ => {}
        }
        match self.sample.id {
            "line-interpolation" => {
                ds.label = (if index == 0 {
                    "Straight (tension 0)"
                } else {
                    "Smooth (tension 0.4)"
                })
                .into();
                ds.tension = if index == 0 { 0.0 } else { 0.4 };
            }
            "line-stepped" => ds.stepped = true,
            "area-line" => ds.fill = index == 0,
            "area-stacked" | "area-radar" => ds.fill = true,
            "combo-bar-line" | "stacked-bar-line" if index == 0 => {
                ds.label = "Line".into();
                ds.series_type = ChartSeriesType::Line;
            }
            // Made-up website numbers: visitors as bars, in thousands, and
            // the conversion rate in percent as a line on the right axis.
            "languages" => {
                let months = self.labels.row_count();
                let values = (0..months)
                    .map(|_| {
                        if index == 0 {
                            self.rng.range(8_000.0, 25_000.0).round()
                        } else {
                            self.rng.range(15.0, 45.0) / 10.0
                        }
                    })
                    .collect();
                ds.values = floats(values);
                let (_, visitors, rate) = LANGUAGE_TEXTS[self.language()];
                if index == 0 {
                    ds.label = visitors.into();
                } else {
                    ds.label = rate.into();
                    ds.series_type = ChartSeriesType::Line;
                    ds.right_axis = true;
                }
            }
            // Made-up monthly weather: temperature as a line on the left
            // axis, rainfall as bars on the right one.
            "axes-two" => {
                let months = self.labels.row_count();
                let values = (0..months)
                    .map(|i| {
                        if index == 0 {
                            let season = ((i as f32 - 3.5) / 12.0 * std::f32::consts::TAU).sin();
                            ((7.0 + 9.0 * season + (self.rng.next() - 0.5) * 3.0) * 10.0).round()
                                / 10.0
                        } else {
                            self.rng.range(40.0, 140.0).round()
                        }
                    })
                    .collect();
                ds.values = floats(values);
                if index == 0 {
                    ds.label = "Temperature".into();
                    ds.series_type = ChartSeriesType::Line;
                    ds.tension = 0.4;
                } else {
                    ds.label = "Rainfall".into();
                    ds.right_axis = true;
                }
            }
            _ => {}
        }
        ds
    }

    fn slice(&mut self, index: usize) -> ChartSlice {
        ChartSlice {
            label: SLICE_NAMES[index % SLICE_NAMES.len()].into(),
            value: self.rng.range(5.0, 100.0),
            ..Default::default()
        }
    }

    fn survey_color(&self, index: usize) -> slint::Color {
        let dark = self.ui.upgrade().is_some_and(|ui| ui.get_dark_mode());
        let rgb = (if dark { SURVEY_DARK } else { SURVEY_LIGHT })[index % 5];
        slint::Color::from_rgb_u8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
    }

    /// The survey's colors have light and dark mode shades.
    fn recolor_survey(&mut self) {
        if self.sample.shape == Shape::Survey {
            self.update_datasets(|state, ds| {
                let index = ANSWER_ORDER
                    .iter()
                    .position(|a| ds.label == *a)
                    .unwrap_or(0);
                ds.color = state.survey_color(index);
            });
        }
    }

    /// Fills the heatmap or calendar values of the current sample.
    fn grid_values(&mut self) {
        let values: Vec<f32> = match self.sample.shape {
            Shape::Matrix => (0..7 * 24)
                .map(|i| {
                    let (day, hour) = (i / 24, (i % 24) as f32);
                    // Busy on weekday afternoons, quiet at night.
                    let daytime = (-((hour - 14.0) / 5.0).powi(2)).exp();
                    let weekday = if day < 5 { 1.0 } else { 0.45 };
                    (100.0 * daytime * weekday + self.rng.next() * 15.0).round()
                })
                .collect(),
            Shape::Correlation => {
                let n = 6;
                let mut m = vec![0.0; n * n];
                for r in 0..n {
                    for c in r..n {
                        let v = if r == c {
                            1.0
                        } else {
                            ((self.rng.next() * 2.0 - 1.0) * 100.0).round() / 100.0
                        };
                        m[r * n + c] = v;
                        m[c * n + r] = v;
                    }
                }
                m
            }
            Shape::Calendar => (0..365)
                .map(|i| {
                    // Fewer contributions on weekends, and some idle days.
                    let weekend = i % 7 == 0 || i % 7 == 6;
                    if self.rng.next() < if weekend { 0.6 } else { 0.25 } {
                        0.0
                    } else {
                        (self.rng.next().powi(2) * 20.0).round() + 1.0
                    }
                })
                .collect(),
            _ => Vec::new(),
        };
        self.grid.set_vec(values);
    }

    /// The treemap's items: the population table, randomized after the first
    /// time.
    fn tree_items(&mut self, randomize: bool) {
        let items: Vec<ChartTreemapItem> = POPULATION
            .iter()
            .map(|&(group, label, value)| ChartTreemapItem {
                label: label.into(),
                group: group.into(),
                value: if randomize {
                    (value * (0.5 + self.rng.next())).round()
                } else {
                    value
                },
                ..Default::default()
            })
            .collect();
        self.tree.set_vec(if self.sample.shape == Shape::Treemap {
            items
        } else {
            Vec::new()
        });
    }

    /// The data of the sunburst, funnel, waterfall and gauge samples,
    /// randomized after the first time.
    fn business_data(&mut self, randomize: bool) {
        // `v`, changed by up to `by` (a share) when randomizing.
        let vary = |rng: &mut Rng, v: f32, by: f32| {
            if randomize {
                (v * (1.0 - by + 2.0 * by * rng.next())).round()
            } else {
                v
            }
        };
        let mut sun = Vec::new();
        if self.sample.shape == Shape::Sunburst {
            // Continents, then their regions, then the countries.
            let index = |items: &Vec<ChartSunburstItem>, label: &str, parent: i32| {
                items
                    .iter()
                    .position(|i| i.label == label && i.parent == parent)
                    .map(|p| p as i32)
            };
            for (i, &(continent, country, value)) in POPULATION.iter().enumerate() {
                let c = index(&sun, continent, -1).unwrap_or_else(|| {
                    sun.push(ChartSunburstItem {
                        label: continent.into(),
                        parent: -1,
                        ..Default::default()
                    });
                    sun.len() as i32 - 1
                });
                let r = index(&sun, REGIONS[i], c).unwrap_or_else(|| {
                    sun.push(ChartSunburstItem {
                        label: REGIONS[i].into(),
                        parent: c,
                        ..Default::default()
                    });
                    sun.len() as i32 - 1
                });
                sun.push(ChartSunburstItem {
                    label: country.into(),
                    parent: r,
                    value: vary(&mut self.rng, value, 0.5),
                    ..Default::default()
                });
            }
        }
        self.sun.set_vec(sun);
        if self.sample.shape == Shape::Funnel {
            // Each stage keeps a share of the one before.
            let mut previous = 0.0;
            let stages: Vec<ChartSlice> = FUNNEL
                .iter()
                .enumerate()
                .map(|(i, &(label, value))| {
                    let value = if randomize && i > 0 {
                        (previous * (0.35 + 0.5 * self.rng.next())).round()
                    } else {
                        value
                    };
                    previous = value;
                    ChartSlice {
                        label: label.into(),
                        value,
                        ..Default::default()
                    }
                })
                .collect();
            self.slices.set_vec(stages);
        }
        let steps = if self.sample.shape == Shape::Waterfall {
            PROFIT_AND_LOSS
                .iter()
                .map(|&(label, value, total)| ChartWaterfallStep {
                    label: label.into(),
                    value: vary(&mut self.rng, value, 0.3),
                    total,
                })
                .collect()
        } else {
            Vec::new()
        };
        self.steps.set_vec(steps);
        if let Some(ui) = self.ui.upgrade() {
            ui.set_gauge_value(if randomize {
                self.rng.range(5.0, 98.0)
            } else {
                72.0
            });
            // The bullet charts: a KPI panel, then sales by region in
            // percent of target.
            const BULLET: [f32; 10] = [
                270.0, 22.5, 320.0, 1650.0, 4.5, 104.0, 87.0, 72.0, 95.0, 118.0,
            ];
            let values: Vec<f32> = BULLET
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    if !randomize {
                        v
                    } else if i == 4 {
                        // Satisfaction, out of 5.
                        (v * (0.7 + 0.4 * self.rng.next()) * 10.0).round().min(50.0) / 10.0
                    } else {
                        vary(&mut self.rng, v, 0.3)
                    }
                })
                .collect();
            ui.set_bullet_values(ModelRc::new(VecModel::from(values)));
        }
    }

    /// The interpolation sample compares two ways of drawing the same values:
    /// give every dataset the first one's values.
    fn share_interpolation_values(&mut self) {
        if self.sample.id != "line-interpolation" {
            return;
        }
        let Some(first) = self.datasets.row_data(0) else {
            return;
        };
        for row in 1..self.datasets.row_count() {
            if let Some(mut ds) = self.datasets.row_data(row) {
                ds.values = first.values.clone();
                self.datasets.set_row_data(row, ds);
            }
        }
    }

    /// The 12 month names in the chart language, or English.
    fn month_names(&self) -> Vec<SharedString> {
        let months = self
            .ui
            .upgrade()
            .map(|ui| ui.global::<ChartTheme>().get_locale().months);
        match months {
            Some(m) if m.row_count() == 12 => m.iter().collect(),
            _ => MONTHS.iter().map(|&m| m.into()).collect(),
        }
    }

    /// The 7 weekday names in the chart language, Monday first, or English.
    fn weekday_names(&self) -> Vec<SharedString> {
        let weekdays = self
            .ui
            .upgrade()
            .map(|ui| ui.global::<ChartTheme>().get_locale().weekdays);
        match weekdays {
            Some(w) if w.row_count() == 7 => w.iter().collect(),
            _ => ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
                .map(SharedString::from)
                .to_vec(),
        }
    }

    /// The index into `LANGUAGES` of the chart language: the picker's for
    /// the "languages" sample, English for the others.
    fn language(&self) -> usize {
        match self.ui.upgrade() {
            Some(ui) if self.sample.id == "languages" => {
                (ui.get_language().max(0) as usize).min(LANGUAGES.len() - 1)
            }
            _ => 0,
        }
    }

    /// Sets the chart language, and the "languages" sample's titles in it.
    fn set_chart_language(&self) {
        let language = self.language();
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<ChartTheme>().set_language(LANGUAGES[language]);
            let (title, visitors, rate) = LANGUAGE_TEXTS[language];
            ui.set_language_title(title.into());
            ui.set_language_y_title(visitors.into());
            ui.set_language_y2_title(rate.into());
        }
    }

    /// Sets the chart language, and puts the month and weekday labels and
    /// the "languages" sample's texts in it.
    fn relabel(&mut self) {
        self.set_chart_language();
        let language = self.language();
        if self.sample.id == "languages" {
            let (_, visitors, rate) = LANGUAGE_TEXTS[language];
            for (row, label) in [(0, visitors), (1, rate)] {
                if let Some(mut ds) = self.datasets.row_data(row) {
                    ds.label = label.into();
                    self.datasets.set_row_data(row, ds);
                }
            }
        }
        match self.sample.shape {
            Shape::Months { .. } | Shape::Exponential => {
                let months = self.month_names();
                for i in 0..self.labels.row_count() {
                    self.labels.set_row_data(i, months[i % 12].clone());
                }
            }
            Shape::Matrix => self.row_labels.set_vec(self.weekday_names()),
            _ => {}
        }
    }

    /// Builds the data of `sample` and shows it.
    fn select(&mut self, sample: &'static Sample) {
        self.sample = sample;
        self.randomized = false;
        self.set_chart_language();
        if let Some(ui) = self.ui.upgrade() {
            ui.set_view_start(0.0);
            ui.set_view_end(1.0);
        }
        let row_labels: Vec<SharedString> = match sample.shape {
            Shape::Matrix => self.weekday_names(),
            Shape::Correlation => ["Price", "Size", "Age", "Rooms", "Floor", "Garden"]
                .map(SharedString::from)
                .to_vec(),
            _ => Vec::new(),
        };
        self.row_labels.set_vec(row_labels);
        let labels: Vec<SharedString> = match sample.shape {
            Shape::Matrix => (0..24).map(|h| format!("{h:02}").into()).collect(),
            Shape::Correlation => ["Price", "Size", "Age", "Rooms", "Floor", "Garden"]
                .map(SharedString::from)
                .to_vec(),
            Shape::Survey => [
                "Engineering",
                "Design",
                "Sales",
                "Marketing",
                "Support",
                "All staff",
            ]
            .map(SharedString::from)
            .to_vec(),
            // A full year for the "languages" sample, to show every month name.
            Shape::Months { .. } if sample.id == "languages" => self.month_names(),
            Shape::Months { .. } | Shape::Exponential => {
                self.month_names().into_iter().take(7).collect()
            }
            Shape::Skills => SKILLS.iter().map(|&s| s.into()).collect(),
            Shape::Dumbbell => LIFE_EXPECTANCY.iter().map(|c| c.0.into()).collect(),
            Shape::Series { count } => (0..count).map(|i| i.to_string().into()).collect(),
            _ => Vec::new(),
        };
        self.labels.set_vec(labels);
        let count = match sample.shape {
            Shape::Months { datasets, .. } => datasets,
            Shape::Survey => 5,
            Shape::Points
            | Shape::Bubbles
            | Shape::Series { .. }
            | Shape::Exponential
            | Shape::TimeSeries { .. }
            | Shape::Skills
            | Shape::Dumbbell => 2,
            _ => 0,
        };
        let rows: Vec<ChartDataset> = (0..count).map(|i| self.dataset(i)).collect();
        self.datasets.set_vec(rows);
        self.share_interpolation_values();
        let slices: Vec<ChartSlice> = if sample.shape == Shape::Slices {
            (0..5).map(|i| self.slice(i)).collect()
        } else {
            Vec::new()
        };
        self.slices.set_vec(slices);
        self.grid_values();
        self.tree_items(false);
        self.business_data(false);

        let Some(ui) = self.ui.upgrade() else { return };
        ui.set_sample(sample.id.into());
        ui.set_sample_title(sample.title.into());
        ui.set_description(sample.description.into());
        ui.set_actions(strings(sample.actions.iter().copied()));
        ui.set_legend_position(ChartLegendPosition::Top);
        ui.set_decimate(true);
        ui.set_status("".into());
        if sample.shape == Shape::Live {
            self.live.paused = false;
            self.live_reset();
            self.show_live_stats(None);
        }
    }

    /// A sensor's next value: noise around a slow wave, different per sensor.
    fn sensor(&mut self, sensor: usize, time: f32) -> f32 {
        let wave = (time / (8.0 + 5.0 * sensor as f32)).sin() * 12.0;
        let target = 20.0 + 25.0 * sensor as f32 + wave;
        let last = self.live.last[sensor];
        let next = last + (target - last) * 0.1 + (self.rng.next() - 0.5) * 1.5;
        self.live.last[sensor] = next;
        next
    }

    /// Fills the live sample with the last minute of values at its rate.
    fn live_reset(&mut self) {
        let rate = self.live.rate;
        let capacity = (LIVE_SECONDS * rate) as usize;
        self.live.time = 0.0;
        self.live.last = [20.0, 45.0, 70.0];
        let mut models = Vec::new();
        let mut rows = Vec::new();
        for sensor in 0..3 {
            let points: Vec<ChartXY> = (0..capacity)
                .map(|i| {
                    let x = i as f32 / rate as f32;
                    ChartXY {
                        x,
                        y: self.sensor(sensor, x),
                        r: 0.0,
                    }
                })
                .collect();
            let model = Rc::new(StreamModel::new(capacity, points));
            rows.push(ChartDataset {
                label: format!("Sensor {}", ["A", "B", "C"][sensor]).into(),
                points: model.clone().into(),
                ..Default::default()
            });
            models.push(model);
        }
        self.live.time = (capacity.max(1) - 1) as f32 / rate as f32;
        self.live.models = models;
        self.live.updates = 0;
        self.live.since = now();
        self.datasets.set_vec(rows);
    }

    /// Appends one value per sensor, and every half second shows the
    /// measured update rate.
    fn live_tick(&mut self) {
        let rate = self.live.rate as f32;
        self.live.time += 1.0 / rate;
        let time = self.live.time;
        for sensor in 0..self.live.models.len() {
            let y = self.sensor(sensor, time);
            self.live.models[sensor].push(ChartXY { x: time, y, r: 0.0 });
        }
        self.live.updates += 1;
        let elapsed = now() - self.live.since;
        if elapsed >= 0.5 {
            let measured = self.live.updates as f64 / elapsed;
            self.live.updates = 0;
            self.live.since = now();
            self.show_live_stats(Some(measured));
        }
    }

    /// The point count, update rate and layout time under the chart.
    fn show_live_stats(&self, measured: Option<f64>) {
        let Some(ui) = self.ui.upgrade() else { return };
        let points = self
            .live
            .models
            .iter()
            .map(|m| m.row_count())
            .sum::<usize>();
        let rate = match measured {
            _ if self.live.paused => "paused".to_string(),
            Some(r) => format!("{r:.0} of {} updates/s", self.live.rate),
            None => format!("{} updates/s", self.live.rate),
        };
        // The browser's standard library has no clock, so layouts aren't timed.
        let layout = if cfg!(target_arch = "wasm32") {
            String::new()
        } else {
            format!(
                " · layout {:.1} ms",
                slintplot::profile::last_layout_time().as_secs_f64() * 1000.0
            )
        };
        ui.set_status(
            format!(
                "{} points · {rate}{layout}",
                slintplot::ticks::format_value(points as f64)
            )
            .into(),
        );
    }

    fn live_action(&mut self, name: &str) {
        match name {
            "Pause / resume" => self.live.paused = !self.live.paused,
            _ => {
                if let Some(rate) = name.split(' ').next().and_then(|r| r.parse().ok()) {
                    self.live.rate = rate;
                    self.live.paused = false;
                    self.live_reset();
                }
            }
        }
        self.show_live_stats(None);
    }

    /// Applies `f` to every dataset.
    fn update_datasets(&mut self, mut f: impl FnMut(&mut Self, &mut ChartDataset)) {
        for row in 0..self.datasets.row_count() {
            let Some(mut ds) = self.datasets.row_data(row) else {
                continue;
            };
            f(self, &mut ds);
            self.datasets.set_row_data(row, ds);
        }
    }

    fn action(&mut self, index: usize) {
        let Some(&name) = self.sample.actions.get(index) else {
            return;
        };
        let Some(ui) = self.ui.upgrade() else { return };
        ui.set_status("".into());
        if self.sample.shape == Shape::Live {
            self.live_action(name);
            return;
        }
        let slices = self.sample.shape == Shape::Slices;
        let points = matches!(self.sample.shape, Shape::Points | Shape::Bubbles);
        // The one-week sample: zoom to a span starting on Wednesday, to see
        // how the time axis labels narrow ranges.
        let span = (7 * 96 - 1) as f32 * 900.0;
        let wednesday = 2.0 * 86_400.0;
        let window = match name {
            "Week" => Some((0.0, span)),
            "Day" => Some((wednesday, wednesday + 86_400.0)),
            "6 hours" => Some((wednesday + 6.0 * 3600.0, wednesday + 12.0 * 3600.0)),
            "1 hour" => Some((wednesday + 8.0 * 3600.0, wednesday + 9.0 * 3600.0)),
            _ => None,
        };
        if let Some((start, end)) = window {
            ui.set_view_start(start / span);
            ui.set_view_end(end / span);
            return;
        }
        match name {
            "Randomize" if slices => {
                for row in 0..self.slices.row_count() {
                    let Some(mut s) = self.slices.row_data(row) else {
                        continue;
                    };
                    s.value = self.rng.range(5.0, 100.0);
                    self.slices.set_row_data(row, s);
                }
            }
            "Randomize" if self.sample.shape == Shape::Treemap => self.tree_items(true),
            "Randomize"
                if matches!(
                    self.sample.shape,
                    Shape::Sunburst
                        | Shape::Funnel
                        | Shape::Waterfall
                        | Shape::Gauge
                        | Shape::Bullet
                ) =>
            {
                self.business_data(true);
            }
            "Randomize"
                if matches!(
                    self.sample.shape,
                    Shape::Matrix | Shape::Correlation | Shape::Calendar
                ) =>
            {
                self.grid_values();
            }
            "Randomize" => {
                self.randomized = true;
                // Keep each dataset's label and visibility; new values only.
                for row in 0..self.datasets.row_count() {
                    let Some(old) = self.datasets.row_data(row) else {
                        continue;
                    };
                    let fresh = self.dataset(row);
                    self.datasets.set_row_data(
                        row,
                        ChartDataset {
                            values: fresh.values,
                            points: fresh.points,
                            ..old
                        },
                    );
                }
                self.share_interpolation_values();
            }
            "Add dataset" if slices => {
                ui.set_status("Pie, doughnut and polar area charts show one set of slices.".into());
            }
            "Add dataset" => {
                let ds = self.dataset(self.datasets.row_count());
                self.datasets.push(ds);
            }
            "Remove dataset" => {
                if self.datasets.row_count() > 0 {
                    self.datasets.remove(self.datasets.row_count() - 1);
                }
            }
            "Add data" if slices => {
                let s = self.slice(self.slices.row_count());
                self.slices.push(s);
            }
            "Add data" if points => {
                let bubbles = self.sample.shape == Shape::Bubbles;
                self.update_datasets(|state, ds| {
                    let mut all: Vec<ChartXY> = ds.points.iter().collect();
                    all.extend(state.points(1, bubbles));
                    ds.points = ModelRc::new(VecModel::from(all));
                });
            }
            "Add data" => {
                let next = self.labels.row_count();
                self.labels.push(self.month_names()[next % 12].clone());
                self.update_datasets(|state, ds| {
                    let mut values: Vec<f32> = ds.values.iter().collect();
                    values.extend(state.values(1));
                    ds.values = floats(values);
                });
            }
            "Remove data" if slices => {
                if self.slices.row_count() > 0 {
                    self.slices.remove(self.slices.row_count() - 1);
                }
            }
            "Remove data" => {
                if !points && self.labels.row_count() > 0 {
                    self.labels.remove(self.labels.row_count() - 1);
                }
                self.update_datasets(|_, ds| {
                    let mut values: Vec<f32> = ds.values.iter().collect();
                    values.pop();
                    ds.values = floats(values);
                    let mut all: Vec<ChartXY> = ds.points.iter().collect();
                    all.pop();
                    ds.points = ModelRc::new(VecModel::from(all));
                });
            }
            "Steps on" | "Steps off" => {
                let on = name == "Steps on";
                self.update_datasets(|_, ds| ds.stepped = on);
            }
            "Position: top" => ui.set_legend_position(ChartLegendPosition::Top),
            "Position: right" => ui.set_legend_position(ChartLegendPosition::Right),
            "Position: bottom" => ui.set_legend_position(ChartLegendPosition::Bottom),
            "Position: left" => ui.set_legend_position(ChartLegendPosition::Left),
            "Decimation on" => ui.set_decimate(true),
            "Decimation off" => {
                ui.set_decimate(false);
                ui.set_status("Without decimation, 100,000-point lines are slow to draw.".into());
            }
            "Reset zoom" => self.select(self.sample),
            _ => {}
        }
    }
}

/// Creates the gallery window with its samples and callbacks.
fn gallery() -> Result<(Gallery, Rc<RefCell<State>>), slint::PlatformError> {
    let ui = Gallery::new()?;
    slintplot::register!(ui);

    // The sample list, with a header row per category, in the order the
    // categories first appear.
    let mut categories: Vec<&str> = Vec::new();
    for s in SAMPLES {
        if !categories.contains(&s.category) {
            categories.push(s.category);
        }
    }
    let mut entries = Vec::new();
    for category in categories {
        entries.push(SampleEntry {
            id: "".into(),
            title: category.into(),
            header: true,
        });
        for s in SAMPLES.iter().filter(|s| s.category == category) {
            entries.push(SampleEntry {
                id: s.id.into(),
                title: s.title.into(),
                header: false,
            });
        }
    }
    ui.set_samples(ModelRc::new(VecModel::from(entries)));

    let state = Rc::new(RefCell::new(State {
        ui: ui.as_weak(),
        rng: Rng(SEED),
        sample: &SAMPLES[0],
        labels: Rc::new(VecModel::default()),
        datasets: Rc::new(VecModel::default()),
        slices: Rc::new(VecModel::default()),
        row_labels: Rc::new(VecModel::default()),
        grid: Rc::new(VecModel::default()),
        tree: Rc::new(VecModel::default()),
        sun: Rc::new(VecModel::default()),
        steps: Rc::new(VecModel::default()),
        randomized: false,
        live: Live {
            rate: 30,
            paused: false,
            time: 0.0,
            last: [0.0; 3],
            models: Vec::new(),
            updates: 0,
            since: 0.0,
        },
    }));
    {
        let s = state.borrow();
        ui.set_labels(s.labels.clone().into());
        ui.set_datasets(s.datasets.clone().into());
        ui.set_slices(s.slices.clone().into());
        ui.set_row_labels(s.row_labels.clone().into());
        ui.set_grid(s.grid.clone().into());
        ui.set_tree(s.tree.clone().into());
        ui.set_sun(s.sun.clone().into());
        ui.set_steps(s.steps.clone().into());
    }
    state.borrow_mut().select(&SAMPLES[0]);

    // The live sample's updates, while it is shown and not paused.
    let timer = Rc::new(slint::Timer::default());
    let timer_rate = Rc::new(Cell::new(0));
    let s = state.clone();
    let (t, r) = (timer.clone(), timer_rate.clone());
    ui.on_select(move |id| {
        if let Some(sample) = SAMPLES.iter().find(|x| x.id == id.as_str()) {
            s.borrow_mut().select(sample);
        }
        run_live_timer(&t, &r, &s);
    });
    let s = state.clone();
    ui.on_action(move |index| {
        s.borrow_mut().action(index.max(0) as usize);
        run_live_timer(&timer, &timer_rate, &s);
    });
    let s = state.clone();
    ui.on_dark_mode_changed(move || s.borrow_mut().recolor_survey());
    let s = state.clone();
    ui.on_language_changed(move || s.borrow_mut().relabel());
    #[cfg(not(target_arch = "wasm32"))]
    ui.on_open_url(|url| open_in_browser(&url));
    Ok((ui, state))
}

/// Runs `timer` at the live sample's update rate while the sample is shown
/// and not paused, and stops it otherwise. `running` is the rate it runs at.
fn run_live_timer(timer: &slint::Timer, running: &Cell<u32>, state: &Rc<RefCell<State>>) {
    let rate = {
        let s = state.borrow();
        if s.sample.shape == Shape::Live && !s.live.paused {
            s.live.rate
        } else {
            0
        }
    };
    if rate == running.get() {
        return;
    }
    running.set(rate);
    if rate == 0 {
        timer.stop();
        return;
    }
    let state = Rc::downgrade(state);
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_secs_f64(1.0 / rate as f64),
        move || {
            if let Some(state) = state.upgrade() {
                state.borrow_mut().live_tick();
            }
        },
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    // `--docs`: the website's screenshots, rather than every snapshot.
    let mut docs_dir = None;
    let snapshot_dir = match (args.next().as_deref(), args.next()) {
        (Some("--snapshot"), Some(dir)) => Some(std::path::PathBuf::from(dir)),
        (Some("--docs"), Some(dir)) => {
            docs_dir = Some(std::path::PathBuf::from(dir));
            docs_dir.clone()
        }
        (None, _) => None,
        _ => return Err("usage: gallery [--snapshot <dir> | --docs <dir>]".into()),
    };
    let headless = snapshot_dir.as_ref().map(|_| snapshot::install());
    // macOS: keep the system frame, so the window has the system's buttons
    // and resizing, but make the title bar see-through, so the content
    // reaches the top edge and the buttons sit over the sample list.
    #[cfg(target_os = "macos")]
    if headless.is_none() && !drawn_frame() {
        use slint::winit_030::winit::platform::macos::WindowAttributesExtMacOS;
        slint::BackendSelector::new()
            .with_winit_window_attributes_hook(|attributes| {
                attributes
                    .with_titlebar_transparent(true)
                    .with_fullsize_content_view(true)
                    .with_title_hidden(true)
            })
            .select()?;
    }

    let (ui, state) = gallery()?;
    match (headless, snapshot_dir) {
        (Some(window), Some(dir)) if docs_dir.is_some() => docs(&ui, &state, &window, &dir),
        (Some(window), Some(dir)) => snapshots(&ui, &state, &window, &dir),
        _ => {
            init_window_frame(&ui);
            // Once the app has finished launching, which would reset it.
            slint::Timer::single_shot(Duration::ZERO, dock_icon::set);
            Ok(ui.run()?)
        }
    }
}

/// Opens `url` in the default browser.
#[cfg(not(target_arch = "wasm32"))]
fn open_in_browser(url: &str) {
    use std::process::Command;
    let result = if cfg!(target_os = "macos") {
        Command::new("open").arg(url).spawn()
    } else if cfg!(windows) {
        Command::new("cmd").args(["/C", "start", "", url]).spawn()
    } else {
        Command::new("xdg-open").arg(url).spawn()
    };
    if let Err(error) = result {
        eprintln!("couldn't open {url}: {error}");
    }
}

/// Whether the app draws the window frame: on Windows and Linux, and on
/// macOS with `SLINTPLOT_DRAWN_FRAME` set, to try it there.
#[cfg(not(target_arch = "wasm32"))]
fn drawn_frame() -> bool {
    !cfg!(target_os = "macos") || std::env::var_os("SLINTPLOT_DRAWN_FRAME").is_some()
}

/// Connects the window frame (see ui/window_frame.slint) to the window.
#[cfg(not(target_arch = "wasm32"))]
fn init_window_frame(ui: &Gallery) {
    use slint::winit_030::{WinitWindowAccessor, winit::window::ResizeDirection};
    let frame = ui.global::<WindowFrame>();
    frame.set_drawn(drawn_frame());
    frame.set_mac_titlebar(!drawn_frame());
    let with_ui = |f: fn(&Gallery)| {
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                f(&ui)
            }
        }
    };
    frame.on_drag(with_ui(|ui| {
        ui.window().with_winit_window(|w| {
            let _ = w.drag_window();
        });
    }));
    frame.on_close(with_ui(|ui| {
        let _ = ui.window().hide();
    }));
    frame.on_minimize(with_ui(|ui| ui.window().set_minimized(true)));
    frame.on_toggle_maximize(with_ui(|ui| {
        let maximized = !ui.window().is_maximized();
        ui.window().set_maximized(maximized);
        ui.global::<WindowFrame>().set_maximized(maximized);
    }));
    let weak = ui.as_weak();
    frame.on_resize(move |direction| {
        let direction = match direction {
            0 => ResizeDirection::East,
            1 => ResizeDirection::North,
            2 => ResizeDirection::NorthEast,
            3 => ResizeDirection::NorthWest,
            4 => ResizeDirection::South,
            5 => ResizeDirection::SouthEast,
            6 => ResizeDirection::SouthWest,
            _ => ResizeDirection::West,
        };
        if let Some(ui) = weak.upgrade() {
            ui.window().with_winit_window(|w| {
                let _ = w.drag_resize_window(direction);
            });
        }
    });
}

/// In the browser: counts the click with Plausible, as for a link that leaves
/// the site, and opens `url` in a new tab, or in this one if that is blocked.
#[cfg(target_arch = "wasm32")]
fn open_page(url: &str) {
    use js_sys::{Function, Object, Reflect};
    use wasm_bindgen::{JsCast, JsValue};
    let global = js_sys::global();
    let function = |name: &str| {
        Reflect::get(&global, &name.into())
            .ok()
            .and_then(|f| f.dyn_into::<Function>().ok())
    };
    if let Some(plausible) = function("plausible") {
        let props = Object::new();
        let options = Object::new();
        let _ = Reflect::set(&props, &"url".into(), &url.into());
        let _ = Reflect::set(&options, &"props".into(), &props);
        let _ = plausible.call2(&global, &"Outbound Link: Click".into(), &options);
    }
    let opened = function("open")
        .and_then(|open| open.call2(&global, &url.into(), &"_blank".into()).ok())
        .is_some_and(|tab| !tab.is_null() && tab != JsValue::UNDEFINED);
    if !opened {
        if let Ok(location) = Reflect::get(&global, &"location".into()) {
            let _ = Reflect::set(&location, &"href".into(), &url.into());
        }
    }
}

/// In the browser: the gallery in the page's `<canvas id="canvas">`.
#[cfg(target_arch = "wasm32")]
fn main() {
    console_error_panic_hook::set_once();
    let (ui, _state) = gallery().expect("the gallery window");
    // The browser has no fonts for Japanese and Chinese.
    ui.set_cjk_fonts(false);
    ui.on_open_url(|url| open_page(url.as_str()));
    ui.run().expect("the event loop");
}

/// Renders the website's screenshots that show the gallery into `dir`
/// (docs/screenshots): the whole gallery for the page's larger pictures, and
/// in `charts/`, the samples the page shows without a code example, 1012 by
/// 660 pixels. The other chart pictures are rendered from the page's code
/// examples by `slintplot-docshots`.
#[cfg(not(target_arch = "wasm32"))]
fn docs(
    ui: &Gallery,
    state: &Rc<RefCell<State>>,
    window: &Rc<slint::platform::software_renderer::MinimalSoftwareWindow>,
    dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let charts = dir.join("charts");
    std::fs::create_dir_all(&charts)?;
    ui.show()?;
    let shot = |sample: &str, name: &str, dark: bool| -> std::io::Result<()> {
        ui.set_dark_mode(dark);
        // The same data every time.
        state.borrow_mut().rng = Rng(SEED);
        let sample = SAMPLES.iter().find(|s| s.id == sample).unwrap();
        state.borrow_mut().select(sample);
        snapshot::render(window);
        snapshot::settle(Duration::from_millis(1100));
        snapshot::render(window);
        snapshot::settle(Duration::from_millis(400));
        snapshot::save(window, &charts.join(format!("{name}.png")))
    };

    // The charts alone.
    ui.set_chart_only(true);
    snapshot::resize(window, 1012, 660);
    for dark in [false, true] {
        let suffix = if dark { "-dark" } else { "" };
        shot(
            "heatmap-diverging",
            &format!("heatmap-diverging{suffix}"),
            dark,
        )?;
        ui.set_language(2);
        shot("languages", &format!("languages-de{suffix}"), dark)?;
        ui.set_language(0);
    }

    // The whole gallery.
    ui.set_chart_only(false);
    snapshot::resize(window, snapshot::WIDTH, snapshot::HEIGHT);
    for (sample, name, dark) in [
        ("axes-format", "light", false),
        ("axes-format", "light-dark", true),
        ("bar-vertical", "gallery", false),
        ("bar-vertical", "gallery-dark", true),
        ("time-hourly", "dark", true),
    ] {
        ui.set_dark_mode(dark);
        state.borrow_mut().rng = Rng(SEED);
        state
            .borrow_mut()
            .select(SAMPLES.iter().find(|s| s.id == sample).unwrap());
        snapshot::render(window);
        snapshot::settle(Duration::from_millis(1100));
        snapshot::save(window, &dir.join(format!("{name}.png")))?;
    }
    Ok(())
}

/// Renders every sample, and a few interactions, to PNG files in `dir`.
#[cfg(not(target_arch = "wasm32"))]
fn snapshots(
    ui: &Gallery,
    state: &Rc<RefCell<State>>,
    window: &Rc<slint::platform::software_renderer::MinimalSoftwareWindow>,
    dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dir)?;
    ui.show()?;
    for (dark, suffix) in [(false, ""), (true, "-dark")] {
        ui.set_dark_mode(dark);
        // The same data in both modes.
        state.borrow_mut().rng = Rng(SEED);
        for sample in SAMPLES {
            state.borrow_mut().select(sample);
            snapshot::render(window);
            snapshot::settle(Duration::from_millis(1100));
            snapshot::save(window, &dir.join(format!("{}{suffix}.png", sample.id)))?;
        }
    }
    // A legend at the side, and hover tooltips.
    ui.set_dark_mode(false);
    let legend = SAMPLES.iter().find(|s| s.id == "legend-position").unwrap();
    state.borrow_mut().select(legend);
    ui.set_legend_position(ChartLegendPosition::Left);
    snapshot::render(window);
    snapshot::settle(Duration::from_millis(1100));
    snapshot::save(window, &dir.join("legend-left.png"))?;
    state.borrow_mut().select(&SAMPLES[0]);
    snapshot::render(window);
    snapshot::settle(Duration::from_millis(1100));
    snapshot::hover(window, 640.0, 400.0);
    snapshot::save(window, &dir.join("hover-bar.png"))?;
    // The one-week time series zoomed to narrower and narrower spans.
    let week = SAMPLES.iter().find(|s| s.id == "time-week").unwrap();
    state.borrow_mut().select(week);
    snapshot::render(window);
    snapshot::settle(Duration::from_millis(1100));
    for (action, name) in [(1, "day"), (2, "6h"), (3, "1h")] {
        state.borrow_mut().action(action);
        snapshot::render(window);
        snapshot::settle(Duration::from_millis(300));
        snapshot::save(window, &dir.join(format!("time-week-{name}.png")))?;
    }
    // Keyboard navigation: click a chart to focus it, move the mouse
    // away, then use the arrow keys.
    use slint::platform::Key;
    for (id, (x, y), keys) in [
        (
            "scatter",
            (700.0, 450.0),
            &[Key::RightArrow, Key::RightArrow, Key::DownArrow][..],
        ),
        (
            "radar",
            (760.0, 420.0),
            &[Key::RightArrow, Key::RightArrow][..],
        ),
        (
            "heatmap",
            (700.0, 400.0),
            &[Key::DownArrow, Key::RightArrow, Key::RightArrow][..],
        ),
        (
            "calendar",
            (700.0, 200.0),
            &[Key::RightArrow, Key::RightArrow, Key::DownArrow][..],
        ),
        (
            "treemap",
            (560.0, 300.0),
            &[Key::RightArrow, Key::RightArrow][..],
        ),
        (
            "sunburst",
            (300.0, 650.0),
            &[Key::RightArrow, Key::RightArrow][..],
        ),
        (
            "funnel",
            (300.0, 650.0),
            &[Key::DownArrow, Key::DownArrow][..],
        ),
        ("waterfall", (700.0, 400.0), &[Key::RightArrow][..]),
        ("dumbbell", (700.0, 400.0), &[Key::DownArrow][..]),
    ] {
        let sample = SAMPLES.iter().find(|s| s.id == id).unwrap();
        state.borrow_mut().select(sample);
        snapshot::render(window);
        snapshot::settle(Duration::from_millis(1100));
        snapshot::click(window, x, y);
        snapshot::leave(window);
        for &k in keys {
            snapshot::key(window, k);
        }
        snapshot::render(window);
        snapshot::settle(Duration::from_millis(300));
        snapshot::save(window, &dir.join(format!("keys-{id}.png")))?;
    }
    // The treemap: a tooltip, then Asia hidden from the legend.
    let tree = SAMPLES.iter().find(|s| s.id == "treemap").unwrap();
    state.borrow_mut().select(tree);
    snapshot::render(window);
    snapshot::settle(Duration::from_millis(1100));
    snapshot::hover(window, 560.0, 300.0);
    snapshot::save(window, &dir.join("treemap-hover.png"))?;
    snapshot::click(window, 625.0, 126.0);
    snapshot::render(window);
    snapshot::settle(Duration::from_millis(300));
    snapshot::save(window, &dir.join("treemap-hidden.png"))?;
    // The sunburst zoomed into Asia, a gauge band and a waterfall step.
    for (id, (x, y), click, name) in [
        ("sunburst", (850.0, 497.0), true, "sunburst-zoomed"),
        ("gauge", (1010.0, 300.0), false, "gauge-hover"),
        ("waterfall", (670.0, 420.0), false, "waterfall-hover"),
        ("axes-two", (1000.0, 420.0), false, "axes-two-hover"),
    ] {
        let sample = SAMPLES.iter().find(|s| s.id == id).unwrap();
        state.borrow_mut().select(sample);
        snapshot::render(window);
        snapshot::settle(Duration::from_millis(1100));
        if click {
            snapshot::click(window, x, y);
            snapshot::render(window);
            snapshot::settle(Duration::from_millis(300));
        } else {
            snapshot::hover(window, x, y);
        }
        snapshot::save(window, &dir.join(format!("{name}.png")))?;
    }
    let zoom = SAMPLES.iter().find(|s| s.id == "zoom").unwrap();
    state.borrow_mut().select(zoom);
    snapshot::render(window);
    snapshot::settle(Duration::from_millis(1100));
    snapshot::scroll(window, 740.0, 400.0, 800.0);
    snapshot::save(window, &dir.join("zoomed.png"))?;
    // The calendar in GitHub's green, from its own color picker.
    let calendar = SAMPLES.iter().find(|s| s.id == "calendar").unwrap();
    state.borrow_mut().select(calendar);
    ui.set_calendar_colors(1);
    snapshot::render(window);
    snapshot::settle(Duration::from_millis(1100));
    snapshot::save(window, &dir.join("calendar-github.png"))?;
    ui.set_calendar_colors(0);
    // The live sample after three seconds of updates, run directly since
    // the headless snapshots have no timers.
    let live = SAMPLES.iter().find(|s| s.id == "live").unwrap();
    state.borrow_mut().select(live);
    for _ in 0..90 {
        state.borrow_mut().live_tick();
    }
    snapshot::render(window);
    snapshot::save(window, &dir.join("live-streamed.png"))?;
    // The "languages" sample in other languages, by index into `LANGUAGES`.
    let languages = SAMPLES.iter().find(|s| s.id == "languages").unwrap();
    for (language, name) in [(2, "de"), (7, "nb"), (12, "ja")] {
        ui.set_language(language);
        state.borrow_mut().select(languages);
        snapshot::render(window);
        snapshot::settle(Duration::from_millis(1100));
        snapshot::save(window, &dir.join(format!("languages-{name}.png")))?;
    }
    ui.set_language(0);
    Ok(())
}

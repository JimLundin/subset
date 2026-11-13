//! `subset report-html`: one static page that breaks the score down. No
//! script: bars are CSS, the treemap and history are inline SVG, and every
//! chart has a table or tooltip with its numbers.

use std::collections::BTreeMap;

use crate::config::{TIERS, Tier, Weights};
use crate::history::Snapshot;
use crate::score::{self, Level, Row, ScoredFile, float, ratio};
use crate::walk::Within;

/// The files of one scope: `include`, or a `--compare` directory.
pub(crate) struct Group {
    pub(crate) title: String,
    pub(crate) note: String,
    pub(crate) comparison: Within,
    pub(crate) files: Vec<ScoredFile>,
}

pub(crate) fn render(
    stamp: &str,
    groups: &[Group],
    history: &Result<Vec<Snapshot>, String>,
    w: &Weights,
) -> String {
    let sections = groups
        .iter()
        .enumerate()
        .map(|(i, g)| group(i, g, w))
        .collect::<String>();
    format!(
        "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
         <title>subset score</title><style>{CSS}</style></head><body>\
         <header><h1>subset score</h1><p class=\"stamp\">{}</p></header>\
         {sections}{}{}</body></html>\n",
        esc(stamp),
        history_section(history),
        method(w)
    )
}

fn group(i: usize, g: &Group, w: &Weights) -> String {
    let total = score::total(&g.files);
    let badge = match g.comparison {
        Within::In => "<span class=\"badge\">Comparison</span>",
        Within::Out => "",
    };
    let violations = g.files.iter().map(|f| f.violations).sum::<usize>();
    format!(
        "<section id=\"g{i}\"><h2>{} {badge}</h2><p class=\"note\">{}</p>{}{}{}{}{}{}</section>",
        esc(&g.title),
        g.note,
        tiles(&total, violations, w),
        crates(&g.files, w),
        treemap(&g.files, w),
        typing(&g.files),
        constructs(&g.files, w),
        top_fns(&g.files, w)
    )
}

fn tile(label: &str, value: String, title: &str) -> String {
    format!(
        "<div class=\"tile\" title=\"{}\"><div class=\"label\">{label}</div><div class=\"value\">{value}</div></div>",
        esc(title)
    )
}

fn tiles(total: &Row, violations: usize, w: &Weights) -> String {
    format!(
        "<div class=\"tiles\">{}{}{}{}{}{}</div>",
        tile(
            "Score",
            whole(total.score(w)),
            "The weighted score of every token."
        ),
        tile(
            "Score in lines",
            format!("≈ {}", whole(total.approx_lines(w))),
            "Approximate: the score over this scope's own tokens per line."
        ),
        tile(
            "Tokens",
            total.tokens().to_string(),
            &format!("{} lines, comments excluded", total.lines)
        ),
        tile(
            "Per token",
            format!("{:.3}", total.per_token(w)),
            "Score per token: 1.0 is neutral."
        ),
        tile(
            "Boundary",
            format!("{:.1}%", total.boundary_share(w)),
            "Share of the score a library fixes."
        ),
        tile(
            "Violations",
            violations.to_string(),
            "Constructs outside the subset."
        )
    )
}

/// A bar of segments, each a share of `total`, as CSS flex.
fn bar(segments: &[(String, usize, String)], total: usize) -> String {
    let parts = segments
        .iter()
        .filter(|(_, n, _)| n > &0)
        .map(|(class, n, title)| {
            format!(
                "<span class=\"{class}\" style=\"flex-grow:{n}\" title=\"{}\"></span>",
                esc(title)
            )
        })
        .collect::<String>();
    let width = ratio(float(total).mul_add(100.0, 0.0), 1);
    format!("<div class=\"bar\" style=\"width:{width:.1}%\">{parts}</div>")
}

fn legend(names: &[&str]) -> String {
    let items = names
        .iter()
        .map(|n| format!("<span><i class=\"{n}\"></i>{n}</span>"))
        .collect::<String>();
    format!("<p class=\"legend\">{items}</p>")
}

fn crates(files: &[ScoredFile], w: &Weights) -> String {
    let rows = score::rows(files, Level::Crate, w);
    let widest = rows.iter().map(|r| r.score(w)).fold(0.0, f64::max);
    let lines = rows
        .iter()
        .map(|r| {
            let segments = TIERS
                .iter()
                .zip(&r.tiers)
                .map(|(t, n)| {
                    let weight = float(n.to_owned()).mul_add(w.weight(t.to_owned()), 0.0);
                    let tenths = scaled(weight);
                    (t.name().to_string(), tenths, format!("{}: {weight:.1}", t.name()))
                })
                .collect::<Vec<(String, usize, String)>>();
            let share = ratio(r.score(w).mul_add(100.0, 0.0), 1).mul_add(widest.recip(), 0.0);
            format!(
                "<tr><td>{}</td><td class=\"chart\">{}</td><td>{}</td><td>≈ {} lines</td><td>{}</td><td>{}</td></tr>",
                esc(&r.name),
                bar(&segments, 1).replace("width:100.0%", &format!("width:{share:.1}%")),
                whole(r.score(w)),
                whole(r.approx_lines(w)),
                r.tokens(),
                r.lines
            )
        })
        .collect::<String>();
    format!(
        "<h3>Score per crate, by tier</h3>{}<table><tr><th>Crate</th><th></th><th>Score</th><th>In lines</th><th>Tokens</th><th>Lines</th></tr>{lines}</table>",
        legend(&TIERS.iter().map(Tier::name).collect::<Vec<&str>>())
    )
}

/// A non-negative score in tenths, as a whole count.
fn scaled(x: f64) -> usize {
    format!("{:.0}", x.mul_add(10.0, 0.0))
        .parse()
        .unwrap_or_default()
}

fn typing(files: &[ScoredFile]) -> String {
    let names = CLASS_NAMES;
    let by_crate = files
        .iter()
        .fold(BTreeMap::<String, Vec<usize>>::new(), |mut map, f| {
            let sum = map
                .remove(&f.krate)
                .unwrap_or_else(|| Vec::from([0; 4]))
                .into_iter()
                .zip(&f.typing)
                .map(|(a, b)| a.saturating_add(b.to_owned()))
                .collect();
            map.insert(f.krate.clone(), sum);
            map
        });
    let lines = by_crate
        .iter()
        .map(|(name, counts)| {
            let total = counts.iter().sum::<usize>();
            let segments = names
                .iter()
                .zip(counts)
                .map(|(n, c)| (n.to_string(), c.to_owned(), format!("{n}: {c} tokens")))
                .collect::<Vec<(String, usize, String)>>();
            let cells = counts
                .iter()
                .map(|c| format!("<td>{c}</td>"))
                .collect::<String>();
            format!(
                "<tr><td>{}</td><td class=\"chart\">{}</td>{cells}<td>{total}</td></tr>",
                esc(name),
                bar(&segments, 1)
            )
        })
        .collect::<String>();
    format!(
        "<h3>Typing of fields and signatures</h3>{}<table><tr><th>Crate</th><th></th><th>Own</th><th>Primitive</th><th>Untyped</th><th>Other</th><th>Tokens</th></tr>{lines}</table>",
        legend(names)
    )
}

const CLASS_NAMES: &[&str] = &["own", "primitive", "untyped", "other"];

const TOP: usize = 25;

fn construct_line(r: &Row, w: &Weights) -> String {
    let (count, tier) = r
        .construct
        .map_or((0, String::new()), |(c, t)| (c, t.name().to_string()));
    format!(
        "<tr><td><code>{}</code></td><td>{tier}</td><td>{count}</td><td>{}</td><td>{}</td><td>{:.3}</td></tr>",
        esc(&r.name),
        whole(r.score(w)),
        r.tokens(),
        r.per_token(w)
    )
}

fn constructs(files: &[ScoredFile], w: &Weights) -> String {
    let rows = score::rows(files, Level::Construct, w);
    let head = "<tr><th>Construct</th><th>Tier</th><th>Count</th><th>Score</th><th>Tokens</th><th>Per token</th></tr>";
    let top = rows
        .iter()
        .take(TOP)
        .map(|r| construct_line(r, w))
        .collect::<String>();
    let all = rows
        .iter()
        .map(|r| construct_line(r, w))
        .collect::<String>();
    format!(
        "<h3>Constructs by score</h3><table>{head}{top}</table><details><summary>All {} constructs</summary><table>{head}{all}</table></details>",
        rows.len()
    )
}

fn top_fns(files: &[ScoredFile], w: &Weights) -> String {
    let lines = score::rows(files, Level::Fn, w)
        .iter()
        .take(TOP)
        .map(|r| {
            format!(
                "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{:.3}</td></tr>",
                esc(&r.name),
                whole(r.score(w)),
                r.tokens(),
                r.per_token(w)
            )
        })
        .collect::<String>();
    format!(
        "<h3>Functions by score</h3><table><tr><th>Function</th><th>Score</th><th>Tokens</th><th>Per token</th></tr>{lines}</table>"
    )
}

/// A rectangle of the treemap.
#[derive(Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// -1, without a unary minus.
const MINUS_ONE: f64 = f64::from_bits(0xBFF0_0000_0000_0000);

fn minus(a: f64, b: f64) -> f64 {
    b.mul_add(MINUS_ONE, a)
}

fn over(a: f64, b: f64) -> f64 {
    a.mul_add(b.recip(), 0.0)
}

/// The worst aspect ratio of a row of `areas` laid along `side`.
fn worst(areas: &[f64], side: f64) -> f64 {
    let sum = areas.iter().sum::<f64>();
    let s2 = sum.mul_add(sum, 0.0);
    let side2 = side.mul_add(side, 0.0);
    areas
        .iter()
        .map(|a| {
            let wide = over(side2.mul_add(a.to_owned(), 0.0), s2);
            let tall = over(s2, side2.mul_add(a.to_owned(), 0.0));
            wide.max(tall)
        })
        .fold(0.0, f64::max)
}

/// How a treemap row lies in its bounds.
#[derive(Clone, Copy)]
enum Lay {
    /// A column along the left edge, for bounds wider than tall.
    Column,
    /// A row along the top edge.
    Row,
}

fn plus(a: f64, b: f64) -> f64 {
    a.mul_add(1.0, b)
}

/// The first `n` areas.
fn head(areas: &[f64], n: usize) -> Vec<f64> {
    areas.iter().take(n).copied().collect()
}

/// The rectangle of one area, `offset` along a row of thickness `thick`.
fn place(bounds: Rect, lay: Lay, offset: f64, length: f64, thick: f64) -> Rect {
    match lay {
        Lay::Column => Rect {
            y: plus(bounds.y, offset),
            w: thick,
            h: length,
            ..bounds
        },
        Lay::Row => Rect {
            x: plus(bounds.x, offset),
            w: length,
            h: thick,
            ..bounds
        },
    }
}

/// Squarified treemap: `areas` sum to the area of `bounds`.
fn squarify(areas: &[f64], bounds: Rect) -> Vec<Rect> {
    let side = bounds.w.min(bounds.h);
    // The row grows while another area doesn't worsen its worst ratio.
    let count = std::ops::RangeInclusive::new(1, areas.len())
        .take_while(|k| {
            k == &1
                || worst(&head(areas, k.to_owned()), side)
                    <= worst(&head(areas, k.saturating_sub(1)), side)
        })
        .last()
        .unwrap_or_default();
    match (count, areas.split_at_checked(count)) {
        (0, _) | (_, None) => Vec::new(),
        (_, Some((row, rest))) => {
            let sum = row.iter().sum::<f64>();
            let lay = match bounds.w.partial_cmp(&bounds.h) {
                Some(std::cmp::Ordering::Less) => Lay::Row,
                _ => Lay::Column,
            };
            let thick = match lay {
                Lay::Column => over(sum, bounds.h),
                Lay::Row => over(sum, bounds.w),
            };
            let placed = row
                .iter()
                .fold((0.0, Vec::new()), |(offset, mut rects), a| {
                    let length = over(a.to_owned(), thick);
                    rects.push(place(bounds, lay, offset, length, thick));
                    (plus(offset, length), rects)
                })
                .1;
            let rest_bounds = match lay {
                Lay::Column => Rect {
                    x: plus(bounds.x, thick),
                    w: minus(bounds.w, thick),
                    ..bounds
                },
                Lay::Row => Rect {
                    y: plus(bounds.y, thick),
                    h: minus(bounds.h, thick),
                    ..bounds
                },
            };
            placed
                .into_iter()
                .chain(squarify(rest, rest_bounds))
                .collect()
        }
    }
}

const BINS: &[f64] = &[0.8, 0.95, 1.1, 1.3, 1.6, 2.0];

fn treemap(files: &[ScoredFile], w: &Weights) -> String {
    let scored = files
        .iter()
        .map(|f| (f, score::weigh(&f.tiers, w)))
        .filter(|(_, s)| s > &0.0)
        .map(|(f, s)| ((std::cmp::Reverse(s.to_bits()), f.path.clone()), (f, s)))
        .collect::<BTreeMap<(std::cmp::Reverse<u64>, String), (&ScoredFile, f64)>>()
        .into_values()
        .collect::<Vec<(&ScoredFile, f64)>>();
    let total = scored.iter().map(|(_, s)| s).sum::<f64>();
    let bounds = Rect {
        x: 0.0,
        y: 0.0,
        w: 960.0,
        h: 480.0,
    };
    let areas = scored
        .iter()
        .map(|(_, s)| over(s.mul_add(bounds.w.mul_add(bounds.h, 0.0), 0.0), total))
        .collect::<Vec<f64>>();
    let cells = scored
        .iter()
        .zip(squarify(&areas, bounds))
        .map(|((f, s), r)| {
            let per = ratio(s.to_owned(), f.tiers.iter().sum());
            let bin = BINS.iter().position(|b| &per < b).unwrap_or(BINS.len());
            let label = f.path.rsplit('/').next().unwrap_or_default();
            let text = Some(r).filter(|r| r.w > 60.0 && r.h > 18.0).map_or(String::new(), |r| {
                format!(
                    "<text x=\"{:.1}\" y=\"{:.1}\">{}</text>",
                    plus(r.x, 4.0),
                    plus(r.y, 14.0),
                    esc(label)
                )
            });
            format!(
                "<g><title>{}: {} score, {:.3} per token, {} violations</title><rect class=\"b{bin}\" x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\"/>{text}</g>",
                esc(&f.path),
                whole(s.to_owned()),
                per,
                f.violations,
                r.x,
                r.y,
                r.w,
                r.h
            )
        })
        .collect::<String>();
    format!(
        "<h3>Files by score</h3><p class=\"note\">Area is a file's score; colour its score per token, from encouraged (light) to outside (dark).</p><svg class=\"treemap\" viewBox=\"0 0 960 480\" role=\"img\" aria-label=\"Files by score\">{cells}</svg>"
    )
}

fn history_section(history: &Result<Vec<Snapshot>, String>) -> String {
    let body = match history {
        Err(note) => format!("<p class=\"note\">{}</p>", esc(note)),
        Ok(snapshots) => history_chart(snapshots),
    };
    format!("<section id=\"history\"><h2>History</h2>{body}</section>")
}

fn history_chart(snapshots: &[Snapshot]) -> String {
    let names = snapshots
        .iter()
        .flat_map(|s| s.crates.iter().map(|(n, _)| n.clone()))
        .collect::<std::collections::BTreeSet<String>>()
        .into_iter()
        .collect::<Vec<String>>();
    let top = snapshots
        .iter()
        .flat_map(|s| s.crates.iter().map(|(_, v)| v.to_owned()))
        .fold(1.0, f64::max);
    let step = over(900.0, float(snapshots.len().saturating_sub(1).max(1)));
    let lines = names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let points = snapshots
                .iter()
                .enumerate()
                .filter_map(|(x, s)| {
                    s.crates.iter().find(|(n, _)| n == name).map(|(_, v)| {
                        let px = step.mul_add(float(x), 30.0);
                        let py = minus(230.0, over(v.mul_add(200.0, 0.0), top));
                        format!("{px:.1},{py:.1} ")
                    })
                })
                .collect::<String>();
            format!(
                "<polyline class=\"s{}\" points=\"{}\"><title>{}</title></polyline>",
                i % 8,
                points.trim_end(),
                esc(name)
            )
        })
        .collect::<String>();
    let head = names
        .iter()
        .map(|n| format!("<th>{}</th>", esc(n)))
        .collect::<String>();
    let rows = snapshots
        .iter()
        .map(|s| {
            let cells = names
                .iter()
                .map(|n| {
                    let value = s
                        .crates
                        .iter()
                        .find(|(c, _)| c == n)
                        .map_or(String::new(), |(_, v)| whole(v.to_owned()));
                    format!("<td>{value}</td>")
                })
                .collect::<String>();
            format!(
                "<tr><td><code>{}</code></td><td>{}</td>{cells}</tr>",
                s.sha.get(..8).unwrap_or_default(),
                s.date
            )
        })
        .collect::<String>();
    format!(
        "<p class=\"note\">Score per crate at each of the last {} commits that changed files under <code>include</code>.</p><svg class=\"history\" viewBox=\"0 0 960 240\" role=\"img\" aria-label=\"Score per crate over time\">{lines}</svg><table><tr><th>Commit</th><th>Date</th>{head}</tr>{rows}</table>",
        snapshots.len()
    )
}

fn method(w: &Weights) -> String {
    let rows = TIERS
        .iter()
        .map(|t| {
            format!(
                "<tr><td>{}</td><td>{:.1}</td></tr>",
                t.name(),
                w.weight(t.to_owned())
            )
        })
        .collect::<String>();
    format!(
        "<section id=\"method\"><h2>Method</h2><p class=\"note\">Every token costs the weight of the tier of the innermost construct that contains it. A violation costs <code>outside</code>; a construct whose shape a library fixes costs <code>boundary</code>.</p><table><tr><th>Tier</th><th>Weight</th></tr>{rows}</table></section>"
    )
}

/// A score rounded to a whole number.
fn whole(x: f64) -> String {
    format!("{x:.0}")
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const CSS: &str = "
:root{--bg:#fff;--fg:#1d1d1f;--muted:#6b6b70;--line:#e3e3e6;
--encouraged:#7fbf7b;--neutral:#b8b8bd;--tolerated:#f0c05a;--costly:#e8833a;--outside:#c8102e;--boundary:#6a8dd8;
--own:#5a9bd5;--primitive:#f0c05a;--untyped:#c8102e;--other:#b8b8bd}
@media (prefers-color-scheme:dark){:root{--bg:#161618;--fg:#ececf0;--muted:#9a9aa2;--line:#2c2c30}}
body{margin:0 auto;max-width:1000px;padding:24px;font:15px/1.5 system-ui,sans-serif;background:var(--bg);color:var(--fg)}
h1{margin:0}h2{margin-top:40px;border-bottom:1px solid var(--line)}.stamp,.note{color:var(--muted)}
.badge{font-size:12px;padding:2px 8px;border-radius:9px;background:var(--line);vertical-align:middle}
.tiles{display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:12px}
.tile{border:1px solid var(--line);border-radius:8px;padding:10px 12px}.label{color:var(--muted);font-size:13px}.value{font-size:24px;font-weight:600}
table{border-collapse:collapse;width:100%;margin:8px 0}td,th{text-align:left;padding:3px 8px;border-bottom:1px solid var(--line)}
td.chart{width:45%}.bar{display:flex;height:14px;border-radius:3px;overflow:hidden}.bar span{display:block}
.legend span{margin-right:12px;font-size:13px}.legend i{display:inline-block;width:10px;height:10px;margin-right:4px;border-radius:2px}
.encouraged{background:var(--encouraged)}.neutral{background:var(--neutral)}.tolerated{background:var(--tolerated)}
.costly{background:var(--costly)}.outside{background:var(--outside)}.boundary{background:var(--boundary)}
.own{background:var(--own)}.primitive{background:var(--primitive)}.untyped{background:var(--untyped)}.other{background:var(--other)}
svg{width:100%;height:auto}.treemap rect{stroke:var(--bg);stroke-width:2}.treemap text{font-size:12px;fill:#111}
.b0{fill:#cfe8cc}.b1{fill:#a6d49f}.b2{fill:#dcdcdf}.b3{fill:#f5d68a}.b4{fill:#f0a35e}.b5{fill:#e0603e}.b6{fill:#b10d27}
.history polyline{fill:none;stroke-width:2}.s0{stroke:#5a9bd5}.s1{stroke:#e8833a}.s2{stroke:#7fbf7b}.s3{stroke:#c8102e}
.s4{stroke:#9b6bd5}.s5{stroke:#3fb6b2}.s6{stroke:#b8862e}.s7{stroke:#6b6b70}
";

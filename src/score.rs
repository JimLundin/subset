//! The weighted score: every token `measure` counts costs the weight of the
//! innermost construct that contains it.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use crate::config::{TIERS, Tier, Weights};
use crate::syntax::Extent;
use crate::walk::{Fixed, Hit, Place, TypeClass, Within};
use crate::{Analysed, Fatal, display, object};

/// One construct occurrence, with the id and tier the score charges it.
struct Judged {
    id: String,
    tier: Tier,
    extent: Extent,
}

#[derive(Clone)]
pub(crate) struct Count {
    pub(crate) count: usize,
    pub(crate) tokens: usize,
    /// Lines on which the construct owns a token.
    pub(crate) lines: BTreeSet<usize>,
}

impl Count {
    fn none() -> Count {
        Count {
            count: 0,
            tokens: 0,
            lines: BTreeSet::new(),
        }
    }
}

impl FnTally {
    fn of(f: &crate::walk::FnSpan) -> FnTally {
        FnTally {
            name: f.name.clone(),
            line: f.line,
            tiers: Vec::from([0; 6]),
            lines: BTreeSet::new(),
            signature_primitives: 0,
        }
    }
}

pub(crate) struct FnTally {
    pub(crate) name: String,
    pub(crate) line: usize,
    /// Tokens per tier, in the order of `TIERS`.
    pub(crate) tiers: Vec<usize>,
    pub(crate) lines: BTreeSet<usize>,
    /// Tokens of primitive types in the fn's signature.
    pub(crate) signature_primitives: usize,
}

/// A scored file and the crate it belongs to.
pub(crate) struct ScoredFile {
    pub(crate) path: String,
    pub(crate) krate: String,
    pub(crate) tiers: Vec<usize>,
    pub(crate) lines: BTreeSet<usize>,
    /// Occurrences and tokens per id and the tier it was charged at.
    pub(crate) constructs: BTreeMap<(String, usize), Count>,
    pub(crate) fns: Vec<FnTally>,
    /// Tokens of field and signature types of our own items, per class:
    /// own, primitive, untyped, other.
    pub(crate) typing: Vec<usize>,
    pub(crate) violations: usize,
}

/// The id of tokens that no construct contains.
const NO_CONSTRUCT: &str = "(no construct)";
/// The id of a field of a wire type in an external protocol module.
const WIRE_FIELD: &str = "field.wire";

/// Ids that exist only in the score: an allowed id refined by where it
/// stands, so it can take its own tier.
const SCORE_IDS: &str = "| Id | Meaning |
|---|---|
| `expr.method_call.adapter` | method call whose name is in `adapters` |
| `stmt.let.discard` | `let _ = value;` |
| `type.field` | type in a field of our own struct or enum variant |
| `type.primitive` | type in `primitives`, as a field type or in a signature of our own |
| `type.untyped` | type in `untyped`, anywhere |
| `field.wire` | field of a wire type in an external protocol module |
";

pub(crate) fn ids_table() -> String {
    SCORE_IDS.to_string()
}

pub(crate) const CLASSES: &[TypeClass] = &[
    TypeClass::Own,
    TypeClass::Primitive,
    TypeClass::Untyped,
    TypeClass::Other,
];

fn tier_index(tier: Tier) -> usize {
    TIERS.iter().position(|t| t == &tier).unwrap_or_default()
}

/// A count as a float, for weighing.
pub(crate) fn float(n: usize) -> f64 {
    u32::try_from(n).map_or(f64::from(u32::MAX), f64::from)
}

/// The score of tokens per tier.
pub(crate) fn weigh(tiers: &[usize], w: &Weights) -> f64 {
    tiers
        .iter()
        .zip(TIERS)
        .map(|(n, t)| float(n.to_owned()).mul_add(w.weight(t.to_owned()), 0.0))
        .sum()
}

/// The id and tier each occurrence is charged at: `outside` for a violation,
/// `boundary` where an item we don't define fixes its shape, and otherwise
/// the tier `[weights]` gives its id.
fn judge(a: &Analysed, w: &Weights) -> Vec<Judged> {
    let wires = a
        .hits
        .iter()
        .filter_map(|h| match h {
            Hit::Wire(e) => Some(e.to_owned()),
            _ => None,
        })
        .collect::<Vec<Extent>>();
    let marks = crate::marks(&a.hits);
    let refinements = a
        .hits
        .iter()
        .filter_map(|h| match h {
            Hit::Refine(r) => Some(r),
            _ => None,
        })
        .collect::<Vec<&crate::walk::Refinement>>();
    let judged = a.occurrences.iter().zip(&a.violating).map(|(o, violating)| {
        let id = refinements
            .iter()
            .find(|r| r.extent == o.extent && refines(r.from.as_deref(), &o.id).is_some())
            .map_or(o.id.clone(), |r| r.to.clone());
        let tier = w.tier(&id);
        let fixed = marks
            .iter()
            .filter(|m| m.extent == o.extent)
            .map(|m| m.fixed)
            .min_by_key(|f| match f {
                Fixed::Always => 0,
                Fixed::IfRestricted => 1,
            });
        let wire = wires.iter().find_map(|f| f.holding(o.extent));
        let tier = match (violating, wire, fixed, tier.restricted()) {
            (Within::In, _, _, _) => Tier::Outside,
            (Within::Out, Some(_), _, _) => Tier::Boundary,
            (Within::Out, None, Some(Fixed::Always), _) => Tier::Boundary,
            (Within::Out, None, Some(Fixed::IfRestricted), Some(_)) => Tier::Boundary,
            _ => tier,
        };
        Judged {
            id,
            tier,
            extent: o.extent,
        }
    });
    let fields = wires.iter().map(|f| Judged {
        id: WIRE_FIELD.to_string(),
        tier: Tier::Boundary,
        extent: f.to_owned(),
    });
    judged.chain(fields).collect()
}

/// Whether a refinement from `from` applies to an occurrence of `id`.
fn refines(from: Option<&str>, id: &str) -> Option<()> {
    match from {
        Some(from) => match from {
            f if f == id => Some(()),
            _ => None,
        },
        None => Some(())
            .filter(|_| id.starts_with("type.") && !id.starts_with("type.lifetime")),
    }
}

/// For each token start, the index of the smallest extent containing it.
/// Among equal extents the later one wins, since the walk reports a node
/// before its children.
fn innermost(starts: &[usize], extents: &[Extent]) -> Vec<Option<usize>> {
    let sorted = extents
        .iter()
        .enumerate()
        .map(|(i, e)| ((e.start, i), (i, e.to_owned())))
        .collect::<BTreeMap<(usize, usize), (usize, Extent)>>()
        .into_values()
        .collect::<Vec<(usize, Extent)>>();
    starts
        .iter()
        .fold(
            (0, Vec::<(usize, Extent)>::new(), Vec::<Option<usize>>::new()),
            |(next, open, mut out), p| {
                let count = sorted
                    .iter()
                    .skip(next)
                    .take_while(|(_, e)| &e.start <= p)
                    .count();
                let open = open
                    .into_iter()
                    .chain(sorted.iter().skip(next).take(count).copied())
                    .filter(|(_, e)| &e.end > p)
                    .collect::<Vec<(usize, Extent)>>();
                out.push(
                    open.iter()
                        .min_by_key(|(i, e)| (e.len(), Reverse(i.to_owned())))
                        .map(|(i, _)| i.to_owned()),
                );
                (next.saturating_add(count), open, out)
            },
        )
        .2
}

/// Scores one analysed file.
pub(crate) fn file(a: &Analysed, w: &Weights, krate: &str) -> Result<ScoredFile, Fatal> {
    let tokens = crate::measure::tokens(&a.src)
        .map_err(|e| Fatal(format!("{}: cannot lex: {e}", display(&a.path))))?;
    let judged = judge(a, w);
    let folds = a
        .hits
        .iter()
        .filter_map(|h| match h {
            Hit::Fold(e) => Some(e.to_owned()),
            _ => None,
        })
        .collect::<Vec<Extent>>();
    let painted = judged
        .iter()
        .map(|j| {
            let folded = folds
                .iter()
                .filter(|f| f != &&j.extent)
                .find_map(|f| f.holding(j.extent))
                .filter(|_| j.tier != Tier::Outside);
            folded.map_or(j.extent, |_| Extent { start: 0, end: 0 })
        })
        .collect::<Vec<Extent>>();
    let fns = a
        .hits
        .iter()
        .filter_map(|h| match h {
            Hit::Function(f) => Some(f),
            _ => None,
        })
        .collect::<Vec<&crate::walk::FnSpan>>();
    let typed = a
        .hits
        .iter()
        .filter_map(|h| match h {
            Hit::Typed(t) => Some(t),
            _ => None,
        })
        .collect::<Vec<&crate::walk::Typed>>();
    let starts = tokens.iter().map(|t| t.start).collect::<Vec<usize>>();
    let owners = innermost(&starts, &painted);
    let fn_owners = innermost(&starts, &fns.iter().map(|f| f.extent).collect::<Vec<Extent>>());
    let typed_owners = innermost(&starts, &typed.iter().map(|t| t.extent).collect::<Vec<Extent>>());
    let counted = judged
        .iter()
        .fold(BTreeMap::<(String, usize), Count>::new(), |mut map, j| {
            let key = (j.id.clone(), tier_index(j.tier));
            let old = map.remove(&key).unwrap_or(Count::none());
            map.insert(key, Count {
                count: old.count.saturating_add(1),
                ..old
            });
            map
        });
    let start = Tallies {
        constructs: counted,
        tiers: Vec::from([0; 6]),
        lines: BTreeSet::new(),
        fns: fns.iter().map(|f| FnTally::of(f)).collect(),
        typing: Vec::from([0; 4]),
    };
    let tally = tokens
        .iter()
        .zip(owners.iter().zip(fn_owners.iter().zip(&typed_owners)))
        .fold(start, |t, (token, (owner, (in_fn, in_type)))| {
            let (id, tier) = owner
                .and_then(|i| judged.get(i))
                .map_or((NO_CONSTRUCT.to_string(), Tier::Neutral), |j| (j.id.clone(), j.tier));
            let class = in_type.and_then(|i| typed.get(i));
            t.add(token, id, tier_index(tier), in_fn.to_owned(), class)
        });
    Ok(ScoredFile {
        path: display(&a.path),
        krate: krate.to_string(),
        tiers: tally.tiers,
        lines: tally.lines,
        constructs: tally.constructs,
        fns: tally.fns,
        typing: tally.typing,
        violations: a.violations.len(),
    })
}

/// A file's counts as its tokens are attributed.
struct Tallies {
    constructs: BTreeMap<(String, usize), Count>,
    tiers: Vec<usize>,
    lines: BTreeSet<usize>,
    fns: Vec<FnTally>,
    typing: Vec<usize>,
}

/// `counts` with one more at `index`.
fn bump(counts: Vec<usize>, index: usize) -> Vec<usize> {
    counts
        .into_iter()
        .enumerate()
        .map(|(i, n)| match i {
            i if i == index => n.saturating_add(1),
            _ => n,
        })
        .collect()
}

impl Tallies {
    fn add(
        self,
        token: &crate::measure::Token,
        id: String,
        tier: usize,
        in_fn: Option<usize>,
        class: Option<&&crate::walk::Typed>,
    ) -> Tallies {
        let mut constructs = self.constructs;
        let old = constructs.remove(&(id.clone(), tier)).unwrap_or(Count::none());
        constructs.insert((id, tier), Count {
            tokens: old.tokens.saturating_add(1),
            lines: old.lines.into_iter().chain(token.lines.iter().copied()).collect(),
            ..old
        });
        let fns = self
            .fns
            .into_iter()
            .enumerate()
            .map(|(i, f)| match in_fn {
                Some(owner) if owner == i => {
                    let primitive = class
                        .filter(|t| matches!((t.class, t.place), (TypeClass::Primitive, Place::Signature)))
                        .map_or(0, |_| 1);
                    FnTally {
                        tiers: bump(f.tiers, tier),
                        lines: f.lines.into_iter().chain(token.lines.iter().copied()).collect(),
                        signature_primitives: f.signature_primitives.saturating_add(primitive),
                        ..f
                    }
                }
                _ => f,
            })
            .collect();
        let typing = match class {
            Some(t) => bump(
                self.typing,
                CLASSES.iter().position(|c| c == &t.class).unwrap_or_default(),
            ),
            None => self.typing,
        };
        Tallies {
            constructs,
            tiers: bump(self.tiers, tier),
            lines: self.lines.into_iter().chain(token.lines.iter().copied()).collect(),
            fns,
            typing,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Level {
    Crate,
    File,
    Fn,
    Construct,
}

impl Level {
    pub(crate) fn parse(name: &str) -> Option<Level> {
        match name {
            "crate" => Some(Level::Crate),
            "file" => Some(Level::File),
            "fn" => Some(Level::Fn),
            "construct" => Some(Level::Construct),
            _ => None,
        }
    }

    fn column(&self) -> &str {
        match self {
            Level::Crate => "crate",
            Level::File => "file",
            Level::Fn => "fn",
            Level::Construct => "construct",
        }
    }
}

#[derive(Clone)]
pub(crate) struct Row {
    pub(crate) name: String,
    /// Occurrences and the tier charged, for a construct row.
    pub(crate) construct: Option<(usize, Tier)>,
    pub(crate) tiers: Vec<usize>,
    /// Lines holding the row's tokens. Construct rows overlap.
    pub(crate) lines: usize,
}

impl Row {
    pub(crate) fn tokens(&self) -> usize {
        self.tiers.iter().sum()
    }

    pub(crate) fn score(&self, w: &Weights) -> f64 {
        weigh(&self.tiers, w)
    }

    pub(crate) fn per_token(&self, w: &Weights) -> f64 {
        ratio(self.score(w), self.tokens())
    }

    pub(crate) fn boundary(&self, w: &Weights) -> f64 {
        self.tiers
            .get(tier_index(Tier::Boundary))
            .map_or(0.0, |n| float(n.to_owned()).mul_add(w.weight(Tier::Boundary), 0.0))
    }

    /// The boundary score as a share of the score, in percent.
    pub(crate) fn boundary_share(&self, w: &Weights) -> f64 {
        let score = self.score(w);
        match score.partial_cmp(&0.0) {
            Some(std::cmp::Ordering::Greater) => self.boundary(w).mul_add(100.0, 0.0).mul_add(score.recip(), 0.0),
            _ => 0.0,
        }
    }

    /// The score in lines: the score over the row's tokens per line.
    pub(crate) fn approx_lines(&self, w: &Weights) -> f64 {
        ratio(self.score(w).mul_add(float(self.lines), 0.0), self.tokens())
    }
}

/// `x` over `n`, or 0 when `n` is.
pub(crate) fn ratio(x: f64, n: usize) -> f64 {
    match n {
        0 => 0.0,
        n => x.mul_add(float(n).recip(), 0.0),
    }
}

fn sum(rows: Vec<Vec<usize>>) -> Vec<usize> {
    rows.into_iter().fold(Vec::from([0; 6]), |total, row| {
        total
            .into_iter()
            .zip(row.into_iter().chain(std::iter::repeat(0)))
            .map(|(a, b)| a.saturating_add(b))
            .collect()
    })
}

pub(crate) fn total(files: &[ScoredFile]) -> Row {
    Row {
        name: "total".to_string(),
        construct: None,
        tiers: sum(files.iter().map(|f| f.tiers.clone()).collect()),
        lines: files.iter().map(|f| f.lines.len()).sum(),
    }
}

/// Rows at one level: crates and files by name, functions and constructs by
/// score, highest first.
pub(crate) fn rows(files: &[ScoredFile], level: Level, w: &Weights) -> Vec<Row> {
    let rows = match level {
        Level::Crate => files
            .iter()
            .fold(BTreeMap::<String, Vec<&ScoredFile>>::new(), |mut crates, f| {
                crates.entry(f.krate.clone()).or_default().push(f);
                crates
            })
            .into_iter()
            .map(|(name, files)| Row {
                name,
                construct: None,
                tiers: sum(files.iter().map(|f| f.tiers.clone()).collect()),
                lines: files.iter().map(|f| f.lines.len()).sum(),
            })
            .collect::<Vec<Row>>(),
        Level::File => files
            .iter()
            .map(|f| Row {
                name: f.path.clone(),
                construct: None,
                tiers: f.tiers.clone(),
                lines: f.lines.len(),
            })
            .collect(),
        Level::Fn => files
            .iter()
            .flat_map(|f| {
                f.fns.iter().map(|n| Row {
                    name: format!("{}:{} {}", f.path, n.line, n.name),
                    construct: None,
                    tiers: n.tiers.clone(),
                    lines: n.lines.len(),
                })
            })
            .collect(),
        Level::Construct => files
            .iter()
            .flat_map(|f| f.constructs.iter())
            .fold(BTreeMap::<(String, usize), (usize, usize, usize)>::new(), |mut all, (key, c)| {
                let (count, tokens, lines) = all.remove(key).unwrap_or((0, 0, 0));
                all.insert(key.clone(), (
                    count.saturating_add(c.count),
                    tokens.saturating_add(c.tokens),
                    lines.saturating_add(c.lines.len()),
                ));
                all
            })
            .into_iter()
            .map(|((id, tier), (count, tokens, lines))| Row {
                name: id,
                construct: TIERS.get(tier).map(|t| (count, t.to_owned())),
                tiers: bump(Vec::from([0; 6]), tier)
                    .into_iter()
                    .map(|n| n.saturating_mul(tokens))
                    .collect(),
                lines,
            })
            .collect(),
    };
    match level {
        Level::Fn | Level::Construct => rows
            .into_iter()
            .enumerate()
            .map(|(i, r)| ((Reverse(r.score(w).to_bits()), r.name.clone(), i), r))
            .collect::<BTreeMap<(Reverse<u64>, String, usize), Row>>()
            .into_values()
            .collect(),
        _ => rows,
    }
}

pub(crate) fn output(files: &[ScoredFile], level: Level, json: Within, w: &Weights) -> String {
    let rows = rows(files, level, w);
    let total = total(files);
    match json {
        Within::In => {
            let value = object(Vec::from([
                ("by", level.column().into()),
                ("rows", rows.iter().map(|r| json_row(r, w)).collect::<Vec<serde_json::Value>>().into()),
                ("total", json_row(&total, w)),
            ]));
            format!("{}\n", serde_json::to_string_pretty(&value).unwrap_or_default())
        }
        Within::Out => text(&rows, &total, level, w),
    }
}

fn json_row(row: &Row, w: &Weights) -> serde_json::Value {
                let base = Vec::from([
                    ("boundary", row.boundary(w).into()),
                    ("boundary_share", row.boundary_share(w).into()),
                    ("lines", row.lines.into()),
                    ("name", row.name.clone().into()),
                    ("per_token", row.per_token(w).into()),
                    ("score", row.score(w).into()),
                    ("tokens", row.tokens().into()),
                ]);
                let extra = row
                    .construct
                    .iter()
                    .flat_map(|(count, tier)| {
                        Vec::from([("count", count.to_owned().into()), ("tier", tier.name().into())])
                    });
                object(base.into_iter().chain(extra).collect())
}

fn text(rows: &[Row], total: &Row, level: Level, w: &Weights) -> String {
    let construct = match level {
        Level::Construct => format!(" {:>6} {:<10}", "count", "tier"),
        _ => String::new(),
    };
    let head = format!(
        "{:>10} {:>8} {:>7} {:>9} {:>9} {:>6}{construct} {}\n",
        "score",
        "tokens",
        "lines",
        "per token",
        "boundary",
        "share",
        level.column()
    );
    let lines = rows.iter().chain(std::iter::once(total)).map(|row| {
        let extra = match (row.construct, level) {
            (Some((count, tier)), _) => format!(" {count:>6} {:<10}", tier.name()),
            (None, Level::Construct) => format!(" {:>6} {:<10}", "", ""),
            (None, _) => String::new(),
        };
        format!(
            "{:>10.1} {:>8} {:>7} {:>9.3} {:>9.1} {:>5.1}%{extra} {}\n",
            row.score(w),
            row.tokens(),
            row.lines,
            row.per_token(w),
            row.boundary(w),
            row.boundary_share(w),
            row.name
        )
    });
    std::iter::once(head).chain(lines).collect()
}

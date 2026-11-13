//! Code volume for proposals: token leaves and code lines, both blind to
//! comments and doc comments.

use std::collections::BTreeSet;

use proc_macro2::{Delimiter, TokenStream, TokenTree};

pub(crate) struct Volume {
    pub(crate) tokens: usize,
    pub(crate) lines: usize,
}

/// One token `measure` counts.
pub(crate) struct Token {
    /// The byte offset of the token, or of a delimited group's opening
    /// delimiter.
    pub(crate) start: usize,
    /// The lines the token counts on: those a leaf spans, or a group's
    /// opening and closing lines.
    pub(crate) lines: Vec<usize>,
}

/// Idents, puncts and literals count one each, and so does each delimited
/// group. A line counts when a counted token starts, ends or spans it.
pub(crate) fn measure(src: &str) -> Result<Volume, proc_macro2::LexError> {
    let tokens = tokens(src)?;
    let lines = tokens.iter().flat_map(|t| t.lines.iter().copied()).collect::<BTreeSet<usize>>();
    Ok(Volume {
        tokens: tokens.len(),
        lines: lines.len(),
    })
}

/// Each token `measure` counts, in source order.
pub(crate) fn tokens(src: &str) -> Result<Vec<Token>, proc_macro2::LexError> {
    let stream = src.parse::<TokenStream>()?;
    Ok(walk(stream, src))
}

/// How a token tree stands to a doc comment, which lexes as `#`, an
/// optional `!`, and a bracket group.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Lexeme {
    /// The `#` of a doc comment.
    Doc,
    Bang,
    Other,
}

fn lexeme(tree: &TokenTree, src: &str) -> Lexeme {
    match tree {
        TokenTree::Punct(p) => match p.as_char() {
            '#' => {
                src.split_at_checked(p.span().byte_range().start)
                    .filter(|(_, rest)| rest.starts_with("//") || rest.starts_with("/*"))
                    .map_or(Lexeme::Other, |_| Lexeme::Doc)
            }
            '!' => Lexeme::Bang,
            _ => Lexeme::Other,
        },
        _ => Lexeme::Other,
    }
}

fn walk(stream: TokenStream, src: &str) -> Vec<Token> {
    let trees = stream.into_iter().collect::<Vec<TokenTree>>();
    let kinds = trees.iter().map(|t| lexeme(t, src)).collect::<Vec<Lexeme>>();
    let before = std::iter::once(Lexeme::Other).chain(kinds.iter().copied());
    let two_before = std::iter::repeat_n(Lexeme::Other, 2).chain(kinds.iter().copied());
    trees
        .into_iter()
        .zip(kinds.iter().copied())
        .zip(before.zip(two_before))
        .filter(|((_, kind), (one, two))| {
            !matches!(
                (kind, one, two),
                (Lexeme::Doc, _, _) | (_, Lexeme::Doc, _) | (_, Lexeme::Bang, Lexeme::Doc)
            )
        })
        .flat_map(|((tree, _), _)| token(tree, src))
        .collect()
}

fn token(tree: TokenTree, src: &str) -> Vec<Token> {
    match tree {
        TokenTree::Group(group) => {
            let open = group.span_open().start().line;
            let close = group.span_close().end().line;
            let own = match group.delimiter() {
                Delimiter::None => None,
                _ => Some(Token {
                    start: group.span_open().byte_range().start,
                    lines: match close {
                        c if c == open => Vec::from([open]),
                        c => Vec::from([open, c]),
                    },
                }),
            };
            own.into_iter().chain(walk(group.stream(), src)).collect()
        }
        leaf => {
            let span = leaf.span();
            let lines = std::ops::RangeInclusive::new(span.start().line, span.end().line).collect();
            Vec::from([Token {
                start: span.byte_range().start,
                lines,
            }])
        }
    }
}

//! Code volume for proposals: token leaves and code lines, both blind to
//! comments and doc comments.

use std::collections::BTreeSet;

use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};

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
    let lines = tokens
        .iter()
        .flat_map(|t| t.lines.iter().copied())
        .collect::<BTreeSet<usize>>();
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

/// A token tree with the kinds of the two trees before it.
struct Placed {
    tree: TokenTree,
    kind: Lexeme,
    before: Lexeme,
    earlier: Lexeme,
}

fn lexeme(tree: &TokenTree, src: &str) -> Lexeme {
    match tree {
        TokenTree::Punct(p) => match p.as_char() {
            '#' => hash_kind(p.span().byte_range().start, src),
            '!' => Lexeme::Bang,
            _ => Lexeme::Other,
        },
        TokenTree::Group(_) | TokenTree::Ident(_) | TokenTree::Literal(_) => Lexeme::Other,
    }
}

/// A `#` that a comment follows in the source is a doc comment's.
fn hash_kind(start: usize, src: &str) -> Lexeme {
    let rest = src.get(start..).unwrap_or("");
    if rest.starts_with("//") || rest.starts_with("/*") {
        Lexeme::Doc
    } else {
        Lexeme::Other
    }
}

fn is_doc_part(placed: &Placed) -> bool {
    matches!(
        (placed.kind, placed.before, placed.earlier),
        (Lexeme::Doc, _, _) | (_, Lexeme::Doc, _) | (_, Lexeme::Bang, Lexeme::Doc)
    )
}

fn walk(stream: TokenStream, src: &str) -> Vec<Token> {
    let trees = stream.into_iter().collect::<Vec<TokenTree>>();
    let kinds = trees
        .iter()
        .map(|t| lexeme(t, src))
        .collect::<Vec<Lexeme>>();
    let before = std::iter::once(Lexeme::Other).chain(kinds.iter().copied());
    let earlier = std::iter::repeat_n(Lexeme::Other, 2).chain(kinds.iter().copied());
    trees
        .into_iter()
        .zip(kinds.iter().copied())
        .zip(before.zip(earlier))
        .map(|((tree, kind), (before, earlier))| Placed {
            tree,
            kind,
            before,
            earlier,
        })
        .filter(|placed| !is_doc_part(placed))
        .flat_map(|placed| token(placed.tree, src))
        .collect()
}

fn token(tree: TokenTree, src: &str) -> Vec<Token> {
    match tree {
        TokenTree::Group(group) => group_tokens(&group, src),
        leaf @ (TokenTree::Ident(_) | TokenTree::Punct(_) | TokenTree::Literal(_)) => {
            Vec::from([leaf_token(&leaf)])
        }
    }
}

fn leaf_token(leaf: &TokenTree) -> Token {
    let span = leaf.span();
    Token {
        start: span.byte_range().start,
        lines: (span.start().line..=span.end().line).collect(),
    }
}

fn group_tokens(group: &Group, src: &str) -> Vec<Token> {
    let inner = walk(group.stream(), src);
    match group.delimiter() {
        Delimiter::None => inner,
        Delimiter::Parenthesis | Delimiter::Brace | Delimiter::Bracket => {
            std::iter::once(delimiter_token(group))
                .chain(inner)
                .collect()
        }
    }
}

fn delimiter_token(group: &Group) -> Token {
    let open = group.span_open().start().line;
    let close = group.span_close().end().line;
    let lines = if open == close {
        Vec::from([open])
    } else {
        Vec::from([open, close])
    };
    Token {
        start: group.span_open().byte_range().start,
        lines,
    }
}

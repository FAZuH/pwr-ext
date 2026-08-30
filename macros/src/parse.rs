#![allow(clippy::needless_range_loop, clippy::indexing_slicing)]

use syn::Expr;
use syn::Ident;
use syn::LitStr;
use syn::Token;
use syn::braced;
use syn::parse::Parse;
use syn::parse::ParseStream;

pub enum ViewItem {
    Attr { key: Ident, value: Expr },
    Element { name: Ident, body: ViewBody },
    Bare(LitStr),
}

pub struct ViewInput {
    pub items: Vec<ViewItem>,
}

pub struct ViewBody {
    pub items: Vec<ViewItem>,
}

impl Parse for ViewInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut items = Vec::new();
        while !input.is_empty() {
            // Optional leading comma (allow `, ,` recovery? no, just skip).
            // Parse one item.
            if input.peek(LitStr) {
                let lit: LitStr = input.parse()?;
                items.push(ViewItem::Bare(lit));
                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }
                continue;
            }
            if input.peek(Ident) {
                let fork = input.fork();
                let _: Ident = fork.parse()?;
                if fork.peek(Token![:]) {
                    let key: Ident = input.parse()?;
                    input.parse::<Token![:]>()?;
                    // Expr may be any Rust expression; syn will parse until
                    // comma or next ident-colon/brace at same depth. Expr
                    // parsing is greedy — it will consume `a, b` as `a` then
                    // comma, which is correct for our separator.
                    let value: Expr = input.parse()?;
                    if input.peek(Token![,]) {
                        input.parse::<Token![,]>()?;
                    }
                    items.push(ViewItem::Attr { key, value });
                    continue;
                } else if fork.peek(syn::token::Brace) {
                    let name: Ident = input.parse()?;
                    let content;
                    braced!(content in input);
                    // Parse inner body without outer braces
                    let body: ViewBody = content.parse()?;
                    if input.peek(Token![,]) {
                        input.parse::<Token![,]>()?;
                    }
                    items.push(ViewItem::Element { name, body });
                    continue;
                } else {
                    return Err(syn::Error::new(
                        input.span(),
                        "expected `:` after attribute name or `{` after element name",
                    ));
                }
            }
            // If none matched, produce a helpful error pointing at the next token.
            let lookahead = input.lookahead1();
            return Err(lookahead.error());
        }
        Ok(Self { items })
    }
}

impl Parse for ViewBody {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut items = Vec::new();
        while !input.is_empty() {
            if input.peek(LitStr) {
                let lit: LitStr = input.parse()?;
                items.push(ViewItem::Bare(lit));
                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }
                continue;
            }
            if input.peek(Ident) {
                let fork = input.fork();
                let _: Ident = fork.parse()?;
                if fork.peek(Token![:]) {
                    let key: Ident = input.parse()?;
                    input.parse::<Token![:]>()?;
                    let value: Expr = input.parse()?;
                    if input.peek(Token![,]) {
                        input.parse::<Token![,]>()?;
                    }
                    items.push(ViewItem::Attr { key, value });
                    continue;
                } else if fork.peek(syn::token::Brace) {
                    let name: Ident = input.parse()?;
                    let content;
                    braced!(content in input);
                    let body: ViewBody = content.parse()?;
                    if input.peek(Token![,]) {
                        input.parse::<Token![,]>()?;
                    }
                    items.push(ViewItem::Element { name, body });
                    continue;
                } else {
                    return Err(syn::Error::new(
                        input.span(),
                        "expected `:` after attribute name or `{` after element name",
                    ));
                }
            }
            let lookahead = input.lookahead1();
            return Err(lookahead.error());
        }
        Ok(Self { items })
    }
}

/// Simple edit-distance helper for "did you mean?" suggestions.
pub fn did_you_mean<'a>(input: &str, candidates: &[&'a str]) -> Option<&'a str> {
    let mut best: Option<(&str, usize)> = None;
    for &cand in candidates {
        let d = edit_distance(input, cand);
        // Suggest only if close: distance 1-2, or distance 1 for short strings.
        let threshold = if cand.len() <= 4 { 1 } else { 2 };
        if d <= threshold {
            match best {
                Some((_, bd)) if d >= bd => {}
                _ => best = Some((cand, d)),
            }
        }
    }
    best.map(|(c, _)| c)
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let n = a.len();
    let m = b.len();
    let mut dp = vec![vec![0; m + 1]; n + 1];
    for i in 0..=n {
        dp[i][0] = i;
    }
    for j in 0..=m {
        dp[0][j] = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[n][m]
}

#[cfg(test)]
mod tests {
    use syn::parse::Parser;

    use super::*;

    fn parse(input: &str) -> syn::Result<ViewInput> {
        let parser = |ps: ParseStream| ps.parse::<ViewInput>();
        parser.parse_str(input)
    }

    fn parse_body(input: &str) -> syn::Result<ViewBody> {
        // Wrap in braces to test body parsing via braced! — easier to test via ViewInput inner.
        let wrapped = format!("dummy {{ {input} }}");
        let vi = parse(&wrapped)?;
        match vi.items.into_iter().next() {
            Some(ViewItem::Element { body, .. }) => Ok(body),
            _ => panic!("expected dummy element"),
        }
    }

    #[test]
    fn empty_input_parses() {
        let vi = parse("").unwrap();
        assert!(vi.items.is_empty());
    }

    #[test]
    fn single_attr() {
        let vi = parse(r#"content: "hi""#).unwrap();
        assert_eq!(vi.items.len(), 1);
        match &vi.items[0] {
            ViewItem::Attr { key, .. } => assert_eq!(key.to_string(), "content"),
            _ => panic!("expected attr"),
        }
    }

    #[test]
    fn attr_trailing_comma() {
        let vi = parse(r#"content: "hi","#).unwrap();
        assert_eq!(vi.items.len(), 1);
    }

    #[test]
    fn multiple_attrs_with_commas() {
        let vi = parse(r#"content: "hi", tts: true"#).unwrap();
        assert_eq!(vi.items.len(), 2);
    }

    #[test]
    fn attr_without_comma_separator() {
        // Dioxus-like: commas optional between items
        let vi = parse(r#"content: "hi" tts: true"#).unwrap();
        // Our parser treats `tts` as next ident-colon, not part of Expr, so this should parse as 2 items
        // But Expr "hi" followed by ident without comma: syn Expr parsing of `"hi"` stops before `tts`,
        // so we correctly get two attrs.
        assert_eq!(vi.items.len(), 2);
    }

    #[test]
    fn element_with_empty_body() {
        let vi = parse(r#"embed {}"#).unwrap();
        assert_eq!(vi.items.len(), 1);
        match &vi.items[0] {
            ViewItem::Element { name, body } => {
                assert_eq!(name.to_string(), "embed");
                assert!(body.items.is_empty());
            }
            _ => panic!("expected element"),
        }
    }

    #[test]
    fn element_with_attrs() {
        let vi = parse(r#"embed { title: "hi", description: "desc" }"#).unwrap();
        match &vi.items[0] {
            ViewItem::Element { body, .. } => {
                assert_eq!(body.items.len(), 2);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn nested_element() {
        let vi = parse(r#"container { text_display { "hello" } }"#).unwrap();
        match &vi.items[0] {
            ViewItem::Element { body, .. } => {
                assert_eq!(body.items.len(), 1);
                match &body.items[0] {
                    ViewItem::Element { name, body } => {
                        assert_eq!(name.to_string(), "text_display");
                        assert_eq!(body.items.len(), 1);
                        assert!(matches!(&body.items[0], ViewItem::Bare(_)));
                    }
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }

    #[test]
    fn bare_string_at_top_level() {
        let vi = parse(r#""hello""#).unwrap();
        assert!(matches!(&vi.items[0], ViewItem::Bare(_)));
    }

    #[test]
    fn mixed_attrs_and_elements_interleaved() {
        let vi =
            parse(r#"content: "hi" embed { title: "t" } action_row { button { label: "x" } }"#)
                .unwrap();
        assert_eq!(vi.items.len(), 3);
        assert!(matches!(vi.items[0], ViewItem::Attr { .. }));
        assert!(matches!(vi.items[1], ViewItem::Element { .. }));
        assert!(matches!(vi.items[2], ViewItem::Element { .. }));
    }

    #[test]
    fn element_trailing_comma() {
        let vi = parse(r#"embed { title: "hi", }, "#).unwrap();
        assert_eq!(vi.items.len(), 1);
    }

    #[test]
    fn bare_string_without_comma_between_elements() {
        // Ensure bare sugar only valid inside text_display, but parser accepts it anywhere
        // (validation happens later)
        let vi = parse(r#"text_display { "hello" } text_display { "world" }"#).unwrap();
        assert_eq!(vi.items.len(), 2);
    }

    #[test]
    fn did_you_mean_suggests_close_match() {
        assert_eq!(
            did_you_mean("contnet", &["content", "tts"]),
            Some("content")
        );
        assert_eq!(did_you_mean("colur", &["colour", "color"]), Some("colour"));
        assert_eq!(did_you_mean("zzzzz", &["content"]), None);
    }

    #[test]
    fn did_you_mean_short_string_threshold() {
        // For short candidates (<=4), threshold is 1
        assert_eq!(did_you_mean("ts", &["tts"]), Some("tts"));
        assert_eq!(did_you_mean("tx", &["tts"]), None); // distance 2 > threshold 1
    }

    #[test]
    fn body_parsing_with_nested() {
        let body =
            parse_body(r#"title: "hi", field { name: "n", value: "v", inline: true }"#).unwrap();
        assert_eq!(body.items.len(), 2);
    }
}

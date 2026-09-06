#![allow(clippy::collapsible_if)]

//! Proc macros for authoring serenity message views.
//!
//! [`view`] compiles a dioxus-like DSL into serenity builder construction
//! code. Element and attribute names mirror the JSON shapes the `pwr-ext`
//! wrapper types parse; generated code references only `::pwr_ext`
//! re-exports, so call sites never need a direct serenity dependency.
//!
//! This crate must not depend on `pwr-ext`: `pwr-ext` depends on it, and
//! generated code is plain text — the `::pwr_ext` paths resolve at the call
//! site.

use proc_macro::TokenStream;
use quote::quote;
use syn::Expr;

mod parse;
use parse::ViewBody;
use parse::ViewInput;
use parse::ViewItem;
use parse::any_splice;
use parse::did_you_mean;

/// Author a serenity message view.
///
/// Two families are statically separated:
///
/// * **Legacy** — implicit root: `content`, `tts`, `embed`, `action_row`, `poll`,
///   `allowed_mentions`. Evaluates to `CreateMessage` without `IS_COMPONENTS_V2`.
/// * **Components v2** — explicit `components_v2` root element containing v2
///   components. The macro auto-sets `IS_COMPONENTS_V2` (OR-ing any user `flags`).
///
/// # Return type
///
/// Literal-only views evaluate to the bare builder (`CreateMessage`). A view
/// containing at least one `{ expr }` splice evaluates to
/// `Result<CreateMessage, ChildRuleError>`: runtime-spliced child lists are
/// checked against the same child laws as literal children, and a violation
/// is returned instead of panicking. The rule is one-directional — a splice
/// always makes the view return `Result`, even where no child law applies.
#[proc_macro]
pub fn view(input: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(input as ViewInput);
    expand(&parsed)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Emit a single bare serenity builder from one element body.
///
/// `component!` takes exactly one element — any element `view!` accepts as a
/// child — and emits the bare builder for it, ready to be composed into a
/// runtime-assembled parent. Unlike [`view`], it does not produce a
/// `CreateMessage`; the consumer wraps the result in the parent's enum
/// variant explicitly (decision D2 — no contextual `Into` conversions):
///
/// ```text
/// // component! emits the bare builder; the consumer wraps it explicitly:
/// let row = component! { action_row { button { custom_id: "nav:1", label: "Home" } } };
/// let msg = CreateMessage::new().components(vec![CreateComponent::ActionRow(row)]);
/// ```
///
/// The same element maps to different enum wraps per parent (`action_row` is
/// `CreateComponent::ActionRow` at the message root but
/// `CreateContainerComponent::ActionRow` inside a `container`), so
/// `component!` always emits the bare builder and leaves the wrapping to the
/// call site. Splices inside the element body behave exactly as in `view!`.
///
/// # Return type
///
/// Literal-only bodies evaluate to the bare builder. A body containing at
/// least one `{ expr }` splice evaluates to
/// `Result<Builder, ChildRuleError>` — spliced child lists are checked
/// against the element's child laws, and a violation is returned instead of
/// panicking.
#[proc_macro]
pub fn component(input: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(input as ViewInput);
    expand_component(&parsed)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand_component(input: &ViewInput) -> syn::Result<proc_macro2::TokenStream> {
    let mut element: Option<(&syn::Ident, &ViewBody)> = None;
    for item in &input.items {
        match item {
            ViewItem::Element { name, body } => {
                if element.is_some() {
                    return Err(syn::Error::new_spanned(
                        name,
                        "`component!` takes exactly one element",
                    ));
                }
                element = Some((name, body));
            }
            ViewItem::Attr { key, .. } => {
                return Err(syn::Error::new_spanned(
                    key,
                    "root attributes are not allowed in `component!` — attributes go inside the element body",
                ));
            }
            ViewItem::Bare(lit) => {
                return Err(syn::Error::new_spanned(
                    lit,
                    "bare strings are not allowed at the root of `component!` — use `text_display { \"...\" }`",
                ));
            }
            ViewItem::Splice { span, .. } => {
                return Err(syn::Error::new(
                    *span,
                    "splices are not allowed at the root of `component!` — put them inside the element body",
                ));
            }
        }
    }
    let (name, body) = element.ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "`component!` takes exactly one element, e.g. `component! { action_row { ... } }`",
        )
    })?;
    let n = name.to_string();
    let expansion = match n.as_str() {
        "action_row" => expand_action_row(body)?,
        "text_display" => expand_text_display(body)?,
        "container" => expand_container(body)?,
        "section" => expand_section(body)?,
        "thumbnail" => expand_thumbnail(body)?,
        "media_gallery" => expand_media_gallery(body)?,
        "file" => expand_file(body)?,
        "separator" => expand_separator(body)?,
        "embed" => expand_embed(body)?,
        "button" => expand_button(body)?,
        "select_menu" => expand_select_menu(body, "select_menu")?,
        "select_menu_option" => expand_select_menu_option(body)?,
        "poll" => expand_poll(body)?,
        "poll_answer" => expand_poll_answer(body)?,
        "components_v2" => {
            return Err(syn::Error::new_spanned(
                name,
                "`components_v2` is a message root — use `view!` to build a full message; `component!` emits a single bare builder",
            ));
        }
        _ => {
            let known = [
                "action_row",
                "text_display",
                "container",
                "section",
                "thumbnail",
                "media_gallery",
                "file",
                "separator",
                "embed",
                "button",
                "select_menu",
                "select_menu_option",
                "poll",
                "poll_answer",
            ];
            let mut msg = format!("unknown element `{n}` in `component!`");
            if let Some(s) = did_you_mean(&n, &known) {
                msg.push_str(&format!(" — did you mean `{s}`?"));
            }
            return Err(syn::Error::new_spanned(name, msg));
        }
    };
    Ok(wrap_if_spliced(expansion, &body.items))
}

fn expand(input: &ViewInput) -> syn::Result<proc_macro2::TokenStream> {
    // Detect family
    let has_v2 = input
        .items
        .iter()
        .any(|i| matches!(i, ViewItem::Element { name, .. } if name == "components_v2"));
    let has_legacy_outside_v2 = input.items.iter().any(|i| match i {
        ViewItem::Element { name, .. } => name != "components_v2",
        ViewItem::Attr { .. } => true,
        ViewItem::Bare(_) => true,
        ViewItem::Splice { .. } => true,
    });
    if has_v2 {
        if has_legacy_outside_v2 {
            // Find first legacy item for span
            for item in &input.items {
                match item {
                    ViewItem::Attr { key, .. } => {
                        return Err(syn::Error::new_spanned(
                            key,
                            "legacy attribute cannot appear alongside `components_v2` — use one family per `view!`",
                        ));
                    }
                    ViewItem::Element { name, .. } if name != "components_v2" => {
                        return Err(syn::Error::new_spanned(
                            name,
                            format!(
                                "legacy element `{name}` cannot appear alongside `components_v2` — use one family per `view!`"
                            ),
                        ));
                    }
                    ViewItem::Bare(lit) => {
                        return Err(syn::Error::new_spanned(
                            lit,
                            "bare string cannot appear alongside `components_v2`",
                        ));
                    }
                    ViewItem::Splice { span, .. } => {
                        return Err(syn::Error::new(
                            *span,
                            "splice cannot appear alongside `components_v2` — use one family per `view!`",
                        ));
                    }
                    _ => {}
                }
            }
        }
        // Ensure exactly one components_v2 element
        let mut v2_bodies = Vec::new();
        for item in &input.items {
            if let ViewItem::Element { name, body } = item {
                if name == "components_v2" {
                    v2_bodies.push(body);
                }
            }
        }
        if v2_bodies.len() != 1 {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "expected exactly one `components_v2` element in v2 family",
            ));
        }
        let expansion = expand_v2_root(v2_bodies[0])?;
        return Ok(wrap_if_spliced(expansion, &v2_bodies[0].items));
    }
    let expansion = expand_legacy_root(input)?;
    Ok(wrap_if_spliced(expansion, &input.items))
}

fn expand_legacy_root(input: &ViewInput) -> syn::Result<proc_macro2::TokenStream> {
    const LEGACY_ATTRS: &[&str] = &[
        "content",
        "tts",
        "nonce",
        "enforce_nonce",
        "flags",
        "sticker_ids",
    ];
    let mut content: Option<&Expr> = None;
    let mut tts: Option<&Expr> = None;
    let mut nonce: Option<&Expr> = None;
    let mut enforce_nonce: Option<&Expr> = None;
    let mut flags: Option<&Expr> = None;
    let mut sticker_ids: Option<&Expr> = None;
    let mut embeds: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut components: Vec<VecStep> = Vec::new();
    let mut poll: Option<proc_macro2::TokenStream> = None;
    let mut allowed_mentions: Option<proc_macro2::TokenStream> = None;

    for item in &input.items {
        match item {
            ViewItem::Attr { key, value } => {
                let k = key.to_string();
                match k.as_str() {
                    "content" => {
                        if content.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `content`"));
                        }
                        content = Some(value);
                    }
                    "tts" => {
                        if tts.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `tts`"));
                        }
                        tts = Some(value);
                    }
                    "nonce" => {
                        if nonce.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `nonce`"));
                        }
                        nonce = Some(value);
                    }
                    "enforce_nonce" => {
                        if enforce_nonce.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `enforce_nonce`"));
                        }
                        enforce_nonce = Some(value);
                    }
                    "flags" => {
                        if flags.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `flags`"));
                        }
                        flags = Some(value);
                    }
                    "sticker_ids" => {
                        if sticker_ids.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `sticker_ids`"));
                        }
                        sticker_ids = Some(value);
                    }
                    _ => {
                        let mut msg = format!("unknown root attribute `{k}`");
                        if let Some(s) = did_you_mean(&k, LEGACY_ATTRS) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        } else {
                            msg.push_str(
                                " (known: content, tts, nonce, enforce_nonce, flags, sticker_ids)",
                            );
                        }
                        return Err(syn::Error::new_spanned(key, msg));
                    }
                }
            }
            ViewItem::Element { name, body } => {
                let n = name.to_string();
                match n.as_str() {
                    "embed" => {
                        embeds.push(expand_embed(body)?);
                    }
                    "action_row" => {
                        let ar = expand_action_row(body)?;
                        // Legacy components are CreateComponent::ActionRow
                        components.push(VecStep::Push(
                            quote! { ::pwr_ext::view_support::CreateComponent::ActionRow(#ar) },
                        ));
                    }
                    "allowed_mentions" => {
                        if allowed_mentions.is_some() {
                            return Err(syn::Error::new_spanned(
                                name,
                                "duplicate `allowed_mentions`",
                            ));
                        }
                        allowed_mentions = Some(expand_allowed_mentions(body)?);
                    }
                    "poll" => {
                        if poll.is_some() {
                            return Err(syn::Error::new_spanned(name, "duplicate `poll`"));
                        }
                        poll = Some(expand_poll(body)?);
                    }
                    // v2 elements at legacy root are errors — suggest components_v2
                    "container" | "section" | "text_display" | "media_gallery" | "file"
                    | "separator" | "thumbnail" | "media_gallery_item" => {
                        return Err(syn::Error::new_spanned(
                            name,
                            format!(
                                "v2 element `{n}` cannot appear in legacy family — wrap v2 content in `components_v2 {{ ... }}`"
                            ),
                        ));
                    }
                    _ => {
                        let known = [
                            "embed",
                            "action_row",
                            "allowed_mentions",
                            "poll",
                            "components_v2",
                        ];
                        let mut msg = format!("unknown element `{n}` at message root");
                        if let Some(s) = did_you_mean(&n, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(name, msg));
                    }
                }
            }
            ViewItem::Splice { expr, .. } => {
                components.push(VecStep::Extend(expr.clone()));
            }
            ViewItem::Bare(lit) => {
                return Err(syn::Error::new_spanned(
                    lit,
                    "bare string not allowed at message root — only inside `text_display { \"...\" }`",
                ));
            }
        }
    }

    let mut msg = quote! { ::pwr_ext::view_support::CreateMessage::new() };
    if let Some(v) = content {
        msg = quote! { #msg .content(#v) };
    }
    if let Some(v) = tts {
        msg = quote! { #msg .tts(#v) };
    }
    if let Some(v) = nonce {
        msg = quote! { #msg .nonce(#v) };
    }
    if !embeds.is_empty() {
        msg = quote! { #msg .embeds(vec![#(#embeds),*]) };
    }
    if let Some(v) = allowed_mentions {
        msg = quote! { #msg .allowed_mentions(#v) };
    }
    if !components.is_empty() {
        // Legacy message components have no runtime law check; the checked
        // path still builds the owned vec positionally when a splice is
        // present, and the literal path stays a direct vec.
        let children = build_child_list(
            "__view_components",
            quote! { ::pwr_ext::view_support::CreateComponent<'static> },
            &components,
            None,
            None,
            None,
            None,
            None,
        );
        msg = quote! { #msg .components(#children) };
    }
    if let Some(v) = sticker_ids {
        msg = quote! { #msg .sticker_ids(#v) };
    }
    if let Some(v) = flags {
        msg = quote! { #msg .flags(#v) };
    }
    if let Some(v) = poll {
        msg = quote! { #msg .poll(#v) };
    }
    if let Some(v) = enforce_nonce {
        // enforce_nonce is terminal builder method that returns CreateMessage directly in From impl
        // but in real builder it's .enforce_nonce(bool) -> Self (same as others). Use chain.
        msg = quote! { #msg .enforce_nonce(#v) };
    }
    Ok(msg)
}

fn expand_v2_root(body: &ViewBody) -> syn::Result<proc_macro2::TokenStream> {
    // v2 root may have `flags` attr and components
    let mut flags: Option<&Expr> = None;
    let mut components: Vec<VecStep> = Vec::new();

    for item in &body.items {
        match item {
            ViewItem::Attr { key, value } => {
                let k = key.to_string();
                match k.as_str() {
                    "flags" => {
                        if flags.is_some() {
                            return Err(syn::Error::new_spanned(
                                key,
                                "duplicate `flags` in components_v2",
                            ));
                        }
                        flags = Some(value);
                    }
                    "content" | "tts" | "nonce" | "enforce_nonce" | "sticker_ids" => {
                        return Err(syn::Error::new_spanned(
                            key,
                            format!("legacy attribute `{k}` cannot appear inside `components_v2`"),
                        ));
                    }
                    _ => {
                        let known = ["flags"];
                        let mut msg = format!("unknown attribute `{k}` in `components_v2`");
                        if let Some(s) = did_you_mean(&k, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(key, msg));
                    }
                }
            }
            ViewItem::Element { name, body } => {
                let n = name.to_string();
                let comp = match n.as_str() {
                    "action_row" => {
                        let ar = expand_action_row(body)?;
                        quote! { ::pwr_ext::view_support::CreateComponent::ActionRow(#ar) }
                    }
                    "section" => {
                        let s = expand_section(body)?;
                        quote! { ::pwr_ext::view_support::CreateComponent::Section(#s) }
                    }
                    "text_display" => {
                        let td = expand_text_display(body)?;
                        quote! { ::pwr_ext::view_support::CreateComponent::TextDisplay(#td) }
                    }
                    "media_gallery" => {
                        let mg = expand_media_gallery(body)?;
                        quote! { ::pwr_ext::view_support::CreateComponent::MediaGallery(#mg) }
                    }
                    "file" => {
                        let f = expand_file(body)?;
                        quote! { ::pwr_ext::view_support::CreateComponent::File(#f) }
                    }
                    "separator" => {
                        let s = expand_separator(body)?;
                        quote! { ::pwr_ext::view_support::CreateComponent::Separator(#s) }
                    }
                    "container" => {
                        let c = expand_container(body)?;
                        quote! { ::pwr_ext::view_support::CreateComponent::Container(#c) }
                    }
                    "embed" => {
                        return Err(syn::Error::new_spanned(
                            name,
                            "`embed` cannot appear inside `components_v2` — embeds are legacy-only",
                        ));
                    }
                    _ => {
                        let known = [
                            "action_row",
                            "section",
                            "text_display",
                            "media_gallery",
                            "file",
                            "separator",
                            "container",
                        ];
                        let mut msg = format!("unknown v2 element `{n}` in `components_v2`");
                        if let Some(s) = did_you_mean(&n, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(name, msg));
                    }
                };
                components.push(VecStep::Push(comp));
            }
            ViewItem::Splice { expr, .. } => {
                components.push(VecStep::Extend(expr.clone()));
            }
            ViewItem::Bare(lit) => {
                return Err(syn::Error::new_spanned(
                    lit,
                    "bare string not allowed directly in `components_v2` — use `text_display { \"...\" }`",
                ));
            }
        }
    }

    let has_splices = has_splice(&components);
    if !has_splices && components.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "`components_v2` must contain at least one component",
        ));
    }

    // Auto-set IS_COMPONENTS_V2 = 1 << 15 = 32768
    let flag_expr = if let Some(user_flags) = flags {
        quote! { { let __user = #user_flags; __user | ::pwr_ext::view_support::MessageFlags::from_bits_truncate(1 << 15) } }
    } else {
        quote! { ::pwr_ext::view_support::MessageFlags::from_bits_truncate(1 << 15) }
    };

    let comps = {
        let children = build_child_list(
            "__view_children",
            quote! { ::pwr_ext::view_support::CreateComponent<'static> },
            &components,
            Some(&quote! { ::pwr_ext::view_support::check_v2_root_children }),
            None,
            None,
            None,
            None,
        );
        quote! { .components(#children) }
    };

    Ok(quote! {
        ::pwr_ext::view_support::CreateMessage::new()
            .flags(#flag_expr)
            #comps
    })
}

fn splice_not_allowed(where_: &str, span: &proc_macro2::Span) -> syn::Error {
    syn::Error::new(
        *span,
        format!("`{{ ... }}` splice is not allowed in {where_}"),
    )
}

/// One position in a runtime-assembled child list: either a literal child
/// (pushed in author order) or a `{ expr }` splice (extended at its author
/// position). The item type of the splice's `IntoIterator` must match the
/// list's element type; `Option<T>` covers the conditional 0-or-1 child.
enum VecStep {
    Push(proc_macro2::TokenStream),
    Extend(Expr),
}

fn has_splice(steps: &[VecStep]) -> bool {
    steps.iter().any(|s| matches!(s, VecStep::Extend(_)))
}

/// Wraps an expansion in an immediately-invoked closure returning
/// `Result<_, ChildRuleError>` so an inner `?` (a spliced child-list check)
/// propagates as the macro's return value. Literal views keep the bare
/// builder: the wrap is applied only when the items contain a splice.
fn wrap_if_spliced(
    expansion: proc_macro2::TokenStream,
    items: &[ViewItem],
) -> proc_macro2::TokenStream {
    if !any_splice(items) {
        return expansion;
    }
    quote! {
        (|| -> ::std::result::Result<_, ::pwr_ext::view_support::ChildRuleError> {
            ::std::result::Result::Ok(#expansion)
        })()
    }
}

/// The literal child tokens of a parent's child list, in author order —
/// the `vec![...]` element list for the literal-only emission path.
fn literal_steps(steps: &[VecStep]) -> Vec<&proc_macro2::TokenStream> {
    steps
        .iter()
        .filter_map(|s| match s {
            VecStep::Push(tokens) => Some(tokens),
            VecStep::Extend(_) => None,
        })
        .collect()
}

/// The child-list expression of a parent: a literal `vec![...]` when the
/// list holds no splice, else a runtime block that assembles the list
/// positionally (literals pushed, splices extended in author order), runs
/// the optional runtime-law check over the combined slice, then evaluates
/// to the Vec (or to `tail` when given). Only parents containing a splice
/// take the block path, so literal-only parents keep their direct
/// `vec![...]` emission.
///
/// `literal` overrides the literal-only token list; embed's plain-tuple
/// form differs from the Into-wrapped form carried in its `VecStep` list.
/// `bind` runs before the Vec declaration and `check_extra` appends a second
/// argument to the check call: `expand_section` binds its accessory to a
/// local so the check can borrow it and `CreateSection::new` can move it.
/// `bind`, `check_extra`, and `tail` serve only `expand_section`; `literal`
/// serves only embed's field tuples. Keeping them here is what lets every
/// parent share one splice-scanning, check-then-build helper.
#[allow(clippy::too_many_arguments)]
fn build_child_list(
    var: &str,
    elem_ty: proc_macro2::TokenStream,
    steps: &[VecStep],
    check: Option<&proc_macro2::TokenStream>,
    bind: Option<proc_macro2::TokenStream>,
    check_extra: Option<proc_macro2::TokenStream>,
    tail: Option<proc_macro2::TokenStream>,
    literal: Option<&[proc_macro2::TokenStream]>,
) -> proc_macro2::TokenStream {
    if !has_splice(steps) {
        let lits: Vec<&proc_macro2::TokenStream> = match literal {
            Some(l) => l.iter().collect(),
            None => literal_steps(steps),
        };
        return quote! { vec![#(#lits),*] };
    }
    let ident = syn::Ident::new(var, proc_macro2::Span::call_site());
    let mut step_tokens = Vec::with_capacity(steps.len());
    for step in steps {
        match step {
            VecStep::Push(tokens) => {
                step_tokens.push(quote! { #ident.push(#tokens); });
            }
            VecStep::Extend(expr) => {
                step_tokens.push(quote! { #ident.extend(#expr); });
            }
        }
    }
    let check_call = match (check, check_extra) {
        (Some(c), Some(extra)) => quote! { #c(&#ident, #extra)?; },
        (Some(c), None) => quote! { #c(&#ident)?; },
        (None, _) => quote! {},
    };
    let bind = bind.unwrap_or_else(|| quote! {});
    let tail = tail.unwrap_or_else(|| quote! { #ident });
    quote! {{
        #bind
        let mut #ident: ::std::vec::Vec<#elem_ty> = ::std::vec::Vec::new();
        #(#step_tokens)*
        #check_call
        #tail
    }}
}

// --- embed ---

fn expand_embed(body: &ViewBody) -> syn::Result<proc_macro2::TokenStream> {
    let mut title: Option<&Expr> = None;
    let mut description: Option<&Expr> = None;
    let mut url: Option<&Expr> = None;
    let mut timestamp: Option<&Expr> = None;
    let mut colour: Option<&Expr> = None;
    let mut fields: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut field_steps: Vec<VecStep> = Vec::new();
    let mut author: Option<proc_macro2::TokenStream> = None;
    let mut footer: Option<proc_macro2::TokenStream> = None;
    let mut image: Option<proc_macro2::TokenStream> = None;
    let mut thumbnail: Option<proc_macro2::TokenStream> = None;

    for item in &body.items {
        match item {
            ViewItem::Attr { key, value } => {
                let k = key.to_string();
                match k.as_str() {
                    "title" => {
                        if title.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `title`"));
                        }
                        title = Some(value);
                    }
                    "description" => {
                        if description.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `description`"));
                        }
                        description = Some(value);
                    }
                    "url" => {
                        if url.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `url`"));
                        }
                        url = Some(value);
                    }
                    "timestamp" => {
                        if timestamp.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `timestamp`"));
                        }
                        timestamp = Some(value);
                    }
                    "colour" | "color" => {
                        if colour.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate colour/color"));
                        }
                        colour = Some(value);
                    }
                    _ => {
                        let known = [
                            "title",
                            "description",
                            "url",
                            "timestamp",
                            "colour",
                            "color",
                        ];
                        let mut msg = format!("unknown embed attribute `{k}`");
                        if let Some(s) = did_you_mean(&k, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(key, msg));
                    }
                }
            }
            ViewItem::Element { name, body } => {
                let n = name.to_string();
                match n.as_str() {
                    "field" => {
                        let (plain, into_wrapped) = expand_embed_field(body)?;
                        fields.push(plain);
                        field_steps.push(VecStep::Push(into_wrapped));
                    }
                    "author" => {
                        if author.is_some() {
                            return Err(syn::Error::new_spanned(name, "duplicate `author`"));
                        }
                        author = Some(expand_embed_author(body)?);
                    }
                    "footer" => {
                        if footer.is_some() {
                            return Err(syn::Error::new_spanned(name, "duplicate `footer`"));
                        }
                        footer = Some(expand_embed_footer(body)?);
                    }
                    "image" => {
                        if image.is_some() {
                            return Err(syn::Error::new_spanned(name, "duplicate `image`"));
                        }
                        image = Some(expand_embed_image(body, false)?);
                    }
                    "thumbnail" => {
                        if thumbnail.is_some() {
                            return Err(syn::Error::new_spanned(name, "duplicate `thumbnail`"));
                        }
                        thumbnail = Some(expand_embed_image(body, true)?);
                    }
                    _ => {
                        let known = ["field", "author", "footer", "image", "thumbnail"];
                        let mut msg = format!("unknown embed child `{n}`");
                        if let Some(s) = did_you_mean(&n, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(name, msg));
                    }
                }
            }
            ViewItem::Splice { expr, .. } => {
                // Embed splices contribute fields (name, value, inline);
                // author/footer/image/thumbnail stay literal-only.
                field_steps.push(VecStep::Extend(expr.clone()));
            }
            ViewItem::Bare(lit) => {
                return Err(syn::Error::new_spanned(
                    lit,
                    "bare string not allowed in `embed`",
                ));
            }
        }
    }

    let mut tokens = quote! { ::pwr_ext::view_support::CreateEmbed::new() };
    if let Some(v) = title {
        tokens = quote! { #tokens .title(#v) };
    }
    if let Some(v) = description {
        tokens = quote! { #tokens .description(#v) };
    }
    if let Some(v) = url {
        tokens = quote! { #tokens .url(#v) };
    }
    if let Some(v) = timestamp {
        tokens = quote! { #tokens .timestamp(#v) };
    }
    if let Some(v) = colour {
        tokens = quote! { #tokens .colour(#v) };
    }
    if let Some(v) = author {
        tokens = quote! { #tokens .author(#v) };
    }
    if let Some(v) = footer {
        tokens = quote! { #tokens .footer(#v) };
    }
    if let Some(v) = image {
        // image/thumbnail handling needs separate: image token already includes call
        // For embed, we have stored tokens that are assignments like `.image(url, desc)` — but our helper returns just the args?
        // Instead, expand_embed_image returns the full method chain fragment.
        // To simplify, we handle directly: image returns a TokenStream that is `.image(url, desc)` already applied?
        // Let's change: image/thumbnail tokens are already ` .image(url, desc)` fragments, so we splice.
        // Our current tokens for image is like `::pwr_ext::view_support::CreateEmbedImage`? No, we made it produce call on embed.
        // Simpler: have expand_embed_image return (url_expr, desc_opt) and we apply here.
        // For now, treat as raw builder for embed — but we stored as a TokenStream that is the full embed modification.
        // Instead, we will have image handling directly apply to tokens.
        // image token is actually a TokenStream like `image_url` + `desc` — we need to reconstruct.
        // To avoid complexity, we will have image/thumbnail stored as (url, desc) tuple tokens.
        // Quick fix: image variable currently holds a TokenStream that is the result of expand_embed_image which currently returns a TokenStream for the image builder fragment.
        // We will adjust expand_embed_image to return a tuple of tokens and handle here.
        // For now, just splice: tokens = quote! { #tokens #v };
        // This will produce `.image(url, None)` via #v which is `.image(...)` — but we already have `.`?
        // Let's make expand_embed_image return `quote! { .image(#url, #desc) }` so splice works.
        tokens = quote! { #tokens #v };
    }
    if let Some(v) = thumbnail {
        tokens = quote! { #tokens #v };
    }
    if !field_steps.is_empty() {
        let fields_expr = build_child_list(
            "__view_fields",
            quote! { (::std::string::String, ::std::string::String, bool) },
            &field_steps,
            None,
            None,
            None,
            None,
            Some(&fields),
        );
        tokens = quote! { #tokens .fields(#fields_expr) };
    }
    Ok(tokens)
}

fn expand_embed_field(
    body: &ViewBody,
) -> syn::Result<(proc_macro2::TokenStream, proc_macro2::TokenStream)> {
    let mut name: Option<&Expr> = None;
    let mut value: Option<&Expr> = None;
    let mut inline: Option<&Expr> = None;
    for item in &body.items {
        match item {
            ViewItem::Attr { key, value: v } => {
                let k = key.to_string();
                match k.as_str() {
                    "name" => {
                        if name.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `name`"));
                        }
                        name = Some(v);
                    }
                    "value" => {
                        if value.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `value`"));
                        }
                        value = Some(v);
                    }
                    "inline" => {
                        if inline.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `inline`"));
                        }
                        inline = Some(v);
                    }
                    _ => {
                        let known = ["name", "value", "inline"];
                        let mut msg = format!("unknown field attribute `{k}`");
                        if let Some(s) = did_you_mean(&k, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(key, msg));
                    }
                }
            }
            ViewItem::Element { name, .. } => {
                return Err(syn::Error::new_spanned(
                    name,
                    format!("field cannot contain element `{name}`"),
                ));
            }
            ViewItem::Splice { span, .. } => {
                return Err(splice_not_allowed("`field`", span));
            }
            ViewItem::Bare(lit) => {
                return Err(syn::Error::new_spanned(
                    lit,
                    "bare string not allowed in `field`",
                ));
            }
        }
    }
    let name = name
        .ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "field requires `name`"))?;
    let value = value
        .ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "field requires `value`"))?;
    let inline_tokens = if let Some(v) = inline {
        quote! { #v }
    } else {
        quote! { false }
    };
    // Return tuple for .fields: (plain, into_wrapped)
    // into_wrapped uses Into::into so the splice path can unify with
    // Vec<(String, String, bool)>.
    let plain = quote! { (#name, #value, #inline_tokens) };
    let into_wrapped = quote! { (::std::convert::Into::into(#name), ::std::convert::Into::into(#value), #inline_tokens) };
    Ok((plain, into_wrapped))
}

fn expand_embed_author(body: &ViewBody) -> syn::Result<proc_macro2::TokenStream> {
    let mut name: Option<&Expr> = None;
    let mut url: Option<&Expr> = None;
    let mut icon_url: Option<&Expr> = None;
    for item in &body.items {
        match item {
            ViewItem::Attr { key, value } => {
                let k = key.to_string();
                match k.as_str() {
                    "name" => {
                        if name.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `name`"));
                        }
                        name = Some(value);
                    }
                    "url" => {
                        if url.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `url`"));
                        }
                        url = Some(value);
                    }
                    "icon_url" => {
                        if icon_url.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `icon_url`"));
                        }
                        icon_url = Some(value);
                    }
                    _ => {
                        let known = ["name", "url", "icon_url"];
                        let mut msg = format!("unknown author attribute `{k}`");
                        if let Some(s) = did_you_mean(&k, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(key, msg));
                    }
                }
            }
            ViewItem::Element { name, .. } => {
                return Err(syn::Error::new_spanned(
                    name,
                    format!("author cannot contain element `{name}`"),
                ));
            }
            ViewItem::Splice { span, .. } => {
                return Err(splice_not_allowed("`author`", span));
            }
            ViewItem::Bare(lit) => {
                return Err(syn::Error::new_spanned(
                    lit,
                    "bare string not allowed in `author`",
                ));
            }
        }
    }
    let name = name
        .ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "author requires `name`"))?;
    let mut tokens = quote! { ::pwr_ext::view_support::CreateEmbedAuthor::new(#name) };
    if let Some(v) = url {
        tokens = quote! { #tokens .url(#v) };
    }
    if let Some(v) = icon_url {
        tokens = quote! { #tokens .icon_url(#v) };
    }
    Ok(tokens)
}

fn expand_embed_footer(body: &ViewBody) -> syn::Result<proc_macro2::TokenStream> {
    let mut text: Option<&Expr> = None;
    let mut icon_url: Option<&Expr> = None;
    for item in &body.items {
        match item {
            ViewItem::Attr { key, value } => {
                let k = key.to_string();
                match k.as_str() {
                    "text" => {
                        if text.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `text`"));
                        }
                        text = Some(value);
                    }
                    "icon_url" => {
                        if icon_url.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `icon_url`"));
                        }
                        icon_url = Some(value);
                    }
                    _ => {
                        let known = ["text", "icon_url"];
                        let mut msg = format!("unknown footer attribute `{k}`");
                        if let Some(s) = did_you_mean(&k, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(key, msg));
                    }
                }
            }
            ViewItem::Element { name, .. } => {
                return Err(syn::Error::new_spanned(
                    name,
                    format!("footer cannot contain element `{name}`"),
                ));
            }
            ViewItem::Splice { span, .. } => {
                return Err(splice_not_allowed("`footer`", span));
            }
            ViewItem::Bare(lit) => {
                return Err(syn::Error::new_spanned(
                    lit,
                    "bare string not allowed in `footer`",
                ));
            }
        }
    }
    let text = text
        .ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "footer requires `text`"))?;
    let mut tokens = quote! { ::pwr_ext::view_support::CreateEmbedFooter::new(#text) };
    if let Some(v) = icon_url {
        tokens = quote! { #tokens .icon_url(#v) };
    }
    Ok(tokens)
}

fn expand_embed_image(
    body: &ViewBody,
    is_thumbnail: bool,
) -> syn::Result<proc_macro2::TokenStream> {
    let mut url: Option<&Expr> = None;
    let mut description: Option<&Expr> = None;
    for item in &body.items {
        match item {
            ViewItem::Attr { key, value } => {
                let k = key.to_string();
                match k.as_str() {
                    "url" => {
                        if url.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `url`"));
                        }
                        url = Some(value);
                    }
                    "description" => {
                        if description.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `description`"));
                        }
                        description = Some(value);
                    }
                    _ => {
                        let known = ["url", "description"];
                        let mut msg = format!("unknown image attribute `{k}`");
                        if let Some(s) = did_you_mean(&k, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(key, msg));
                    }
                }
            }
            ViewItem::Element { name, .. } => {
                return Err(syn::Error::new_spanned(
                    name,
                    format!("image cannot contain element `{name}`"),
                ));
            }
            ViewItem::Splice { span, .. } => {
                return Err(splice_not_allowed("`image`/`thumbnail`", span));
            }
            ViewItem::Bare(lit) => {
                return Err(syn::Error::new_spanned(
                    lit,
                    "bare string not allowed in `image`",
                ));
            }
        }
    }
    let url =
        url.ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "image requires `url`"))?;
    let method = if is_thumbnail {
        quote! { thumbnail }
    } else {
        quote! { image }
    };
    if let Some(desc) = description {
        Ok(quote! { .#method(#url, Some(#desc.into())) })
    } else {
        Ok(quote! { .#method(#url, None) })
    }
}

// --- allowed_mentions ---

fn expand_allowed_mentions(body: &ViewBody) -> syn::Result<proc_macro2::TokenStream> {
    let mut tokens = quote! { ::pwr_ext::view_support::CreateAllowedMentions::new() };
    for item in &body.items {
        match item {
            ViewItem::Attr { key, value } => {
                let k = key.to_string();
                match k.as_str() {
                    "parse" => {
                        // parse may be single value or array? Accept expr passed to `parse` via handling of parse field
                        // The builder doesn't have a `.parse` method; it has `.everyone`, `.all_users`, `.all_roles`, `.users`, `.roles`
                        // For DSL, allow `parse: everyone` or `parse: [everyone, users]` etc. But simplest: treat `parse` attr value as expr that should be a list of parse strings?
                        // Instead, we interpret `parse` as mapping: if value is `everyone` then .everyone(true), etc.
                        // If value is array like `[everyone, users]` we need to handle multiple.
                        // For now, handle single or vec: generate code that checks string.
                        // Simplify: expect `parse` value to be a single ident like `everyone`, `users`, `roles` or `all`
                        // We'll handle by generating match on stringified value? Instead, just pass through as method call:
                        // If value is `everyone`, generate `.everyone(true)`
                        // If value is `users`, generate `.all_users(true)`
                        // If value is `roles`, generate `.all_roles(true)`
                        // If value is array expr `[everyone, users]`, we cannot easily detect at macro time without parsing Expr.
                        // For now, support only single value; array case will be handled as bulk via separate `parse` handling that iterates.
                        // We will handle both: if Expr is Array, iterate elements.
                        let v_str = value.to_token_stream_string();
                        if v_str.contains('[') {
                            // Assume array of idents — generate multiple calls? We need to parse Expr kind.
                            // For simplicity, if it's an array, generate loop-like: we cannot at compile time know elements, so we expect user to write `parse: users` etc. as multiple attrs? But duplicate `parse` would error.
                            // So we will just error and suggest using multiple `parse`? Actually we could allow duplicate parse attrs by merging.
                            // To keep simple, return error for array and tell user to use separate `parse`? But duplicate is error.
                            // Instead, we handle array by generating an immediately invoked closure? Simpler: just generate `.parse`? But there is no parse method.
                            // For now, we will support `parse: everyone` as single, and if array, we generate multiple calls via helper.
                            // Detect if value is Expr::Array
                            if let Expr::Array(arr) = value {
                                for elem in &arr.elems {
                                    let elem_str = elem.to_token_stream_string();
                                    match elem_str.as_str() {
                                        "everyone" => tokens = quote! { #tokens .everyone(true) },
                                        "users" => tokens = quote! { #tokens .all_users(true) },
                                        "roles" => tokens = quote! { #tokens .all_roles(true) },
                                        _ => {
                                            return Err(syn::Error::new_spanned(
                                                elem,
                                                format!(
                                                    "unknown parse value `{elem_str}` (expected everyone, users, roles)"
                                                ),
                                            ));
                                        }
                                    }
                                }
                            } else {
                                return Err(syn::Error::new_spanned(
                                    value,
                                    "parse must be an array like `[everyone, users]` or a single value",
                                ));
                            }
                        } else {
                            match v_str.as_str() {
                                "everyone" => tokens = quote! { #tokens .everyone(true) },
                                "users" => tokens = quote! { #tokens .all_users(true) },
                                "roles" => tokens = quote! { #tokens .all_roles(true) },
                                _ => {
                                    return Err(syn::Error::new_spanned(
                                        value,
                                        format!(
                                            "unknown parse value `{v_str}` (expected everyone, users, roles)"
                                        ),
                                    ));
                                }
                            }
                        }
                    }
                    "users" => {
                        tokens = quote! { #tokens .users(#value) };
                    }
                    "roles" => {
                        tokens = quote! { #tokens .roles(#value) };
                    }
                    "replied_user" => {
                        tokens = quote! { #tokens .replied_user(#value) };
                    }
                    _ => {
                        let known = ["parse", "users", "roles", "replied_user"];
                        let mut msg = format!("unknown allowed_mentions attribute `{k}`");
                        if let Some(s) = did_you_mean(&k, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(key, msg));
                    }
                }
            }
            ViewItem::Element { name, .. } => {
                return Err(syn::Error::new_spanned(
                    name,
                    format!("allowed_mentions cannot contain element `{name}`"),
                ));
            }
            ViewItem::Splice { span, .. } => {
                return Err(splice_not_allowed("`allowed_mentions`", span));
            }
            ViewItem::Bare(lit) => {
                return Err(syn::Error::new_spanned(
                    lit,
                    "bare string not allowed in `allowed_mentions`",
                ));
            }
        }
    }
    Ok(tokens)
}

// helper to get token stream string for Expr (quick)
trait ToTokenStreamString {
    fn to_token_stream_string(&self) -> String;
}
impl ToTokenStreamString for Expr {
    fn to_token_stream_string(&self) -> String {
        let ts = quote! { #self };
        ts.to_string().replace(' ', "")
    }
}

// --- poll ---

fn expand_poll(body: &ViewBody) -> syn::Result<proc_macro2::TokenStream> {
    let mut question: Option<&Expr> = None;
    let mut duration: Option<&Expr> = None;
    let mut allow_multiselect: Option<&Expr> = None;
    let mut layout_type: Option<&Expr> = None;
    let mut answers: Vec<proc_macro2::TokenStream> = Vec::new();

    for item in &body.items {
        match item {
            ViewItem::Attr { key, value } => {
                let k = key.to_string();
                match k.as_str() {
                    "question" => {
                        if question.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `question`"));
                        }
                        question = Some(value);
                    }
                    "duration" => {
                        if duration.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `duration`"));
                        }
                        duration = Some(value);
                    }
                    "allow_multiselect" => {
                        if allow_multiselect.is_some() {
                            return Err(syn::Error::new_spanned(
                                key,
                                "duplicate `allow_multiselect`",
                            ));
                        }
                        allow_multiselect = Some(value);
                    }
                    "layout_type" => {
                        if layout_type.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `layout_type`"));
                        }
                        layout_type = Some(value);
                    }
                    _ => {
                        let known = ["question", "duration", "allow_multiselect", "layout_type"];
                        let mut msg = format!("unknown poll attribute `{k}`");
                        if let Some(s) = did_you_mean(&k, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(key, msg));
                    }
                }
            }
            ViewItem::Element { name, body } => {
                let n = name.to_string();
                match n.as_str() {
                    "poll_answer" | "answer" => {
                        answers.push(expand_poll_answer(body)?);
                    }
                    _ => {
                        let known = ["poll_answer", "answer"];
                        let mut msg = format!("unknown poll child `{n}`");
                        if let Some(s) = did_you_mean(&n, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(name, msg));
                    }
                }
            }
            ViewItem::Splice { span, .. } => {
                return Err(splice_not_allowed("`poll`", span));
            }
            ViewItem::Bare(lit) => {
                return Err(syn::Error::new_spanned(
                    lit,
                    "bare string not allowed in `poll`",
                ));
            }
        }
    }
    let question = question.ok_or_else(|| {
        syn::Error::new(proc_macro2::Span::call_site(), "poll requires `question`")
    })?;
    let duration = duration.ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "poll requires `duration` (hours)",
        )
    })?;
    let mut tokens = quote! {
        ::pwr_ext::view_support::CreatePoll::new()
            .question(#question)
            .answers(vec![#(#answers),*])
            .duration(::std::time::Duration::from_secs((#duration as u64) * 3600))
    };
    if let Some(v) = allow_multiselect {
        // If true, call .allow_multiselect(), if false, do nothing? But DSL may provide bool expr.
        // We generate conditional: if #v { poll.allow_multiselect() } else { poll }
        // Simpler: if v is literal true/false, we can check, but we treat as runtime bool:
        tokens = quote! { { let mut __poll = #tokens; if #v { __poll = __poll.allow_multiselect(); } __poll } };
    }
    if let Some(v) = layout_type {
        tokens = quote! { #tokens .layout_type(#v) };
    }
    Ok(tokens)
}

fn expand_poll_answer(body: &ViewBody) -> syn::Result<proc_macro2::TokenStream> {
    let mut text: Option<&Expr> = None;
    let mut emoji: Option<&Expr> = None;
    for item in &body.items {
        match item {
            ViewItem::Attr { key, value } => {
                let k = key.to_string();
                match k.as_str() {
                    "text" => {
                        if text.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `text`"));
                        }
                        text = Some(value);
                    }
                    "emoji" => {
                        if emoji.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `emoji`"));
                        }
                        emoji = Some(value);
                    }
                    _ => {
                        let known = ["text", "emoji"];
                        let mut msg = format!("unknown poll_answer attribute `{k}`");
                        if let Some(s) = did_you_mean(&k, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(key, msg));
                    }
                }
            }
            ViewItem::Element { name, .. } => {
                return Err(syn::Error::new_spanned(
                    name,
                    format!("poll_answer cannot contain element `{name}`"),
                ));
            }
            ViewItem::Splice { span, .. } => {
                return Err(splice_not_allowed("`poll_answer`", span));
            }
            ViewItem::Bare(lit) => {
                return Err(syn::Error::new_spanned(
                    lit,
                    "bare string not allowed in `poll_answer`",
                ));
            }
        }
    }
    let mut tokens = quote! { ::pwr_ext::view_support::CreatePollAnswer::new() };
    if let Some(v) = text {
        tokens = quote! { #tokens .text(#v) };
    }
    if let Some(v) = emoji {
        tokens = quote! { #tokens .emoji(#v) };
    }
    Ok(tokens)
}

// --- action_row, button, select_menu ---

fn expand_action_row(body: &ViewBody) -> syn::Result<proc_macro2::TokenStream> {
    let mut buttons: Vec<VecStep> = Vec::new();
    let mut select_menu: Option<proc_macro2::TokenStream> = None;
    // Decide once whether the row contains any splice, over ALL items, so
    // the compile-vs-runtime count guard is order-independent: any splice
    // anywhere defers the button limit to the runtime check.
    let has_any_splice = any_splice(&body.items);

    for item in &body.items {
        match item {
            ViewItem::Element { name, body } => {
                let n = name.to_string();
                match n.as_str() {
                    "button" => {
                        if select_menu.is_some() {
                            return Err(syn::Error::new_spanned(
                                name,
                                "action_row cannot contain both buttons and a select menu",
                            ));
                        }
                        if !has_any_splice && buttons.len() >= 5 {
                            return Err(syn::Error::new_spanned(
                                name,
                                "action_row cannot contain more than 5 buttons",
                            ));
                        }
                        buttons.push(VecStep::Push(expand_button(body)?));
                    }
                    "select_menu" | "string_select" | "user_select" | "role_select"
                    | "mentionable_select" | "channel_select" => {
                        if !buttons.is_empty() {
                            return Err(syn::Error::new_spanned(
                                name,
                                "action_row cannot contain both buttons and a select menu",
                            ));
                        }
                        if select_menu.is_some() {
                            return Err(syn::Error::new_spanned(
                                name,
                                "action_row can contain at most one select menu",
                            ));
                        }
                        select_menu = Some(expand_select_menu(body, &n)?);
                    }
                    "select_menu_option" | "option" => {
                        return Err(syn::Error::new_spanned(
                            name,
                            "`select_menu_option` must be inside `select_menu`, not directly in `action_row`",
                        ));
                    }
                    _ => {
                        let known = ["button", "select_menu"];
                        let mut msg = format!("unknown action_row child `{n}`");
                        if let Some(s) = did_you_mean(&n, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(name, msg));
                    }
                }
            }
            ViewItem::Attr { key, .. } => {
                return Err(syn::Error::new_spanned(
                    key,
                    format!("action_row cannot contain attribute `{key}`"),
                ));
            }
            ViewItem::Splice { expr, span } => {
                if select_menu.is_some() {
                    return Err(syn::Error::new(
                        *span,
                        "action_row cannot contain both buttons and a select menu",
                    ));
                }
                buttons.push(VecStep::Extend(expr.clone()));
            }
            ViewItem::Bare(lit) => {
                return Err(syn::Error::new_spanned(
                    lit,
                    "bare string not allowed in `action_row`",
                ));
            }
        }
    }

    if let Some(sm) = select_menu {
        Ok(quote! { ::pwr_ext::view_support::CreateActionRow::select_menu(#sm) })
    } else if !buttons.is_empty() {
        let children = build_child_list(
            "__view_buttons",
            quote! { ::pwr_ext::view_support::CreateButton<'static> },
            &buttons,
            Some(&quote! { ::pwr_ext::view_support::check_action_row_children }),
            None,
            None,
            None,
            None,
        );
        Ok(quote! { ::pwr_ext::view_support::CreateActionRow::buttons(#children) })
    } else {
        Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "action_row must contain at least one button or a select menu",
        ))
    }
}

fn expand_button(body: &ViewBody) -> syn::Result<proc_macro2::TokenStream> {
    let mut style: Option<&Expr> = None;
    let mut label: Option<&Expr> = None;
    let mut custom_id: Option<&Expr> = None;
    let mut url: Option<&Expr> = None;
    let mut sku_id: Option<&Expr> = None;
    let mut emoji: Option<&Expr> = None;
    let mut disabled: Option<&Expr> = None;

    for item in &body.items {
        match item {
            ViewItem::Attr { key, value } => {
                let k = key.to_string();
                match k.as_str() {
                    "style" => {
                        if style.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `style`"));
                        }
                        style = Some(value);
                    }
                    "label" => {
                        if label.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `label`"));
                        }
                        label = Some(value);
                    }
                    "custom_id" => {
                        if custom_id.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `custom_id`"));
                        }
                        custom_id = Some(value);
                    }
                    "url" => {
                        if url.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `url`"));
                        }
                        url = Some(value);
                    }
                    "sku_id" => {
                        if sku_id.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `sku_id`"));
                        }
                        sku_id = Some(value);
                    }
                    "emoji" => {
                        if emoji.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `emoji`"));
                        }
                        emoji = Some(value);
                    }
                    "disabled" => {
                        if disabled.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `disabled`"));
                        }
                        disabled = Some(value);
                    }
                    _ => {
                        let known = [
                            "style",
                            "label",
                            "custom_id",
                            "url",
                            "sku_id",
                            "emoji",
                            "disabled",
                        ];
                        let mut msg = format!("unknown button attribute `{k}`");
                        if let Some(s) = did_you_mean(&k, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(key, msg));
                    }
                }
            }
            ViewItem::Element { name, .. } => {
                return Err(syn::Error::new_spanned(
                    name,
                    format!("button cannot contain element `{name}`"),
                ));
            }
            ViewItem::Splice { span, .. } => {
                return Err(splice_not_allowed("`button`", span));
            }
            ViewItem::Bare(lit) => {
                return Err(syn::Error::new_spanned(
                    lit,
                    "bare string not allowed in `button`",
                ));
            }
        }
    }

    // Determine constructor based on url/sku_id presence; check style laws when literal
    // We attempt to detect literal style value for validation: if value is like `1`, `5`, `ButtonStyle::Primary` etc.
    // For literal int, we can check stringified value.
    if let (Some(_), Some(_)) = (url, custom_id) {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "link button (`url`) cannot have `custom_id`",
        ));
    }
    if let (Some(_), Some(_)) = (url, sku_id) {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "link button (`url`) cannot have `sku_id`",
        ));
    }
    if let (Some(_), Some(_)) = (sku_id, custom_id) {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "premium button (`sku_id`) cannot have `custom_id`",
        ));
    }
    if sku_id.is_some() && label.is_some() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "premium button (`sku_id`) cannot have `label`",
        ));
    }
    if sku_id.is_some() && url.is_some() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "premium button (`sku_id`) cannot have `url`",
        ));
    }
    if sku_id.is_some() && emoji.is_some() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "premium button (`sku_id`) cannot have `emoji`",
        ));
    }

    // Literal style-law checks: if style is a literal integer, validate field requirements
    if let Some(s) = style {
        let s_str = s.to_token_stream_string();
        // s_str for `ButtonStyle::Primary` becomes `ButtonStyle::Primary` with no spaces after our helper (we removed spaces)
        // So we check both int style and enum style
        let is_link_style = s_str == "5" || s_str.contains("Link");
        let is_premium_style = s_str == "6" || s_str.contains("Premium");
        if is_link_style && custom_id.is_some() {
            return Err(syn::Error::new_spanned(
                s,
                "Link style button cannot have `custom_id` — use `url` instead",
            ));
        }
        if is_premium_style && custom_id.is_some() {
            return Err(syn::Error::new_spanned(
                s,
                "Premium style button cannot have `custom_id`",
            ));
        }
        if is_link_style && url.is_none() {
            // For link style, url required — but if style is Link and url is provided via constructor, it's ok; but if style literal says Link and no url, error.
            // However our constructor already handles url vs custom_id; just enforce url presence for link style.
            return Err(syn::Error::new_spanned(
                s,
                "Link style button requires `url`",
            ));
        }
    } else {
        // If no style provided but is link/premium via url/sku_id, we don't enforce style; builder will default to Primary but will be link/premium via constructor.
    }

    let mut tokens = if let Some(u) = url {
        quote! { ::pwr_ext::view_support::CreateButton::new_link(#u) }
    } else if let Some(sku) = sku_id {
        quote! { ::pwr_ext::view_support::CreateButton::new_premium(#sku) }
    } else if let Some(cid) = custom_id {
        quote! { ::pwr_ext::view_support::CreateButton::new(#cid) }
    } else {
        // No custom_id/url/sku_id — but for link/premium we already handled; for normal, custom_id is required.
        // If url/sku_id not present and no custom_id, we need to error, but we can also allow deferred runtime: require custom_id for non-link/premium.
        // Since style may be link/premium without url/sku_id (error above), we check.
        // For now, error.
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "button requires `custom_id` (or `url` for link, `sku_id` for premium)",
        ));
    };

    if let Some(s) = style {
        // Only apply style if not link/premium (builder ignores it otherwise, but we already validated)
        if url.is_none() && sku_id.is_none() {
            tokens = quote! { #tokens .style(#s) };
        }
    }
    if let Some(v) = label {
        tokens = quote! { #tokens .label(#v) };
    }
    if let Some(v) = emoji {
        tokens = quote! { #tokens .emoji(#v) };
    }
    if let Some(v) = disabled {
        tokens = quote! { #tokens .disabled(#v) };
    }
    Ok(tokens)
}

fn expand_select_menu(body: &ViewBody, kind_name: &str) -> syn::Result<proc_macro2::TokenStream> {
    let mut custom_id: Option<&Expr> = None;
    let mut placeholder: Option<&Expr> = None;
    let mut min_values: Option<&Expr> = None;
    let mut max_values: Option<&Expr> = None;
    let mut disabled: Option<&Expr> = None;
    let mut required: Option<&Expr> = None;
    let mut options: Vec<proc_macro2::TokenStream> = Vec::new();
    // For non-string kinds, we ignore options; but we need to detect kind from element name or `kind` attr?
    // The DSL may have `select_menu { custom_id: "...", kind: string, ... }` or `string_select` element.
    // We'll treat element name as kind if it is specific; else if name is generic `select_menu`, look for `kind` attr.
    let mut kind_attr: Option<&Expr> = None;

    for item in &body.items {
        match item {
            ViewItem::Attr { key, value } => {
                let k = key.to_string();
                match k.as_str() {
                    "custom_id" => {
                        if custom_id.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `custom_id`"));
                        }
                        custom_id = Some(value);
                    }
                    "placeholder" => {
                        if placeholder.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `placeholder`"));
                        }
                        placeholder = Some(value);
                    }
                    "min_values" => {
                        if min_values.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `min_values`"));
                        }
                        min_values = Some(value);
                    }
                    "max_values" => {
                        if max_values.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `max_values`"));
                        }
                        max_values = Some(value);
                    }
                    "disabled" => {
                        if disabled.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `disabled`"));
                        }
                        disabled = Some(value);
                    }
                    "required" => {
                        if required.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `required`"));
                        }
                        required = Some(value);
                    }
                    "kind" => {
                        if kind_attr.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `kind`"));
                        }
                        kind_attr = Some(value);
                    }
                    _ => {
                        let known = [
                            "custom_id",
                            "placeholder",
                            "min_values",
                            "max_values",
                            "disabled",
                            "required",
                            "kind",
                        ];
                        let mut msg = format!("unknown select_menu attribute `{k}`");
                        if let Some(s) = did_you_mean(&k, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(key, msg));
                    }
                }
            }
            ViewItem::Element { name, body } => {
                let n = name.to_string();
                match n.as_str() {
                    "select_menu_option" | "option" => {
                        if kind_name != "select_menu"
                            && kind_name != "string_select"
                            && kind_attr.is_some()
                        {
                            // kind is not string — options should not be allowed
                            let kind_str = kind_attr
                                .map(|e| e.to_token_stream_string())
                                .unwrap_or_default();
                            if kind_str != "string" && !kind_str.contains("String") {
                                return Err(syn::Error::new_spanned(
                                    name,
                                    format!("`{n}` only allowed for string select (kind=string)"),
                                ));
                            }
                        }
                        if options.len() >= 25 {
                            return Err(syn::Error::new_spanned(
                                name,
                                "select menu cannot have more than 25 options",
                            ));
                        }
                        options.push(expand_select_menu_option(body)?);
                    }
                    _ => {
                        let known = ["select_menu_option", "option"];
                        let mut msg = format!("unknown select_menu child `{n}`");
                        if let Some(s) = did_you_mean(&n, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(name, msg));
                    }
                }
            }
            ViewItem::Splice { span, .. } => {
                return Err(splice_not_allowed("`select_menu`", span));
            }
            ViewItem::Bare(lit) => {
                return Err(syn::Error::new_spanned(
                    lit,
                    "bare string not allowed in `select_menu`",
                ));
            }
        }
    }

    let custom_id = custom_id.ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "select_menu requires `custom_id`",
        )
    })?;

    // Determine kind
    let kind_tokens = match kind_name {
        "string_select" => {
            quote! { ::pwr_ext::view_support::CreateSelectMenuKind::String { options: ::std::borrow::Cow::Owned(vec![#(#options),*]) } }
        }
        "user_select" => {
            if !options.is_empty() {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "user select cannot have options",
                ));
            }
            quote! { ::pwr_ext::view_support::CreateSelectMenuKind::User { default_users: None } }
        }
        "role_select" => {
            if !options.is_empty() {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "role select cannot have options",
                ));
            }
            quote! { ::pwr_ext::view_support::CreateSelectMenuKind::Role { default_roles: None } }
        }
        "mentionable_select" => {
            if !options.is_empty() {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "mentionable select cannot have options",
                ));
            }
            quote! { ::pwr_ext::view_support::CreateSelectMenuKind::Mentionable { default_users: None, default_roles: None } }
        }
        "channel_select" => {
            if !options.is_empty() {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "channel select cannot have options",
                ));
            }
            quote! { ::pwr_ext::view_support::CreateSelectMenuKind::Channel { channel_types: None, default_channels: None } }
        }
        _ => {
            // generic select_menu — look at kind attr
            if let Some(k) = kind_attr {
                let k_str = k.to_token_stream_string();
                // k may be like `string`, `user`, `role`, `mentionable`, `channel`, or `CreateSelectMenuKind::String` etc.
                // Simplify: accept string literals "string", "user", etc., or ident like `string`
                match k_str.as_str() {
                    "string" | "\"string\"" | "String" => {
                        quote! { ::pwr_ext::view_support::CreateSelectMenuKind::String { options: ::std::borrow::Cow::Owned(vec![#(#options),*]) } }
                    }
                    "user" | "\"user\"" | "User" => {
                        if !options.is_empty() {
                            return Err(syn::Error::new_spanned(
                                k,
                                "user select cannot have options",
                            ));
                        }
                        quote! { ::pwr_ext::view_support::CreateSelectMenuKind::User { default_users: None } }
                    }
                    "role" | "\"role\"" | "Role" => {
                        if !options.is_empty() {
                            return Err(syn::Error::new_spanned(
                                k,
                                "role select cannot have options",
                            ));
                        }
                        quote! { ::pwr_ext::view_support::CreateSelectMenuKind::Role { default_roles: None } }
                    }
                    "mentionable" | "\"mentionable\"" | "Mentionable" => {
                        if !options.is_empty() {
                            return Err(syn::Error::new_spanned(
                                k,
                                "mentionable select cannot have options",
                            ));
                        }
                        quote! { ::pwr_ext::view_support::CreateSelectMenuKind::Mentionable { default_users: None, default_roles: None } }
                    }
                    "channel" | "\"channel\"" | "Channel" => {
                        if !options.is_empty() {
                            return Err(syn::Error::new_spanned(
                                k,
                                "channel select cannot have options",
                            ));
                        }
                        quote! { ::pwr_ext::view_support::CreateSelectMenuKind::Channel { channel_types: None, default_channels: None } }
                    }
                    _ => {
                        // Fallback: if k is an expr that already is a CreateSelectMenuKind, just use it directly
                        // But we need to ensure options handling: if user provided custom kind expr, we ignore options? Instead, use k directly.
                        quote! { #k }
                    }
                }
            } else {
                // Default to string if options present, else error?
                if !options.is_empty() {
                    quote! { ::pwr_ext::view_support::CreateSelectMenuKind::String { options: ::std::borrow::Cow::Owned(vec![#(#options),*]) } }
                } else {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "select_menu requires `kind` or options (for string select)",
                    ));
                }
            }
        }
    };

    let mut tokens =
        quote! { ::pwr_ext::view_support::CreateSelectMenu::new(#custom_id, #kind_tokens) };
    if let Some(v) = placeholder {
        tokens = quote! { #tokens .placeholder(#v) };
    }
    if let Some(v) = min_values {
        tokens = quote! { #tokens .min_values(#v) };
    }
    if let Some(v) = max_values {
        tokens = quote! { #tokens .max_values(#v) };
    }
    if let Some(v) = required {
        tokens = quote! { #tokens .required(#v) };
    }
    if let Some(v) = disabled {
        tokens = quote! { #tokens .disabled(#v) };
    }
    Ok(tokens)
}

fn expand_select_menu_option(body: &ViewBody) -> syn::Result<proc_macro2::TokenStream> {
    let mut label: Option<&Expr> = None;
    let mut value: Option<&Expr> = None;
    let mut description: Option<&Expr> = None;
    let mut emoji: Option<&Expr> = None;
    let mut default: Option<&Expr> = None;

    for item in &body.items {
        match item {
            ViewItem::Attr { key, value: v } => {
                let k = key.to_string();
                match k.as_str() {
                    "label" => {
                        if label.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `label`"));
                        }
                        label = Some(v);
                    }
                    "value" => {
                        if value.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `value`"));
                        }
                        value = Some(v);
                    }
                    "description" => {
                        if description.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `description`"));
                        }
                        description = Some(v);
                    }
                    "emoji" => {
                        if emoji.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `emoji`"));
                        }
                        emoji = Some(v);
                    }
                    "default" | "default_selection" => {
                        if default.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `default`"));
                        }
                        default = Some(v);
                    }
                    _ => {
                        let known = ["label", "value", "description", "emoji", "default"];
                        let mut msg = format!("unknown option attribute `{k}`");
                        if let Some(s) = did_you_mean(&k, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(key, msg));
                    }
                }
            }
            ViewItem::Element { name, .. } => {
                return Err(syn::Error::new_spanned(
                    name,
                    format!("option cannot contain element `{name}`"),
                ));
            }
            ViewItem::Splice { span, .. } => {
                return Err(splice_not_allowed("`option`", span));
            }
            ViewItem::Bare(lit) => {
                return Err(syn::Error::new_spanned(
                    lit,
                    "bare string not allowed in `option`",
                ));
            }
        }
    }
    let label = label.ok_or_else(|| {
        syn::Error::new(proc_macro2::Span::call_site(), "option requires `label`")
    })?;
    let value = value.ok_or_else(|| {
        syn::Error::new(proc_macro2::Span::call_site(), "option requires `value`")
    })?;
    let mut tokens =
        quote! { ::pwr_ext::view_support::CreateSelectMenuOption::new(#label, #value) };
    if let Some(v) = description {
        tokens = quote! { #tokens .description(#v) };
    }
    if let Some(v) = emoji {
        tokens = quote! { #tokens .emoji(#v) };
    }
    if let Some(v) = default {
        tokens = quote! { #tokens .default_selection(#v) };
    }
    Ok(tokens)
}

// --- v2 components ---

fn expand_text_display(body: &ViewBody) -> syn::Result<proc_macro2::TokenStream> {
    // May be bare string `text_display { "hello" }` or `text_display { content: "hello" }`
    if body.items.len() == 1 {
        if let ViewItem::Bare(lit) = &body.items[0] {
            return Ok(quote! { ::pwr_ext::view_support::CreateTextDisplay::new(#lit) });
        }
    }
    let mut content: Option<&Expr> = None;
    let mut bare: Option<&syn::LitStr> = None;
    for item in &body.items {
        match item {
            ViewItem::Attr { key, value } => {
                let k = key.to_string();
                if k == "content" {
                    if content.is_some() || bare.is_some() {
                        return Err(syn::Error::new_spanned(key, "duplicate content"));
                    }
                    content = Some(value);
                } else {
                    let known = ["content"];
                    let mut msg = format!("unknown text_display attribute `{k}`");
                    if let Some(s) = did_you_mean(&k, &known) {
                        msg.push_str(&format!(" — did you mean `{s}`?"));
                    }
                    return Err(syn::Error::new_spanned(key, msg));
                }
            }
            ViewItem::Splice { span, .. } => {
                return Err(splice_not_allowed("`text_display`", span));
            }
            ViewItem::Bare(lit) => {
                if content.is_some() || bare.is_some() {
                    return Err(syn::Error::new_spanned(lit, "duplicate content"));
                }
                bare = Some(lit);
            }
            ViewItem::Element { name, .. } => {
                return Err(syn::Error::new_spanned(
                    name,
                    format!("text_display cannot contain element `{name}`"),
                ));
            }
        }
    }
    if let Some(c) = content {
        Ok(quote! { ::pwr_ext::view_support::CreateTextDisplay::new(#c) })
    } else if let Some(b) = bare {
        Ok(quote! { ::pwr_ext::view_support::CreateTextDisplay::new(#b) })
    } else {
        Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "text_display requires content (bare string or `content: ...`)",
        ))
    }
}

fn expand_thumbnail(body: &ViewBody) -> syn::Result<proc_macro2::TokenStream> {
    let mut media: Option<&Expr> = None;
    let mut description: Option<&Expr> = None;
    let mut spoiler: Option<&Expr> = None;
    for item in &body.items {
        match item {
            ViewItem::Attr { key, value } => {
                let k = key.to_string();
                match k.as_str() {
                    "media" | "url" => {
                        if media.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `media`"));
                        }
                        media = Some(value);
                    }
                    "description" => {
                        if description.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `description`"));
                        }
                        description = Some(value);
                    }
                    "spoiler" => {
                        if spoiler.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `spoiler`"));
                        }
                        spoiler = Some(value);
                    }
                    _ => {
                        let known = ["media", "url", "description", "spoiler"];
                        let mut msg = format!("unknown thumbnail attribute `{k}`");
                        if let Some(s) = did_you_mean(&k, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(key, msg));
                    }
                }
            }
            ViewItem::Element { name, .. } => {
                return Err(syn::Error::new_spanned(
                    name,
                    format!("thumbnail cannot contain element `{name}`"),
                ));
            }
            ViewItem::Splice { span, .. } => {
                return Err(splice_not_allowed("`thumbnail`", span));
            }
            ViewItem::Bare(lit) => {
                return Err(syn::Error::new_spanned(
                    lit,
                    "bare string not allowed in `thumbnail`",
                ));
            }
        }
    }
    let media = media.ok_or_else(|| {
        syn::Error::new(proc_macro2::Span::call_site(), "thumbnail requires `media`")
    })?;
    let mut tokens = quote! { ::pwr_ext::view_support::CreateThumbnail::new(::pwr_ext::view_support::CreateUnfurledMediaItem::new(#media)) };
    if let Some(v) = description {
        tokens = quote! { #tokens .description(#v) };
    }
    if let Some(v) = spoiler {
        tokens = quote! { #tokens .spoiler(#v) };
    }
    Ok(tokens)
}

fn expand_media_gallery(body: &ViewBody) -> syn::Result<proc_macro2::TokenStream> {
    let mut items: Vec<VecStep> = Vec::new();
    for item in &body.items {
        match item {
            ViewItem::Element { name, body } => {
                let n = name.to_string();
                if n == "media_gallery_item" || n == "item" {
                    items.push(VecStep::Push(expand_media_gallery_item(body)?));
                } else {
                    let known = ["media_gallery_item", "item"];
                    let mut msg = format!("unknown media_gallery child `{n}`");
                    if let Some(s) = did_you_mean(&n, &known) {
                        msg.push_str(&format!(" — did you mean `{s}`?"));
                    }
                    return Err(syn::Error::new_spanned(name, msg));
                }
            }
            ViewItem::Attr { key, .. } => {
                return Err(syn::Error::new_spanned(
                    key,
                    format!("media_gallery cannot contain attribute `{key}`"),
                ));
            }
            ViewItem::Splice { expr, .. } => {
                items.push(VecStep::Extend(expr.clone()));
            }
            ViewItem::Bare(lit) => {
                return Err(syn::Error::new_spanned(
                    lit,
                    "bare string not allowed in `media_gallery`",
                ));
            }
        }
    }
    let has_splices = has_splice(&items);
    if !has_splices && items.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "media_gallery must contain at least one `media_gallery_item`",
        ));
    }
    if !has_splices && items.len() > 10 {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "media_gallery cannot contain more than 10 items",
        ));
    }
    let children = build_child_list(
        "__view_items",
        quote! { ::pwr_ext::view_support::CreateMediaGalleryItem<'static> },
        &items,
        Some(&quote! { ::pwr_ext::view_support::check_media_gallery_items }),
        None,
        None,
        None,
        None,
    );
    Ok(quote! { ::pwr_ext::view_support::CreateMediaGallery::new(#children) })
}

fn expand_media_gallery_item(body: &ViewBody) -> syn::Result<proc_macro2::TokenStream> {
    let mut media: Option<&Expr> = None;
    let mut description: Option<&Expr> = None;
    let mut spoiler: Option<&Expr> = None;
    for item in &body.items {
        match item {
            ViewItem::Attr { key, value } => {
                let k = key.to_string();
                match k.as_str() {
                    "media" | "url" => {
                        if media.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `media`"));
                        }
                        media = Some(value);
                    }
                    "description" => {
                        if description.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `description`"));
                        }
                        description = Some(value);
                    }
                    "spoiler" => {
                        if spoiler.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `spoiler`"));
                        }
                        spoiler = Some(value);
                    }
                    _ => {
                        let known = ["media", "url", "description", "spoiler"];
                        let mut msg = format!("unknown media_gallery_item attribute `{k}`");
                        if let Some(s) = did_you_mean(&k, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(key, msg));
                    }
                }
            }
            ViewItem::Element { name, .. } => {
                return Err(syn::Error::new_spanned(
                    name,
                    format!("media_gallery_item cannot contain element `{name}`"),
                ));
            }
            ViewItem::Splice { span, .. } => {
                return Err(splice_not_allowed("`media_gallery_item`", span));
            }
            ViewItem::Bare(lit) => {
                return Err(syn::Error::new_spanned(
                    lit,
                    "bare string not allowed in `media_gallery_item`",
                ));
            }
        }
    }
    let media = media.ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "media_gallery_item requires `media`",
        )
    })?;
    let mut tokens = quote! { ::pwr_ext::view_support::CreateMediaGalleryItem::new(::pwr_ext::view_support::CreateUnfurledMediaItem::new(#media)) };
    if let Some(v) = description {
        tokens = quote! { #tokens .description(#v) };
    }
    if let Some(v) = spoiler {
        tokens = quote! { #tokens .spoiler(#v) };
    }
    Ok(tokens)
}

fn expand_file(body: &ViewBody) -> syn::Result<proc_macro2::TokenStream> {
    let mut file: Option<&Expr> = None;
    let mut media: Option<&Expr> = None;
    let mut url: Option<&Expr> = None;
    let mut spoiler: Option<&Expr> = None;
    for item in &body.items {
        match item {
            ViewItem::Attr { key, value } => {
                let k = key.to_string();
                match k.as_str() {
                    "file" => {
                        if file.is_some() || media.is_some() || url.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate file/media/url"));
                        }
                        file = Some(value);
                    }
                    "media" => {
                        if file.is_some() || media.is_some() || url.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate file/media/url"));
                        }
                        media = Some(value);
                    }
                    "url" => {
                        if file.is_some() || media.is_some() || url.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate file/media/url"));
                        }
                        url = Some(value);
                    }
                    "spoiler" => {
                        if spoiler.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `spoiler`"));
                        }
                        spoiler = Some(value);
                    }
                    _ => {
                        let known = ["file", "media", "url", "spoiler"];
                        let mut msg = format!("unknown file attribute `{k}`");
                        if let Some(s) = did_you_mean(&k, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(key, msg));
                    }
                }
            }
            ViewItem::Element { name, .. } => {
                return Err(syn::Error::new_spanned(
                    name,
                    format!("file cannot contain element `{name}`"),
                ));
            }
            ViewItem::Splice { span, .. } => {
                return Err(splice_not_allowed("`file`", span));
            }
            ViewItem::Bare(lit) => {
                return Err(syn::Error::new_spanned(
                    lit,
                    "bare string not allowed in `file`",
                ));
            }
        }
    }
    let media_expr = file.or(media).or(url).ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "file requires `file` (or `media`/`url`)",
        )
    })?;
    let mut tokens = quote! { ::pwr_ext::view_support::CreateFile::new(::pwr_ext::view_support::CreateUnfurledMediaItem::new(#media_expr)) };
    if let Some(v) = spoiler {
        tokens = quote! { #tokens .spoiler(#v) };
    }
    Ok(tokens)
}

fn expand_separator(body: &ViewBody) -> syn::Result<proc_macro2::TokenStream> {
    let mut divider: Option<&Expr> = None;
    let mut spacing: Option<&Expr> = None;
    for item in &body.items {
        match item {
            ViewItem::Attr { key, value } => {
                let k = key.to_string();
                match k.as_str() {
                    "divider" => {
                        if divider.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `divider`"));
                        }
                        divider = Some(value);
                    }
                    "spacing" => {
                        if spacing.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `spacing`"));
                        }
                        spacing = Some(value);
                    }
                    _ => {
                        let known = ["divider", "spacing"];
                        let mut msg = format!("unknown separator attribute `{k}`");
                        if let Some(s) = did_you_mean(&k, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(key, msg));
                    }
                }
            }
            ViewItem::Element { name, .. } => {
                return Err(syn::Error::new_spanned(
                    name,
                    format!("separator cannot contain element `{name}`"),
                ));
            }
            ViewItem::Splice { span, .. } => {
                return Err(splice_not_allowed("`separator`", span));
            }
            ViewItem::Bare(lit) => {
                return Err(syn::Error::new_spanned(
                    lit,
                    "bare string not allowed in `separator`",
                ));
            }
        }
    }
    let mut tokens = quote! { ::pwr_ext::view_support::CreateSeparator::new() };
    if let Some(v) = divider {
        tokens = quote! { #tokens .divider(#v) };
    }
    if let Some(v) = spacing {
        // spacing may be 1, 2, or SeparatorSpacingSize::Small etc. We need to handle integer literals.
        let v_str = v.to_token_stream_string();
        let spacing_tokens = match v_str.as_str() {
            "1" => quote! { ::pwr_ext::view_support::SeparatorSpacingSize::Small },
            "2" => quote! { ::pwr_ext::view_support::SeparatorSpacingSize::Large },
            _ => quote! { #v },
        };
        tokens = quote! { #tokens .spacing(#spacing_tokens) };
    }
    Ok(tokens)
}

fn expand_container(body: &ViewBody) -> syn::Result<proc_macro2::TokenStream> {
    let mut accent_color: Option<&Expr> = None;
    let mut accent_colour: Option<&Expr> = None;
    let mut spoiler: Option<&Expr> = None;
    let mut components: Vec<VecStep> = Vec::new();

    for item in &body.items {
        match item {
            ViewItem::Attr { key, value } => {
                let k = key.to_string();
                match k.as_str() {
                    "accent_color" => {
                        if accent_color.is_some() || accent_colour.is_some() {
                            return Err(syn::Error::new_spanned(
                                key,
                                "duplicate accent_color/accent_colour",
                            ));
                        }
                        accent_color = Some(value);
                    }
                    "accent_colour" => {
                        if accent_color.is_some() || accent_colour.is_some() {
                            return Err(syn::Error::new_spanned(
                                key,
                                "duplicate accent_color/accent_colour",
                            ));
                        }
                        accent_colour = Some(value);
                    }
                    "spoiler" => {
                        if spoiler.is_some() {
                            return Err(syn::Error::new_spanned(key, "duplicate `spoiler`"));
                        }
                        spoiler = Some(value);
                    }
                    _ => {
                        let known = ["accent_color", "accent_colour", "spoiler"];
                        let mut msg = format!("unknown container attribute `{k}`");
                        if let Some(s) = did_you_mean(&k, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(key, msg));
                    }
                }
            }
            ViewItem::Element { name, body } => {
                let n = name.to_string();
                let comp = match n.as_str() {
                    "action_row" => {
                        let ar = expand_action_row(body)?;
                        quote! { ::pwr_ext::view_support::CreateContainerComponent::ActionRow(#ar) }
                    }
                    "section" => {
                        let s = expand_section(body)?;
                        quote! { ::pwr_ext::view_support::CreateContainerComponent::Section(#s) }
                    }
                    "text_display" => {
                        let td = expand_text_display(body)?;
                        quote! { ::pwr_ext::view_support::CreateContainerComponent::TextDisplay(#td) }
                    }
                    "media_gallery" => {
                        let mg = expand_media_gallery(body)?;
                        quote! { ::pwr_ext::view_support::CreateContainerComponent::MediaGallery(#mg) }
                    }
                    "file" => {
                        let f = expand_file(body)?;
                        quote! { ::pwr_ext::view_support::CreateContainerComponent::File(#f) }
                    }
                    "separator" => {
                        let s = expand_separator(body)?;
                        quote! { ::pwr_ext::view_support::CreateContainerComponent::Separator(#s) }
                    }
                    "container" => {
                        return Err(syn::Error::new_spanned(
                            name,
                            "`container` cannot be nested inside another `container`",
                        ));
                    }
                    _ => {
                        let known = [
                            "action_row",
                            "section",
                            "text_display",
                            "media_gallery",
                            "file",
                            "separator",
                        ];
                        let mut msg = format!("unknown container child `{n}`");
                        if let Some(s) = did_you_mean(&n, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(name, msg));
                    }
                };
                components.push(VecStep::Push(comp));
            }
            ViewItem::Splice { expr, .. } => {
                components.push(VecStep::Extend(expr.clone()));
            }
            ViewItem::Bare(lit) => {
                return Err(syn::Error::new_spanned(
                    lit,
                    "bare string not allowed directly in `container` — use `text_display { \"...\" }`",
                ));
            }
        }
    }

    let accent = accent_color.or(accent_colour);
    let children = build_child_list(
        "__view_children",
        quote! { ::pwr_ext::view_support::CreateContainerComponent<'static> },
        &components,
        Some(&quote! { ::pwr_ext::view_support::check_container_children }),
        None,
        None,
        None,
        None,
    );
    let mut tokens = quote! { ::pwr_ext::view_support::CreateContainer::new(#children) };
    if let Some(v) = accent {
        tokens = quote! { #tokens .accent_colour(#v) };
    }
    if let Some(v) = spoiler {
        tokens = quote! { #tokens .spoiler(#v) };
    }
    Ok(tokens)
}

fn expand_section(body: &ViewBody) -> syn::Result<proc_macro2::TokenStream> {
    let mut text_displays: Vec<VecStep> = Vec::new();
    let mut accessory: Option<proc_macro2::TokenStream> = None;
    // Decide once, over ALL items, whether the section contains any splice.
    // The count guard and the compile-vs-runtime branch share this boolean.
    let has_any_splice = any_splice(&body.items);

    for item in &body.items {
        match item {
            ViewItem::Element { name, body } => {
                let n = name.to_string();
                match n.as_str() {
                    "text_display" => {
                        if !has_any_splice && text_displays.len() >= 3 {
                            return Err(syn::Error::new_spanned(
                                name,
                                "section cannot contain more than 3 text_display components",
                            ));
                        }
                        if accessory.is_some() {
                            return Err(syn::Error::new_spanned(
                                name,
                                "`text_display` must appear before accessory in `section`",
                            ));
                        }
                        let td = expand_text_display(body)?;
                        text_displays.push(VecStep::Push(
                            quote! { ::pwr_ext::view_support::CreateSectionComponent::TextDisplay(#td) },
                        ));
                    }
                    "thumbnail" => {
                        if accessory.is_some() {
                            return Err(syn::Error::new_spanned(
                                name,
                                "section can have only one accessory",
                            ));
                        }
                        let thumb = expand_thumbnail(body)?;
                        accessory = Some(
                            quote! { ::pwr_ext::view_support::CreateSectionAccessory::Thumbnail(#thumb) },
                        );
                    }
                    "button" => {
                        if accessory.is_some() {
                            return Err(syn::Error::new_spanned(
                                name,
                                "section can have only one accessory",
                            ));
                        }
                        let btn = expand_button(body)?;
                        accessory = Some(
                            quote! { ::pwr_ext::view_support::CreateSectionAccessory::Button(#btn) },
                        );
                    }
                    _ => {
                        let known = ["text_display", "thumbnail", "button"];
                        let mut msg = format!("unknown section child `{n}`");
                        if let Some(s) = did_you_mean(&n, &known) {
                            msg.push_str(&format!(" — did you mean `{s}`?"));
                        }
                        return Err(syn::Error::new_spanned(name, msg));
                    }
                }
            }
            ViewItem::Attr { key, .. } => {
                return Err(syn::Error::new_spanned(
                    key,
                    format!("section cannot contain attribute `{key}`"),
                ));
            }
            ViewItem::Splice { expr, .. } => {
                // Section splices contribute text displays; the accessory
                // position stays literal-only (exactly-one is not a list).
                text_displays.push(VecStep::Extend(expr.clone()));
            }
            ViewItem::Bare(lit) => {
                return Err(syn::Error::new_spanned(
                    lit,
                    "bare string not allowed in `section` — use `text_display { \"...\" }`",
                ));
            }
        }
    }

    if !has_any_splice && text_displays.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "section must contain at least one `text_display`",
        ));
    }
    let acc = accessory.ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "section requires exactly one accessory (`thumbnail` or `button`)",
        )
    })?;

    if has_any_splice {
        // Bind the accessory to a local: the check borrows it, then the
        // builder takes it by value.
        Ok(build_child_list(
            "__view_children",
            quote! { ::pwr_ext::view_support::CreateSectionComponent<'static> },
            &text_displays,
            Some(&quote! { ::pwr_ext::view_support::check_section_children }),
            Some(quote! { let __view_accessory = #acc; }),
            Some(quote! { ::std::option::Option::Some(&__view_accessory) }),
            Some(
                quote! { ::pwr_ext::view_support::CreateSection::new(__view_children, __view_accessory) },
            ),
            None,
        ))
    } else {
        let lits = literal_steps(&text_displays);
        Ok(quote! { ::pwr_ext::view_support::CreateSection::new(vec![#(#lits),*], #acc) })
    }
}

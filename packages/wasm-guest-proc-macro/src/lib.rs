use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    fold::{fold_block, Fold},
    parse::Parse,
    parse_macro_input, parse_quote,
    spanned::Spanned,
    Error, Expr, ExprIf, Ident, ItemFn, Result,
};

#[proc_macro_attribute]
pub fn workflow(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    impl_workflow(parse_macro_input!(item))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn impl_workflow(
    ItemFn {
        attrs,
        vis,
        sig,
        block,
    }: ItemFn,
) -> Result<TokenStream> {
    let generics = &sig.generics;
    let inputs = &sig.inputs;

    fold_errors([
        ensure_free_function(&sig),
        ensure_async(&sig),
        ensure_no_parameters(generics, generics.const_params(), "`const`"),
        ensure_no_parameters(generics, generics.lifetimes(), "lifetime"),
        ensure_no_parameters(generics, generics.type_params(), "type"),
        ensure_no_parameters(inputs, inputs.iter(), "runtime"),
    ])?;

    let ident = &sig.ident;
    let name = &ident.to_string();
    let block = fold_block(&mut Instrument, *block);

    Ok(quote! {
        #(#attrs)*
        #[inline(never)]
        #vis #sig {
            let __codastraea_trace = ::codastraea_wasm_guest::TraceFn::new(
                ::std::module_path!(),
                #name
            );

            #block
        }

        ::codastraea_wasm_guest::inventory::submit!(
            {
                fn set_main_fn() {
                    ::codastraea_wasm_guest::set_main_fn(#ident());
                }

                ::codastraea_wasm_guest::Workflow::new(
                    ::std::module_path!(),
                    #name,
                    set_main_fn
                )
            }
        );
    })
}

fn ensure_async(sig: &syn::Signature) -> Result<()> {
    sig.asyncness
        .ok_or(Error::new_spanned(
            sig,
            "`workflow` functions should be async",
        ))
        .map(|_| ())
}

/// Check this is a free function.
///
/// **Note**: From the perspective of proc macros, there's no way to distinguish
/// a free function and a function in an `impl` block without a `self` parameter
fn ensure_free_function(sig: &syn::Signature) -> Result<()> {
    let receiver = &sig.receiver();

    if receiver.is_some() {
        Err(Error::new_spanned(
            receiver,
            "`workflow` functions should be free functions, not methods",
        ))?;
    }

    Ok(())
}

fn ensure_no_parameters(item: impl ToTokens, mut iter: impl Iterator, name: &str) -> Result<()> {
    if iter.next().is_some() {
        Err(Error::new_spanned(
            item,
            format!("`workflow` functions should have no {name} parameters",),
        ))?;
    }

    Ok(())
}

fn fold_errors(errors: impl IntoIterator<Item = Result<()>>) -> Result<()> {
    #[allow(clippy::manual_try_fold)]
    errors
        .into_iter()
        .fold(Ok(()), |left, right| match (left, right) {
            (Err(e1), Err(mut e2)) => {
                e2.combine(e1);
                Err(e2)
            }
            (r1, r2) => r2.and(r1),
        })
}

struct Instrument;

impl Fold for Instrument {
    fn fold_expr_if(&mut self, expr_if: ExprIf) -> ExprIf {
        self.fold_expr_if_branch("if_condition", expr_if)
    }
}

impl Instrument {
    fn fold_expr_if_branch(&mut self, condition_name: &str, expr_if: ExprIf) -> ExprIf {
        let ExprIf {
            attrs,
            if_token,
            cond,
            then_branch,
            else_branch,
        } = expr_if;

        let cond = self.fold_expr(*cond);
        let then_branch = self.fold_block(then_branch);
        let else_branch = else_branch.map(|(else_token, else_expr)| {
            let else_expr = match *else_expr {
                Expr::If(if_expr) => {
                    // This will instrument all child nodes
                    Expr::If(self.fold_expr_if_branch("else_if_condition", if_expr))
                }
                expr => {
                    // We need to instrument child nodes, then trace
                    let expr = self.fold_expr(expr);
                    Self::traced("else", expr)
                }
            };

            (else_token, Box::new(else_expr))
        });

        ExprIf {
            attrs,
            if_token,
            cond: Box::new(Self::traced(condition_name, cond)),
            then_branch: Self::traced("then", then_branch),
            else_branch,
        }
    }

    fn traced<T: Spanned + ToTokens + Parse>(trace_type: &str, item: T) -> T {
        let begin = Self::begin_ident(trace_type, &item);
        let end = Self::end_ident(trace_type, &item);

        parse_quote! {
            {
                #[cfg(target_family = "wasm")]
                extern "C" {
                    fn #begin();
                    fn #end();
                }

                #[cfg(not(target_family = "wasm"))]
                unsafe extern "C" fn #begin() {}

                #[cfg(not(target_family = "wasm"))]
                unsafe extern "C" fn #end() {}

                let __codastraea_trace = ::codastraea_wasm_guest::Trace::new(
                    #begin,
                    #end
                );

                (#item)
            }
        }
    }

    fn begin_ident(name: &str, item: &impl Spanned) -> Ident {
        Ident::new(&format!("__codastraea_begin_{name}"), item.span())
    }

    fn end_ident(name: &str, item: &impl Spanned) -> Ident {
        Ident::new(&format!("__codastraea_end_{name}"), item.span())
    }
}

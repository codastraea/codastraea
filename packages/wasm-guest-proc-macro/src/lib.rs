use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    fold::{fold_block, Fold},
    parse_macro_input, parse_quote, Error, ExprIf, Generics, Ident, ItemFn, Result,
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
    let ident = &sig.ident;
    let name = &ident.to_string();
    let exported_ident = Ident::new(&format!("__enhedron_ident_{name}"), Span::call_site());
    let block = fold_block(&mut Instrument, *block);
    let generics = &sig.generics;

    fold_errors([
        check_empty(generics, generics.const_params(), "`const`"),
        check_empty(generics, generics.lifetimes(), "lifetime"),
        check_empty(generics, generics.type_params(), "type"),
    ])?;

    Ok(quote! {
        #(#attrs)*
        #vis #sig {
            let __enhedron_trace = ::serpent_automation_wasm_guest::TraceFn::new(
                ::std::module_path!(),
                #name
            );

            #block
        }

        extern "C" fn #exported_ident() {
            ::serpent_automation_wasm_guest::set_fn(#ident());
        }

        ::serpent_automation_wasm_guest::inventory::submit!(
            ::serpent_automation_wasm_guest::Workflow::new(
                ::std::module_path!(),
                #name,
                #exported_ident
            )
        );
    })
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

fn check_empty<T>(
    generics: &Generics,
    mut iter: impl Iterator<Item = T>,
    name: &str,
) -> Result<()> {
    if iter.next().is_some() {
        Err(Error::new_spanned(
            generics,
            format!("There should be no {name} parameters",),
        ))
    } else {
        Ok(())
    }
}

struct Instrument;

impl Fold for Instrument {
    fn fold_expr_if(&mut self, i: ExprIf) -> ExprIf {
        let ExprIf {
            attrs,
            if_token,
            cond,
            then_branch,
            else_branch,
        } = i;

        let cond = parse_quote! {
            {
                extern "C" {
                    // TODO: Embed the path in these names
                    fn __enhedron_condition_begin();
                    fn __enhedron_condition_end();
                }

                let __enhedron_trace = ::serpent_automation_wasm_guest::Trace::new(
                    __enhedron_condition_begin,
                    __enhedron_condition_end
                );

                (#cond)
            }
        };

        ExprIf {
            attrs,
            if_token,
            cond,
            then_branch,
            else_branch,
        }
    }
}

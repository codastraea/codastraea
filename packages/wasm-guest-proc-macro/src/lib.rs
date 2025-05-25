use proc_macro::TokenStream;
use quote::quote;
use syn::{
    fold::{fold_block, Fold},
    parse_macro_input, parse_quote, ExprIf, ItemFn,
};

#[proc_macro_attribute]
pub fn workflow(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = parse_macro_input!(item);
    let name = &sig.ident.to_string();
    let block = fold_block(&mut Instrument, *block);

    quote! {
        #(#attrs)*
        #vis #sig {
            let __enhedron_trace = ::serpent_automation_wasm_guest::TraceFn::new(::std::module_path!(), &#name);

            #block
        }
    }
    .into()
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

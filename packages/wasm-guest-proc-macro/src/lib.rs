use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    fold::{fold_block, Fold},
    parse::Parse,
    parse_macro_input, parse_quote,
    spanned::Spanned,
    Error, Expr, ExprIf, Generics, Ident, ItemFn, Result,
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
        self.fold_expr_if_branch("if_condition", i)
    }
}

impl Instrument {
    fn fold_expr_if_branch(&mut self, condition_name: &str, i: ExprIf) -> ExprIf {
        let ExprIf {
            attrs,
            if_token,
            cond,
            then_branch,
            else_branch,
        } = i;

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
                    self.traced("else", expr)
                }
            };

            (else_token, Box::new(else_expr))
        });

        ExprIf {
            attrs,
            if_token,
            cond: Box::new(self.traced(condition_name, cond)),
            then_branch: self.traced("then", then_branch),
            else_branch,
        }
    }

    fn traced<T: Spanned + ToTokens + Parse>(&self, trace_type: &str, item: T) -> T {
        let begin = self.begin_ident(trace_type, &item);
        let end = self.end_ident(trace_type, &item);

        parse_quote! {
            {
                extern "C" {
                    // TODO: Embed the path in these names
                    fn #begin();
                    fn #end();
                }

                let __enhedron_trace = ::serpent_automation_wasm_guest::Trace::new(
                    #begin,
                    #end
                );

                (#item)
            }
        }
    }

    fn begin_ident(&self, name: &str, item: &impl Spanned) -> Ident {
        Ident::new(&format!("__enhedron_begin_{name}"), item.span())
    }

    fn end_ident(&self, name: &str, item: &impl Spanned) -> Ident {
        Ident::new(&format!("__enhedron_end_{name}"), item.span())
    }
}

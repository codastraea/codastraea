use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn workflow(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = parse_macro_input!(item);
    let name = &sig.ident.to_string();

    quote! {
        #(#attrs)*
        #vis #sig {
            let __enhedron_trace = ::serpent_automation_wasm_guest::TraceFn::new(::std::module_path!(), &#name);
            #block
        }
    }
    .into()
}

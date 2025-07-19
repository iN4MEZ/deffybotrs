extern crate proc_macro;

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, Ident, Token};
use syn::parse::{Parse, ParseStream};

struct EventFnArgs {
    e_expr: Ident,
}

impl Parse for EventFnArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        if ident != "e" {
            return Err(syn::Error::new(ident.span(), "expected `e = ...`"));
        }
        let value: Ident = input.parse()?;
        Ok(EventFnArgs { e_expr: value })
    }
}

#[proc_macro_attribute]
pub fn event(attr: TokenStream, item: TokenStream) -> TokenStream {
    let EventFnArgs { e_expr } = parse_macro_input!(attr as EventFnArgs);
    let func = parse_macro_input!(item as ItemFn);
    let fn_name = &func.sig.ident;
    let fn_args = &func.sig.inputs;
    let fn_body = &func.block;
    let fn_vis = &func.vis;

    let registry_struct = quote::format_ident!("EVENT_HOOK{}_",fn_name.to_string().to_case(Case::UpperCamel));

    let expanded = quote! {
        #fn_vis async fn #fn_name(#fn_args) #fn_body

        struct #registry_struct;

        #[serenity::async_trait]
        impl crate::event::event_registry::Hookable for #registry_struct {
            async fn call(&self, event: &str, ctx: serenity::prelude::Context, data: Arc<Mutex<Box<dyn Any + Send>>>) {
                if event == stringify!(#e_expr) {
                    #fn_name(ctx, data).await;

                }
            }

            fn event_type(&self) -> &'static str {
                stringify!(#e_expr)
            }
        }

        inventory::submit! {
            &#registry_struct as &dyn crate::event::event_registry::Hookable
        }
    };

    TokenStream::from(expanded)
}

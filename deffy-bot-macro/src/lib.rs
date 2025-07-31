extern crate proc_macro;

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Ident, ItemFn, ItemStruct, Token};
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
        impl crate::event::manager::Hookable for #registry_struct {
            async fn call(&self, event: &str, ctx: serenity::prelude::Context, data: crate::event::manager::EventData) {
                if event == stringify!(#e_expr) {

                    #fn_name(ctx, data).await;

                }
            }

            // fn event_type(&self) -> &'static str {
            //     stringify!(#e_expr)
            // }
        }

        inventory::submit! {
            &#registry_struct as &dyn crate::event::manager::Hookable
        }
    };

    TokenStream::from(expanded)
}

/// Parse `cmd = test`
struct CommandAttrArgs {
    cmd_ident: Ident,
}

impl Parse for CommandAttrArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;         // should be `cmd`
        input.parse::<Token![=]>()?;
        if key != "cmd" {
            return Err(syn::Error::new(key.span(), "expected `cmd = ...`"));
        }
        let value: Ident = input.parse()?;       // actual command name
        Ok(CommandAttrArgs { cmd_ident: value })
    }
}


#[proc_macro_attribute]
pub fn command(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as CommandAttrArgs);
    let input = parse_macro_input!(item as ItemStruct);

    let cmd_name = args.cmd_ident.to_string();
    let struct_name = &input.ident;

    let expanded = quote! {
        #input
    
        impl crate::command::system::manager::CommandInfo for #struct_name {
            fn name(&self) -> &'static str {
                #cmd_name
            }
        }
    
        inventory::submit! {
            crate::command::system::manager::CommandRegistration {
                constructor: || std::sync::Arc::new(#struct_name),
            }
        }
    };

    TokenStream::from(expanded)
}
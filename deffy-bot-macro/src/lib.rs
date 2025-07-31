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

/// Parse `cmd = test, cooldown`
struct CommandAttrArgs {
    cmd_ident: Ident,
    cooldown: syn::LitInt,
}

impl Parse for CommandAttrArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut cmd_ident = None;
        let mut cooldown = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match key.to_string().as_str() {
                "cmd" => {
                    if cmd_ident.is_some() {
                        return Err(syn::Error::new(key.span(), "duplicate `cmd`"));
                    }
                    cmd_ident = Some(input.parse()?);
                }
                "cooldown" => {
                    if cooldown.is_some() {
                        return Err(syn::Error::new(key.span(), "duplicate `cooldown`"));
                    }
                    cooldown = Some(input.parse()?);
                }
                _ => {
                    return Err(syn::Error::new(key.span(), "unexpected key, expected `cmd` or `cooldown`"));
                }
            }

            // Try to parse a comma if there's more input
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(CommandAttrArgs {
            cmd_ident: cmd_ident.ok_or_else(|| syn::Error::new(input.span(), "`cmd` is required"))?,
            cooldown: cooldown.ok_or_else(|| syn::Error::new(input.span(), "`cooldown` is required"))?,
        })
    }
}


#[proc_macro_attribute]
pub fn command(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as CommandAttrArgs);
    let input = parse_macro_input!(item as ItemStruct);

    let cmd_name = args.cmd_ident.to_string();
    let cooldown_value = args.cooldown.base10_parse::<u64>().unwrap_or(0); // แปลงเป็น u64

    let struct_name = &input.ident;

    let expanded = quote! {
        #input

        impl crate::command::system::manager::CommandInfo for #struct_name {
            fn name(&self) -> &'static str {
                #cmd_name
            }

            fn cooldown(&self) -> u64 {
                #cooldown_value
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
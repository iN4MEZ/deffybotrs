extern crate proc_macro;

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemFn, ItemStruct, LitStr, Token, parse_macro_input};

struct EventFnArgs {
    e_expr: Ident,
}

impl Parse for EventFnArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        if ident != "e" {
            return Err(syn::Error::new(ident.span(), "expected e = ..."));
        }
        let value: Ident = input.parse()?;
        Ok(EventFnArgs { e_expr: value })
    }
}

struct EventFnArgsRoute {
    e_expr: Option<Ident>,
    route: Option<LitStr>,
}

impl Parse for EventFnArgsRoute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut e_expr = None;
        let mut route = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            if key == "e" {
                let value: Ident = input.parse()?;
                e_expr = Some(value);
            } else if key == "route" {
                let value: LitStr = input.parse()?;
                route = Some(value);
            } else {
                return Err(syn::Error::new(
                    key.span(),
                    format!("unexpected argument `{}`", key),
                ));
            }

            // optional comma
            let _ = input.parse::<Token![,]>();
        }

        Ok(EventFnArgsRoute { e_expr, route })
    }
}

#[proc_macro_attribute]
pub fn event(attr: TokenStream, item: TokenStream) -> TokenStream {
    let EventFnArgsRoute { e_expr, route } = parse_macro_input!(attr as EventFnArgsRoute);
    let func = parse_macro_input!(item as ItemFn);
    let fn_name = &func.sig.ident;
    let fn_args = &func.sig.inputs;
    let fn_body = &func.block;
    let fn_vis = &func.vis;
    let route_token = if let Some(route_str) = &route {
        quote! { Some(#route_str) }
    } else {
        quote! { None }
    };

    let registry_struct = quote::format_ident!(
        "EVENT_HOOK{}_",
        fn_name.to_string().to_case(Case::UpperCamel)
    );

    let expanded = quote! {
        #fn_vis async fn #fn_name(#fn_args) -> Result<(), anyhow::Error> #fn_body

        struct #registry_struct;

        #[serenity::async_trait]
        impl crate::event::manager::Hookable for #registry_struct {
            async fn call(
                &self,
                event: &str,
                ctx: serenity::prelude::Context,
                data: crate::event::manager::EventData
            ) -> Result<(), anyhow::Error> {
                if event == stringify!(#e_expr) {
                    #fn_name(ctx, data).await?;
                }
                Ok(())
            }

            fn route(&self) -> Option<&'static str> {
                #route_token
            }
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
                    return Err(syn::Error::new(
                        key.span(),
                        "unexpected key, expected `cmd` or `cooldown`",
                    ));
                }
            }

            // Try to parse a comma if there's more input
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(CommandAttrArgs {
            cmd_ident: cmd_ident
                .ok_or_else(|| syn::Error::new(input.span(), "`cmd` is required"))?,
            cooldown: cooldown
                .ok_or_else(|| syn::Error::new(input.span(), "`cooldown` is required"))?,
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

/// สร้าง macro attribute ใหม่ `#[event(e = ...)]`
#[proc_macro_attribute]
pub fn event_handle(attr: TokenStream, item: TokenStream) -> TokenStream {
    let EventFnArgs { e_expr } = parse_macro_input!(attr as EventFnArgs);
    let func = parse_macro_input!(item as ItemFn);
    let func_name = &func.sig.ident;

    // สร้างชื่อ struct จากชื่อฟังก์ชัน
    let struct_name = Ident::new(
        &format!(
            "EventHandler{}",
            func_name.to_string().to_case(Case::Pascal)
        ),
        func_name.span(),
    );

    // สร้างชื่อ static instance
    let static_name = Ident::new(
        &format!(
            "EVENT_HANDLER_{}",
            func_name.to_string().to_case(Case::Snake).to_uppercase()
        ),
        func_name.span(),
    );

    let expanded = quote! {
        #func

        struct #struct_name;

        #[serenity::async_trait]
        impl deffy_bot_utils::event::manager::UtilsEventHandler for #struct_name {
            async fn handle(&self, data: deffy_bot_utils::event::manager::EventTypeData) -> Result<(), anyhow::Error> {
                #func_name(data).await.map_err(|e| e.into())
            }
        }

        impl EventInfo for #struct_name {
            fn event_type(&self) -> deffy_bot_utils::event::manager::EventType {
                #e_expr
            }

            fn boxed(&self) -> std::sync::Arc<dyn deffy_bot_utils::event::manager::UtilsEventHandler> {
                std::sync::Arc::new(Self)
            }
        }

        // ลงทะเบียนไว้ใน inventory
        static #static_name: #struct_name = #struct_name;

        inventory::submit! {
            &#static_name as &dyn deffy_bot_utils::event::manager::EventInfo
        }
    };

    TokenStream::from(expanded)
}

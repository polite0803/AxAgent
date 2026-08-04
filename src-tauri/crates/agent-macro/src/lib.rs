// SPDX-License-Identifier: AGPL-3.0-only

//! Agent 命令宏 - 为 Tauri 命令添加元数据
//!
//! 自动生成 `full_path`（通过 `module_path!()` + 函数名拼接），无需手动指定。
//!
//! # 使用方式
//!
//! ```rust,ignore
//! #[tauri::command]
//! #[agent_command(
//!     domain = provider,
//!     safety = Safe,
//!     call_mode = StateOnly,
//!     description = "列出所有可用的 LLM 提供商"
//! )]
//! pub async fn list_providers(state: State<'_, AppState>) -> Result<Vec<ProviderConfig>, String> {
//!     // 命令实现
//! }
//! ```

use proc_macro::TokenStream;
use quote::quote;
use syn::{Token, parse_macro_input};

/// 命令元数据属性宏
#[proc_macro_attribute]
pub fn agent_command(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as AgentCommandArgs);
    let cmd = parse_macro_input!(item as syn::ItemFn);
    let cmd_name = &cmd.sig.ident;

    let safety = &args.safety;
    let call_mode = &args.call_mode;
    let description = &args.description;

    // 使用命令名生成唯一的常量名
    let meta_const_name = syn::Ident::new(
        &format!("__AGENT_META_{}", cmd_name.to_string().to_uppercase()),
        cmd_name.span(),
    );

    let domain_lit = syn::LitStr::new(&args.domain, proc_macro2::Span::call_site());

    let expanded = quote! {
        // 保留原始命令实现
        #cmd

        // 命令元数据常量（唯一命名避免冲突）
        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        pub const #meta_const_name: agent_command_types::CommandMetadata =
            agent_command_types::CommandMetadata::new(
                stringify!(#cmd_name),
                module_path!(),
                #domain_lit,
                agent_command_types::CommandSafety::#safety,
                agent_command_types::CallMode::#call_mode,
                #description,
            );

        // 使用 inventory 注册命令元数据
        inventory::submit!(#meta_const_name);
    };

    expanded.into()
}

/// 命令参数解析
struct AgentCommandArgs {
    domain: String,
    safety: syn::Ident,
    call_mode: syn::Ident,
    description: syn::LitStr,
}

impl syn::parse::Parse for AgentCommandArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut domain = None;
        let mut safety = None;
        let mut call_mode = None;
        let mut description = None;

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match key.to_string().as_str() {
                "domain" => {
                    // 支持字符串字面量 "agent" 和标识符 agent 两种形式
                    let value: syn::Expr = input.parse()?;
                    let domain_str = match &value {
                        syn::Expr::Lit(lit) => {
                            if let syn::Lit::Str(s) = &lit.lit {
                                s.value()
                            } else {
                                return Err(syn::Error::new(
                                    proc_macro2::Span::call_site(),
                                    "domain 必须是字符串字面量或标识符",
                                ));
                            }
                        },
                        syn::Expr::Path(path) => {
                            path.path
                                .get_ident()
                                .map(|id| id.to_string())
                                .ok_or_else(|| {
                                    syn::Error::new(
                                        proc_macro2::Span::call_site(),
                                        "domain 必须是字符串字面量或标识符",
                                    )
                                })?
                        },
                        _ => {
                            return Err(syn::Error::new(
                                proc_macro2::Span::call_site(),
                                "domain 必须是字符串字面量或标识符",
                            ));
                        },
                    };
                    domain = Some(domain_str);
                },
                "safety" => {
                    let value: syn::Ident = input.parse()?;
                    let s = value.to_string();
                    match s.as_str() {
                        "Safe" | "Caution" | "Dangerous" => {},
                        _ => {
                            return Err(syn::Error::new(
                                value.span(),
                                format!(
                                    "无效的安全级别: '{}'。有效值: Safe, Caution, Dangerous",
                                    s
                                ),
                            ));
                        },
                    }
                    safety = Some(value);
                },
                "call_mode" => {
                    let value: syn::Ident = input.parse()?;
                    let s = value.to_string();
                    match s.as_str() {
                        "StateOnly" | "StateInput" | "Manual" => {},
                        _ => {
                            return Err(syn::Error::new(
                                value.span(),
                                format!(
                                    "无效的调用模式: '{}'。有效值: StateOnly, StateInput, Manual",
                                    s
                                ),
                            ));
                        },
                    }
                    call_mode = Some(value);
                },
                "description" => {
                    let value: syn::LitStr = input.parse()?;
                    description = Some(value);
                },
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("未知的命令属性: '{}'", other),
                    ));
                },
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        let domain = domain
            .ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "缺少 'domain' 参数"))?;
        let safety = safety
            .ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "缺少 'safety' 参数"))?;
        let call_mode = call_mode.ok_or_else(|| {
            syn::Error::new(proc_macro2::Span::call_site(), "缺少 'call_mode' 参数")
        })?;
        let description =
            description.unwrap_or_else(|| syn::LitStr::new("", proc_macro2::Span::call_site()));

        Ok(AgentCommandArgs { domain, safety, call_mode, description })
    }
}

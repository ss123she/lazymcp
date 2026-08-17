use proc_macro::TokenStream;
use syn::{ExprLit, ItemFn, Lit, Meta::NameValue, Type, parse_macro_input};

#[proc_macro_attribute]
pub fn tool(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item_fn = parse_macro_input!(item as ItemFn);

    let name_fn = &item_fn.sig.ident;
    let comments_fn = extract_docs(&item_fn.attrs);
    let description_fn = (!comments_fn.is_empty()).then(|| comments_fn.join("\n"));

    let is_async = item_fn.sig.asyncness.is_some();
    let maybe_await = if is_async {
        quote::quote! { .await }
    } else {
        quote::quote! {}
    };

    let mut struct_fields = Vec::new();
    let mut state_extractions = Vec::new();
    let mut call_args = Vec::new();

    for arg in item_fn.sig.inputs.iter_mut() {
        if let syn::FnArg::Typed(pat_type) = arg {
            let ident = match &*pat_type.pat {
                syn::Pat::Ident(pat_ident) => &pat_ident.ident,
                pat => {
                    return syn::Error::new_spanned(
                        pat,
                        "lazymcp: destructuring patterns in tool arguments are not supported, use simple identifiers (e.g. `name: Type`)"
                    )
                    .to_compile_error()
                    .into();
                }
            };
            let ty = &pat_type.ty;

            if let Some(inner_ty) = extract_state_inner_type(ty) {
                state_extractions.push(quote::quote! {
                    let #ident = states
                        .get(&::std::any::TypeId::of::<#inner_ty>())
                        .cloned()
                        .and_then(|arc| arc.downcast::<#inner_ty>().ok())
                        .map(lazymcp::State)
                        .ok_or_else(|| lazymcp::McpError::InternalError(
                            format!("State of type `{}` is not registered", stringify!(#inner_ty))
                        ))?;
                });
                call_args.push(quote::quote! { #ident });
            } else {
                let all_attrs = std::mem::take(&mut pat_type.attrs);
                let (doc_attrs, other_attrs): (Vec<_>, Vec<_>) = all_attrs
                    .into_iter()
                    .partition(|attr| attr.path().is_ident("doc"));

                pat_type.attrs = other_attrs;

                struct_fields.push(quote::quote! {
                    #(#doc_attrs)*
                    pub #ident: #ty
                });

                call_args.push(quote::quote! { args.#ident });
            }
        }
    }

    let args_struct_name = quote::format_ident!("{}_args", name_fn);
    let tool_struct_name = quote::format_ident!("{}_tool", name_fn);

    let name_str = name_fn.to_string();
    let vis = &item_fn.vis;

    let doc_tokens = match &description_fn {
        Some(doc) => quote::quote! { Some(#doc) },
        None => quote::quote! { None },
    };

    let expanded = quote::quote! {
        #[derive(lazymcp::schemars::JsonSchema, lazymcp::serde::Deserialize)]
        #[allow(non_camel_case_types)]
        struct #args_struct_name {
            #(#struct_fields),*
        }

        #item_fn

        #[allow(non_camel_case_types)]
        #vis struct #tool_struct_name;

        impl lazymcp::McpTool for #tool_struct_name {
            fn name(&self) -> &'static str {
                #name_str
            }

            fn description(&self) -> Option<&'static str> {
                #doc_tokens
            }

            fn schema(&self) -> std::sync::Arc<lazymcp::rmcp::model::JsonObject> {
                let mut settings = lazymcp::schemars::generate::SchemaSettings::openapi3();
                settings.inline_subschemas = true;
                let generator = settings.into_generator();
                let schema = generator.into_root_schema_for::<#args_struct_name>();

                match lazymcp::serde_json::to_value(&schema).unwrap() {
                    lazymcp::serde_json::Value::Object(map) => std::sync::Arc::new(map),
                    _ => std::sync::Arc::new(lazymcp::serde_json::Map::new()),
                }
            }

            fn call<'a>(
                &'a self,
                arguments: lazymcp::serde_json::Value,
                states: &'a lazymcp::StateMap,
            ) -> ::std::pin::Pin<Box<dyn ::std::future::Future<Output = Result<lazymcp::CallToolResult, lazymcp::McpError>> + Send + 'a>> {
                Box::pin(async move {
                    let args: #args_struct_name = lazymcp::serde_json::from_value(arguments)
                        .map_err(|e| lazymcp::McpError::InvalidArguments(e.to_string()))?;

                    #(#state_extractions)*

                    let result = #name_fn(#(#call_args),*) #maybe_await;
                    Ok(lazymcp::IntoToolResult::into_tool_result(result))
                })
            }
        }
    };

    expanded.into()
}

fn extract_state_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "State" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        return Some(inner_ty);
                    }
                }
            }
        }
    }
    None
}

fn extract_docs(attrs: &[syn::Attribute]) -> Vec<String> {
    let mut docs = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("doc") {
            if let NameValue(nv) = &attr.meta {
                if let syn::Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = &nv.value
                {
                    docs.push(s.value().trim().to_string());
                }
            }
        }
    }
    docs
}

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, Expr, ExprArray, ExprLit, ExprPath, ExprRange, Fields, Ident,
    Lit, LitBool, LitFloat, LitInt, LitStr, Result, Token, Type,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

#[proc_macro_derive(
    GoldenNode,
    attributes(node_id, param, child, folder, container, potential_child)
)]
pub fn golden_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;

    let mut param_decls = Vec::new();
    let mut folder_decls = Vec::new();
    let mut declared_children = Vec::new();
    let mut potential_slots = Vec::new();
    let mut container_decl = None;

    for attr in &input.attrs {
        if attr.path().is_ident("container") {
            container_decl = Some(parse_container_attr(attr));
        }
    }

    let Data::Struct(data) = input.data else {
        return syn::Error::new_spanned(ident, "GoldenNode only supports structs")
            .to_compile_error()
            .into();
    };

    let Fields::Named(fields) = data.fields else {
        return syn::Error::new_spanned(ident, "GoldenNode requires named fields")
            .to_compile_error()
            .into();
    };

    for field in fields.named {
        let Some(field_ident) = field.ident else {
            continue;
        };

        if field
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("node_id"))
        {
            continue;
        }

        for attr in &field.attrs {
            if attr.path().is_ident("param") {
                match build_param_decl(&field_ident, &field.ty, attr) {
                    Ok((decl, child_decl, folder_decl)) => {
                        param_decls.push(decl);
                        if let Some(folder_decl) = folder_decl {
                            folder_decls.push(folder_decl);
                        }
                        declared_children.push(child_decl);
                    }
                    Err(err) => return err.to_compile_error().into(),
                }
            }

            if attr.path().is_ident("folder") {
                match build_folder_decl(attr) {
                    Ok((folder_decl, child_decl)) => {
                        folder_decls.push(folder_decl);
                        declared_children.push(child_decl);
                    }
                    Err(err) => return err.to_compile_error().into(),
                }
            }

            if attr.path().is_ident("potential_child") {
                match build_potential_slot(attr) {
                    Ok(slot_decl) => potential_slots.push(slot_decl),
                    Err(err) => return err.to_compile_error().into(),
                }
            }

            if attr.path().is_ident("child") {
                match build_child_decl(attr) {
                    Ok(child_decl) => declared_children.push(child_decl),
                    Err(err) => return err.to_compile_error().into(),
                }
            }
        }
    }

    let node_type = ident.to_string();
    let container_decl = container_decl.unwrap_or_else(|| quote! { None });
    let has_attr_schema = !(param_decls.is_empty()
        && folder_decls.is_empty()
        && declared_children.is_empty()
        && potential_slots.is_empty());

    let schema_tokens = if has_attr_schema {
        quote! {
            let mut schema = golden_core::schema::NodeSchema::new();
            schema.declared_children = vec![#(#declared_children),*];
            schema.potential_slots = vec![#(#potential_slots),*];
            schema.params = vec![#(#param_decls),*];
            schema.folders = vec![#(#folder_decls),*];
            schema.container = #container_decl;
            schema
        }
    } else {
        quote! {
            let mut schema = golden_core::schema::NodeSchema::new();
            schema.declared_children = Self::declared_children();
            schema.params = Self::param_decls();
            schema.folders = Self::folder_decls();
            schema.container = #container_decl;
            schema
        }
    };

    let expanded = quote! {
        impl golden_core::schema::GoldenNodeDecl for #ident {
            fn node_type() -> golden_schema::NodeTypeId {
                golden_schema::NodeTypeId(#node_type.to_string())
            }

            fn schema() -> golden_core::schema::NodeSchema {
                #schema_tokens
            }
        }
    };

    expanded.into()
}

#[proc_macro]
pub fn params(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ParamsInput);
    let mut param_decls = Vec::new();
    let mut folder_decls = Vec::new();
    let mut declared_children = Vec::new();

    if let Err(err) = validate_params_items(&input.items) {
        return err.to_compile_error().into();
    }

    for item in input.items {
        collect_params_from_item(
            item,
            &mut param_decls,
            &mut folder_decls,
            &mut declared_children,
            None,
            None,
        );
    }

    let expanded = quote! {
        #[allow(dead_code)]
        pub fn param_decls() -> Vec<golden_core::schema::ParamDecl> {
            vec![#(#param_decls),*]
        }

        #[allow(dead_code)]
        pub fn folder_decls() -> Vec<golden_core::schema::FolderDecl> {
            vec![#(#folder_decls),*]
        }

        #[allow(dead_code)]
        pub fn declared_children() -> Vec<golden_core::schema::DeclaredChild> {
            vec![#(#declared_children),*]
        }
    };

    expanded.into()
}

fn build_param_decl(
    field_ident: &Ident,
    field_ty: &Type,
    attr: &Attribute,
) -> Result<(
    proc_macro2::TokenStream,
    proc_macro2::TokenStream,
    Option<proc_macro2::TokenStream>,
)> {
    let args = parse_param_args(attr, Some(field_ident))?;
    let kind = extract_param_kind(field_ty)?;
    let decl_id = field_ident.to_string();
    let default_tokens = value_tokens_from_args(&kind, &args)?;
    let constraints_tokens = constraints_tokens_from_args(&kind, &args)?;
    let semantics_tokens = semantics_tokens(&args.semantics, &args.unit);
    let presentation_tokens = presentation_tokens(&args.presentation);
    let behavior_tokens = behavior_tokens(&args.behavior);
    let read_only = args.read_only;
    let update_tokens = update_policy_tokens(&args.update);
    let change_tokens = change_policy_tokens(&args.change);
    let save_tokens = save_policy_tokens(&args.save);
    let folder_tokens = args.folder.as_ref().map(|value| {
        let decl = value.value();
        quote! { Some(golden_schema::DeclId(#decl.to_string())) }
    });
    let folder_tokens = folder_tokens.unwrap_or_else(|| quote! { None });

    let alias_tokens = args.alias.as_ref().map(|value| {
        let alias = value.value();
        quote! { Some(#alias.to_string()) }
    });
    let alias_tokens = alias_tokens.unwrap_or_else(|| quote! { None });

    let folder_decl_tokens = args.folder.as_ref().map(|value| {
        let decl = value.value();
        quote! {
            golden_core::schema::FolderDecl {
                decl_id: golden_schema::DeclId(#decl.to_string()),
                label: None,
                alias_prefix: None,
            }
        }
    });

    let param_decl = quote! {
        golden_core::schema::ParamDecl {
            decl_id: golden_schema::DeclId(#decl_id.to_string()),
            default: #default_tokens,
            constraints: #constraints_tokens,
            read_only: #read_only,
            update: #update_tokens,
            change: #change_tokens,
            save: #save_tokens,
            semantics: #semantics_tokens,
            presentation: #presentation_tokens,
            folder: #folder_tokens,
            behavior: #behavior_tokens,
            alias: #alias_tokens,
        }
    };

    let child_decl = quote! {
        golden_core::schema::DeclaredChild {
            decl_id: golden_schema::DeclId(#decl_id.to_string()),
            node_type: golden_schema::NodeTypeId("Parameter".to_string()),
            default_label: Some(#decl_id.to_string()),
            default_enabled: true,
        }
    };

    Ok((param_decl, child_decl, folder_decl_tokens))
}

fn build_folder_decl(
    attr: &Attribute,
) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream)> {
    let mut slot = None::<LitStr>;
    let mut label = None::<LitStr>;
    let mut alias_prefix = None::<LitStr>;

    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("slot") {
            slot = Some(meta.value()?.parse()?);
            return Ok(());
        }
        if meta.path.is_ident("label") {
            label = Some(meta.value()?.parse()?);
            return Ok(());
        }
        if meta.path.is_ident("alias_prefix") {
            alias_prefix = Some(meta.value()?.parse()?);
            return Ok(());
        }
        Ok(())
    })?;

    let slot = slot.ok_or_else(|| syn::Error::new_spanned(attr, "folder slot is required"))?;
    let slot_value = slot.value();
    let label_tokens = label
        .as_ref()
        .map(|value| {
            let value = value.value();
            quote! { Some(#value.to_string()) }
        })
        .unwrap_or_else(|| quote! { None });
    let alias_tokens = alias_prefix
        .as_ref()
        .map(|value| {
            let value = value.value();
            quote! { Some(#value.to_string()) }
        })
        .unwrap_or_else(|| quote! { None });

    let folder_decl = quote! {
        golden_core::schema::FolderDecl {
            decl_id: golden_schema::DeclId(#slot_value.to_string()),
            label: #label_tokens,
            alias_prefix: #alias_tokens,
        }
    };

    let child_decl = quote! {
        golden_core::schema::DeclaredChild {
            decl_id: golden_schema::DeclId(#slot_value.to_string()),
            node_type: golden_schema::NodeTypeId("Folder".to_string()),
            default_label: Some(#slot_value.to_string()),
            default_enabled: true,
        }
    };

    Ok((folder_decl, child_decl))
}

fn build_potential_slot(attr: &Attribute) -> Result<proc_macro2::TokenStream> {
    let mut decl_id = None::<LitStr>;
    let mut allowed = Vec::<LitStr>::new();

    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("decl_id") {
            decl_id = Some(meta.value()?.parse()?);
            return Ok(());
        }
        if meta.path.is_ident("allowed") {
            let array: ExprArray = meta.value()?.parse()?;
            for expr in array.elems {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(value),
                    ..
                }) = expr
                {
                    allowed.push(value);
                }
            }
            return Ok(());
        }
        Ok(())
    })?;

    let decl_id = decl_id.ok_or_else(|| syn::Error::new_spanned(attr, "decl_id is required"))?;
    let decl_value = decl_id.value();
    let allowed_tokens = allowed.iter().map(|value| {
        let value = value.value();
        quote! { golden_schema::NodeTypeId(#value.to_string()) }
    });

    Ok(quote! {
        golden_core::schema::PotentialSlot {
            decl_id: golden_schema::DeclId(#decl_value.to_string()),
            allowed_types: vec![#(#allowed_tokens),*],
        }
    })
}

fn build_child_decl(attr: &Attribute) -> Result<proc_macro2::TokenStream> {
    let mut slot = None::<LitStr>;
    let mut allowed = None::<LitStr>;

    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("slot") {
            slot = Some(meta.value()?.parse()?);
            return Ok(());
        }
        if meta.path.is_ident("allowed") {
            allowed = Some(meta.value()?.parse()?);
            return Ok(());
        }
        Ok(())
    })?;

    let slot = slot.ok_or_else(|| syn::Error::new_spanned(attr, "slot is required"))?;
    let allowed = allowed.ok_or_else(|| syn::Error::new_spanned(attr, "allowed is required"))?;
    let slot_value = slot.value();
    let allowed_value = allowed.value();

    Ok(quote! {
        golden_core::schema::DeclaredChild {
            decl_id: golden_schema::DeclId(#slot_value.to_string()),
            node_type: golden_schema::NodeTypeId(#allowed_value.to_string()),
            default_label: Some(#slot_value.to_string()),
            default_enabled: true,
        }
    })
}

fn parse_container_attr(attr: &Attribute) -> proc_macro2::TokenStream {
    let mut allowed = Vec::<LitStr>::new();
    let mut folders = None::<LitStr>;

    let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("allowed") {
            if let Ok(array) = meta.value()?.parse::<ExprArray>() {
                for expr in array.elems {
                    if let Expr::Lit(ExprLit {
                        lit: Lit::Str(value),
                        ..
                    }) = expr
                    {
                        allowed.push(value);
                    }
                }
            }
            return Ok(());
        }
        if meta.path.is_ident("folders") {
            folders = Some(meta.value()?.parse()?);
            return Ok(());
        }
        Ok(())
    });

    let allowed_tokens = if allowed.is_empty() {
        quote! { golden_core::AllowedTypes::Any }
    } else {
        let tokens = allowed.iter().map(|value| {
            let value = value.value();
            quote! { golden_schema::NodeTypeId(#value.to_string()) }
        });
        quote! { golden_core::AllowedTypes::Only(vec![#(#tokens),*]) }
    };

    let folders_token = folders
        .as_ref()
        .map(|value| match value.value().as_str() {
            "Allowed" => quote! { golden_core::FolderPolicy::Allowed },
            "Forbidden" => quote! { golden_core::FolderPolicy::Forbidden },
            _ => quote! { golden_core::FolderPolicy::Allowed },
        })
        .unwrap_or_else(|| quote! { golden_core::FolderPolicy::Allowed });

    quote! {
        Some(golden_core::schema::ContainerDecl {
            allowed_types: #allowed_tokens,
            folders: #folders_token,
        })
    }
}

#[derive(Default)]
struct ParamArgs {
    default: Option<Expr>,
    min: Option<Expr>,
    max: Option<Expr>,
    step: Option<Expr>,
    clamp: Option<LitBool>,
    read_only: bool,
    save: Option<LitStr>,
    update: Option<LitStr>,
    change: Option<LitStr>,
    semantics: Option<LitStr>,
    unit: Option<LitStr>,
    presentation: Option<LitStr>,
    folder: Option<LitStr>,
    behavior: Option<LitStr>,
    alias: Option<LitStr>,
    enum_id: Option<LitStr>,
    allowed: Vec<LitStr>,
    target: Option<LitStr>,
    pattern: Option<LitStr>,
    max_len: Option<LitInt>,
}

fn parse_param_args(attr: &Attribute, field_ident: Option<&Ident>) -> Result<ParamArgs> {
    let mut args = ParamArgs::default();

    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("default") {
            args.default = Some(meta.value()?.parse()?);
            return Ok(());
        }
        if meta.path.is_ident("min") {
            args.min = Some(meta.value()?.parse()?);
            return Ok(());
        }
        if meta.path.is_ident("max") {
            args.max = Some(meta.value()?.parse()?);
            return Ok(());
        }
        if meta.path.is_ident("step") {
            args.step = Some(meta.value()?.parse()?);
            return Ok(());
        }
        if meta.path.is_ident("clamp") {
            args.clamp = Some(meta.value()?.parse()?);
            return Ok(());
        }
        if meta.path.is_ident("read_only") {
            let value: LitBool = meta.value()?.parse()?;
            args.read_only = value.value();
            return Ok(());
        }
        if meta.path.is_ident("save") {
            args.save = Some(meta.value()?.parse()?);
            return Ok(());
        }
        if meta.path.is_ident("update") {
            args.update = Some(meta.value()?.parse()?);
            return Ok(());
        }
        if meta.path.is_ident("change") {
            args.change = Some(meta.value()?.parse()?);
            return Ok(());
        }
        if meta.path.is_ident("semantics") || meta.path.is_ident("sem") {
            args.semantics = Some(meta.value()?.parse()?);
            return Ok(());
        }
        if meta.path.is_ident("unit") {
            args.unit = Some(meta.value()?.parse()?);
            return Ok(());
        }
        if meta.path.is_ident("presentation") {
            args.presentation = Some(meta.value()?.parse()?);
            return Ok(());
        }
        if meta.path.is_ident("folder") {
            args.folder = Some(meta.value()?.parse()?);
            return Ok(());
        }
        if meta.path.is_ident("behavior") {
            args.behavior = Some(meta.value()?.parse()?);
            return Ok(());
        }
        if meta.path.is_ident("alias") {
            args.alias = Some(meta.value()?.parse()?);
            return Ok(());
        }
        if meta.path.is_ident("direct_access") {
            if let Some(field_ident) = field_ident {
                let alias = field_ident.to_string();
                args.alias = Some(LitStr::new(&alias, field_ident.span()));
            }
            return Ok(());
        }
        if meta.path.is_ident("enum_id") {
            args.enum_id = Some(meta.value()?.parse()?);
            return Ok(());
        }
        if meta.path.is_ident("allowed") {
            let array: ExprArray = meta.value()?.parse()?;
            for expr in array.elems {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(value),
                    ..
                }) = expr
                {
                    args.allowed.push(value);
                }
            }
            return Ok(());
        }
        if meta.path.is_ident("target") {
            args.target = Some(meta.value()?.parse()?);
            return Ok(());
        }
        if meta.path.is_ident("pattern") {
            args.pattern = Some(meta.value()?.parse()?);
            return Ok(());
        }
        if meta.path.is_ident("max_len") {
            args.max_len = Some(meta.value()?.parse()?);
            return Ok(());
        }
        Ok(())
    })?;

    Ok(args)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParamKind {
    Bool,
    Int,
    Float,
    String,
    Vec2,
    Vec3,
    ColorRgba,
    Trigger,
    Enum,
    Reference,
}

fn extract_param_kind(ty: &Type) -> Result<ParamKind> {
    let Type::Path(path) = ty else {
        return Err(syn::Error::new_spanned(
            ty,
            "Unsupported parameter handle type",
        ));
    };

    let segment = path
        .path
        .segments
        .last()
        .ok_or_else(|| syn::Error::new_spanned(ty, "Unsupported parameter handle type"))?;

    if segment.ident != "ParameterHandle" {
        return Err(syn::Error::new_spanned(ty, "Expected ParameterHandle<T>"));
    }

    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return Err(syn::Error::new_spanned(ty, "Expected ParameterHandle<T>"));
    };

    let Some(first_arg) = args.args.first() else {
        return Err(syn::Error::new_spanned(ty, "Missing ParameterHandle type"));
    };

    let syn::GenericArgument::Type(Type::Path(inner_path)) = first_arg else {
        return Err(syn::Error::new_spanned(
            ty,
            "Unsupported ParameterHandle type",
        ));
    };

    let ident = inner_path
        .path
        .segments
        .last()
        .map(|seg| seg.ident.to_string());
    let Some(ident) = ident else {
        return Err(syn::Error::new_spanned(
            ty,
            "Unsupported ParameterHandle type",
        ));
    };

    let kind = match ident.as_str() {
        "bool" | "Bool" => ParamKind::Bool,
        "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" => {
            ParamKind::Int
        }
        "f32" | "f64" => ParamKind::Float,
        "String" => ParamKind::String,
        "Vec2" => ParamKind::Vec2,
        "Vec3" => ParamKind::Vec3,
        "ColorRgba" => ParamKind::ColorRgba,
        "Trigger" => ParamKind::Trigger,
        "ReferenceValue" => ParamKind::Reference,
        _ => ParamKind::Enum,
    };

    Ok(kind)
}

fn extract_param_kind_value_type(ty: &Type) -> Result<ParamKind> {
    let Type::Path(path) = ty else {
        return Err(syn::Error::new_spanned(ty, "Unsupported parameter type"));
    };

    let ident = path
        .path
        .segments
        .last()
        .map(|seg| seg.ident.to_string())
        .ok_or_else(|| syn::Error::new_spanned(ty, "Unsupported parameter type"))?;

    let kind = match ident.as_str() {
        "bool" | "Bool" => ParamKind::Bool,
        "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" => {
            ParamKind::Int
        }
        "f32" | "f64" => ParamKind::Float,
        "String" => ParamKind::String,
        "Vec2" => ParamKind::Vec2,
        "Vec3" => ParamKind::Vec3,
        "ColorRgba" => ParamKind::ColorRgba,
        "Trigger" => ParamKind::Trigger,
        "ReferenceValue" => ParamKind::Reference,
        _ => ParamKind::Enum,
    };

    Ok(kind)
}

fn value_tokens_from_args(kind: &ParamKind, args: &ParamArgs) -> Result<proc_macro2::TokenStream> {
    if let Some(default) = &args.default {
        return value_tokens_from_expr(kind, default, args);
    }

    let tokens = match kind {
        ParamKind::Bool => quote! { golden_schema::Value::Bool(false) },
        ParamKind::Int => quote! { golden_schema::Value::Int(0) },
        ParamKind::Float => quote! { golden_schema::Value::Float(0.0) },
        ParamKind::String => quote! { golden_schema::Value::String(String::new()) },
        ParamKind::Vec2 => {
            quote! { golden_schema::Value::Vec2(golden_schema::Vec2 { x: 0.0, y: 0.0 }) }
        }
        ParamKind::Vec3 => {
            quote! { golden_schema::Value::Vec3(golden_schema::Vec3 { x: 0.0, y: 0.0, z: 0.0 }) }
        }
        ParamKind::ColorRgba => {
            quote! { golden_schema::Value::ColorRgba(golden_schema::ColorRgba { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }) }
        }
        ParamKind::Trigger => quote! { golden_schema::Value::Trigger },
        ParamKind::Reference => {
            return Err(syn::Error::new_spanned(
                default_error_tokens(),
                "Reference parameters require an explicit default",
            ));
        }
        ParamKind::Enum => {
            if let Some(enum_id) = &args.enum_id {
                let enum_id = enum_id.value();
                let variant = args
                    .allowed
                    .first()
                    .map(|item| item.value())
                    .unwrap_or_else(|| "default".to_string());
                quote! {
                    golden_schema::Value::Enum {
                        enum_id: golden_schema::EnumId(#enum_id.to_string()),
                        variant: golden_schema::EnumVariantId(#variant.to_string()),
                    }
                }
            } else {
                return Err(syn::Error::new_spanned(
                    default_error_tokens(),
                    "Enum parameters require enum_id or default",
                ));
            }
        }
    };

    Ok(tokens)
}

fn value_tokens_from_expr(
    kind: &ParamKind,
    expr: &Expr,
    args: &ParamArgs,
) -> Result<proc_macro2::TokenStream> {
    match expr {
        Expr::Lit(ExprLit { lit, .. }) => match lit {
            Lit::Bool(value) => Ok(quote! { golden_schema::Value::Bool(#value) }),
            Lit::Int(value) => {
                let literal = value.base10_parse::<i64>()?;
                Ok(quote! { golden_schema::Value::Int(#literal) })
            }
            Lit::Float(value) => {
                let literal = value.base10_parse::<f64>()?;
                Ok(quote! { golden_schema::Value::Float(#literal) })
            }
            Lit::Str(value) => {
                let literal = value.value();
                Ok(quote! { golden_schema::Value::String(#literal.to_string()) })
            }
            _ => Err(syn::Error::new_spanned(expr, "Unsupported default literal")),
        },
        Expr::Path(ExprPath { path, .. }) => {
            let ident = path.segments.last().map(|seg| seg.ident.to_string());
            if let Some(ident) = ident {
                if ident == "Trigger" {
                    return Ok(quote! { golden_schema::Value::Trigger });
                }
            }

            if *kind == ParamKind::Enum {
                let enum_id = args
                    .enum_id
                    .as_ref()
                    .map(|value| value.value())
                    .unwrap_or_else(|| {
                        path.segments
                            .first()
                            .map(|seg| seg.ident.to_string())
                            .unwrap_or_else(|| "enum".to_string())
                    });
                let variant = path
                    .segments
                    .last()
                    .map(|seg| seg.ident.to_string())
                    .unwrap_or_else(|| "variant".to_string());
                return Ok(quote! {
                    golden_schema::Value::Enum {
                        enum_id: golden_schema::EnumId(#enum_id.to_string()),
                        variant: golden_schema::EnumVariantId(#variant.to_string()),
                    }
                });
            }

            Err(syn::Error::new_spanned(
                expr,
                "Unsupported default expression",
            ))
        }
        _ => Err(syn::Error::new_spanned(
            expr,
            "Unsupported default expression",
        )),
    }
}

fn constraints_tokens_from_args(
    kind: &ParamKind,
    args: &ParamArgs,
) -> Result<proc_macro2::TokenStream> {
    match kind {
        ParamKind::Int => {
            if args.min.is_some()
                || args.max.is_some()
                || args.step.is_some()
                || args.clamp.is_some()
            {
                let min = option_expr_tokens(&args.min);
                let max = option_expr_tokens(&args.max);
                let step = option_expr_tokens(&args.step);
                let clamp = args
                    .clamp
                    .as_ref()
                    .map(|value| value.value())
                    .unwrap_or(false);
                Ok(quote! {
                    golden_schema::ValueConstraints::Int {
                        min: #min,
                        max: #max,
                        clamp: #clamp,
                        step: #step,
                    }
                })
            } else {
                Ok(quote! { golden_schema::ValueConstraints::None })
            }
        }
        ParamKind::Float => {
            if args.min.is_some()
                || args.max.is_some()
                || args.step.is_some()
                || args.clamp.is_some()
            {
                let min = option_expr_tokens(&args.min);
                let max = option_expr_tokens(&args.max);
                let step = option_expr_tokens(&args.step);
                let clamp = args
                    .clamp
                    .as_ref()
                    .map(|value| value.value())
                    .unwrap_or(false);
                Ok(quote! {
                    golden_schema::ValueConstraints::Float {
                        min: #min,
                        max: #max,
                        clamp: #clamp,
                        step: #step,
                    }
                })
            } else {
                Ok(quote! { golden_schema::ValueConstraints::None })
            }
        }
        ParamKind::String => {
            if args.pattern.is_some() || args.max_len.is_some() {
                let pattern = args
                    .pattern
                    .as_ref()
                    .map(|value| {
                        let value = value.value();
                        quote! { Some(#value.to_string()) }
                    })
                    .unwrap_or_else(|| quote! { None });
                let max_len = args
                    .max_len
                    .as_ref()
                    .map(|value| {
                        let value = value.base10_parse::<usize>().unwrap_or(0usize);
                        quote! { Some(#value) }
                    })
                    .unwrap_or_else(|| quote! { None });
                Ok(quote! {
                    golden_schema::ValueConstraints::String {
                        max_len: #max_len,
                        pattern: #pattern,
                    }
                })
            } else {
                Ok(quote! { golden_schema::ValueConstraints::None })
            }
        }
        ParamKind::Enum => {
            if let Some(enum_id) = &args.enum_id {
                let enum_id = enum_id.value();
                let allowed_tokens = args.allowed.iter().map(|value| {
                    let value = value.value();
                    quote! { golden_schema::EnumVariantId(#value.to_string()) }
                });
                Ok(quote! {
                    golden_schema::ValueConstraints::Enum {
                        enum_id: golden_schema::EnumId(#enum_id.to_string()),
                        allowed: vec![#(#allowed_tokens),*],
                    }
                })
            } else {
                Ok(quote! { golden_schema::ValueConstraints::None })
            }
        }
        ParamKind::Reference => {
            let target = args
                .target
                .as_ref()
                .map(|value| {
                    let value = value.value();
                    quote! { Some(#value.to_string()) }
                })
                .unwrap_or_else(|| quote! { None });
            Ok(quote! { golden_schema::ValueConstraints::Reference { target: #target } })
        }
        _ => Ok(quote! { golden_schema::ValueConstraints::None }),
    }
}

fn semantics_tokens(semantics: &Option<LitStr>, unit: &Option<LitStr>) -> proc_macro2::TokenStream {
    let intent = semantics
        .as_ref()
        .map(|value| {
            let value = value.value();
            quote! { Some(#value.to_string()) }
        })
        .unwrap_or_else(|| quote! { None });
    let unit = unit
        .as_ref()
        .map(|value| {
            let value = value.value();
            quote! { Some(#value.to_string()) }
        })
        .unwrap_or_else(|| quote! { None });
    quote! { golden_schema::SemanticsHint { intent: #intent, unit: #unit } }
}

fn presentation_tokens(presentation: &Option<LitStr>) -> proc_macro2::TokenStream {
    let widget = presentation
        .as_ref()
        .map(|value| {
            let value = value.value();
            quote! { Some(#value.to_string()) }
        })
        .unwrap_or_else(|| quote! { None });
    quote! { golden_schema::PresentationHint { widget: #widget } }
}

fn behavior_tokens(behavior: &Option<LitStr>) -> proc_macro2::TokenStream {
    match behavior.as_ref().map(|value| value.value()) {
        Some(value) if value == "Append" => quote! { golden_core::schema::InboxBehavior::Append },
        Some(value) if value == "Coalesce" => {
            quote! { golden_core::schema::InboxBehavior::Coalesce }
        }
        _ => quote! { golden_core::schema::InboxBehavior::Coalesce },
    }
}

fn update_policy_tokens(update: &Option<LitStr>) -> proc_macro2::TokenStream {
    match update.as_ref().map(|value| value.value()) {
        Some(value) if value == "Immediate" => quote! { golden_schema::UpdatePolicy::Immediate },
        Some(value) if value == "EndOfTick" => quote! { golden_schema::UpdatePolicy::EndOfTick },
        Some(value) if value == "NextTick" => quote! { golden_schema::UpdatePolicy::NextTick },
        _ => quote! { golden_schema::UpdatePolicy::Immediate },
    }
}

fn change_policy_tokens(change: &Option<LitStr>) -> proc_macro2::TokenStream {
    match change.as_ref().map(|value| value.value()) {
        Some(value) if value == "Always" => quote! { golden_schema::ChangePolicy::Always },
        _ => quote! { golden_schema::ChangePolicy::ValueChange },
    }
}

fn save_policy_tokens(save: &Option<LitStr>) -> proc_macro2::TokenStream {
    match save.as_ref().map(|value| value.value()) {
        Some(value) if value == "None" => quote! { golden_schema::SavePolicy::None },
        Some(value) if value == "Full" => quote! { golden_schema::SavePolicy::Full },
        _ => quote! { golden_schema::SavePolicy::Delta },
    }
}

fn option_expr_tokens(expr: &Option<Expr>) -> proc_macro2::TokenStream {
    expr.as_ref()
        .map(|value| quote! { Some(#value) })
        .unwrap_or_else(|| quote! { None })
}

fn default_error_tokens() -> proc_macro2::TokenStream {
    quote! { golden_schema::Value::Trigger }
}

struct ParamsInput {
    items: Vec<ParamsItem>,
}

enum ParamsItem {
    Param(ParamItem),
    Folder(FolderItem),
}

struct ParamItem {
    name: Ident,
    ty: Type,
    default: Option<Expr>,
    options: ParamOptions,
}

#[derive(Default)]
struct ParamOptions {
    min: Option<Expr>,
    max: Option<Expr>,
    step: Option<Expr>,
    sem: Option<LitStr>,
    unit: Option<LitStr>,
    behavior: Option<LitStr>,
    alias: Option<LitStr>,
    direct_access: bool,
}

struct FolderItem {
    name: Ident,
    label: Option<LitStr>,
    alias_prefix: Option<LitStr>,
    items: Vec<ParamsItem>,
}

impl Parse for ParamsInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut items = Vec::new();
        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            if ident == "folder" {
                let folder = parse_folder_item(input)?;
                items.push(ParamsItem::Folder(folder));
            } else {
                let param = parse_param_item(input, ident)?;
                items.push(ParamsItem::Param(param));
            }
        }
        Ok(Self { items })
    }
}

fn parse_folder_item(input: ParseStream) -> Result<FolderItem> {
    let content;
    syn::parenthesized!(content in input);
    let name: Ident = content.parse()?;
    let mut label = None::<LitStr>;
    let mut alias_prefix = None::<LitStr>;

    while !content.is_empty() {
        let _comma: Option<Token![,]> = content.parse()?;
        if content.is_empty() {
            break;
        }
        let key: Ident = content.parse()?;
        let _eq: Token![=] = content.parse()?;
        let value: LitStr = content.parse()?;
        if key == "label" {
            label = Some(value);
        } else if key == "alias_prefix" {
            alias_prefix = Some(value);
        }
    }

    let block;
    syn::braced!(block in input);
    let mut items = Vec::new();
    while !block.is_empty() {
        let ident: Ident = block.parse()?;
        if ident == "folder" {
            let folder = parse_folder_item(&block)?;
            items.push(ParamsItem::Folder(folder));
        } else {
            let param = parse_param_item(&block, ident)?;
            items.push(ParamsItem::Param(param));
        }
    }

    Ok(FolderItem {
        name,
        label,
        alias_prefix,
        items,
    })
}

fn parse_param_item(input: ParseStream, name: Ident) -> Result<ParamItem> {
    let _colon: Token![:] = input.parse()?;
    let ty: Type = input.parse()?;
    let mut default = None;
    let mut options = ParamOptions::default();

    if input.peek(Token![=]) {
        let _eq: Token![=] = input.parse()?;
        default = Some(parse_simple_expr(input)?);
    }

    if input.peek(syn::token::Bracket) {
        let content;
        syn::bracketed!(content in input);
        let range: ExprRange = content.parse()?;
        options.min = range.start.map(|expr| *expr);
        options.max = range.end.map(|expr| *expr);
    }

    if input.peek(syn::token::Paren) {
        let content;
        syn::parenthesized!(content in input);
        while !content.is_empty() {
            let key: Ident = content.parse()?;
            if key == "direct_access" {
                options.direct_access = true;
            } else {
                let _eq: Token![=] = content.parse()?;
                if key == "min" {
                    options.min = Some(content.parse()?);
                } else if key == "max" {
                    options.max = Some(content.parse()?);
                } else if key == "step" {
                    options.step = Some(content.parse()?);
                } else if key == "sem" || key == "semantics" {
                    options.sem = Some(content.parse()?);
                } else if key == "unit" {
                    options.unit = Some(content.parse()?);
                } else if key == "behavior" {
                    options.behavior = Some(content.parse()?);
                } else if key == "alias" {
                    options.alias = Some(content.parse()?);
                } else {
                    let _skip: Expr = content.parse()?;
                }
            }

            let _comma: Option<Token![,]> = content.parse()?;
        }
    }

    let _semi: Token![;] = input.parse()?;

    Ok(ParamItem {
        name,
        ty,
        default,
        options,
    })
}

fn parse_simple_expr(input: ParseStream) -> Result<Expr> {
    if input.peek(LitBool) || input.peek(LitInt) || input.peek(LitFloat) || input.peek(LitStr) {
        let literal: ExprLit = input.parse()?;
        return Ok(Expr::Lit(literal));
    }

    let path: ExprPath = input.parse()?;
    Ok(Expr::Path(path))
}

fn collect_params_from_item(
    item: ParamsItem,
    param_decls: &mut Vec<proc_macro2::TokenStream>,
    folder_decls: &mut Vec<proc_macro2::TokenStream>,
    declared_children: &mut Vec<proc_macro2::TokenStream>,
    folder_path: Option<String>,
    alias_prefix: Option<String>,
) {
    match item {
        ParamsItem::Param(param) => {
            let decl_id = param.name.to_string();
            let folder_decl_id = folder_path.clone();
            let args = ParamArgs {
                default: param.default,
                min: param.options.min.clone(),
                max: param.options.max.clone(),
                step: param.options.step.clone(),
                semantics: param.options.sem.clone(),
                unit: param.options.unit.clone(),
                behavior: param.options.behavior.clone(),
                alias: param.options.alias.clone(),
                read_only: false,
                ..Default::default()
            };

            let kind = match extract_param_kind_value_type(&param.ty) {
                Ok(kind) => kind,
                Err(err) => {
                    param_decls.push(err.to_compile_error());
                    return;
                }
            };

            let default_tokens = match value_tokens_from_args(&kind, &args) {
                Ok(tokens) => tokens,
                Err(err) => {
                    param_decls.push(err.to_compile_error());
                    return;
                }
            };

            let constraints_tokens = match constraints_tokens_from_args(&kind, &args) {
                Ok(tokens) => tokens,
                Err(err) => {
                    param_decls.push(err.to_compile_error());
                    return;
                }
            };

            let folder_tokens = folder_decl_id
                .as_ref()
                .map(|value| quote! { Some(golden_schema::DeclId(#value.to_string())) })
                .unwrap_or_else(|| quote! { None });

            let behavior_tokens = behavior_tokens(&args.behavior);
            let semantics_tokens = semantics_tokens(&args.semantics, &args.unit);
            let alias_tokens = if param.options.direct_access {
                let mut alias = param.name.to_string();
                if let Some(prefix) = &alias_prefix {
                    alias = format!("{prefix}{alias}");
                }
                quote! { Some(#alias.to_string()) }
            } else if let Some(alias) = &param.options.alias {
                let alias = alias.value();
                quote! { Some(#alias.to_string()) }
            } else {
                quote! { None }
            };

            param_decls.push(quote! {
                golden_core::schema::ParamDecl {
                    decl_id: golden_schema::DeclId(#decl_id.to_string()),
                    default: #default_tokens,
                    constraints: #constraints_tokens,
                    read_only: false,
                    update: golden_schema::UpdatePolicy::Immediate,
                    change: golden_schema::ChangePolicy::ValueChange,
                    save: golden_schema::SavePolicy::Delta,
                    semantics: #semantics_tokens,
                    presentation: golden_schema::PresentationHint { widget: None },
                    folder: #folder_tokens,
                    behavior: #behavior_tokens,
                    alias: #alias_tokens,
                }
            });

            declared_children.push(quote! {
                golden_core::schema::DeclaredChild {
                    decl_id: golden_schema::DeclId(#decl_id.to_string()),
                    node_type: golden_schema::NodeTypeId("Parameter".to_string()),
                    default_label: Some(#decl_id.to_string()),
                    default_enabled: true,
                }
            });
        }
        ParamsItem::Folder(folder) => {
            let folder_name = folder.name.to_string();
            let decl_id = match &folder_path {
                Some(prefix) => format!("{prefix}.{folder_name}"),
                None => folder_name,
            };

            let label_tokens = folder
                .label
                .as_ref()
                .map(|value| {
                    let value = value.value();
                    quote! { Some(#value.to_string()) }
                })
                .unwrap_or_else(|| quote! { None });
            let alias_tokens = folder
                .alias_prefix
                .as_ref()
                .map(|value| {
                    let value = value.value();
                    quote! { Some(#value.to_string()) }
                })
                .unwrap_or_else(|| quote! { None });

            folder_decls.push(quote! {
                golden_core::schema::FolderDecl {
                    decl_id: golden_schema::DeclId(#decl_id.to_string()),
                    label: #label_tokens,
                    alias_prefix: #alias_tokens,
                }
            });

            declared_children.push(quote! {
                golden_core::schema::DeclaredChild {
                    decl_id: golden_schema::DeclId(#decl_id.to_string()),
                    node_type: golden_schema::NodeTypeId("Folder".to_string()),
                    default_label: Some(#decl_id.to_string()),
                    default_enabled: true,
                }
            });

            let folder_path = Some(decl_id);
            let next_alias_prefix = match (alias_prefix, folder.alias_prefix.as_ref()) {
                (Some(prefix), Some(next)) => Some(format!("{prefix}{}", next.value())),
                (None, Some(next)) => Some(next.value()),
                (Some(prefix), None) => Some(prefix),
                (None, None) => None,
            };
            for item in folder.items {
                collect_params_from_item(
                    item,
                    param_decls,
                    folder_decls,
                    declared_children,
                    folder_path.clone(),
                    next_alias_prefix.clone(),
                );
            }
        }
    }
}

fn validate_params_items(items: &[ParamsItem]) -> Result<()> {
    use std::collections::HashMap;

    let mut param_names: HashMap<String, proc_macro2::Span> = HashMap::new();
    let mut top_folders: HashMap<String, proc_macro2::Span> = HashMap::new();
    let mut alias_names: HashMap<String, proc_macro2::Span> = HashMap::new();
    let mut errors: Option<syn::Error> = None;

    fn push_error(errors: &mut Option<syn::Error>, error: syn::Error) {
        if let Some(existing) = errors.as_mut() {
            existing.combine(error);
        } else {
            *errors = Some(error);
        }
    }

    fn walk(
        items: &[ParamsItem],
        depth: usize,
        alias_prefix: Option<String>,
        param_names: &mut HashMap<String, proc_macro2::Span>,
        top_folders: &mut HashMap<String, proc_macro2::Span>,
        alias_names: &mut HashMap<String, proc_macro2::Span>,
        errors: &mut Option<syn::Error>,
    ) {
        for item in items {
            match item {
                ParamsItem::Param(param) => {
                    let param_name = param.name.to_string();
                    let param_span = param.name.span();
                    if let Some(prev) = param_names.insert(param_name.clone(), param_span) {
                        let err = syn::Error::new(
                            param_span,
                            format!("duplicate parameter name: {param_name}"),
                        );
                        let note = syn::Error::new(prev, "previous parameter declared here");
                        let mut combined = err;
                        combined.combine(note);
                        push_error(errors, combined);
                    }

                    let alias_value = if param.options.direct_access {
                        let mut alias = param_name.clone();
                        if let Some(prefix) = &alias_prefix {
                            alias = format!("{prefix}{alias}");
                        }
                        Some((alias, param_span))
                    } else if let Some(alias) = &param.options.alias {
                        Some((alias.value(), alias.span()))
                    } else {
                        None
                    };

                    if let Some((alias, alias_span)) = alias_value {
                        if let Some(prev) = alias_names.get(&alias) {
                            let err = syn::Error::new(
                                alias_span,
                                format!("duplicate alias name: {alias}"),
                            );
                            let note = syn::Error::new(*prev, "previous alias declared here");
                            let mut combined = err;
                            combined.combine(note);
                            push_error(errors, combined);
                        } else {
                            alias_names.insert(alias.clone(), alias_span);
                        }

                        if alias != param_name {
                            if let Some(prev) = param_names.get(&alias) {
                                let err = syn::Error::new(
                                    alias_span,
                                    format!("alias '{alias}' collides with parameter name"),
                                );
                                let note = syn::Error::new(*prev, "parameter declared here");
                                let mut combined = err;
                                combined.combine(note);
                                push_error(errors, combined);
                            }
                        }

                        if let Some(prev) = top_folders.get(&alias) {
                            let err = syn::Error::new(
                                alias_span,
                                format!("alias '{alias}' collides with folder name"),
                            );
                            let note = syn::Error::new(*prev, "folder declared here");
                            let mut combined = err;
                            combined.combine(note);
                            push_error(errors, combined);
                        }
                    }
                }
                ParamsItem::Folder(folder) => {
                    let folder_name = folder.name.to_string();
                    let folder_span = folder.name.span();
                    if depth == 0 {
                        if let Some(prev) = top_folders.insert(folder_name.clone(), folder_span) {
                            let err = syn::Error::new(
                                folder_span,
                                format!("duplicate folder name: {folder_name}"),
                            );
                            let note = syn::Error::new(prev, "previous folder declared here");
                            let mut combined = err;
                            combined.combine(note);
                            push_error(errors, combined);
                        }
                    }

                    let next_alias_prefix = match (&alias_prefix, &folder.alias_prefix) {
                        (Some(prefix), Some(next)) => Some(format!("{prefix}{}", next.value())),
                        (None, Some(next)) => Some(next.value()),
                        (Some(prefix), None) => Some(prefix.clone()),
                        (None, None) => None,
                    };

                    walk(
                        &folder.items,
                        depth + 1,
                        next_alias_prefix,
                        param_names,
                        top_folders,
                        alias_names,
                        errors,
                    );
                }
            }
        }
    }

    walk(
        items,
        0,
        None,
        &mut param_names,
        &mut top_folders,
        &mut alias_names,
        &mut errors,
    );

    match errors {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

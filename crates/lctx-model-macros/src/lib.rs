//! Derive for concrete domain records. Unsupported shapes fail at their declaration.
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, LitStr, parse_macro_input};

#[proc_macro_derive(Domain, attributes(model))]
pub fn domain(input: TokenStream) -> TokenStream {
    match expand(parse_macro_input!(input as DeriveInput)) {
        Ok(tokens) => quote!(#tokens).into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn expand(input: DeriveInput) -> syn::Result<impl quote::ToTokens> {
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(&input.generics, "domain records are concrete"));
    }
    let mut table = None;
    let mut validator: Option<syn::Path> = None;
    let mut semantic_source: Option<syn::Expr> = None;
    for attr in &input.attrs {
        if attr.path().is_ident("model") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") { table = Some(meta.value()?.parse::<LitStr>()?); }
                else if meta.path.is_ident("validate") { validator = Some(meta.value()?.parse()?); }
                else if meta.path.is_ident("semantic_source") { semantic_source = Some(meta.value()?.parse()?); }
                else { return Err(meta.error("expected name, validate or semantic_source")); }
                Ok(())
            })?;
        }
    }
    let table = table.ok_or_else(|| syn::Error::new_spanned(&input.ident, "model(name = \"…\") is required"))?;
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(&input, "Domain requires a named record; model sum values separately"));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(syn::Error::new_spanned(&data.fields, "Domain requires named fields"));
    };
    let mut keys = Vec::new();
    let mut key_types = Vec::new();
    let mut names = Vec::new();
    let mut types = Vec::new();
    let mut descriptors = Vec::new();
    for field in &fields.named {
        let name = field.ident.as_ref().expect("named fields");
        if name == "id" || name == "generation_id" {
            return Err(syn::Error::new_spanned(name, "identity and generation columns are generated"));
        }
        let ty = &field.ty;
        let mut key = false;
        let mut provenance = false;
        for attr in &field.attrs {
            if attr.path().is_ident("model") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("key") { key = true; }
                    else if meta.path.is_ident("provenance") { provenance = true; }
                    else { return Err(meta.error("expected key or provenance")); }
                    Ok(())
                })?;
            }
        }
        if key { keys.push(name); key_types.push(ty); }
        names.push(name); types.push(ty);
        descriptors.push(quote! {
            ::lctx_model::domain::Field::of::<#ty>(stringify!(#name), #key, #provenance)
        });
    }
    if keys.is_empty() { return Err(syn::Error::new_spanned(&input.ident, "at least one semantic key field is required")); }
    let declaration = quote!(#input).to_string();
    let name = &input.ident;
    let key_name = format_ident!("{}Key", name);
    let physical = format_ident!("__{}Physical", name);
    let vis = &input.vis;
    let semantic_source = semantic_source.map(|expr| quote!(#expr)).unwrap_or_else(|| quote!(b""));
    let validation = validator.map(|v| quote! { #v(self)?; });
    Ok(quote! {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        #vis struct #key_name { #(pub #keys: #key_types,)* }
        impl ::lctx_model::domain::Key for #key_name {
            fn encode(&self, sink: &mut ::lctx_model::domain::KeySink) {
                #(::lctx_model::domain::Key::encode(&self.#keys, sink);)*
            }
        }
        #[derive(::lctx_model::domain::__private::serde::Serialize, ::lctx_model::domain::__private::serde::Deserialize)]
        #[serde(crate = "::lctx_model::domain::__private::serde")]
        struct #physical {
            id: ::lctx_model::domain::Id<#name>,
            #(#names: #types,)*
        }
        impl ::lctx_model::domain::Record for #name {
            type Key = #key_name;
            const NAME: &'static str = #table;
            const CONTRACT: &'static str = #declaration;
            const OWNER: &'static str = env!("CARGO_PKG_NAME");
            const SEMANTIC_SOURCE: &'static [u8] = #semantic_source;
            fn key(&self) -> Self::Key { #key_name { #(#keys: self.#keys.clone(),)* } }
            fn fields() -> Vec<::lctx_model::domain::Field> { vec![#(#descriptors,)*] }
            fn validate(&self) -> Result<(), ::lctx_model::domain::ModelError> {
                #validation
                Ok(())
            }
            fn encode(rows: &[Self]) -> Result<::lctx_model::domain::__private::RecordBatch, ::lctx_model::domain::ModelError> {
                let physical = rows.iter().map(|row| #physical {
                    id: <Self as ::lctx_model::domain::Record>::id(row),
                    #(#names: row.#names.clone(),)*
                }).collect::<Vec<_>>();
                ::lctx_model::domain::__private::serde_arrow::to_record_batch(
                    Self::schema().fields(), &physical
                ).map_err(::lctx_model::domain::ModelError::codec)
            }
            fn decode(batch: &::lctx_model::domain::__private::RecordBatch) -> Result<Vec<Self>, ::lctx_model::domain::ModelError> {
                if batch.schema().as_ref() != Self::schema().as_ref() {
                    return Err(::lctx_model::domain::ModelError::Schema(Self::NAME));
                }
                let physical: Vec<#physical> = ::lctx_model::domain::__private::serde_arrow::from_record_batch(batch)
                    .map_err(::lctx_model::domain::ModelError::codec)?;
                physical.into_iter().map(|physical| {
                    let row = Self { #(#names: physical.#names,)* };
                    <Self as ::lctx_model::domain::Record>::validate(&row)?;
                    if physical.id != <Self as ::lctx_model::domain::Record>::id(&row) {
                        return Err(::lctx_model::domain::ModelError::Identity(Self::NAME));
                    }
                    Ok(row)
                }).collect()
            }
        }
    })
}

/// Closed, explicitly numbered codebooks. Existing discriminants are never inferred or renumbered.
#[proc_macro_derive(DomainCode)]
pub fn domain_code(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_code(input) {
        Ok(tokens) => quote!(#tokens).into(),
        Err(error) => error.into_compile_error().into(),
    }
}
fn expand_code(input: DeriveInput) -> syn::Result<impl quote::ToTokens> {
    let Data::Enum(data) = &input.data else { return Err(syn::Error::new_spanned(&input, "DomainCode requires an enum")); };
    let name = &input.ident;
    let mut variants = Vec::new();
    let mut codes = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for variant in &data.variants {
        if !matches!(variant.fields, Fields::Unit) { return Err(syn::Error::new_spanned(variant, "codebook variants have no payload")); }
        let Some((_, syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(code), .. }))) = &variant.discriminant else {
            return Err(syn::Error::new_spanned(variant, "codebook requires an explicit nonnegative i16 code"));
        };
        let value = code.base10_parse::<i16>()?;
        if !seen.insert(value) { return Err(syn::Error::new_spanned(code, "duplicate code")); }
        variants.push(&variant.ident); codes.push(value);
    }
    Ok(quote! {
        impl ::lctx_model::domain::FlatValue for #name {}
        impl ::lctx_model::domain::FieldValue for #name {
            const SCALAR: ::lctx_model::domain::Scalar = ::lctx_model::domain::Scalar::Int16;
            fn codes() -> &'static [(i16, &'static str)] { &[#((#codes, stringify!(#variants)),)*] }
        }
        impl ::lctx_model::domain::Key for #name {
            fn encode(&self, sink: &mut ::lctx_model::domain::KeySink) {
                let code: i16 = match self { #(Self::#variants => #codes,)* };
                sink.part(stringify!(#name).as_bytes(), &code.to_le_bytes());
            }
        }
        impl ::lctx_model::domain::__private::serde::Serialize for #name {
            fn serialize<S: ::lctx_model::domain::__private::serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_i16(match self { #(Self::#variants => #codes,)* })
            }
        }
        impl<'de> ::lctx_model::domain::__private::serde::Deserialize<'de> for #name {
            fn deserialize<D: ::lctx_model::domain::__private::serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                match <i16 as ::lctx_model::domain::__private::serde::Deserialize>::deserialize(d)? {
                    #(#codes => Ok(Self::#variants),)*
                    _ => Err(::lctx_model::domain::__private::serde::de::Error::custom(concat!("unknown ", stringify!(#name), " code"))),
                }
            }
        }
    })
}

/// Finite tagged domain alternatives lower to a discriminant and nullable payload columns.
#[proc_macro_derive(DomainSum, attributes(model))]
pub fn domain_sum(input: TokenStream) -> TokenStream {
    match expand_sum(parse_macro_input!(input as DeriveInput)) {
        Ok(tokens) => quote!(#tokens).into(),
        Err(error) => error.into_compile_error().into(),
    }
}
fn expand_sum(input: DeriveInput) -> syn::Result<impl quote::ToTokens> {
    let Data::Enum(data) = &input.data else { return Err(syn::Error::new_spanned(&input, "DomainSum requires an enum")); };
    if !input.generics.params.is_empty() { return Err(syn::Error::new_spanned(&input.generics, "domain sums are concrete")); }
    let mut table = None;
    let mut semantic_source: Option<syn::Expr> = None;
    for attr in &input.attrs {
        if attr.path().is_ident("model") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") { table = Some(meta.value()?.parse::<LitStr>()?); Ok(()) }
                else if meta.path.is_ident("semantic_source") { semantic_source = Some(meta.value()?.parse()?); Ok(()) }
                else { Err(meta.error("expected name or semantic_source")) }
            })?;
        }
    }
    let table = table.ok_or_else(|| syn::Error::new_spanned(&input.ident, "model(name = …) is required"))?;
    let semantic_source = semantic_source.map(|expr| quote!(#expr)).unwrap_or_else(|| quote!(b""));
    let name = &input.ident;
    let physical = format_ident!("__{}Physical", name);
    let declaration = quote!(#input).to_string();
    let mut physical_names = Vec::new();
    let mut physical_types = Vec::new();
    let mut descriptors = Vec::new();
    let mut variants = Vec::new();
    let mut codes = Vec::new();
    let mut arm_fields = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for variant in &data.variants {
        let mut code = None;
        for attr in &variant.attrs {
            if attr.path().is_ident("model") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("code") { code = Some(meta.value()?.parse::<syn::LitInt>()?.base10_parse::<i16>()?); Ok(()) }
                    else { Err(meta.error("expected code")) }
                })?;
            }
        }
        let code = code.ok_or_else(|| syn::Error::new_spanned(variant, "each arm requires an explicit model(code = N)"))?;
        if !seen.insert(code) { return Err(syn::Error::new_spanned(variant, "duplicate arm code")); }
        let mut members = Vec::new();
        match &variant.fields {
            Fields::Named(fields) => for field in &fields.named {
                let member = field.ident.as_ref().expect("named");
                let column = format_ident!("{}_{}", variant.ident.to_string().to_lowercase(), member);
                let (storage_ty, optional) = optional_inner(&field.ty);
                physical_names.push(column.clone()); physical_types.push(storage_ty.clone());
                let mut provenance = false;
                for attr in &field.attrs {
                    if attr.path().is_ident("model") {
                        attr.parse_nested_meta(|meta| {
                            if meta.path.is_ident("provenance") { provenance = true; Ok(()) }
                            else { Err(meta.error("sum payloads are key fields; only provenance may be annotated")) }
                        })?;
                    }
                }
                descriptors.push(quote! { ::lctx_model::domain::Field::of::<Option<#storage_ty>>(stringify!(#column), true, #provenance) });
                members.push((member.clone(), column, optional));
            },
            Fields::Unit => {},
            _ => return Err(syn::Error::new_spanned(variant, "sum payloads require named fields")),
        }
        variants.push(&variant.ident); codes.push(code); arm_fields.push(members);
    }
    let visibility = &input.vis;
    let aliases: Vec<_> = variants.iter().map(|variant| format_ident!("{}{}Id", name, variant)).collect();
    let mut key_arms = Vec::new();
    let mut encode_arms = Vec::new();
    let mut decode_arms = Vec::new();
    let mut sum_arms = Vec::new();
    for ((variant, code), fields) in variants.iter().zip(&codes).zip(&arm_fields) {
        let names: Vec<_> = fields.iter().map(|(name,_,_)| name).collect();
        key_arms.push(quote! { Self::#variant { #(#names,)* } => {
            ::lctx_model::domain::Key::encode(&(#code as i16), sink);
            #(::lctx_model::domain::Key::encode(#names, sink);)*
        }});
        let values: Vec<_> = physical_names.iter().map(|column| {
            if let Some((member, _, optional)) = fields.iter().find(|(_,c,_)| c == column) {
                if *optional { quote! { #column: #member.clone() } } else { quote! { #column: Some(#member.clone()) } }
            } else { quote! { #column: None } }
        }).collect();
        encode_arms.push(quote! { Self::#variant { #(#names,)* } => #physical { id: Self::id(row), kind: #code, #(#values,)* } });
        let inactive: Vec<_> = physical_names.iter().filter(|column| !fields.iter().any(|(_,c,_)| c == *column)).collect();
        let decoded: Vec<_> = fields.iter().map(|(member, column, optional)| {
            if *optional { quote! { #member: physical.#column } }
            else { quote! { #member: physical.#column.ok_or_else(|| ::lctx_model::domain::ModelError::Invalid(concat!("missing active payload ", stringify!(#column)).into()))? } }
        }).collect();
        decode_arms.push(quote! { #code => {
            if false #(|| physical.#inactive.is_some())* { return Err(::lctx_model::domain::ModelError::Invalid("inactive sum payload".into())); }
            Self::#variant { #(#decoded,)* }
        }});
        let requirements = fields.iter().map(|(_, column, optional)| {
            let required = !optional;
            quote! { ::lctx_model::domain::ArmField { name: stringify!(#column), required: #required } }
        });
        sum_arms.push(quote! { ::lctx_model::domain::Arm { code: #code, fields: vec![#(#requirements,)*] } });
    }
    Ok(quote! {
        #(#visibility type #aliases = ::lctx_model::domain::ArmId<#name, #codes>;)*
        impl ::lctx_model::domain::SumRecord for #name {
            fn tag(&self) -> i16 { match self { #(Self::#variants { .. } => #codes,)* } }
        }
        #[derive(::lctx_model::domain::__private::serde::Serialize, ::lctx_model::domain::__private::serde::Deserialize)]
        #[serde(crate = "::lctx_model::domain::__private::serde")]
        struct #physical { id: ::lctx_model::domain::Id<#name>, kind: i16, #(#physical_names: Option<#physical_types>,)* }
        impl ::lctx_model::domain::Key for #name {
            fn encode(&self, sink: &mut ::lctx_model::domain::KeySink) { match self { #(#key_arms,)* } }
        }
        impl ::lctx_model::domain::Record for #name {
            type Key = Self;
            const NAME: &'static str = #table;
            const CONTRACT: &'static str = #declaration;
            const OWNER: &'static str = env!("CARGO_PKG_NAME");
            const SEMANTIC_SOURCE: &'static [u8] = #semantic_source;
            fn key(&self) -> Self { self.clone() }
            fn fields() -> Vec<::lctx_model::domain::Field> { vec![::lctx_model::domain::Field::of::<i16>("kind", true, false), #(#descriptors,)*] }
            fn sum() -> Option<::lctx_model::domain::Sum> { Some(::lctx_model::domain::Sum { tag: "kind", arms: vec![#(#sum_arms,)*] }) }
            fn validate(&self) -> Result<(), ::lctx_model::domain::ModelError> { Ok(()) }
            fn encode(rows: &[Self]) -> Result<::lctx_model::domain::__private::RecordBatch, ::lctx_model::domain::ModelError> {
                let physical: Vec<_> = rows.iter().map(|row| match row { #(#encode_arms,)* }).collect();
                ::lctx_model::domain::__private::serde_arrow::to_record_batch(Self::schema().fields(), &physical).map_err(::lctx_model::domain::ModelError::codec)
            }
            fn decode(batch: &::lctx_model::domain::__private::RecordBatch) -> Result<Vec<Self>, ::lctx_model::domain::ModelError> {
                if batch.schema().as_ref() != Self::schema().as_ref() { return Err(::lctx_model::domain::ModelError::Schema(Self::NAME)); }
                let physical: Vec<#physical> = ::lctx_model::domain::__private::serde_arrow::from_record_batch(batch).map_err(::lctx_model::domain::ModelError::codec)?;
                physical.into_iter().map(|physical| {
                    let row = match physical.kind { #(#decode_arms,)* _ => return Err(::lctx_model::domain::ModelError::Invalid("unknown sum tag".into())) };
                    if row.id() != physical.id { return Err(::lctx_model::domain::ModelError::Identity(Self::NAME)); }
                    Ok(row)
                }).collect()
            }
        }
    })
}
fn optional_inner(ty: &syn::Type) -> (syn::Type, bool) {
    if let syn::Type::Path(path) = ty {
        if let Some(segment) = path.path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() { return (inner.clone(), true); }
                }
            }
        }
    }
    (ty.clone(), false)
}

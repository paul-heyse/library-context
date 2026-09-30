//! Derive for concrete domain records. Unsupported shapes fail at their declaration.
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, LitStr, parse_macro_input};

/// Generates only the mechanically identical support relationship and contract accessors.
/// Proposition fields remain ordinary Rust declarations; semantic validation is library code.
#[proc_macro_derive(Assertion, attributes(assertion))]
pub fn assertion(input: TokenStream) -> TokenStream {
    match expand_assertion(parse_macro_input!(input as DeriveInput)) {
        Ok(tokens) => quote!(#tokens).into(),
        Err(error) => error.into_compile_error().into(),
    }
}
fn expand_assertion(input: DeriveInput) -> syn::Result<impl quote::ToTokens> {
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input,
            "assertion requires a concrete record",
        ));
    };
    let qualification = data
        .fields
        .iter()
        .find(|field| {
            field
                .ident
                .as_ref()
                .is_some_and(|name| name == "qualification")
        })
        .ok_or_else(|| {
            syn::Error::new_spanned(
                &input.ident,
                "assertion requires a keyed qualification field",
            )
        })?;
    let mut keyed = false;
    for attr in &qualification.attrs {
        if attr.path().is_ident("model") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("key") {
                    keyed = true;
                }
                Ok(())
            })?;
        }
    }
    if !keyed {
        return Err(syn::Error::new_spanned(
            qualification,
            "assertion qualification must participate in the semantic key",
        ));
    }
    let mut support: Option<syn::Ident> = None;
    let mut table: Option<LitStr> = None;
    let mut family: Option<syn::Path> = None;
    let mut subjects: Vec<syn::Ident> = Vec::new();
    let mut fidelity: Option<syn::Path> = None;
    let mut referents: Vec<syn::Ident> = Vec::new();
    for attr in &input.attrs {
        if attr.path().is_ident("assertion") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("support") {
                    support = Some(meta.value()?.parse()?);
                } else if meta.path.is_ident("name") {
                    table = Some(meta.value()?.parse()?);
                } else if meta.path.is_ident("family") {
                    family = Some(meta.value()?.parse()?);
                } else if meta.path.is_ident("fidelity") {
                    fidelity = Some(meta.value()?.parse()?);
                } else if meta.path.is_ident("referents") {
                    meta.parse_nested_meta(|field| {
                        referents.push(
                            field
                                .path
                                .get_ident()
                                .cloned()
                                .ok_or_else(|| field.error("expected referent field"))?,
                        );
                        Ok(())
                    })?;
                } else if meta.path.is_ident("subjects") {
                    meta.parse_nested_meta(|field| {
                        subjects.push(
                            field
                                .path
                                .get_ident()
                                .cloned()
                                .ok_or_else(|| field.error("expected subject field"))?,
                        );
                        Ok(())
                    })?;
                } else {
                    return Err(meta
                        .error("expected support, name, family, fidelity, subjects or referents"));
                }
                Ok(())
            })?;
        }
    }
    let error = || {
        syn::Error::new_spanned(
            &input.ident,
            "assertion requires support, name, family and nonempty subjects",
        )
    };
    let support = support.ok_or_else(error)?;
    let table = table.ok_or_else(error)?;
    let family = family.ok_or_else(error)?;
    if subjects.is_empty() {
        return Err(error());
    }
    let subject_types = subjects
        .iter()
        .chain(&referents)
        .map(|subject| {
            data.fields
                .iter()
                .find(|field| field.ident.as_ref() == Some(subject))
                .map(|field| &field.ty)
                .ok_or_else(|| {
                    syn::Error::new_spanned(subject, "unknown subject or referent field")
                })
        })
        .collect::<syn::Result<Vec<_>>>()?;
    let name = &input.ident;
    let vis = &input.vis;
    let fidelity = fidelity.map(|fidelity| quote! { const FIDELITY: Option<::lctx_model::domain::attribution::Fidelity> = Some(#fidelity); });
    Ok(quote! {
        impl ::lctx_model::domain::assertion::Assertion for #name {
            const FAMILY: ::lctx_model::domain::attribution::FactFamily = #family;
            #fidelity
            fn qualification(&self) -> ::lctx_model::domain::Id<::lctx_model::domain::assertion::AssertionQualification> { self.qualification }
            fn subjects(&self) -> Vec<::lctx_model::domain::assertion::Subject> {
                let mut subjects = Vec::new();
                #(::lctx_model::domain::assertion::SubjectValue::append_subjects(&self.#subjects, &mut subjects);)*
                subjects
            }
            fn referents(&self) -> Vec<::lctx_model::domain::assertion::Subject> {
                let mut referents = Vec::new();
                #(::lctx_model::domain::assertion::SubjectValue::append_subjects(&self.#referents, &mut referents);)*
                referents
            }
            fn subject_inputs() -> Vec<::lctx_model::domain::ValidationInput> {
                let mut inputs = Vec::new();
                #(inputs.extend(<#subject_types as ::lctx_model::domain::assertion::SubjectValue>::inputs());)*
                inputs
            }
        }
        #[derive(Debug, Clone, PartialEq, Eq, ::lctx_model::Domain)]
        #[model(name = #table, family = #family, invariants = ::lctx_model::domain::assertion::support_invariants::<#name, #support>)]
        #vis struct #support {
            #[model(key)] pub assertion: ::lctx_model::domain::Id<#name>,
            #[model(key, provenance)] pub run: ::lctx_model::domain::Id<::lctx_model::domain::attribution::ProviderRun>,
            #[model(key, provenance)] pub surface: ::lctx_model::domain::Id<::lctx_model::domain::assertion::ProviderSurface>,
            #[model(key, provenance)] pub evidence: ::lctx_model::domain::Id<::lctx_model::domain::assertion::Evidence>,
            #[model(key, provenance)] pub origin: ::lctx_model::domain::attribution::Origin,
            #[model(key, provenance)] pub mode: ::lctx_model::domain::attribution::ExtractionMode,
            #[model(key, provenance)] pub fidelity: ::lctx_model::domain::attribution::Fidelity,
        }
        impl ::lctx_model::domain::assertion::Support for #support {
            type Assertion = #name;
            fn assertion(&self) -> ::lctx_model::domain::Id<#name> { self.assertion }
            fn attribution(&self) -> ::lctx_model::domain::assertion::SupportAttribution {
                ::lctx_model::domain::assertion::SupportAttribution { run: self.run, surface: self.surface, evidence: self.evidence, fidelity: self.fidelity }
            }
        }
    })
}

#[proc_macro_derive(Domain, attributes(model))]
pub fn domain(input: TokenStream) -> TokenStream {
    match expand(parse_macro_input!(input as DeriveInput)) {
        Ok(tokens) => quote!(#tokens).into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn expand(input: DeriveInput) -> syn::Result<impl quote::ToTokens> {
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "domain records are concrete",
        ));
    }
    let mut table = None;
    let mut rule: Option<LitStr> = None;
    let mut conclusion: Option<syn::Ident> = None;
    let mut validator: Option<syn::Path> = None;
    let mut invariants: Option<syn::Path> = None;
    let mut semantic_source: Option<syn::Expr> = None;
    let mut required_support: Option<syn::Ident> = None;
    let mut family: Option<syn::Path> = None;
    for attr in &input.attrs {
        if attr.path().is_ident("assertion") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("support") {
                    required_support = Some(meta.value()?.parse()?);
                } else if meta.path.is_ident("subjects") || meta.path.is_ident("referents") {
                    meta.parse_nested_meta(|_| Ok(()))?;
                } else if meta.path.is_ident("family") {
                    family = Some(meta.value()?.parse()?);
                } else {
                    let _: syn::Expr = meta.value()?.parse()?;
                }
                Ok(())
            })?;
        }
        if attr.path().is_ident("model") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") {
                    table = Some(meta.value()?.parse::<LitStr>()?);
                } else if meta.path.is_ident("rule") {
                    rule = Some(meta.value()?.parse()?);
                } else if meta.path.is_ident("conclusion") {
                    conclusion = Some(meta.value()?.parse()?);
                } else if meta.path.is_ident("validate") {
                    validator = Some(meta.value()?.parse()?);
                } else if meta.path.is_ident("invariants") {
                    invariants = Some(meta.value()?.parse()?);
                } else if meta.path.is_ident("semantic_source") {
                    semantic_source = Some(meta.value()?.parse()?);
                } else if meta.path.is_ident("family") {
                    family = Some(meta.value()?.parse()?);
                } else {
                    return Err(meta
                        .error("expected name, validate, invariants, family or semantic_source"));
                }
                Ok(())
            })?;
        }
    }
    let table = table
        .ok_or_else(|| syn::Error::new_spanned(&input.ident, "model(name = \"…\") is required"))?;
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input,
            "Domain requires a named record; model sum values separately",
        ));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(syn::Error::new_spanned(
            &data.fields,
            "Domain requires named fields",
        ));
    };
    let mut keys = Vec::new();
    let mut key_types = Vec::new();
    let mut names = Vec::new();
    let mut types = Vec::new();
    let mut descriptors = Vec::new();
    let mut premises = Vec::new();
    let mut premise_types = Vec::new();
    for field in &fields.named {
        let name = field.ident.as_ref().expect("named fields");
        if name == "id" || name == "generation_id" {
            return Err(syn::Error::new_spanned(
                name,
                "identity and generation columns are generated",
            ));
        }
        let ty = &field.ty;
        let mut key = false;
        let mut provenance = false;
        for attr in &field.attrs {
            if attr.path().is_ident("model") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("key") {
                        key = true;
                    } else if meta.path.is_ident("premise") {
                        premises.push(name);
                        premise_types.push(ty);
                    } else if meta.path.is_ident("provenance") {
                        provenance = true;
                    } else {
                        return Err(meta.error("expected key or provenance"));
                    }
                    Ok(())
                })?;
            }
        }
        if key {
            keys.push(name);
            key_types.push(ty);
        }
        names.push(name);
        types.push(ty);
        descriptors.push(quote! {
            ::lctx_model::domain::Field::of::<#ty>(stringify!(#name), #key, #provenance)
        });
    }
    if keys.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "at least one semantic key field is required",
        ));
    }
    if rule.is_none() && (conclusion.is_some() || !premises.is_empty()) {
        return Err(syn::Error::new_spanned(
            &input,
            "derivation fields require a declared rule",
        ));
    }
    let derivation = if let Some(rule) = rule {
        if premises.is_empty() {
            return Err(syn::Error::new_spanned(
                &input,
                "derivation requires typed premises",
            ));
        }
        let (conclusion_metadata, conclusion_row) = if let Some(column) = conclusion {
            let field = fields
                .named
                .iter()
                .find(|f| f.ident.as_ref() == Some(&column))
                .ok_or_else(|| syn::Error::new_spanned(&column, "unknown derivation conclusion"))?;
            let ty = &field.ty;
            (
                quote!(Some(::lctx_model::domain::derivation::ReferenceColumn::of::<#ty>(stringify!(#column)))),
                quote!(::lctx_model::domain::derivation::DerivationReference::row_ref(&self.#column)?),
            )
        } else {
            (
                quote!(None),
                quote!(::lctx_model::domain::derivation::RowRef::of(
                    <Self as ::lctx_model::domain::Record>::id(self)
                )),
            )
        };
        quote! {
            fn derivation() -> Option<::lctx_model::domain::derivation::Derivation> {
                Some(::lctx_model::domain::derivation::Derivation { rule: #rule,conclusion: #conclusion_metadata,
                    premises: vec![#(::lctx_model::domain::derivation::ReferenceColumn::of::<#premise_types>(stringify!(#premises)),)*] })
            }
            fn proof(&self) -> Option<::lctx_model::domain::derivation::Proof> {
                Some(::lctx_model::domain::derivation::Proof { source: ::lctx_model::domain::derivation::RowRef::of(<Self as ::lctx_model::domain::Record>::id(self)),conclusion: #conclusion_row,
                    premises: [#(::lctx_model::domain::derivation::DerivationReference::row_ref(&self.#premises),)*].into_iter().flatten().collect() })
            }
        }
    } else {
        quote!()
    };
    let family_fn = family.map(|family| {
        quote! {
            fn family() -> Option<::lctx_model::domain::attribution::FactFamily> { Some(#family) }
        }
    });
    let declaration = quote!(#input).to_string();
    let name = &input.ident;
    let key_name = format_ident!("{}Key", name);
    let physical = format_ident!("__{}Physical", name);
    let physical_ref = format_ident!("__{}PhysicalRef", name);
    let vis = &input.vis;
    let semantic_source = semantic_source
        .map(|expr| quote!(#expr))
        .unwrap_or_else(|| quote!(b""));
    let validation = validator.map(|v| quote! { #v(self)?; });
    let invariants = invariants
        .map(|v| quote! { #v() })
        .unwrap_or_else(|| quote! { Vec::new() });
    let required_support = required_support.map(|support| quote! {
        vec![(::std::any::TypeId::of::<#support>(), <#support as ::lctx_model::domain::Record>::NAME)]
    }).unwrap_or_else(|| quote! { Vec::new() });
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
        #[derive(::lctx_model::domain::__private::serde::Serialize)]
        #[serde(crate = "::lctx_model::domain::__private::serde")]
        struct #physical_ref<'a> {
            id: ::lctx_model::domain::Id<#name>,
            #(#names: &'a #types,)*
        }
        impl ::lctx_model::domain::HeapSize for #name {
            fn heap_bytes(&self) -> usize {
                0usize #(.saturating_add(::lctx_model::domain::HeapSize::heap_bytes(&self.#names)))*
            }
        }
        impl ::lctx_model::domain::Record for #name {
            #derivation
            type Key = #key_name;
            const NAME: &'static str = #table;
            const CONTRACT: &'static str = #declaration;
            const OWNER: &'static str = env!("CARGO_PKG_NAME");
            const SEMANTIC_SOURCE: &'static [u8] = #semantic_source;
            fn key(&self) -> Self::Key { #key_name { #(#keys: self.#keys.clone(),)* } }
            fn write_key(&self, sink: &mut ::lctx_model::domain::KeySink) {
                #(::lctx_model::domain::Key::encode(&self.#keys, sink);)*
            }
            fn fields() -> Vec<::lctx_model::domain::Field> { vec![#(#descriptors,)*] }
            fn invariants() -> Vec<::lctx_model::domain::Invariant> { #invariants }
            fn required_relations() -> Vec<(::std::any::TypeId, &'static str)> { #required_support }
            #family_fn
            fn content_digest(&self) -> ::lctx_model::domain::ContentHash {
                let mut sink = ::lctx_model::domain::KeySink::new(Self::NAME);
                #(::lctx_model::domain::Key::encode(&self.#names, &mut sink);)*
                sink.finish()
            }

            fn validate(&self) -> Result<(), ::lctx_model::domain::ModelError> {
                #validation
                Ok(())
            }
            fn encode(rows: &[Self]) -> Result<::lctx_model::domain::__private::RecordBatch, ::lctx_model::domain::ModelError> {
                let mut builder = ::lctx_model::domain::__private::serde_arrow::ArrayBuilder::from_arrow(Self::schema().fields())
                    .map_err(::lctx_model::domain::ModelError::codec)?;
                builder.reserve(rows.len());
                for row in rows {
                    let physical = #physical_ref {
                        id: <Self as ::lctx_model::domain::Record>::id(row),
                        #(#names: &row.#names,)*
                    };
                    builder.push(&physical).map_err(::lctx_model::domain::ModelError::codec)?;
                }
                builder.into_record_batch().map_err(::lctx_model::domain::ModelError::codec)
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
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input,
            "DomainCode requires an enum",
        ));
    };
    let name = &input.ident;
    let mut variants = Vec::new();
    let mut codes = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for variant in &data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(syn::Error::new_spanned(
                variant,
                "codebook variants have no payload",
            ));
        }
        let Some((
            _,
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(code),
                ..
            }),
        )) = &variant.discriminant
        else {
            return Err(syn::Error::new_spanned(
                variant,
                "codebook requires an explicit nonnegative i16 code",
            ));
        };
        let value = code.base10_parse::<i16>()?;
        if !seen.insert(value) {
            return Err(syn::Error::new_spanned(code, "duplicate code"));
        }
        variants.push(&variant.ident);
        codes.push(value);
    }
    Ok(quote! {
        impl ::lctx_model::domain::FlatValue for #name {}
        impl ::lctx_model::domain::HeapSize for #name {}
        impl ::lctx_model::domain::FieldValue for #name {
            const SCALAR: ::lctx_model::domain::Scalar = ::lctx_model::domain::Scalar::Int16;
            fn codes() -> &'static [(i16, &'static str)] { &[#((#codes, stringify!(#variants)),)*] }
        }
        impl ::lctx_model::domain::Codebook for #name {
            fn code(&self) -> i16 { match self { #(Self::#variants => #codes,)* } }
            fn from_code(code: i16) -> Option<Self> { match code { #(#codes => Some(Self::#variants),)* _ => None } }
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
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input,
            "DomainSum requires an enum",
        ));
    };
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "domain sums are concrete",
        ));
    }
    let mut table = None;
    let mut validator: Option<syn::Path> = None;
    let mut invariants: Option<syn::Path> = None;
    let mut semantic_source: Option<syn::Expr> = None;
    for attr in &input.attrs {
        if attr.path().is_ident("model") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") {
                    table = Some(meta.value()?.parse::<LitStr>()?);
                    Ok(())
                } else if meta.path.is_ident("semantic_source") {
                    semantic_source = Some(meta.value()?.parse()?);
                    Ok(())
                } else if meta.path.is_ident("validate") {
                    validator = Some(meta.value()?.parse()?);
                    Ok(())
                } else if meta.path.is_ident("invariants") {
                    invariants = Some(meta.value()?.parse()?);
                    Ok(())
                } else {
                    Err(meta.error("expected name, semantic_source, validate or invariants"))
                }
            })?;
        }
    }
    let table = table
        .ok_or_else(|| syn::Error::new_spanned(&input.ident, "model(name = …) is required"))?;
    let semantic_source = semantic_source
        .map(|expr| quote!(#expr))
        .unwrap_or_else(|| quote!(b""));
    let validation = validator.map(|v| quote! { #v(self)?; });
    let invariant_creation = invariants
        .map(|v| quote! { #v() })
        .unwrap_or_else(|| quote! { Vec::new() });
    let name = &input.ident;
    let physical = format_ident!("__{}Physical", name);
    let physical_ref = format_ident!("__{}PhysicalRef", name);
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
                    if meta.path.is_ident("code") {
                        code = Some(
                            meta.value()?
                                .parse::<syn::LitInt>()?
                                .base10_parse::<i16>()?,
                        );
                        Ok(())
                    } else {
                        Err(meta.error("expected code"))
                    }
                })?;
            }
        }
        let code = code.ok_or_else(|| {
            syn::Error::new_spanned(variant, "each arm requires an explicit model(code = N)")
        })?;
        if !seen.insert(code) {
            return Err(syn::Error::new_spanned(variant, "duplicate arm code"));
        }
        let mut members = Vec::new();
        match &variant.fields {
            Fields::Named(fields) => {
                for field in &fields.named {
                    let member = field.ident.as_ref().expect("named");
                    let column =
                        format_ident!("{}_{}", variant.ident.to_string().to_lowercase(), member);
                    let (storage_ty, optional) = optional_inner(&field.ty);
                    physical_names.push(column.clone());
                    physical_types.push(storage_ty.clone());
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
                }
            }
            Fields::Unit => {}
            _ => {
                return Err(syn::Error::new_spanned(
                    variant,
                    "sum payloads require named fields",
                ));
            }
        }
        variants.push(&variant.ident);
        codes.push(code);
        arm_fields.push(members);
    }
    let visibility = &input.vis;
    let aliases: Vec<_> = variants
        .iter()
        .map(|variant| format_ident!("{}{}Id", name, variant))
        .collect();
    let mut key_arms = Vec::new();
    let mut heap_arms = Vec::new();
    let mut encode_arms = Vec::new();
    let mut decode_arms = Vec::new();
    let mut sum_arms = Vec::new();
    for ((variant, code), fields) in variants.iter().zip(&codes).zip(&arm_fields) {
        let names: Vec<_> = fields.iter().map(|(name, _, _)| name).collect();
        key_arms.push(quote! { Self::#variant { #(#names,)* } => {
            ::lctx_model::domain::Key::encode(&(#code as i16), sink);
            #(::lctx_model::domain::Key::encode(#names, sink);)*
        }});
        heap_arms.push(quote! { Self::#variant { #(#names,)* } => 0usize #(.saturating_add(::lctx_model::domain::HeapSize::heap_bytes(#names)))* });
        let values: Vec<_> = physical_names
            .iter()
            .map(|column| {
                if let Some((member, _, optional)) = fields.iter().find(|(_, c, _)| c == column) {
                    if *optional {
                        quote! { #column: #member.as_ref() }
                    } else {
                        quote! { #column: Some(#member) }
                    }
                } else {
                    quote! { #column: None }
                }
            })
            .collect();
        encode_arms.push(quote! { Self::#variant { #(#names,)* } => #physical_ref { id: Self::id(row), kind: #code, #(#values,)* } });
        let inactive: Vec<_> = physical_names
            .iter()
            .filter(|column| !fields.iter().any(|(_, c, _)| c == *column))
            .collect();
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
        sum_arms.push(
            quote! { ::lctx_model::domain::Arm { code: #code, fields: vec![#(#requirements,)*] } },
        );
    }
    let borrow_lifetime = if physical_names.is_empty() {
        quote! {}
    } else {
        quote! { <'a> }
    };
    Ok(quote! {
        #(#visibility type #aliases = ::lctx_model::domain::ArmId<#name, #codes>;)*
        impl ::lctx_model::domain::SumRecord for #name {
            fn tag(&self) -> i16 { match self { #(Self::#variants { .. } => #codes,)* } }
        }
        #[derive(::lctx_model::domain::__private::serde::Serialize, ::lctx_model::domain::__private::serde::Deserialize)]
        #[serde(crate = "::lctx_model::domain::__private::serde")]
        struct #physical { id: ::lctx_model::domain::Id<#name>, kind: i16, #(#physical_names: Option<#physical_types>,)* }
        #[derive(::lctx_model::domain::__private::serde::Serialize)]
        #[serde(crate = "::lctx_model::domain::__private::serde")]
        struct #physical_ref #borrow_lifetime { id: ::lctx_model::domain::Id<#name>, kind: i16, #(#physical_names: Option<&'a #physical_types>,)* }
        impl ::lctx_model::domain::Key for #name {
            fn encode(&self, sink: &mut ::lctx_model::domain::KeySink) { match self { #(#key_arms,)* } }
        }
        impl ::lctx_model::domain::HeapSize for #name {
            fn heap_bytes(&self) -> usize { match self { #(#heap_arms,)* } }
        }
        impl ::lctx_model::domain::Record for #name {
            type Key = Self;
            const NAME: &'static str = #table;
            const CONTRACT: &'static str = #declaration;
            const OWNER: &'static str = env!("CARGO_PKG_NAME");
            const SEMANTIC_SOURCE: &'static [u8] = #semantic_source;
            fn key(&self) -> Self { self.clone() }
            fn write_key(&self, sink: &mut ::lctx_model::domain::KeySink) { ::lctx_model::domain::Key::encode(self,sink); }
            fn content_digest(&self) -> ::lctx_model::domain::ContentHash {
                let mut sink = ::lctx_model::domain::KeySink::new(Self::NAME);
                ::lctx_model::domain::Key::encode(self, &mut sink);
                sink.finish()
            }

            fn fields() -> Vec<::lctx_model::domain::Field> { vec![::lctx_model::domain::Field::of::<i16>("kind", true, false), #(#descriptors,)*] }
            fn sum() -> Option<::lctx_model::domain::Sum> { Some(::lctx_model::domain::Sum { tag: "kind", arms: vec![#(#sum_arms,)*] }) }
            fn validate(&self) -> Result<(), ::lctx_model::domain::ModelError> { #validation Ok(()) }
            fn invariants() -> Vec<::lctx_model::domain::Invariant> { #invariant_creation }
            fn encode(rows: &[Self]) -> Result<::lctx_model::domain::__private::RecordBatch, ::lctx_model::domain::ModelError> {
                let mut builder = ::lctx_model::domain::__private::serde_arrow::ArrayBuilder::from_arrow(Self::schema().fields())
                    .map_err(::lctx_model::domain::ModelError::codec)?;
                builder.reserve(rows.len());
                for row in rows {
                    let physical = match row { #(#encode_arms,)* };
                    builder.push(&physical).map_err(::lctx_model::domain::ModelError::codec)?;
                }
                builder.into_record_batch().map_err(::lctx_model::domain::ModelError::codec)
            }
            fn decode(batch: &::lctx_model::domain::__private::RecordBatch) -> Result<Vec<Self>, ::lctx_model::domain::ModelError> {
                if batch.schema().as_ref() != Self::schema().as_ref() { return Err(::lctx_model::domain::ModelError::Schema(Self::NAME)); }
                let physical: Vec<#physical> = ::lctx_model::domain::__private::serde_arrow::from_record_batch(batch).map_err(::lctx_model::domain::ModelError::codec)?;
                physical.into_iter().map(|physical| {
                    let row = match physical.kind { #(#decode_arms,)* _ => return Err(::lctx_model::domain::ModelError::Invalid("unknown sum tag".into())) };
                    row.validate()?;
                    if row.id() != physical.id { return Err(::lctx_model::domain::ModelError::Identity(Self::NAME)); }
                    Ok(row)
                }).collect()
            }
        }
    })
}
fn optional_inner(ty: &syn::Type) -> (syn::Type, bool) {
    if let syn::Type::Path(path) = ty
        && let Some(segment) = path.path.segments.last()
        && segment.ident == "Option"
        && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
    {
        return (inner.clone(), true);
    }
    (ty.clone(), false)
}

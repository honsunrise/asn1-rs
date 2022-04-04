use proc_macro2::{Literal, Span, TokenStream};
use quote::quote;
use syn::{
    parse_quote, Data, DataStruct, DeriveInput, Field, Ident, Lifetime, LitInt, Type,
    WherePredicate,
};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Asn1Type {
    Ber,
    Der,
}

pub fn derive_ber_sequence(s: synstructure::Structure) -> proc_macro2::TokenStream {
    let ast = s.ast();

    let container = match &ast.data {
        Data::Struct(ds) => Container::from_datastruct(ds, ast),
        _ => panic!("Unsupported type, cannot derive"),
    };

    let debug_derive = ast.attrs.iter().any(|attr| {
        attr.path
            .is_ident(&Ident::new("debug_derive", Span::call_site()))
    });

    let impl_tryfrom = container.gen_tryfrom();
    let impl_tagged = container.gen_tagged();
    let ts = s.gen_impl(quote! {
        extern crate asn1_rs;

        #impl_tryfrom
        #impl_tagged
    });
    if debug_derive {
        eprintln!("{}", ts.to_string());
    }
    ts
}

pub fn derive_der_sequence(s: synstructure::Structure) -> proc_macro2::TokenStream {
    let ast = s.ast();

    let container = match &ast.data {
        Data::Struct(ds) => Container::from_datastruct(ds, ast),
        _ => panic!("Unsupported type, cannot derive"),
    };

    let debug_derive = ast.attrs.iter().any(|attr| {
        attr.path
            .is_ident(&Ident::new("debug_derive", Span::call_site()))
    });

    let impl_tryfrom = container.gen_tryfrom();
    let impl_tagged = container.gen_tagged();
    let impl_checkconstraints = container.gen_checkconstraints();
    let impl_fromder = container.gen_fromder();
    let ts = s.gen_impl(quote! {
        extern crate asn1_rs;

        #impl_tryfrom
        #impl_tagged
        #impl_checkconstraints
        #impl_fromder
    });
    if debug_derive {
        eprintln!("{}", ts.to_string());
    }
    ts
}

pub struct Container {
    pub fields: Vec<FieldInfo>,
    pub where_predicates: Vec<WherePredicate>,
}

impl Container {
    pub fn from_datastruct(ds: &DataStruct, ast: &DeriveInput) -> Self {
        let fields = ds.fields.iter().map(FieldInfo::from).collect();

        // get lifetimes from generics
        let lfts: Vec<_> = ast.generics.lifetimes().collect();
        let mut where_predicates = Vec::new();
        if !lfts.is_empty() {
            // input slice must outlive all lifetimes from Self
            let lft = Lifetime::new("'ber", Span::call_site());
            let wh: WherePredicate = parse_quote! { #lft: #(#lfts)+* };
            where_predicates.push(wh);
        };

        Container {
            fields,
            where_predicates,
        }
    }

    pub fn gen_tryfrom(&self) -> TokenStream {
        let field_names = &self.fields.iter().map(|f| &f.name).collect::<Vec<_>>();
        let parse_content = derive_ber_sequence_content(&self.fields, Asn1Type::Ber);
        let lifetime = Lifetime::new("'ber", Span::call_site());
        let wh = &self.where_predicates;
        // note: `gen impl` in synstructure takes care of appending extra where clauses if any, and removing
        // the `where` statement if there are none.
        quote! {
            use asn1_rs::{Any, FromBer};
            use core::convert::TryFrom;

            gen impl<#lifetime> TryFrom<Any<#lifetime>> for @Self where #(#wh)+* {
                type Error = asn1_rs::Error;

                fn try_from(any: Any<#lifetime>) -> asn1_rs::Result<Self> {
                    any.tag().assert_eq(Self::TAG)?;

                    // no need to parse sequence, we already have content
                    let i = any.data;
                    //
                    #parse_content
                    //
                    let _ = i; // XXX check if empty?
                    Ok(Self{#(#field_names),*})
                }
            }
        }
    }

    pub fn gen_tagged(&self) -> TokenStream {
        quote! {
            gen impl<'ber> asn1_rs::Tagged for @Self {
                const TAG: asn1_rs::Tag = asn1_rs::Tag::Sequence;
            }
        }
    }

    pub fn gen_checkconstraints(&self) -> TokenStream {
        let lifetime = Lifetime::new("'ber", Span::call_site());
        let wh = &self.where_predicates;
        // let parse_content = derive_ber_sequence_content(&field_names, Asn1Type::Der);
        let check_fields: Vec<_> = self
            .fields
            .iter()
            .map(|field| {
                let ty = &field.type_;
                quote! {
                    let (rem, any) = Any::from_der(rem)?;
                    <#ty as CheckDerConstraints>::check_constraints(&any)?;
                }
            })
            .collect();
        // note: `gen impl` in synstructure takes care of appending extra where clauses if any, and removing
        // the `where` statement if there are none.
        quote! {
            use asn1_rs::{CheckDerConstraints, Tagged};
            gen impl<#lifetime> CheckDerConstraints for @Self where #(#wh)+* {
                fn check_constraints(any: &Any) -> asn1_rs::Result<()> {
                    any.tag().assert_eq(Self::TAG)?;
                    let rem = &any.data;
                    #(#check_fields)*
                    Ok(())
                }
            }
        }
    }

    pub fn gen_fromder(&self) -> TokenStream {
        let lifetime = Lifetime::new("'ber", Span::call_site());
        let wh = &self.where_predicates;
        let field_names = &self.fields.iter().map(|f| &f.name).collect::<Vec<_>>();
        let parse_content = derive_ber_sequence_content(&self.fields, Asn1Type::Der);
        // note: `gen impl` in synstructure takes care of appending extra where clauses if any, and removing
        // the `where` statement if there are none.
        quote! {
            use asn1_rs::FromDer;

            gen impl<#lifetime> asn1_rs::FromDer<#lifetime> for @Self where #(#wh)+* {
                fn from_der(bytes: &#lifetime [u8]) -> asn1_rs::ParseResult<#lifetime, Self> {
                    let (rem, any) = asn1_rs::Any::from_der(bytes)?;
                    any.header.assert_tag(Self::TAG)?;
                    let i = any.data;
                    //
                    #parse_content
                    //
                    // let _ = i; // XXX check if empty?
                    Ok((rem,Self{#(#field_names),*}))
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct FieldInfo {
    pub name: Ident,
    pub type_: Type,
    pub optional: bool,
    pub tag: Option<(Asn1TagKind, u16)>,
}

impl From<&Field> for FieldInfo {
    fn from(field: &Field) -> Self {
        // parse attributes and keep supported ones
        let mut optional = false;
        let mut tag = None;
        for attr in &field.attrs {
            let ident = match attr.path.get_ident() {
                Some(ident) => ident.to_string(),
                None => continue,
            };
            match ident.as_str() {
                "optional" => optional = true,
                "tag_explicit" => {
                    if tag.is_some() {
                        panic!("tag cannot be set twice!");
                    }
                    let lit: LitInt = attr.parse_args().unwrap();
                    let value = lit.base10_parse::<u16>().unwrap();
                    tag = Some((Asn1TagKind::Explicit, value));
                }
                "tag_implicit" => {
                    if tag.is_some() {
                        panic!("tag cannot be set twice!");
                    }
                    let lit: LitInt = attr.parse_args().unwrap();
                    let value = lit.base10_parse::<u16>().unwrap();
                    tag = Some((Asn1TagKind::Implicit, value));
                }
                // ignore unknown attributes
                _ => (),
            }
        }
        FieldInfo {
            name: field.ident.clone().unwrap(),
            type_: field.ty.clone(),
            optional,
            tag,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Asn1TagKind {
    Explicit,
    Implicit,
}

fn derive_ber_sequence_content(fields: &[FieldInfo], asn1_type: Asn1Type) -> TokenStream {
    let field_parsers: Vec<_> = fields
        .iter()
        .map(|f| get_field_parser(f, asn1_type))
        .collect();

    quote! {
        #(#field_parsers)*
    }
}

fn get_field_parser(f: &FieldInfo, asn1_type: Asn1Type) -> TokenStream {
    let from = match asn1_type {
        Asn1Type::Ber => quote! {FromBer::from_ber},
        Asn1Type::Der => quote! {FromDer::from_der},
    };
    let name = &f.name;
    if let Some((tag_kind, n)) = f.tag {
        let tagged = match tag_kind {
            Asn1TagKind::Explicit => quote! { TaggedExplicit },
            Asn1TagKind::Implicit => quote! { TaggedImplicit },
        };
        let tag = Literal::u16_unsuffixed(n);
        // test if tagged + optional
        if f.optional {
            return quote! {
                let (i, #name) = {
                    if i.is_empty() {
                        (i, None)
                    } else {
                        let (_, header): (_, asn1_rs::Header) = #from(i)?;
                        if header.tag().0 == #tag {
                            let (i, t): (_, asn1_rs::#tagged::<_, _, #tag>) = #from(i)?;
                            (i, Some(t.into_inner()))
                        } else {
                            (i, None)
                        }
                    }
                };
            };
        } else {
            // tagged, but not OPTIONAL
            return quote! {
                let (i, #name) = {
                    let (i, t): (_, asn1_rs::#tagged::<_, _, #tag>) = #from(i)?;
                    (i, t.into_inner())
                };
            };
        }
    } else {
        // neither tagged nor optional
        quote! {
            let (i, #name) = #from(i)?;
        }
    }
}

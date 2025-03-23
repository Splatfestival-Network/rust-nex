mod protos;

extern crate proc_macro;

use proc_macro2::{Ident, Literal, Span, TokenTree};
use proc_macro::TokenStream;
use std::iter::FromIterator;
use std::mem;
use syn::{parse_macro_input, DeriveInput, Data, PathSegment, TraitItem, FieldsNamed, Fields, Visibility, Type, TypePath, Path, ImplItem, ImplItemConst, Expr, ExprLit, Lit, TypeParamBound, TraitBound, TraitBoundModifier, LitInt, Token, FnArg, Receiver, PatType, Pat, TypeInfer, TypeReference, TraitItemFn, Signature, Block, Stmt, Local, LocalInit, LitStr, PathArguments, ReturnType};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::buffer::TokenBuffer;
use syn::parse::{Parse, ParseBuffer, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::Visibility::Public;
use crate::protos::{ProtoMethodData, RmcProtocolData};

fn self_referece_type() -> Type {
    Type::Reference(
        TypeReference {
            and_token: Default::default(),
            lifetime: None,
            mutability: None,
            elem: Box::new(Type::Path(
                TypePath {
                    qself: None,
                    path: Path {
                        leading_colon: None,
                        segments: {
                            let mut punct = Punctuated::new();

                            punct.push_value(PathSegment{
                                ident: Ident::new("Self", Span::call_site()),
                                arguments: PathArguments::None
                            });

                            punct
                        }
                    }
                }
            ))
        }
    )
}

struct ProtoInputParams{
    proto_num: LitInt,
    properties: Option<(Token![,], Punctuated<Ident, Token![,]>)>
}

impl Parse for ProtoInputParams{
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let proto_num = input.parse()?;

        if let Some(seperator) = input.parse()?{
            let mut punctuated = Punctuated::new();
            loop {
                punctuated.push_value(
                    input.parse()?
                );
                if let Some(punct) = input.parse()? {
                    punctuated.push_punct(punct);
                } else {
                    return Ok(
                        Self{
                            proto_num,
                            properties: Some((seperator, punctuated))
                        }
                    )
                }
            }
        } else {
            Ok(
                Self{
                    proto_num,
                    properties: None
                }
            )
        }
    }
}

fn single_ident_path(ident: Ident) -> Path{
    Path{
        segments: {
            let mut punc = Punctuated::new();
            punc.push(PathSegment::from(ident));
            punc
        },
        leading_colon: None,
    }
}


#[proc_macro_derive(RmcSerialize, attributes(extends, rmc_struct))]
pub fn rmc_serialize(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);

    let struct_attr = derive_input.attrs.iter()
        .find(|a| a.path().segments.len() == 1 &&
            a.path().segments.first().is_some_and(|p| p.ident.to_string() == "rmc_struct"));

    let Data::Struct(s) = derive_input.data else {
        panic!("rmc struct type MUST be a struct");
    };

    // generate base data

    let serialize_base_content = {
        let mut serialize_content = quote! {};

        for f in &s.fields{
            if f.attrs.iter()
                .any(|a| a.path().segments.len() == 1 &&
                    a.path().segments.first().is_some_and(|p| p.ident.to_string() == "extends")){
                continue;
            }
            let ident = f.ident.as_ref().unwrap();

            serialize_content.append_all(quote!{
                self.#ident.serialize(writer)?;
            })
        }

        quote!{
            #serialize_content

            Ok(())
        }
    };

    let struct_ctor = {
        let mut structure_content = quote! {};
        for f in &s.fields {
            let ident = f.ident.as_ref().unwrap();

            structure_content.append_all(quote!{#ident, });
        }

        quote!{
            Ok(Self{
                #structure_content
            })
        }
    };

    let deserialize_base_content = {
        let mut deserialize_content = quote! {};

        for f in &s.fields{
            if f.attrs.iter()
                .any(|a| a.path().segments.len() == 1 &&
                    a.path().segments.first().is_some_and(|p| p.ident.to_string() == "extends")){
                continue;
            }

            let ident = f.ident.as_ref().unwrap();
            let ty = &f.ty;

            deserialize_content.append_all(quote!{
                let #ident = <#ty> :: deserialize(reader)?;
            })
        }

        quote!{
            #deserialize_content
            #struct_ctor
        }
    };

    // generate base with extends stuff

    let serialize_base_content = if let Some(attr) = struct_attr{
        let version: Literal = attr.parse_args().expect("has to be a literal");

        let pre_inner = if let Some(f) = s.fields.iter().find(|f| {
            f.attrs.iter()
                .any(|a| a.path().segments.len() == 1 &&
                    a.path().segments.first().is_some_and(|p| p.ident.to_string() == "extends"))
        }){
            let ident= f.ident.as_ref().unwrap();
            quote! {
                self.#ident.serialize(writer)?;
            }
        } else {
            quote! {}
        };

        quote! {
            #pre_inner
            crate::rmc::structures::rmc_struct::write_struct(writer, #version, |mut writer|{
                #serialize_base_content
            })?;

            Ok(())
        }
    } else {
        serialize_base_content
    };

    let deserialize_base_content = if let Some(attr) = struct_attr{
        let version: Literal = attr.parse_args().expect("has to be a literal");

        let pre_inner = if let Some(f) = s.fields.iter().find(|f| {
            f.attrs.iter()
                .any(|a| a.path().segments.len() == 1 &&
                    a.path().segments.first().is_some_and(|p| p.ident.to_string() == "extends"))
        }){
            let ident= f.ident.as_ref().unwrap();
            let ty= &f.ty;
            quote! {
                let #ident = <#ty> :: deserialize(reader)?;
            }
        } else {
            quote! {}
        };

        quote! {
            #pre_inner
            Ok(crate::rmc::structures::rmc_struct::read_struct(reader, #version, move |mut reader|{
                #deserialize_base_content
            })?)
        }
    } else {
        deserialize_base_content
    };

    let ident = derive_input.ident;

    let tokens = quote! {
        impl crate::rmc::structures::RmcSerialize for #ident{
            fn serialize(&self, writer: &mut dyn ::std::io::Write) -> crate::rmc::structures::Result<()>{
                #serialize_base_content


            }

            fn deserialize(reader: &mut dyn ::std::io::Read) -> crate::rmc::structures::Result<Self>{
                #deserialize_base_content
            }
        }
    };

    tokens.into()
}

/// Macro to automatically generate code to use a specific trait as an rmc protocol for calling to
/// remote objects or accepting incoming remote requests.
/// This is needed in order to be able to use this as part of an rmc server interface.
///
/// The protocol id which is needed to be specified is specified as a parameter to this attribute.
///
/// You will also need to assign each function inside the trait a method id by using the
/// [`macro@method_id`] attribute.
///
/// You can also specify to have the protocol to be non-returning by adding a second parameter to
/// the attribute which is just `NoReturn` e.g. ` #[rmc_proto(1, NoReturn)]`
///
/// Example
/// ```
/// // this rmc protocol has protocol id 1
/// use macros::rmc_proto;
///
/// #[rmc_proto(1)]
/// trait ExampleProtocol{
///     // this defines an rmc method with id 1
///     #[rmc_method(1)]
///     async fn hello_world_method(&self, name: String) -> Result<String, ErrorCode>;
/// }
/// ```
#[proc_macro_attribute]
pub fn rmc_proto(attr: TokenStream, input: TokenStream) -> TokenStream{

    let mut params = parse_macro_input!(attr as ProtoInputParams);

    let ProtoInputParams{
        proto_num,
        properties
    } = params;

    let no_return_data = properties.is_some_and(|p| p.1.iter().any(|i|{
        i.to_string() == "NoReturn"
    }));

    let mut input = parse_macro_input!(input as syn::ItemTrait);

    // gigantic ass struct initializer (to summarize this gets all of the data)
    let raw_data = RmcProtocolData{
        has_returns: !no_return_data,
        name: input.ident.clone(),
        id: proto_num,
        methods: input
            .items
            .iter()
            .filter_map(|v| match v{ TraitItem::Fn(v) => Some(v), _ => None })
            .map(|func|{
                let Some(attr) = func.attrs.iter()
                    .find(|a| a.path().segments.last().is_some_and(|s| s.ident.to_string() == "method_id")) else {
                    panic!( "every function inside of an rmc protocol must have a method id");
                };

                let Ok(id): Result<LitInt, _> = attr.parse_args() else {
                    panic!("todo: put a propper error message here");
                };

                let funcs = func
                    .sig
                    .inputs
                    .iter()
                    .skip(1)
                    .map(|f| {
                        let FnArg::Typed(t) = f else {
                            panic!("what");
                        };
                        let Pat::Ident(i) = &*t.pat else {
                            panic!("unable to handle non identifier patterns as parameter bindings");
                        };

                        (i.ident.clone(), t.ty.as_ref().clone())
                    }).collect();

                ProtoMethodData{
                    id,
                    name: func.sig.ident.clone(),
                    parameters: funcs
                }
            }).collect()

    };

    quote!{
        #input
        #raw_data
    }.into()
}

/// Used to specify the method id of methods when making rmc protocols.
/// See [`macro@rmc_proto`] for further details.
///
/// Note: This attribute doesn't do anything by itself and just returns the thing it was attached to
/// unchanged.
#[proc_macro_attribute]
pub fn method_id(_attr: TokenStream, input: TokenStream) -> TokenStream{
    // this attribute doesnt do anything by itself, see `rmc_proto`
    input
}



#[proc_macro_attribute]
pub fn rmc_struct(attr: TokenStream, input: TokenStream) -> TokenStream{
    let mut type_data = parse_macro_input!(input as DeriveInput);
    let mut ident = parse_macro_input!(attr as syn::Path);
    let last_token = ident.segments.last_mut().expect("empty path?");

    last_token.ident = Ident::new(&("Local".to_owned() + &last_token.ident.to_string()), last_token.span());


    let struct_name = &type_data.ident;

    let out = quote!{
        #type_data

        impl #ident for #struct_name{

        }

        impl crate::rmc::protocols::RmcCallable for #struct_name{
            async fn rmc_call(&self, remote_response_connection: &crate::prudp::socket::SendingConnection, protocol_id: u16, method_id: u32, call_id: u32, rest: Vec<u8>){
                <Self as #ident>::rmc_call(self, remote_response_connection, protocol_id, method_id, call_id, rest).await;
            }
        }
    };

    out.into()
}

#[proc_macro_attribute]
pub fn connection(_attr: TokenStream, input: TokenStream) -> TokenStream{
    // this attribute doesnt do anything by itself, see `rmc_struct`
    input
}
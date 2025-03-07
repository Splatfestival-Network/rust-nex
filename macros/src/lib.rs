extern crate proc_macro;

use proc_macro2::{Ident, Literal, Span, TokenTree};
use proc_macro::TokenStream;
use std::iter::FromIterator;
use std::mem;
use syn::{parse_macro_input, DeriveInput, Data, PathSegment, TraitItem, FieldsNamed, Fields, Visibility, Type, TypePath, Path, ImplItem, ImplItemConst, Expr, ExprLit, Lit, TypeParamBound, TraitBound, TraitBoundModifier, LitInt, Token, FnArg, Receiver, PatType, Pat, TypeInfer, TypeReference, TraitItemFn, Signature, Block, Stmt, Local, LocalInit, LitStr, PathArguments};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::buffer::TokenBuffer;
use syn::parse::{Parse, ParseBuffer, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::Visibility::Public;

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


/// Example of user-defined [derive mode macro][1]
///
/// [1]: https://doc.rust-lang.org/reference/procedural-macros.html#derive-mode-macros
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

    let param_err_return = match no_return_data{
        true => quote!{
            return;
        },
        false => quote!{
            return Err(ErrorCode::Core_InvalidArgument);
        }
    };

    let mut input = parse_macro_input!(input as syn::ItemTrait);

    let info_struct_ident = format!("Raw{}Info",input.ident.to_string());
    let info_struct_ident = Ident::new(&info_struct_ident, input.ident.span());

    let raw_details_struct = syn::ItemStruct{
        vis: Public(Default::default()),
        struct_token: Default::default(),
        fields: Fields::Unit,
        semi_token: Some(Default::default()),
        ident: info_struct_ident.clone(),
        attrs: vec![],
        generics: Default::default()
    };

    let raw_details_impl_block = syn::ItemImpl{
        impl_token: Default::default(),
        generics: Default::default(),
        attrs: vec![],
        brace_token: Default::default(),
        defaultness: None,
        trait_: None,
        self_ty: Box::new(Type::Path(TypePath{
            qself: None,
            path: Path{
                segments: {
                    let mut punc = Punctuated::new();
                    punc.push(PathSegment::from(info_struct_ident));
                    punc
                },
                leading_colon: None,
            }
        })),
        unsafety: None,
        items: vec![
            ImplItem::Const(
                ImplItemConst{
                    defaultness: None,
                    semi_token: Default::default(),
                    attrs: vec![],
                    generics: Default::default(),
                    ident: Ident::new("PROTOCOL_ID", Span::call_site()),
                    vis: Public(Default::default()),
                    colon_token: Default::default(),
                    const_token: Default::default(),
                    eq_token: Default::default(),
                    expr: Expr::Lit(ExprLit{
                        attrs: vec![],
                        lit: Lit::Int(proto_num),
                    }),
                    ty: Type::Path(TypePath{
                        qself: None,
                        path: Path{
                            segments: {
                                let mut punc = Punctuated::new();
                                punc.push(PathSegment::from(Ident::new("u16", Span::call_site())));
                                punc
                            },
                            leading_colon: None,
                        }
                    })
                }
            )
        ]
    };

    let mut raw_trait = input.clone();

    raw_trait.ident = Ident::new(&format!("Raw{}",raw_trait.ident.to_string()), raw_trait.ident.span());
    raw_trait.colon_token = Some(Default::default());
    raw_trait.supertraits = {
        let mut punct = Punctuated::new();
        punct.push(TypeParamBound::Trait(TraitBound{
            path: single_ident_path(input.ident.clone()),
            lifetimes: None,
            modifier: TraitBoundModifier::None,
            paren_token: None
        }));
        punct
    };

    let mut functions: Vec<(LitInt, Ident)> = Vec::new();

    let funcs = raw_trait.items.iter_mut().filter_map(|v| if let TraitItem::Fn(v) = v {Some(v)} else { None });

    for func in funcs{

        if matches!(func.default, Some(_)){
            return syn::Error::new(func.default.span(), "rmc methods may not have bodies").to_compile_error().into();
        }

        let Some(attr) = func.attrs.iter()
            .find(|a| a.path().segments.last().is_some_and(|s| s.ident.to_string() == "method_id")) else {
            let span = func.sig.asyncness.span().join(func.semi_token.unwrap().span()).unwrap_or(func.sig.span());
            return syn::Error::new(span, "every function inside of an rmc protocol must have a method id").to_compile_error().into();
        };

        let Ok(func_id): Result<LitInt, _> = attr.parse_args() else {
            return syn::Error::new(Span::call_site(), "todo: put a propper error message here").to_compile_error().into();
        };


        if !func.sig.inputs.first().is_some_and(|v| matches!(v, FnArg::Receiver(Receiver{
            colon_token: None,
            mutability: None,
            reference: Some(_),
            ..
        }))){
            return syn::Error::new(func.sig.inputs.span(), "every protocol function must have a ` & self ` as its first parameter.").to_compile_error().into();
        }

        let old_ident = func.sig.ident.clone();

        func.sig.ident = Ident::new(&format!("raw_{}", func.sig.ident), func.sig.ident.span());

        let mut new_params: Punctuated<_,_> = Punctuated::new();

        new_params.push_value(FnArg::Receiver(Receiver{
            attrs: vec![],
            mutability: None,
            ty: Box::new(self_referece_type()),
            colon_token: None,
            self_token: Default::default(),
            reference: Some((Default::default(), None))
        }));

        new_params.push_punct(Comma::default());
/*
        new_params.push_value(FnArg::Typed(PatType{
            attrs: vec![],
            pat: Box::new(Pat::Verbatim(quote! { method_id })),
            colon_token: Default::default(),
            ty: Box::new(Type::Verbatim(quote!{u32}))
        }));

        new_params.push_punct(Comma::default());*/

        new_params.push_value(FnArg::Typed(PatType{
            attrs: vec![],
            pat: Box::new(Pat::Verbatim(quote! {data})),
            colon_token: Default::default(),
            ty: Box::new(Type::Verbatim(quote!{::std::vec::Vec<u8>}))
        }));

        mem::swap(&mut new_params, &mut func.sig.inputs);
        let old_params = new_params;


        let mut inner_raw_tokens = quote!{
            let mut cursor = ::std::io::Cursor::new(data);
        };

        let mut call_params = quote!{};


        for param in old_params.iter().skip(1).filter_map(|v| if let FnArg::Typed(t) = v {
            Some(t)
        } else {
            None
        }){
            let param_name = &*param.pat;
            let ty = &*param.ty;

            inner_raw_tokens.append_all(quote!{
                let Ok(#param_name) = <#ty as crate::rmc::structures::RmcSerialize>::deserialize(&mut cursor) else {
                    #param_err_return
                };
            });

            call_params.append_all(quote!{#param_name ,})
        }

        inner_raw_tokens.append_all(quote!{
            let retval = self.#old_ident(#call_params).await;
        });

        if !no_return_data{
            //let

            //inner_raw_tokens.append_all()
        }


        let braced = quote! {
            {
                #inner_raw_tokens
            }
        };

        let braced = braced.into();

        func.default = Some(
            parse_macro_input!(braced as Block)
        );

        functions.push((func_id, func.sig.ident.clone()));
    }

    let mut inner_match = quote!{};

    for toks in functions.iter().map(|(lit, ident)|{
        quote! {
            #lit => self.#ident(data).await,
        }
    }){
        inner_match.append_all(toks);
    }

    if no_return_data{
        inner_match.append_all(quote!{
            _ => return
        })
    } else {
        //
        inner_match.append_all(quote!{
            _ => return
        })
    }

    raw_trait.items.push(
        TraitItem::Verbatim(
            quote!{
                async fn rmc_call_proto(&self, method_id: u32, data: Vec<u8>){
                    match method_id{
                        #inner_match
                    }
                }
            }
        )
    );



    let regular_trait_name = &input.ident;
    let raw_trait_name = &raw_trait.ident;

    quote!{
        #input
        #raw_details_struct
        #raw_details_impl_block
        #raw_trait

        impl<T: #regular_trait_name + crate::rmc::protocols::ImplementRemoteCalls> #raw_trait_name for T{}
    }.into()

}


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
            async fn rmc_call(&self, protocol_id: u16, method_id: u32, rest: Vec<u8>){
                <Self as #ident>::rmc_call(self, protocol_id, method_id, rest).await;
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
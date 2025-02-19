extern crate proc_macro;

use proc_macro2::{Ident, Literal, Span, TokenTree};
use proc_macro::TokenStream;
use std::iter::FromIterator;
use syn::{parse_macro_input, DeriveInput, Data, PathSegment, TraitItem, FieldsNamed, Fields, Visibility, Type, TypePath, Path, ImplItem, ImplItemConst, Expr, ExprLit, Lit};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::Visibility::Public;

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
    let mut proto_num = parse_macro_input!(attr as syn::LitInt);

    let mut input = parse_macro_input!(input as syn::ItemTrait);

    let info_struct_ident = format!("Raw{}Info",input.ident.to_string());
    let info_struct_ident = Ident::new(&info_struct_ident, input.ident.span());

    let raw_details_struct = syn::ItemStruct{
        vis: Visibility::Public(Default::default()),
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

    let funcs = input.items.iter().filter_map(|v| if let TraitItem::Fn(v) = v {Some(v)} else { None });

    for func in funcs{
        if matches!(func.default, Some(_)){
            return syn::Error::new(func.default.span(), "rmc methods may not have bodies").to_compile_error().into();
        }

        let Some(attr) = func.attrs.iter()
            .find(|a| a.path().segments.last().is_some_and(|s| s.ident.to_string() == "method_id")) else {
            let span = func.sig.asyncness.span().join(func.semi_token.unwrap().span()).unwrap_or(func.sig.span());
            return syn::Error::new(span, "every function inside of an rmc protocol must have a method id").to_compile_error().into();
        };

        todo!("generate raw impl")
    }

    quote!{
        #input
        #raw_details_struct
        #raw_details_impl_block
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
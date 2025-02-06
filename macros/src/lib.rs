extern crate proc_macro;

use proc_macro2::TokenTree;
use proc_macro::TokenStream;

use syn::{parse_macro_input, DeriveInput, Data};
use quote::{quote, TokenStreamExt};


/// Example of user-defined [derive mode macro][1]
///
/// [1]: https://doc.rust-lang.org/reference/procedural-macros.html#derive-mode-macros
#[proc_macro_derive(RmcSerialize, attributes(extends, rmc_struct))]
pub fn rmc_serialize(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);

    let struct_attr = derive_input.attrs.iter()
        .find(|a| a.path.segments.len() == 1 &&
            a.path.segments.first().is_some_and(|p| p.ident.to_string() == "rmc_struct"));

    let Data::Struct(s) = derive_input.data else {
        panic!("rmc struct type MUST be a struct");
    };

    // generate base data

    let serialize_base_content = {
        let mut serialize_content = quote! {};

        for f in &s.fields{
            if f.attrs.iter()
                .any(|a| a.path.segments.len() == 1 &&
                    a.path.segments.first().is_some_and(|p| p.ident.to_string() == "extends")){
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
                .any(|a| a.path.segments.len() == 1 &&
                    a.path.segments.first().is_some_and(|p| p.ident.to_string() == "extends")){
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
        let tokens = attr.tokens.clone();
        let token = tokens.into_iter().next().unwrap();

        let version = match token {
            TokenTree::Group(g) => {
                match g.stream().into_iter().next().unwrap(){
                    TokenTree::Literal(l) => l,
                    _ => panic!("expected literal")
                }
            },
            _ => panic!("expected group")
        };

        let pre_inner = if let Some(f) = s.fields.iter().find(|f| {
            f.attrs.iter()
                .any(|a| a.path.segments.len() == 1 &&
                    a.path.segments.first().is_some_and(|p| p.ident.to_string() == "extends"))
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
        let tokens = attr.tokens.clone();
        let token = tokens.into_iter().next().unwrap();

        let version = match token {
            TokenTree::Group(g) => {
                match g.stream().into_iter().next().unwrap(){
                    TokenTree::Literal(l) => l,
                    _ => panic!("expected literal")
                }
            },
            _ => panic!("expected group")
        };
        let pre_inner = if let Some(f) = s.fields.iter().find(|f| {
            f.attrs.iter()
                .any(|a| a.path.segments.len() == 1 &&
                    a.path.segments.first().is_some_and(|p| p.ident.to_string() == "extends"))
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
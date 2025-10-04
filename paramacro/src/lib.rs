extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse::Parse, Type};

// TODO: clean up this mess.

#[proc_macro_attribute]
pub fn opcode_data(_metadata: TokenStream, input: TokenStream)
                  -> TokenStream {
    let mut input = syn::parse_macro_input!(input as syn::ItemEnum);
    let Type::Path(opcode_typename) = syn::parse_macro_input!(_metadata as syn::Type) else {
        panic!("This must be given a name!")
    };

    let data_ident = &input.ident;

    let opcode_ident = opcode_typename.path.get_ident().unwrap();

    let mut variant_idents = Vec::new();

    // add doc comments
    for variant in &mut input.variants {
        let variant_name = variant.ident.clone();
        let variant_tokens = variant.to_token_stream();
        let new_field_token_stream = if variant_name.to_string() != "Unknown" {
            quote! {
                #[br(pre_assert(*magic == #opcode_ident::#variant_name))]
                #variant_tokens
            }
        } else {
            variant_tokens
        };
        let buffer = ::syn::parse::Parser::parse2(
            syn::Variant::parse,
            new_field_token_stream,
        ).unwrap();
        *variant = buffer;
    }

    // collect idents for later
    let output;
    {
        for variant in &mut input.variants {
            // Unknown is special-cased, see below
            if variant.ident.to_string() != "Unknown" {
                variant_idents.push(&variant.ident);
            }
        }

        output = quote! {
            impl crate::packet::ReadWriteIpcOpcode<#data_ident> for #opcode_ident {
                /// Returns the opcode that's associated with the #data_ident.
                fn from_data(data: &#data_ident) -> Self {
                    match data {
                        #data_ident::Unknown { .. } => unreachable!(),
                        #(#data_ident::#variant_idents { .. } => Self::#variant_idents),*
                    }
                }
            }
        };
    }

    quote! {
        #input
        #output
    }.into_token_stream().into()
}

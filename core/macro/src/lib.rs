extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenTree;
use quote::{ToTokens, quote};
use syn::{Meta, Type, parse::Parse};

// TODO: clean up this mess.

#[proc_macro_attribute]
pub fn opcode_data(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    let mut input = syn::parse_macro_input!(input as syn::ItemEnum);
    let Type::Path(opcode_typename) = syn::parse_macro_input!(_metadata as syn::Type) else {
        panic!("This must be given a name!")
    };

    let data_ident = &input.ident;

    let opcode_ident = opcode_typename.path.get_ident().unwrap();

    let mut variant_idents = Vec::new();
    let mut default_pat_structs = Vec::new();

    // TODO: add doc comments
    for variant in &mut input.variants {
        let variant_name = variant.ident.clone();
        let variant_tokens = variant.to_token_stream();
        let new_field_token_stream = if variant_name != "Unknown" {
            quote! {
                #[br(pre_assert(*magic == #opcode_ident::#variant_name))]
                #variant_tokens
            }
        } else {
            variant_tokens
        };
        let buffer =
            ::syn::parse::Parser::parse2(syn::Variant::parse, new_field_token_stream).unwrap();
        *variant = buffer;

        // create default implementation for tests
        let mut field_idents = Vec::new();
        let mut field_defaults = Vec::new();
        let mut is_struct_variant = false;

        'outer_field: for field in &variant.fields {
            // Check if we have a #[bw(calc)], because those don't actually exist in the struct.
            for attr in &field.attrs {
                if let Meta::List(list) = &attr.meta {
                    for token in list.tokens.clone().into_iter() {
                        if let TokenTree::Ident(ident) = token
                            && ident == "calc"
                        {
                            continue 'outer_field;
                        }
                    }
                }
            }

            if let Some(ident) = &field.ident {
                field_idents.push(ident.clone());

                // Because Rust is stupid, we need to manually implement Default for >32 length arrays.
                if let Type::Array(array) = &field.ty {
                    let len = &array.len;
                    field_defaults.push(quote! { [Default::default(); #len] });
                } else {
                    field_defaults.push(quote! { Default::default() });
                }
            } else {
                is_struct_variant = true;
            }
        }

        let pat_struct_token_stream = if is_struct_variant {
            quote! {
                Self::#variant_name(Default::default())
            }
        } else {
            quote! {
                Self::#variant_name {
                    #(#field_idents: #field_defaults),*
                }
            }
        };
        default_pat_structs.push(pat_struct_token_stream);
    }

    // Push the Unknown variant
    input.variants.push_value(::syn::parse::Parser::parse2(syn::Variant::parse, quote! {
        #[doc(hidden)]
        Unknown {
            #[br(count = size - (crate::packet::IPC_HEADER_SIZE + crate::packet::PACKET_SEGMENT_HEADER_SIZE))]
            unk: Vec<u8>,
        }
    }).unwrap());

    // collect idents for later
    let output;
    {
        for variant in &mut input.variants {
            // Unknown is special-cased, see below
            if variant.ident != "Unknown" {
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

            impl crate::packet::HasUnknownData for #data_ident {
                fn unknown_size(&self) -> Option<usize> {
                    match self {
                        Self::Unknown { unk } => Some(unk.len()),
                        _ => None,
                    }
                }

                #[cfg(test)]
                fn create_default_variants() -> Vec<#data_ident> {
                    vec![
                        #(#default_pat_structs),*
                    ]
                }
            }
        };
    }

    quote! {
        #input
        #output
    }
    .into_token_stream()
    .into()
}

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};
#[proc_macro_derive(Component)]
pub fn component(input: TokenStream) -> TokenStream {
    let i = parse_macro_input!(input as DeriveInput);
    let n = i.ident;
    quote!(impl honknet_ecs::Component for #n {
    })
    .into()
}

#[proc_macro_derive(NetworkMessage)]
pub fn message(input: TokenStream) -> TokenStream {
    let i = parse_macro_input!(input as DeriveInput);
    let n = i.ident;
    quote!(impl honknet_net_core::NetworkMessage for #n {
        const ID: u16 = honknet_net_core::const_message_id(stringify!(#n));
    })
    .into()
}

#[proc_macro_derive(Reflect, attributes(field, networked))]
pub fn reflect(input: TokenStream) -> TokenStream {
    let i = parse_macro_input!(input as DeriveInput);
    let n = i.ident;
    let mut entries = Vec::new();
    if let Data::Struct(s) = i.data {
        if let Fields::Named(fs) = s.fields {
            for f in fs.named {
                let id = f.ident.unwrap();
                let ty = f.ty;
                entries.push(quote!(honknet_reflection::FieldDescriptor {
                    name: stringify!(#id),
                    type_name: stringify!(#ty),
                    networked: false,
                    min: None,
                    max: None
                }));
            }
        }
    }
    quote!(impl honknet_reflection::Reflect for #n {
        fn descriptor() -> honknet_reflection::TypeDescriptor {
            static FIELDS: &[honknet_reflection::FieldDescriptor] = &[#(#entries), *];
            honknet_reflection::TypeDescriptor {
                name: stringify!(#n), type_id: std::any::TypeId::of::<#n>(), fields: FIELDS, version: 1
            }
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }).into()
}

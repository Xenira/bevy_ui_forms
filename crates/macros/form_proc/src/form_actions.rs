use darling::{ast, FromDeriveInput, FromMeta, FromVariant};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::DeriveInput;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(form_action), supports(enum_any))]
struct FormActionsDeriveInput {
    ident: syn::Ident,
    data: ast::Data<FormActionsVariant, ()>,
    form_type: Option<syn::Ident>,
}

#[derive(Debug, FromVariant)]
#[darling(attributes(form_action))]
struct FormActionsVariant {
    ident: syn::Ident,
    fields: ast::Fields<syn::Field>,
    #[darling(default)]
    action: Action,
    text: Option<String>,
}

#[derive(FromMeta, Default, Debug)]
#[darling(default)]
enum Action {
    #[default]
    Submit,
    Apply,
    Cancel,
    Custom(String),
}

pub(crate) fn form_actions_derive(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    let input = match FormActionsDeriveInput::from_derive_input(&input) {
        Ok(input) => input,
        Err(e) => return e.write_errors().into(),
    };

    let ident = &input.ident;
    let form_type = &input.form_type.unwrap_or(format_ident!("Entity"));
    let variants = input.data.take_enum().expect("expected enum");

    let bundles = variants.iter().map(|variant| {
        let ident = &variant.ident;
        let text = variant.text.clone().unwrap_or_else(|| ident.to_string());
        match variant.action {
            Action::Submit => quote! {
                FormButtonBundle::new(#text).with_role(ButtonRole::Submit).with_form(form)
            },
            Action::Apply => quote! {
                FormButtonBundle::new(#text).with_role(ButtonRole::Apply).with_form(form)
            },
            Action::Cancel => quote! {
                FormButtonBundle::new(#text).with_role(ButtonRole::Cancel).with_form(form)
            },
            Action::Custom(ref name) => quote! {
                FormButtonBundle::new(#text).with_role(ButtonRole::Custom(stringify!(#name))).with_form(form)
            }
        }
    });

    let variants = variants.iter().enumerate().map(|(i, variant)| {
        let action_variant = &variant.ident;
        let constructor = if variant.fields.is_empty() {
            quote! { Ok(#ident::#action_variant) }
        } else {
            quote! {
                match entity {
                    Some(entity) => Ok(#ident::#action_variant(entity)),
                    None => Err("Expected entity for action variant".to_string())
                }
            }
        };
        // let action = match variant.action {
        //     Action::Default | Action::Submit => quote! {
        //         #constructor
        //     },
        //     Action::Apply => quote! {
        //         Ok(#ident::#action_variant)
        //         Ok(FormEvent::Apply(entity))
        //     },
        //     Action::Cancel => quote! {
        //         Ok(FormEvent::Cancel(entity))
        //     },
        //     Action::Custom(ref name) => quote! {
        //         Ok(FormEvent::Custom(entity, stringify!(#name).to_string(), None))
        //     },
        // };
        quote! {
            #i => #constructor
        }
    });

    quote! {
        impl FormActions for #ident {
            type FormEntity = #form_type;

            fn get_button_bundles(form: Entity) -> Vec<FormButtonBundle> {
                let mut buttons = vec![
                    #(#bundles),*
                ];
                buttons
            }

            fn from_id_and_data(id: usize, entity: Option<Self::FormEntity>) -> Result<Self, String> {
                match id {
                    #(
                        #variants,
                    )*
                    _ => Err(format!("Unknown action id: {}", id))
                }
            }
        }
    }
    .into()
}

//! Derive macro for generating a form plugin
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_not_so_simple_text_input::form::*;
//! use bevy_not_so_simple_text_input::form_input;
//! use bevy_not_so_simple_text_input::{TextInputBundle, TextInputSettings, TextInputValue};
//!
//! #[form_struct]
//! #[derive(Debug, Clone)]
//! pub struct LoginData {
//!     #[form_field(active)]
//!     #[text_box(placeholder = "Username")]
//!     pub username: String,
//!     #[text_box(placeholder = "Password", mask = '*')]
//!     pub password: String,
//! }
//! ```

use darling::FromField;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput, Ident};

/// Optional attribute for form fields
/// - `optional`: Indicates that the field is optional. Field needs to be an `Option<T>`.
/// - `order`: The order of the field in the form (not implemented)
/// - `label`: The label of the field (currently defaults to the placeholder)
/// - `active`: Whether the field is the active field. Should only be used once. Behaviour might be unexpected if used multiple times.
///
/// ```no_run
/// #[form_field(optional, order = 1, label = "Username", active)]
/// pub foo: Option<String>,
/// ```
#[derive(FromField)]
#[darling(attributes(form_field))]
struct FormFieldOpts {
    ident: Option<syn::Ident>,

    optional: Option<bool>,
    _order: Option<usize>,
    label: Option<String>,
    active: Option<bool>,
}

impl FormFieldOpts {
    pub(crate) fn new(ident: syn::Ident) -> Self {
        Self {
            ident: Some(ident),
            optional: None,
            _order: None,
            label: None,
            active: None,
        }
    }
}

/// Required attribute for text box fields. All fields are optional.
/// - `placeholder`: The placeholder text for the text box
/// - `mask`: The mask character for the text box
/// - `text_style`: The text style for the text box. If not provided uses the `FormInputTextStyle` resource.
/// - `default_value`: The default value for the text box
///
/// ```no_run
/// #[text_box(placeholder = "Password", mask = '*', text_style = TextStyle { font_sieze: 22.0, color: Color::Black, ..default() })]
/// pub password: String,
/// ```
#[derive(FromField, Clone, Debug)]
#[darling(attributes(text_box))]
struct TextBoxOpts {
    ident: Option<syn::Ident>,
    placeholder: Option<String>,
    mask: Option<char>,
    text_style: Option<syn::Expr>,
    default_value: Option<String>,
}

struct FormField {
    form_field_opts: FormFieldOpts,
    field_specific_opts: FormFieldType,
}

enum FormFieldType {
    TextBox(TextBoxOpts),
}

/// Derive macro for generating a form plugin
/// This macro is dirty and a struct should be placed in a seperate file
#[proc_macro_attribute]
pub fn form_struct(_args: TokenStream, input: TokenStream) -> TokenStream {
    let parse_input = input.clone();
    let inputs = parse_macro_input!(parse_input as DeriveInput);
    let fields = match &inputs.data {
        syn::Data::Struct(data) => &data.fields,
        _ => panic!("Expected struct"),
    };
    if fields
        .iter()
        .any(|f| !matches!(f.vis, syn::Visibility::Public(_)))
    {
        panic!("All fields must be public");
    }

    let form_field_opts = fields
        .iter()
        .map(|f| {
            FormFieldOpts::from_field(f).unwrap_or(FormFieldOpts::new(f.ident.clone().unwrap()))
        })
        .collect::<Vec<_>>();

    let text_box_field_opts = fields
        .iter()
        .filter(|f| f.attrs.iter().any(|a| a.path().is_ident("text_box")))
        .filter_map(|f| TextBoxOpts::from_field(f).ok())
        .collect::<Vec<_>>();

    let form_fields = form_field_opts
        .into_iter()
        .filter_map(|f| {
            let specific_opts = text_box_field_opts
                .iter()
                .find(|t| t.ident == f.ident)
                .map(|text_box| FormFieldType::TextBox(text_box.clone()));

            specific_opts.map(|s| FormField {
                form_field_opts: f,
                field_specific_opts: s,
            })
        })
        .collect::<Vec<_>>();

    if form_fields.len() != fields.len() {
        panic!("All fields must have an associated input field");
    }

    let DeriveInput { ident, attrs, .. } = inputs;

    let marker_component_name = format_ident!("{}Form", ident);
    let marker_form_element_name = format_ident!("{}FormElement", ident);
    let plugin_name = format_ident!("{}FormPlugin", ident);
    let event_name = format_ident!("{}FormEvent", ident);
    let entity_resouce_name = format_ident!("{}FormFields", ident);

    let plugin = generate_plugin(
        &ident,
        &form_fields,
        &marker_component_name,
        &marker_form_element_name,
        &plugin_name,
        &event_name,
        &entity_resouce_name,
    );
    let setup = generate_setup(&ident, &form_fields, &marker_component_name);
    let submit = generate_submit_system(
        &ident,
        &marker_component_name,
        &event_name,
        &entity_resouce_name,
        form_fields,
    );

    let field_definitions = fields
        .iter()
        .map(|f| {
            let ident = f.ident.as_ref().unwrap();
            let ty = &f.ty;
            quote! {
                pub #ident: #ty,
            }
        })
        .collect::<Vec<_>>();

    quote! {
        #( #attrs )*
        pub struct #ident {
            #(#field_definitions)*
        }
        #plugin
        #setup
        #submit
    }
    .into()
}

fn generate_plugin(
    name: &Ident,
    opts: &[FormField],
    marker_component_name: &Ident,
    marker_form_element_name: &Ident,
    plugin_name: &Ident,
    event_name: &Ident,
    entity_resouce_name: &Ident,
) -> proc_macro2::TokenStream {
    let name = format_ident!("{}", name);
    let input_fields = opts
        .iter()
        .map(|o| format_ident!("{}_input", o.form_field_opts.ident.as_ref().unwrap()))
        .collect::<Vec<_>>();

    quote! {
        pub(crate) struct #plugin_name;
        impl Plugin for #plugin_name {
            fn build(&self, app: &mut App) {
                app.add_plugins(FormPlugin)
                    .add_event::<#event_name>()
                    .add_systems(Update, (setup, submit));
            }
        }

        #[derive(Component, Reflect)]
        pub(crate) struct #marker_component_name;

        #[derive(Component, Reflect)]
        pub struct #marker_form_element_name;

        #[derive(Resource, Debug)]
        pub(crate) struct #entity_resouce_name {
            #(
                pub(crate) #input_fields: Entity,
            )*
        }

        #[derive(Event, Debug)]
        pub(crate) struct #event_name {
            pub(crate) event: FormEvent<#name>,
        }
    }
}

fn generate_setup(
    name: &Ident,
    opts: &[FormField],
    marker_component_name: &Ident,
) -> proc_macro2::TokenStream {
    let form_field_setups = opts
        .iter()
        .enumerate()
        .map(|(i, o)| match &o.field_specific_opts {
            FormFieldType::TextBox(text_box_opts) => {
                generate_input_field_setup(&o.form_field_opts, text_box_opts, i)
            }
        })
        .collect::<Vec<_>>();

    let input_field_names = opts
        .iter()
        .map(|o| format_ident!("{}_input", o.form_field_opts.ident.as_ref().unwrap()))
        .collect::<Vec<_>>();

    let entity_resouce_name = format_ident!("{}FormFields", name);

    quote! {
        fn setup(
            mut commands: Commands,
            q_added: Query<Entity, Added<#marker_component_name>>,
            res_form_input_text_style: Res<FormInputTextStyle>,
        ) {
            for entity in q_added.iter() {
                #(#form_field_setups)*

                commands.insert_resource(#entity_resouce_name {
                    #(#input_field_names),*
                });

                commands.entity(entity)
                    .insert(Form)
                    #( .add_child(#input_field_names) )*;
            }
        }
    }
}

fn generate_input_field_setup(
    field_opts: &FormFieldOpts,
    text_box_opts: &TextBoxOpts,
    order: usize,
) -> proc_macro2::TokenStream {
    let field_name = format_ident!("{}_input", field_opts.ident.as_ref().unwrap());

    let placeholder = text_box_opts
        .placeholder
        .as_ref()
        .or(field_opts.label.as_ref())
        .map(|placeholder| quote! { .with_placeholder(#placeholder, None) })
        .unwrap_or_default();

    let default_value = text_box_opts
        .default_value
        .as_ref()
        .map(|default_value| quote! { .with_value(#default_value) })
        .unwrap_or_default();

    let active = field_opts
        .active
        .as_ref()
        .map(|active| quote! { .with_active(#active) })
        .unwrap_or_default();

    let settings = generate_input_field_settings(text_box_opts);

    let optional = field_opts
        .optional
        .as_ref()
        .filter(|optional| **optional)
        .map(|_| quote! { FormElementOptional })
        .unwrap_or_default();

    let text_style = text_box_opts
        .text_style
        .as_ref()
        .map(|text_style| quote! { #text_style })
        .unwrap_or(quote! { res_form_input_text_style.0.clone() });

    quote! {
        let #field_name = commands.spawn((
            NodeBundle::default(),
            TextInputBundle::default()
                .with_text_style(#text_style)
                #placeholder
                #settings
                #default_value
                #active,
            #optional
            FormElementOrder(#order),
        )).id();
    }
}

fn generate_input_field_settings(opts: &TextBoxOpts) -> proc_macro2::TokenStream {
    let mask = opts
        .mask
        .as_ref()
        // .map(|mask| mask.chars().next().unwrap())
        .map(|mask| quote! { Some(#mask) })
        .unwrap_or(quote! { None });

    quote! {
        .with_settings(TextInputSettings {
            mask_character: #mask,
            retain_on_submit: true,
        })
    }
}

fn generate_submit_system(
    name: &Ident,
    form_marker_component: &Ident,
    form_event: &Ident,
    entity_resouce_name: &Ident,
    opts: Vec<FormField>,
) -> proc_macro2::TokenStream {
    let input_field_names = opts
        .iter()
        .map(|o| o.form_field_opts.ident.as_ref().unwrap())
        .collect::<Vec<_>>();

    let input_field_query_resolvers = opts
        .iter()
        .map(|o| match o.field_specific_opts {
            FormFieldType::TextBox(_) => {
                let field_name = o.form_field_opts.ident.as_ref().unwrap();
                let input_field_name = format_ident!("{}_input", field_name);
                match o.form_field_opts.optional {
                    Some(true) => quote! {
                        let #field_name = if let Ok(value) = q_text_input.get(res_form_fields.#input_field_name) {
                            Some(value.0.clone())
                        } else {
                            None
                        };
                    },
                    _ => quote! {
                        let #field_name = q_text_input.get(res_form_fields.#input_field_name).unwrap().0.clone();
                    },
                }
            }
        })
        .collect::<Vec<_>>();

    quote! {
        fn submit(
            mut commands: Commands,
            mut ev_form: EventReader<GenericFormEvent>,
            mut ev_specific_form_event: EventWriter<#form_event>,
            mut q_form: Query<&#form_marker_component, With<FormValid>>,
            mut res_form_fields: Option<ResMut<#entity_resouce_name>>,
            mut q_text_input: Query<&TextInputValue>,
        ) {
            for ev in ev_form.read() {
                match ev.form {
                    FormEvent::Submit(form) => {
                        if let Some(res_form_fields) = res_form_fields.as_mut() {
                            if let Ok(form) = q_form.get(form) {
                                #(#input_field_query_resolvers)*
                                let data = #name {
                                    #(
                                        #input_field_names,
                                    )*
                                };
                                ev_specific_form_event.send(#form_event { event: FormEvent::Submit(data) });
                            }
                        }
                    }
                    FormEvent::Cancel => { ev_specific_form_event.send(#form_event { event: FormEvent::Cancel }); }
                }
            }
        }
    }
}

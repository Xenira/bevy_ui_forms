//! Proc macro for generating a form plugin
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_ui_forms::prelude::*;
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

use darling::{ast::NestedMeta, Error, FromField, FromMeta};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput, Ident};

#[derive(Debug, FromMeta)]
struct FormOpts {
    actions: Option<syn::Path>,
    submit: Option<String>,
    cancel: Option<String>,
}

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
/// #[text_box(placeholder = "Password", mask = '*', text_style = TextStyle { font_size: 22.0, color: Color::Black, ..default() })]
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

struct FormIdentifiers {
    marker_component: Ident,
    marker_form_element: Ident,
    plugin: Ident,
    event: Ident,
    entity_resource: Ident,
}

/// Proc macro for generating a form plugin
/// This macro is dirty and a struct should be placed in a separate file
///
/// # Panics
/// - If the annotated element is not a struct
/// - If any field is not public
/// - If any field does not have an associated input field
pub(crate) fn form_struct(args: TokenStream, input: &TokenStream) -> TokenStream {
    let parse_input = input.clone();
    let args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(args) => args,
        Err(e) => return TokenStream::from(Error::from(e).write_errors()),
    };
    let args = match FormOpts::from_list(&args) {
        Ok(args) => args,
        Err(e) => return TokenStream::from(e.write_errors()),
    };
    let inputs = parse_macro_input!(parse_input as DeriveInput);
    let fields = match &inputs.data {
        syn::Data::Struct(data) => &data.fields,
        _ => return TokenStream::from(Error::unsupported_shape("Expected struct").write_errors()),
    };
    if fields
        .iter()
        .any(|f| !matches!(f.vis, syn::Visibility::Public(_)))
    {
        return TokenStream::from(
            Error::unsupported_shape("All fields must be public").write_errors(),
        );
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
        return TokenStream::from(
            Error::missing_field("All fields must have an associated input field").write_errors(),
        );
    }

    let DeriveInput { ident, attrs, .. } = inputs;

    let form_identifiers = FormIdentifiers {
        marker_component: format_ident!("{}Form", ident),
        marker_form_element: format_ident!("{}FormElement", ident),
        plugin: format_ident!("{}FormPlugin", ident),
        event: format_ident!("{}FormEvent", ident),
        entity_resource: format_ident!("{}FormFields", ident),
    };

    let plugin = generate_plugin(&ident, &args, &form_fields, &form_identifiers);
    let setup = generate_setup(
        &ident,
        &args,
        &form_fields,
        &form_identifiers.marker_component,
    );
    let submit = generate_submit_system(&ident, &form_fields, &args, &form_identifiers);

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
    opts: &FormOpts,
    fields: &[FormField],
    form_identifiers: &FormIdentifiers,
) -> proc_macro2::TokenStream {
    let name = format_ident!("{}", name);
    let input_fields = fields
        .iter()
        .map(|o| format_ident!("{}_input", o.form_field_opts.ident.as_ref().unwrap()))
        .collect::<Vec<_>>();
    let action_event = opts.actions.as_ref().map_or(quote! {}, |actions| {
        quote! {
            .add_event::<#actions>()
        }
    });
    let FormIdentifiers {
        marker_component,
        marker_form_element,
        plugin,
        event,
        entity_resource,
    } = form_identifiers;

    quote! {
        pub(crate) struct #plugin;
        impl Plugin for #plugin {
            fn build(&self, app: &mut App) {
                app
                    .add_event::<#event>()
                    #action_event
                    .add_systems(Update, (setup, submit, btn_submit));
            }
        }

        #[derive(Component, Reflect)]
        pub(crate) struct #marker_component;

        #[derive(Component, Reflect)]
        pub struct #marker_form_element;

        #[derive(Resource, Debug)]
        pub(crate) struct #entity_resource {
            #(
                pub(crate) #input_fields: Entity,
            )*
        }

        #[derive(Event, Debug)]
        pub(crate) struct #event {
            pub(crate) event: FormEvent<#name>,
        }
    }
}

fn generate_setup(
    name: &Ident,
    form_opts: &FormOpts,
    form_field_opts: &[FormField],
    marker_component_name: &Ident,
) -> proc_macro2::TokenStream {
    let form_field_setups = form_field_opts
        .iter()
        .enumerate()
        .map(|(i, o)| match &o.field_specific_opts {
            FormFieldType::TextBox(text_box_opts) => {
                generate_input_field_setup(&o.form_field_opts, text_box_opts, i)
            }
        })
        .collect::<Vec<_>>();

    let input_field_names = form_field_opts
        .iter()
        .map(|o| format_ident!("{}_input", o.form_field_opts.ident.as_ref().unwrap()))
        .collect::<Vec<_>>();

    let actions_setup = generate_actions_setup(form_opts);

    let entity_resource_name = format_ident!("{}FormFields", name);

    quote! {
        fn setup(
            mut commands: Commands,
            q_added: Query<Entity, Added<#marker_component_name>>,
            res_form_input_text_style: Res<FormInputTextStyle>,
        ) {
            for entity in q_added.iter() {
                #(#form_field_setups)*

                commands.insert_resource(#entity_resource_name {
                    #(#input_field_names),*
                });

                #actions_setup

                commands.entity(entity)
                    .insert((Form, FormValid))
                    .insert(Name::new("form"))
                    #( .add_child(#input_field_names) )*
                    .add_child(actions);
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
        .map(|_| quote! { FormElementOptional, })
        .unwrap_or_default();

    let text_style = text_box_opts
        .text_style
        .as_ref()
        .map(|text_style| quote! { #text_style })
        .unwrap_or(quote! { res_form_input_text_style.0.clone() });

    quote! {
        let #field_name = commands.spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    ..default()
                },
                ..default()
            },
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

fn generate_actions_setup(opts: &FormOpts) -> proc_macro2::TokenStream {
    let mut actions = Vec::new();
    if let Some(cancel_text) = &opts.cancel {
        actions.push(quote! {
            let cancel = commands.spawn((
                FormButtonBundle::new(#cancel_text)
                    .with_form(entity)
                    .with_role(ButtonRole::Cancel)
            )).id();

            commands.entity(actions)
                .add_child(cancel);
        });
    }

    if let Some(submit_text) = &opts.submit {
        actions.push(quote! {
            let submit = commands.spawn((
                FormButtonBundle::new(#submit_text)
                    .with_form(entity)
                    .with_role(ButtonRole::Submit)
            )).id();

            commands.entity(actions)
                .add_child(submit);
        });
    }

    if let Some(button_enum) = &opts.actions {
        actions.push(quote! {
            for (i, btn) in #button_enum::get_button_bundles(entity).into_iter().enumerate() {
                let btn = commands.spawn((btn, FormActionId(i))).id();
                commands.entity(actions)
                    .add_child(btn);
            }
        });
    }

    quote! {
        let actions = commands.spawn((
            NodeBundle::default(),
            Name::new("action-row"),
        )).id();

        #(#actions)*
    }
}

fn generate_submit_system(
    name: &Ident,
    fields: &[FormField],
    opts: &FormOpts,
    form_identifiers: &FormIdentifiers,
) -> proc_macro2::TokenStream {
    let input_field_names = fields
        .iter()
        .map(|o| o.form_field_opts.ident.as_ref().unwrap())
        .collect::<Vec<_>>();

    let input_field_query_resolvers = fields
        .iter()
        .map(|o| match o.field_specific_opts {
            FormFieldType::TextBox(_) => {
                let field_name = o.form_field_opts.ident.as_ref().unwrap();
                let input_field_name = format_ident!("{}_input", field_name);
                if let Some(true) = o.form_field_opts.optional {
                    quote! {
                         let #field_name = if let Ok(value) = q_text_input.get(res_form_fields.#input_field_name) {
                            Some(value.0.clone())
                        } else {
                            None
                        };
                    }
                } else {
                    quote! {
                        let #field_name = q_text_input.get(res_form_fields.#input_field_name).unwrap().0.clone();
                    }
                }
            }
        })
        .collect::<Vec<_>>();

    let button_submit = generate_button_submit(opts, form_identifiers);

    let FormIdentifiers {
        marker_component,
        entity_resource,
        event,
        ..
    } = form_identifiers;

    quote! {
        fn submit(
            mut commands: Commands,
            mut ev_form: EventReader<GenericFormEvent>,
            mut ev_specific_form_event: EventWriter<#event>,
            mut q_form: Query<&#marker_component, With<FormValid>>,
            q_form_entity: Query<Entity, With<#marker_component>>,
            mut q_text_input: Query<&TextInputValue>,
            res_form_fields: Option<Res<#entity_resource>>,
        ) {
            for ev in ev_form.read() {
                match ev.form {
                    FormEvent::Submit(form) => {
                        let form = if let Ok(form) = q_form_entity.get_single() {
                            form
                        } else {
                            continue;
                        };
                        ev_specific_form_event.send(#event { event: FormEvent::Submit(get_form_data(&q_form, &q_text_input, &res_form_fields).unwrap()) });
                    }
                    FormEvent::Cancel(e) => { ev_specific_form_event.send(#event { event: FormEvent::Cancel(e) }); }
                    _ => {}
                }
            }
        }

        #button_submit

        fn get_form_data(
            q_form: &Query<&#marker_component, With<FormValid>>,
            q_text_input: &Query<&TextInputValue>,
            res_form_fields: &Option<Res<#entity_resource>>,
        ) -> Option<#name> {
            if let Ok(form) = q_form.get_single() {
                let res_form_fields = res_form_fields.as_ref().unwrap();
                #(#input_field_query_resolvers)*
                Some(#name {
                    #(
                        #input_field_names,
                    )*
                })
            } else {
                error!("Failed to get form entity");
                None
            }
        }
    }
}

fn generate_button_submit(
    opts: &FormOpts,
    form_identifiers: &FormIdentifiers,
) -> proc_macro2::TokenStream {
    let FormIdentifiers {
        marker_component,
        entity_resource,
        event,
        ..
    } = form_identifiers;

    let (action_event, action) = if let Some(action) = &opts.actions {
        (
            quote! {
                mut ev_action: EventWriter<#action>,
                q_id_button: Query<&FormActionId>,
            },
            quote! {
                if let Ok(id) = q_id_button.get(ev.entity) {
                    let form_data = get_form_data(&q_form, &q_text_input, &res_form_fields);
                    warn!("{:?}", form_data);
                    let action = #action::from_id_and_data(id.0, form_data).unwrap();
                    ev_action.send(action);
                    continue;
                }
            },
        )
    } else {
        (quote! {}, quote! {})
    };

    quote! {
        fn btn_submit(
            mut commands: Commands,
            mut ev_form: EventWriter<#event>,
            #action_event
            mut ev_btn: EventReader<ButtonPressEvent>,
            q_generic_button: Query<&ButtonRole, Without<FormActionId>>,
            q_form: Query<&#marker_component, With<FormValid>>,
            q_form_entity: Query<Entity, With<#marker_component>>,
            q_text_input: Query<&TextInputValue>,
            res_form_fields: Option<Res<#entity_resource>>,
        ) {
            for ev in ev_btn.read() {
                let form = if let Ok(form) = q_form_entity.get_single() {
                    form
                } else {
                    continue;
                };
                if ev.button.form.is_none() || ev.button.form.unwrap() != form {
                    continue;
                }
                #action
                if let Ok(role) = q_generic_button.get(ev.entity) {
                    let form_data = get_form_data(&q_form, &q_text_input, &res_form_fields);
                    let form = ev.button.form.unwrap();
                    match role {
                        ButtonRole::Submit => {
                            if let Some(form_data) = form_data {
                                ev_form.send(#event { event: FormEvent::Submit(form_data) });
                            }
                        }
                        ButtonRole::Cancel => {
                            ev_form.send(#event { event: FormEvent::Cancel(form) });
                        }
                        ButtonRole::Custom(name) => {
                            ev_form.send(#event { event: FormEvent::Custom(form, name.to_string(), form_data) });
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

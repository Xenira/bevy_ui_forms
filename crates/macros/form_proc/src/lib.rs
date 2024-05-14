//! Procedural macros for generating form plugins

use proc_macro::TokenStream;

mod form_actions;
mod form_struct;

/// Proc macro for generating a form plugin
/// This macro is dirty and a struct should be placed in a separate file
///
/// # Panics
/// - If the annotated element is not a struct
/// - If any field is not public
/// - If any field does not have an associated input field
#[proc_macro_attribute]
pub fn form_struct(args: TokenStream, input: TokenStream) -> TokenStream {
    form_struct::form_struct(args, &input)
}

/// Proc macro for deriving form actions
/// This is intended to be used on an enum in conjunction with the `form_struct` macro
#[proc_macro_derive(FormActions, attributes(form_action))]
pub fn form_actions_derive(input: TokenStream) -> TokenStream {
    form_actions::form_actions_derive(input)
}

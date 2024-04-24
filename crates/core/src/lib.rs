//! A Bevy plugin the provides a simple single-line text input widget.
//!
//! # Examples
//!
//! See the [examples](https://github.com/rparrett/bevy_simple_text_input/tree/latest/examples) folder.
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_simple_text_input::{TextInputBundle, TextInputPlugin};
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(TextInputPlugin)
//!         .add_systems(Startup, setup)
//!         .run();
//! }
//!
//! fn setup(mut commands: Commands) {
//!     commands.spawn(Camera2dBundle::default());
//!     commands.spawn((NodeBundle::default(), TextInputBundle::default()));
//! }
//! ```

/// Clipboard support for text input.
#[cfg(feature = "clipboard")]
pub mod clipboard;

/// Forms
pub mod form;
/// Form element
pub mod form_element;
/// Form elements
pub mod form_elements;

use bevy::app::{PluginGroup, PluginGroupBuilder};
/// Derive macro available if serde is built with `features = ["derive"]`.
#[cfg(feature = "derive")]
pub use bevy_ui_forms_form_proc::form_struct;

/// Re-export common use items for easy access.
pub mod prelude {
    pub use crate::form::*;
    pub use crate::form_element::*;
    pub use crate::form_elements::text_input::*;
    pub use crate::form_struct;
}

/// Plugin group for all `bevy_ui_forms` plugins.
pub struct BevyUiFormsPlugins;

impl PluginGroup for BevyUiFormsPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(form::FormPlugin)
            .add(form_element::FormElementPlugin)
            .add(form_elements::text_input::TextInputPlugin)
            .add(form_elements::button::ButtonPlugin)
    }
}

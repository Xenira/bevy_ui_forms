//! Button elements for forms.
//!
//! Form buttons are not yet ready for use.
//! They will be used to submit, cancel, or apply a form. Currently submitting a form is done by pressing the `KeyCode::Enter` key.
#![allow(clippy::module_name_repetitions)]
use bevy::prelude::*;

/// A Bevy `Plugin` providing the systems and assets required to make a [`FormButtonBundle`] work.
pub struct ButtonPlugin;

impl Plugin for ButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, setup);
    }
}

/// Bundle for a form button.
#[derive(Bundle, Default)]
pub struct FormButtonBundle {
    button: ButtonBundle,
    button_role: ButtonRole,
}

/// Role a button plays in a form.
#[derive(Component, Default, Debug)]
#[non_exhaustive]
pub enum ButtonRole {
    /// Submits the form.
    #[default]
    Submit,
    /// Cancels the form.
    Cancel,
    /// Submits the form but does not close it.
    Apply,
    /// Custom role.
    Custom(String),
}

impl From<&str> for ButtonRole {
    fn from(s: &str) -> Self {
        match s {
            "submit" => ButtonRole::Submit,
            "cancel" => ButtonRole::Cancel,
            "apply" => ButtonRole::Apply,
            _ => ButtonRole::Custom(s.to_string()),
        }
    }
}

impl From<String> for ButtonRole {
    fn from(s: String) -> Self {
        ButtonRole::from(s.as_str())
    }
}

/// System to set ua a newly spawned form button.
fn setup(mut _commands: Commands) {}

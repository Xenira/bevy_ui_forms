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
        app.add_event::<ButtonPressEvent>()
            .add_systems(Update, (setup, interact));
    }
}

/// Bundle for a form button.
#[derive(Bundle)]
pub struct FormButtonBundle {
    form_button: FormButton,
    button: ButtonBundle,
    button_role: ButtonRole,
}

impl FormButtonBundle {
    /// Creates a new form button bundle.
    pub fn new(text: impl Into<String>) -> Self {
        FormButtonBundle {
            form_button: FormButton {
                text: text.into(),
                form: None,
            },
            button: ButtonBundle::default(),
            button_role: ButtonRole::default(),
        }
    }

    /// Sets the role of the button.
    #[must_use]
    pub fn with_role(mut self, role: ButtonRole) -> Self {
        self.button_role = role;
        self
    }

    /// Sets the form the button belongs to.
    #[must_use]
    pub fn with_form(mut self, form: Entity) -> Self {
        self.form_button.form = Some(form);
        self
    }
}

/// Marker component for a form button.
#[derive(Component, Clone, Default, Debug)]
pub struct FormButton {
    /// Text displayed on the button.
    pub text: String,
    /// The form the button belongs to.
    pub form: Option<Entity>,
}

/// Interaction state of a form button.
/// Maps to the `Interaction` component. This is needed to use it in a `HashMap`.
#[derive(Component, Clone, PartialEq, Eq, Hash, Debug)]
pub enum FormInteraction {
    /// No interaction.
    None,
    /// Hovered interaction.
    Hovered,
    /// Pressed interaction.
    Pressed,
}

/// Event that is sent when a form button is pressed.
#[derive(Event, Debug)]
pub struct ButtonPressEvent {
    /// The entity of the button that was pressed.
    pub entity: Entity,
    /// The button that was pressed.
    pub button: FormButton,
    /// The role the button plays in the form.
    pub role: ButtonRole,
}

impl From<&Interaction> for FormInteraction {
    fn from(interaction: &Interaction) -> Self {
        match interaction {
            Interaction::None => FormInteraction::None,
            Interaction::Hovered => FormInteraction::Hovered,
            Interaction::Pressed => FormInteraction::Pressed,
        }
    }
}
/// Role a button plays in a form.
#[derive(Component, Clone, PartialEq, Eq, Hash, Default, Debug)]
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

fn setup(mut commands: Commands, mut q_button: Query<(Entity, &FormButton), Added<FormButton>>) {
    for (entity, button) in &mut q_button {
        let text = commands
            .spawn(TextBundle::from_section(
                button.text.clone(),
                TextStyle::default(),
            ))
            .id();

        commands
            .entity(entity)
            // .insert(style.element_style)
            .add_child(text);
    }
}

#[allow(clippy::needless_pass_by_value)]
fn interact(
    q_button: Query<(Entity, &FormButton, &ButtonRole, &Interaction), Changed<Interaction>>,
    mut ev_button: EventWriter<ButtonPressEvent>,
) {
    for (entity, button, role, _) in q_button
        .iter()
        .filter(|(_, _, _, interaction)| **interaction == Interaction::Pressed)
    {
        ev_button.send(ButtonPressEvent {
            entity,
            button: button.clone(),
            role: role.clone(),
        });
    }
}

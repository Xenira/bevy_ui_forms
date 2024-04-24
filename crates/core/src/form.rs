#![allow(clippy::module_name_repetitions)]

use bevy::prelude::*;

/// Plugin for forms consisting of multiple input fields.
pub struct FormPlugin;

impl Plugin for FormPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FormInputTextStyle>()
            .add_event::<GenericFormEvent>()
            .add_systems(Update, form_keyboard);
    }
}

/// Marker component indicating that the entity is a form.
#[derive(Component, Reflect)]
pub struct Form;

/// Marker component indicating that the form is valid.
#[derive(Component, Reflect)]
pub struct FormValid;

/// Marker component indicating that the form is invalid.
#[derive(Component, Reflect)]
pub struct FormInvalid(pub Vec<FormValidationError>);

/// Text style for form input fields.
/// Default is `TextStyle` with `font_size` 20.0 and `color` `Color::BLACK`.
#[derive(Resource, Debug)]
pub struct FormInputTextStyle(pub TextStyle);

impl Default for FormInputTextStyle {
    fn default() -> Self {
        FormInputTextStyle(TextStyle {
            font_size: 20.0,
            color: Color::BLACK,
            ..default()
        })
    }
}

/// Event that is sent when a generic form event occurs.
#[derive(Event, Debug)]
pub struct GenericFormEvent {
    /// The form event containing the form entity.
    pub form: FormEvent<Entity>,
}

/// Event that is sent when a form is submitted.
#[derive(Debug)]
pub enum FormEvent<T> {
    /// Submit event with the form data.
    Submit(T),
    /// Cancel event.
    Cancel(Entity),
}

/// Event that is sent when a form is validated.
#[derive(Event, Debug)]
pub struct FormValidationEvent {
    /// Whether the form is valid.
    pub valid: bool,
    /// Whether the form is dirty.
    pub dirty: bool,
    /// Validation errors.
    pub fields: Vec<FormValidationError>,
}

/// Validation errors for form elements.
#[derive(Debug, Clone, Reflect)]
pub enum FormValidationError {
    /// Required field is empty.
    Required(Entity),
    /// Field is invalid.
    Invalid(Entity),
    /// Custom error with a message.
    Custom(Entity, String),
}

#[allow(clippy::needless_pass_by_value)]
fn form_keyboard(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    q_form: Query<(Entity, Option<&FormInvalid>), With<Form>>,
    mut form_events: EventWriter<GenericFormEvent>,
) {
    if let Ok((entity, invalid)) = q_form.get_single() {
        if keyboard_input.just_released(KeyCode::Enter) && invalid.is_none() {
            form_events.send(GenericFormEvent {
                form: FormEvent::Submit(entity),
            });
        } else if keyboard_input.just_released(KeyCode::Escape) {
            form_events.send(GenericFormEvent {
                form: FormEvent::Cancel(entity),
            });
        }
    }
}

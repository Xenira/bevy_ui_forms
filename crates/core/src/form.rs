#![allow(clippy::module_name_repetitions)]

use bevy::prelude::*;

/// Plugin for forms consisting of multiple input fields.
pub struct FormPlugin;

impl Plugin for FormPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FormInputTextStyle>()
            .add_event::<GenericFormEvent>()
            .add_systems(
                Update,
                (
                    form_element_touched,
                    form_element_invalid,
                    form_element_valid,
                    form_keyboard,
                ),
            )
            .register_type::<FormElementDirty>()
            .register_type::<FormElementValid>()
            .register_type::<FormElementInvalid>()
            .register_type::<FormElementTouched>()
            .register_type::<FormElementOptional>();
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

/// Marker component indicating that the entity is a form element.
#[derive(Component, Reflect)]
pub struct FromElement;

/// Marker component indicating that the element is focused.
#[derive(Component, Reflect)]
pub struct FormElementFocus;

/// Marker component indicating that a value was changed.
#[derive(Component, Reflect)]
pub struct FormElementDirty;

/// Marker component indicating that a value is valid.
#[derive(Component, Reflect)]
pub struct FormElementValid;

/// Marker component indicating that a value is invalid.
#[derive(Component, Reflect)]
pub struct FormElementInvalid(pub FormValidationError);

/// Marker component indicating that the element was focused.
#[derive(Component, Reflect)]
pub struct FormElementTouched;

/// Marker component indicating that the element is optional.
#[derive(Component, Reflect)]
pub struct FormElementOptional;

/// Order of form elements. Elements are focused in ascending.
#[derive(Component, Reflect)]
pub struct FormElementOrder(pub usize);

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
    Cancel,
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
fn form_element_touched(
    mut commands: Commands,
    q_form_element_touched: Query<Entity, (With<FormElementFocus>, Without<FormElementTouched>)>,
) {
    for entity in q_form_element_touched.iter() {
        commands.entity(entity).insert(FormElementTouched);
    }
}

#[allow(clippy::needless_pass_by_value)]
fn form_element_invalid(
    mut commands: Commands,
    q_form_element_invalid: Query<(&Parent, &FormElementInvalid), Added<FormElementInvalid>>,
    mut q_form: Query<Option<&mut FormInvalid>, With<Form>>,
) {
    for (parent, element_invalid) in q_form_element_invalid.iter() {
        if let Ok(form_invalid) = q_form.get_mut(parent.get()) {
            if let Some(mut form_invalid) = form_invalid {
                form_invalid.0.push(element_invalid.0.clone());
            } else {
                commands
                    .entity(parent.get())
                    .insert(FormInvalid(vec![element_invalid.0.clone()]))
                    .remove::<FormValid>();
            }
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn form_element_valid(
    mut commands: Commands,
    q_form_element_valid: Query<(&Parent, Entity), Added<FormElementValid>>,
    mut q_form: Query<&mut FormInvalid, With<Form>>,
) {
    for (parent, element_entity) in q_form_element_valid.iter() {
        if let Ok(mut form_invalid) = q_form.get_mut(parent.get()) {
            form_invalid.0.retain(|error| match error {
                FormValidationError::Required(entity)
                | FormValidationError::Invalid(entity)
                | FormValidationError::Custom(entity, _) => *entity != element_entity,
            });

            if form_invalid.0.is_empty() {
                commands
                    .entity(parent.get())
                    .remove::<FormInvalid>()
                    .insert(FormValid);
            }
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn form_keyboard(
    mut commands: Commands,
    mut form_events: EventWriter<GenericFormEvent>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    q_form_element_focus: Query<(Entity, &Children, Option<&FormInvalid>), With<Form>>,
    q_focused: Query<(&FormElementFocus, Option<&FormElementOrder>)>,
    q_form_elements: Query<(Entity, Option<&FormElementOrder>)>,
) {
    let form = q_form_element_focus.get_single();
    if form.is_err() {
        return;
    }

    if keyboard_input.just_released(KeyCode::Enter) {
        if let (entity, _, None) = form.unwrap() {
            form_events.send(GenericFormEvent {
                form: FormEvent::Submit(entity),
            });
        }
    } else if keyboard_input.just_pressed(KeyCode::Escape) {
        form_events.send(GenericFormEvent {
            form: FormEvent::Cancel,
        });
    } else if keyboard_input.just_pressed(KeyCode::Tab) {
        let (_, children, _) = form.unwrap();
        let focus_order = q_focused
            .get_single()
            .map(|(_, order)| order.map_or(0, |o| o.0))
            .unwrap_or(0);

        let order = children
            .iter()
            .filter_map(|child| q_form_elements.get(*child).ok())
            .filter(|(_, order)| order.is_some())
            .map(|(entity, order)| (entity, order.unwrap().0));

        let next = order
            .clone()
            .filter(|(_, order)| *order > focus_order)
            .min_by_key(|(_, order)| *order);

        if let Some((entity, _)) = next.or(order.min_by_key(|(_, order)| *order)) {
            commands.entity(entity).insert(FormElementFocus);
        }
    }
}

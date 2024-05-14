#![allow(clippy::module_name_repetitions)]
use bevy::prelude::*;

use crate::form::{Form, FormInvalid, FormValid, FormValidationError};

/// Plugin for form elements.
pub struct FormElementPlugin;

impl Plugin for FormElementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                form_element_touched,
                form_element_invalid,
                form_element_valid,
                form_element_keyboard,
            ),
        )
        .register_type::<FormElementDirty>()
        .register_type::<FormElementValid>()
        .register_type::<FormElementInvalid>()
        .register_type::<FormElementTouched>()
        .register_type::<FormElementOptional>();
    }
}

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

/// Style of a form element.
#[derive(Component, Default, Clone, Debug, Reflect)]
pub struct FormElementStyle {
    /// The style of the node.
    pub style: Style,
    /// Optional image to display.
    pub image: Option<Handle<Image>>,
    /// Optional scale mode for the image.
    pub image_scale_mode: Option<ImageScaleMode>,
    /// Optional color for the background.
    pub background_color: Option<BackgroundColor>,
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
fn form_element_keyboard(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    q_form_children: Query<&Children, With<Form>>,
    q_focused: Query<Option<&FormElementOrder>, With<FormElementFocus>>,
    q_form_elements: Query<(Entity, Option<&FormElementOrder>)>,
) {
    if keyboard_input.just_released(KeyCode::Tab) {
        if let Ok(children) = q_form_children.get_single() {
            let focus_order = q_focused
                .get_single()
                .map(|order| order.map_or(0, |o| o.0))
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
}

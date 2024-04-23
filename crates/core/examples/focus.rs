//! An example showing a more advanced implementation with focus.

use bevy::prelude::*;
use bevy_ui_forms::{
    clipboard::ClipboardPlugin, TextInputActive, TextInputBundle, TextInputPlugin,
};

const BORDER_COLOR_ACTIVE: Color = Color::rgb(0.75, 0.52, 0.99);
const BORDER_COLOR_INACTIVE: Color = Color::rgb(0.25, 0.25, 0.25);
const TEXT_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);
const BACKGROUND_COLOR: Color = Color::rgb(0.15, 0.15, 0.15);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TextInputPlugin)
        .add_plugins(ClipboardPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, focus)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                ..default()
            },
            // Make this container node bundle to be Interactive so that clicking on it removes
            // focus from the text input.
            Interaction::None,
        ))
        .with_children(|parent| {
            parent.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Px(200.0),
                        border: UiRect::all(Val::Px(5.0)),
                        padding: UiRect::all(Val::Px(5.0)),
                        ..default()
                    },
                    border_color: BORDER_COLOR_INACTIVE.into(),
                    background_color: BACKGROUND_COLOR.into(),
                    ..default()
                },
                TextInputBundle::default()
                    .with_text_style(TextStyle {
                        font_size: 40.,
                        color: TEXT_COLOR,
                        ..default()
                    })
                    .with_value("Click Me"),
            ));
        });
}

fn focus(
    mut text_input_query: Query<(&TextInputActive, &mut BorderColor), Changed<TextInputActive>>,
) {
    for (active, mut border_color) in &mut text_input_query {
        if active.0 {
            *border_color = BORDER_COLOR_ACTIVE.into();
        } else {
            *border_color = BORDER_COLOR_INACTIVE.into();
        }
    }
}

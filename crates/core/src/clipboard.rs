#[cfg(not(target_family = "wasm"))]
use arboard::Clipboard;
use bevy::prelude::*;

#[cfg(target_family = "wasm")]
use async_channel::Receiver;
#[cfg(target_family = "wasm")]
pub(crate) use wasm_bindgen_futures::spawn_local as spawn;
#[cfg(target_family = "wasm")]
use wasm_bindgen_futures::JsFuture;

/// A Bevy plugin that provides clipboard functionality.
pub struct ClipboardPlugin;

impl Plugin for ClipboardPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ClipboardEvent>()
            .add_systems(Update, keyboard);

        #[cfg(target_family = "wasm")]
        app.add_systems(Update, async_clipboard);
    }
}

#[cfg(target_family = "wasm")]
#[derive(Component, Debug)]
struct ClipboardContentReceiver(Receiver<String>);

/// Events that can be sent by the clipboard plugin.
#[derive(Event, Debug, Clone)]
pub enum ClipboardEvent {
    /// User requested to copy the current selection.
    /// Currently this is only a placeholder and does not actually copy anything.
    Copy,
    /// User requested to paste the current selection.
    Paste(String),
}

#[cfg(not(target_family = "wasm"))]
fn keyboard(keys: Res<ButtonInput<KeyCode>>, mut submit_writer: EventWriter<ClipboardEvent>) {
    if keys.just_pressed(KeyCode::Insert) {
        request_clipboard_content(submit_writer);
        return;
    }

    if keys.just_pressed(KeyCode::Copy) {
        submit_writer.send(ClipboardEvent::Copy);
        return;
    }

    if !keys.pressed(KeyCode::ControlLeft) && !keys.pressed(KeyCode::ControlRight) {
        return;
    }

    if keys.just_pressed(KeyCode::KeyC) {
        submit_writer.send(ClipboardEvent::Copy);
        return;
    }

    if keys.just_pressed(KeyCode::KeyV) {
        request_clipboard_content(submit_writer);
    }
}

#[cfg(target_family = "wasm")]
fn keyboard(
    commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut submit_writer: EventWriter<ClipboardEvent>,
) {
    if keys.just_pressed(KeyCode::Insert) {
        request_clipboard_content(commands);
        return;
    }

    if keys.just_pressed(KeyCode::Copy) {
        submit_writer.send(ClipboardEvent::Copy);
        return;
    }

    if !keys.pressed(KeyCode::ControlLeft) && !keys.pressed(KeyCode::ControlRight) {
        return;
    }

    if keys.just_pressed(KeyCode::KeyC) {
        submit_writer.send(ClipboardEvent::Copy);
        return;
    }

    if keys.just_pressed(KeyCode::KeyV) {
        request_clipboard_content(commands);
    }
}

#[cfg(target_family = "wasm")]
fn async_clipboard(
    mut commands: Commands,
    q_clipboard_content: Query<(Entity, &ClipboardContentReceiver)>,
    mut ev_clipboard: EventWriter<ClipboardEvent>,
) {
    for (entity, receiver) in q_clipboard_content.iter() {
        if let Ok(content) = receiver.0.try_recv() {
            commands.entity(entity).despawn_recursive();
            ev_clipboard.send(ClipboardEvent::Paste(content));
        } else if receiver.0.is_closed() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

#[cfg(not(target_family = "wasm"))]
fn request_clipboard_content(mut ev_clipboard: EventWriter<ClipboardEvent>) {
    ev_clipboard.send(ClipboardEvent::Paste(
        get_clipboard_content().unwrap_or_default(),
    ));
}

#[cfg(target_family = "wasm")]
fn request_clipboard_content(mut commands: Commands) {
    let receiver = get_clipboard_content();
    commands.spawn(ClipboardContentReceiver(receiver));
}

#[cfg(not(target_family = "wasm"))]
fn get_clipboard_content() -> Option<String> {
    let mut clipboard = Clipboard::new().ok()?;
    clipboard.get_text().ok()
}

#[cfg(target_family = "wasm")]
fn get_clipboard_content() -> Receiver<String> {
    let (s, r) = async_channel::unbounded();
    spawn(async move {
        let clipboard = web_sys::window().unwrap().navigator().clipboard().unwrap();
        let value = JsFuture::from(clipboard.read_text()).await.unwrap();
        let value = value.as_string().unwrap_or_default();
        s.send(value).await.unwrap();
    });

    r
}

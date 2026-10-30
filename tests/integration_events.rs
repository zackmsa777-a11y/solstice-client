// SPDX-License-Identifier: MIT
// AI-assisted integration tests by Codex (OpenAI), 2026.
// Host vocabulary only; observer bodies and policy models are production code.
mod winit {
    pub mod keyboard {
        #[derive(Clone, Copy)] pub enum KeyCode { Escape, CapsLock, KeyA, KeyZ }
        #[derive(Clone, Copy)] pub enum NamedKey { Escape, CapsLock, ArrowLeft, ArrowRight, Home, End, Backspace, Delete, Tab, ArrowUp, ArrowDown, Enter }
        #[derive(Clone)] pub enum Key { Named(NamedKey), Character(String) }
        #[derive(Clone, Copy)] pub enum PhysicalKey { Code(KeyCode), Unidentified }
        #[derive(Clone, Copy, Default)] pub struct ModifiersState;
        impl ModifiersState {
            pub fn empty() -> Self { Self }
            pub fn control_key(self) -> bool { false }
            pub fn alt_key(self) -> bool { false }
            pub fn super_key(self) -> bool { false }
        }
    }
    pub mod event {
        use super::keyboard::*;
        #[derive(Clone, Copy, PartialEq)] pub enum ElementState { Pressed, Released }
        #[derive(Clone, Copy)] pub enum MouseButton { Left, Right, Other(u16) }
        pub struct KeyEvent { pub logical_key: Key, pub physical_key: PhysicalKey, pub state: ElementState, pub text: Option<String>, pub repeat: bool }
        pub struct Modifiers;
        impl Modifiers { pub fn state(&self) -> ModifiersState { ModifiersState } }
        pub enum WindowEvent {
            KeyboardInput { event: KeyEvent, is_synthetic: bool }, MouseInput { state: ElementState, button: MouseButton },
            ModifiersChanged(Modifiers), Focused(bool), CloseRequested, Destroyed,
        }
    }
}
mod horizon_client { pub mod keybinds {
    use crate::winit::{keyboard::KeyCode, event::MouseButton};
    pub fn key_name(key: KeyCode) -> Option<&'static str> { Some(match key {
        KeyCode::Escape => "key.keyboard.escape", KeyCode::CapsLock => "key.keyboard.caps.lock",
        KeyCode::KeyA => "key.keyboard.a", KeyCode::KeyZ => "key.keyboard.z",
    }) }
    pub fn mouse_name(button: MouseButton) -> String { match button {
        MouseButton::Left => "key.mouse.left".into(), MouseButton::Right => "key.mouse.right".into(),
            MouseButton::Other(n) => format!("key.mouse.{}", n + 1),
    } }
} }
mod runtime {
    use crate::{horizon_client, winit};
    use winit::keyboard::KeyCode;
    use crate::solstice_text_edit::{Draft, EditKey};
    use crate::solstice_registry_picker::{Panel as EditorPanel, Picker};
    use std::cell::RefCell;
    struct TextEditor { draft: Result<Draft, &'static str>, picker: Option<Picker> }
    struct Gui {
        open: bool, text_edit: Option<TextEditor>, text_key_consumed: bool,
        text_modifiers: winit::keyboard::ModifiersState, view_w: f32, view_h: f32,
    }
    struct Module { on: bool }
    struct State {
        modules: Vec<Module>, gui: Gui, inputs: crate::solstice_bind_state::Inputs,
        hud_edit: crate::solstice_hud_layout::Editor, binding: Option<usize>, pending_mouse_bind: Option<String>,
    }
    impl Default for State {
        fn default() -> Self { Self {
            modules: vec![Module { on: false }], inputs: Default::default(), hud_edit: Default::default(), binding: None, pending_mouse_bind: None,
            gui: Gui { open: true, text_edit: None, text_key_consumed: false, text_modifiers: Default::default(), view_w: 427.0, view_h: 240.0 },
        } }
    }
    thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }
    fn with<R>(f: impl FnOnce(&mut State) -> R) -> R { STATE.with(|state| f(&mut state.borrow_mut())) }
    fn sync_flags(_: &State) {}
    // PRODUCTION_EVENT_OBSERVERS

    fn reset(picker: bool) {
        with(|state| {
            *state = State::default();
            let picker = picker.then(|| Picker::new(std::sync::Arc::new(crate::solstice_registry_picker::Catalog::new(
                ["minecraft:stone".into(), "minecraft:iron_ore".into()], []
            )), crate::solstice_registry_picker::Kind::Blocks));
            state.gui.text_edit = Some(TextEditor { draft: Draft::new("stone"), picker });
        });
    }
    fn event(logical_key: winit::keyboard::Key, text: Option<&str>, pressed: bool) -> winit::event::WindowEvent {
        winit::event::WindowEvent::KeyboardInput {
            event: winit::event::KeyEvent { logical_key, physical_key: winit::keyboard::PhysicalKey::Code(KeyCode::KeyZ),
                state: if pressed { winit::event::ElementState::Pressed } else { winit::event::ElementState::Released }, text: text.map(str::to_owned), repeat: false },
            is_synthetic: false,
        }
    }
    fn dispatch(event: &winit::event::WindowEvent, bind_first: bool) {
        if bind_first { bind_event(event); text_event(event); } else { text_event(event); bind_event(event); }
    }

    #[test]
    fn hud_mode_rejects_stale_text_and_picker_capture_in_either_hook_order() {
        for bind_first in [false, true] {
            reset(true);
            with(|state| { state.hud_edit.begin(); state.gui.text_key_consumed = true; });
            dispatch(&event(winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape), None, true), bind_first);
            with(|state| {
                assert!(state.hud_edit.active()); // Its gui_key handler retains ownership.
                assert!(state.gui.text_edit.is_none());
                assert!(!state.gui.text_key_consumed);
                assert!(!state.inputs.press("key.keyboard.escape"));
            });
        }
    }

    #[test]
    fn text_modal_does_not_swallow_bind_releases_in_either_hook_order() {
        for bind_first in [false, true] {
            reset(false);
            with(|state| {
                state.inputs.press("key.keyboard.z"); state.inputs.hold(0, "key.keyboard.z", false); state.modules[0].on = true;
            });
            dispatch(&event(winit::keyboard::Key::Character("z".into()), None, false), bind_first);
            with(|state| {
                assert!(!state.modules[0].on);
                assert!(state.gui.text_edit.is_some());
                assert!(state.inputs.press("key.keyboard.z"));
            });
        }
    }

    #[test]
    fn cancelling_text_consumes_escape_without_rearming_its_bind() {
        reset(false);
        dispatch(&event(winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape), None, true), true);
        with(|state| {
            assert!(state.gui.text_edit.is_none());
            assert!(state.gui.text_key_consumed);
            assert!(!state.inputs.press("key.keyboard.escape"));
        });
    }

    #[test]
    fn picker_search_and_raw_draft_keep_logical_navigation_and_focus_separate() {
        reset(true);
        use winit::keyboard::{Key, NamedKey};
        text_event(&event(Key::Named(NamedKey::Tab), None, true));
        text_event(&event(Key::Character("ore".into()), Some("ore"), true));
        text_event(&event(Key::Named(NamedKey::Home), None, true));
        text_event(&event(Key::Character("x".into()), Some("x"), true));
        with(|state| {
            let edit = state.gui.text_edit.as_ref().unwrap();
            assert_eq!(edit.picker.as_ref().unwrap().query.text(), "xore");
            assert_eq!(edit.draft.as_ref().unwrap().text(), "stone");
        });
        text_event(&event(Key::Named(NamedKey::Tab), None, true));
        text_event(&event(Key::Named(NamedKey::Home), None, true));
        text_event(&event(Key::Character("!".into()), Some("!"), true));
        with(|state| assert_eq!(state.gui.text_edit.as_ref().unwrap().draft.as_ref().unwrap().text(), "!stone"));
    }

    fn repeating(mut event: winit::event::WindowEvent) -> winit::event::WindowEvent {
        if let winit::event::WindowEvent::KeyboardInput { event, .. } = &mut event { event.repeat = true; }
        event
    }

    #[test]
    fn repeated_tab_retains_the_documented_focus_navigation_policy() {
        reset(true);
        use winit::keyboard::{Key, NamedKey};
        text_event(&repeating(event(Key::Named(NamedKey::Tab), None, true)));
        with(|state| assert!(state.gui.text_edit.as_ref().unwrap().picker.as_ref().unwrap().search_focus));
        text_event(&repeating(event(Key::Named(NamedKey::Tab), None, true)));
        with(|state| assert!(!state.gui.text_edit.as_ref().unwrap().picker.as_ref().unwrap().search_focus));
    }

    #[test]
    fn repeated_enter_does_not_toggle_the_registry_draft() {
        reset(true);
        use winit::keyboard::{Key, NamedKey};
        text_event(&event(Key::Named(NamedKey::Tab), None, true));
        text_event(&repeating(event(Key::Named(NamedKey::Enter), None, true)));
        with(|state| assert_eq!(state.gui.text_edit.as_ref().unwrap().draft.as_ref().unwrap().text(), "stone"));
        text_event(&event(Key::Named(NamedKey::Enter), None, true));
        let once = with(|state| state.gui.text_edit.as_ref().unwrap().draft.as_ref().unwrap().text().to_owned());
        assert_ne!(once, "stone");
        text_event(&repeating(event(Key::Named(NamedKey::Enter), None, true)));
        with(|state| assert_eq!(state.gui.text_edit.as_ref().unwrap().draft.as_ref().unwrap().text(), once));
    }

    #[test]
    fn repeated_navigation_still_moves_picker_cursor_and_edits_query() {
        reset(true);
        use winit::keyboard::{Key, NamedKey};
        text_event(&event(Key::Named(NamedKey::Tab), None, true));
        text_event(&repeating(event(Key::Named(NamedKey::ArrowDown), None, true)));
        with(|state| assert_eq!(state.gui.text_edit.as_ref().unwrap().picker.as_ref().unwrap().cursor, 1));
        text_event(&event(Key::Character("ore".into()), Some("ore"), true));
        text_event(&repeating(event(Key::Named(NamedKey::Home), None, true)));
        text_event(&event(Key::Character("x".into()), Some("x"), true));
        text_event(&repeating(event(Key::Named(NamedKey::Backspace), None, true)));
        with(|state| {
            let edit = state.gui.text_edit.as_ref().unwrap();
            assert_eq!(edit.picker.as_ref().unwrap().query.text(), "ore");
            assert_eq!(edit.draft.as_ref().unwrap().text(), "stone");
        });
    }

    #[test]
    fn focus_loss_cancels_draft_and_releases_owned_hold() {
        reset(true);
        with(|state| { state.inputs.hold(0, "key.keyboard.z", false); state.modules[0].on = true; });
        dispatch(&winit::event::WindowEvent::Focused(false), false);
        with(|state| { assert!(state.gui.text_edit.is_none()); assert!(!state.modules[0].on); });
    }
}

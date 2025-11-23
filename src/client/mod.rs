use cursive::Cursive;
use cursive::views::{ScrollView, NamedView, TextView};

#[derive(Default)]
pub struct ScrollState {
    pub auto_scroll: bool,
}

impl ScrollState {
    pub fn new() -> Self {
        Self { auto_scroll: true }
    }
}

pub fn scroll_to_bottom(siv: &mut Cursive) {
    if let Some(scroll_state) = siv.user_data::<ScrollState>() {
        scroll_state.auto_scroll = true;
    }
    siv.call_on_name("chat_scroll", |view: &mut ScrollView<NamedView<TextView>>| {
        view.scroll_to_bottom();
    });
}

pub fn check_scroll_position(siv: &mut Cursive) {
    if let Some(scroll_state) = siv.user_data::<ScrollState>() {
        scroll_state.auto_scroll = false;
    }
}

pub fn enable_auto_scroll(siv: &mut Cursive) {
    if let Some(scroll_state) = siv.user_data::<ScrollState>() {
        scroll_state.auto_scroll = true;
    }
}

pub fn add_scroll_callbacks(siv: &mut Cursive) {
    siv.add_global_callback(cursive::event::Key::Up, check_scroll_position);
    siv.add_global_callback(cursive::event::Key::Down, check_scroll_position);
    siv.add_global_callback(cursive::event::Key::PageUp, check_scroll_position);
    siv.add_global_callback(cursive::event::Key::PageDown, check_scroll_position);

    siv.add_global_callback(cursive::event::Event::Mouse {
        event: cursive::event::MouseEvent::WheelUp,
        position: cursive::vec::Vec2::new(0, 0),
        offset: cursive::vec::Vec2::new(0, 0),
    }, check_scroll_position);

    siv.add_global_callback(cursive::event::Event::Mouse {
        event: cursive::event::MouseEvent::WheelDown,
        position: cursive::vec::Vec2::new(0, 0),
        offset: cursive::vec::Vec2::new(0, 0),
    }, check_scroll_position);

    siv.add_global_callback(cursive::event::Key::End, enable_auto_scroll);
}
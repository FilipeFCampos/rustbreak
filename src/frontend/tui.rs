use cursive::{
    Cursive,
    align::HAlign,
    event::Key,
    theme::{BaseColor, Color, PaletteColor},
    traits::*,
    views::{Dialog, DummyView, EditView, LinearLayout, Panel, ScrollView, TextView},
};

/// Handles the TUI setup.
///
/// ### Parameters
/// - `siv`: The TUI struct from the Cursive crate;
/// - `username`: The username of the current client;
/// - `input_action`: Function to be run on input box submission.
pub fn handle_tui(siv: &mut Cursive, username: String, input_action: fn(&mut Cursive, String)) {
    // Load theme from file
    siv.load_toml(include_str!("assets/style.toml")).unwrap();

    // Header to be displayed at the top
    let header = TextView::new(format!(r#"⋘ ( *^-^)ρ ⭐ WELCOME {username} ⭐ &(^0^* )⋙"#))
        .style(PaletteColor::Secondary)
        .h_align(HAlign::Center);

    // Message area
    let messages = TextView::new("")
        .style(PaletteColor::Secondary)
        .with_name("messages");

    let messages = ScrollView::new(messages)
        .scroll_strategy(cursive::view::ScrollStrategy::StickToBottom)
        .with_name("chat_scroll")
        .min_height(10)
        .full_height()
        .min_width(60)
        .full_width();

    // Input area
    let input = EditView::new()
        .on_submit(move |s, text| input_action(s, text.to_string()))
        .with_name("input")
        .max_height(3)
        .min_width(50)
        .full_width();

    // Help text for user commands
    let help_text = TextView::new("ESC:quit | Enter:send | Commands: /help, /clear, /quit, ...")
        .style(Color::Dark(BaseColor::White));

    let layout = LinearLayout::vertical()
        .child(Panel::new(header))
        .child(
            Dialog::around(messages)
                .title("Chat")
                .title_position(HAlign::Center)
                .full_width(),
        )
        .child(
            Dialog::around(input)
                .title("Message")
                .title_position(HAlign::Center)
                .full_width(),
        )
        .child(Panel::new(help_text).full_width());

    let centered_layout = LinearLayout::horizontal()
        .child(DummyView.full_width())
        .child(layout)
        .child(DummyView.full_width());

    siv.add_fullscreen_layer(centered_layout);

    // Global keybindinds
    siv.add_global_callback(Key::Esc, |s| s.quit());
    siv.add_global_callback('/', |s| {
        s.call_on_name("input", |view: &mut EditView| {
            view.set_content("/");
        });
    });
}

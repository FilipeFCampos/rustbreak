use cursive::{
    Cursive,
    align::HAlign,
    event::Key,
    theme::{BaseColor, BorderStyle, Color, Palette, PaletteColor, Theme},
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
    // Header to be displayed at the top
    let header = TextView::new(format!(r#"⋘ ( *^-^)ρ ⭐ WELCOME {username} ⭐ &(^0^* )⋙"#))
        .style(Color::Dark(BaseColor::Yellow))
        .h_align(HAlign::Center);

    // Message area
    let messages = TextView::new("")
        .with_name("messages")
        .min_height(20)
        .scrollable();

    // TODO: For some unknown reason the scroll is not sticking to the bottom. Fix it.
    let messages = ScrollView::new(messages)
        .scroll_strategy(cursive::view::ScrollStrategy::StickToBottom)
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

/// Creates a personalized theme for the TUI and returns it.
pub fn create_theme() -> Theme {
    let mut theme = Theme::default();
    theme.shadow = true;
    theme.borders = BorderStyle::Simple;

    let mut palette = Palette::default();
    palette[PaletteColor::Background] = Color::Rgb(0, 0, 20);
    palette[PaletteColor::View] = Color::Rgb(0, 0, 0);
    palette[PaletteColor::Primary] = Color::Rgb(10, 36, 207);
    palette[PaletteColor::TitlePrimary] = Color::Rgb(0, 107, 189);
    palette[PaletteColor::Secondary] = Color::Dark(BaseColor::Yellow);
    palette[PaletteColor::Highlight] = Color::Rgb(0, 128, 255);
    palette[PaletteColor::HighlightInactive] = Color::Rgb(64, 64, 64);
    palette[PaletteColor::Shadow] = Color::Rgb(0, 0, 40);
    theme.palette = palette;

    theme
}

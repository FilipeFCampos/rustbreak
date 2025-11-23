// src/client/mod.rs
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

// Função para forçar scroll para baixo (usada quando novas mensagens chegam)
pub fn scroll_to_bottom(siv: &mut Cursive) {
    // Primeiro atualizamos o estado
    if let Some(scroll_state) = siv.user_data::<ScrollState>() {
        scroll_state.auto_scroll = true;
    }

    // Depois fazemos o scroll
    siv.call_on_name("chat_scroll", |view: &mut ScrollView<NamedView<TextView>>| {
        view.scroll_to_bottom();
    });
}

// Função para atualizar o estado baseado na posição atual do scroll
pub fn check_scroll_position(siv: &mut Cursive) {
    // Não podemos verificar facilmente a posição do scroll, então vamos
    // simplesmente desativar o auto-scroll quando o usuário interagir com o scroll
    if let Some(scroll_state) = siv.user_data::<ScrollState>() {
        scroll_state.auto_scroll = false;
    }
}

// Função para reativar o auto-scroll (por exemplo, quando o usuário vai manualmente para o final)
pub fn enable_auto_scroll(siv: &mut Cursive) {
    if let Some(scroll_state) = siv.user_data::<ScrollState>() {
        scroll_state.auto_scroll = true;
    }
}

pub fn add_scroll_callbacks(siv: &mut Cursive) {
    // Desativar auto-scroll quando o usuário interage com o scroll
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

    // Adicionar uma tecla para reativar o auto-scroll (por exemplo, End)
    siv.add_global_callback(cursive::event::Key::End, enable_auto_scroll);
}
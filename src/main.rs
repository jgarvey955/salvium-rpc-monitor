mod app;
mod inventory;
mod rpc;
mod settings;

use app::AppState;
use settings::WindowState;
use iced::{Theme, application, theme};

fn main() -> iced::Result {
    let window_state = WindowState::load().unwrap_or_default();

    application(AppState::init, AppState::update, AppState::view)
        .subscription(AppState::subscription)
        .theme(app_theme)
        .style(app_style)
        .title(app_title)
        .centered()
        .window_size(window_state.size())
        .run()
}

fn app_theme(_: &AppState) -> Theme {
    Theme::Dark
}

fn app_style(_: &AppState, _: &Theme) -> theme::Style {
    theme::Style {
        background_color: iced::Color::from_rgb(0.07, 0.07, 0.08),
        text_color: iced::Color::from_rgb(0.93, 0.93, 0.93),
    }
}

fn app_title(_: &AppState) -> String {
    String::from("Salvium Monitor")
}

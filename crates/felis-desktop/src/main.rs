use iced::widget::{column, container, text};
use iced::{Color, Element, Fill, Settings, Task, window};

fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view)
        .title("felis desktop")
        .settings(Settings {
            antialiasing: true,
            ..Settings::default()
        })
        .window(window::Settings {
            maximized: true,
            decorations: false,
            ..window::Settings::default()
        })
        .run()
}

#[derive(Default)]
struct App;

#[derive(Debug, Clone)]
enum Message {}

impl App {
    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        container(column![text("felis").size(14).color(Color::WHITE),].padding(12))
            .width(Fill)
            .height(Fill)
            .style(|_theme| container::Style {
                background: Some(Color::from_rgb(0.08, 0.10, 0.15).into()),
                ..Default::default()
            })
            .into()
    }
}

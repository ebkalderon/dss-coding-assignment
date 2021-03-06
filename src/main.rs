//! Main entry point for the application.

use anyhow::Error;
use dss_menu::app::App;
use dss_menu::fetcher;
use dss_menu::menu::{Menu, WidgetKind};

const WINDOW_WIDTH: u32 = 1920;
const WINDOW_HEIGHT: u32 = 1080;
const WINDOW_TITLE: &str = "Disney Streaming Services";

fn main() -> anyhow::Result<()> {
    let context = sdl2::init().map_err(Error::msg)?;
    let video_sys = context.video().map_err(Error::msg)?;

    let window = video_sys
        .window(WINDOW_TITLE, WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .build()?;

    let fetcher = fetcher::spawn();
    let (width, height) = window.size();
    let root_widget = WidgetKind::new_root(width, height);
    App::new(Menu::new(fetcher), root_widget)
        .with_error_message_box(WINDOW_TITLE)
        .run(context, window)
}

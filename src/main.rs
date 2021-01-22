use anyhow::Error;
use dss_menu::app::App;
use dss_menu::menu::{Menu, WidgetKind};

const WINDOW_WIDTH: u32 = 1280;
const WINDOW_HEIGHT: u32 = 800;

fn main() -> anyhow::Result<()> {
    let context = sdl2::init().map_err(Error::msg)?;
    let video_sys = context.video().map_err(Error::msg)?;

    let window = video_sys
        .window("Disney Streaming Services", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .build()?;

    let root_widget = WidgetKind::new_root(WINDOW_WIDTH, WINDOW_HEIGHT);
    App::new(Menu::default(), root_widget).run(context, window)
}

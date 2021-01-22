//! Business logic for the application.

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Texture;

use crate::app::{Action, State};
use crate::widget::{Context, Properties, Widget, Widgets};

const BACKGROUND_COLOR: Color = Color::RGB(8, 76, 97);
const TILE_COLOR: Color = Color::RGB(23, 126, 127);
const TILE_WIDTH: u32 = 500;
const TILE_HEIGHT: u32 = 281;

/// Contains the state for the main menu.
#[derive(Debug, Default)]
pub struct Menu;

impl State<WidgetKind> for Menu {
    fn initialize(&mut self, widgets: &mut Widgets<WidgetKind>) -> anyhow::Result<()> {
        let _tile = widgets
            .insert(WidgetKind::new_tile(16, 32), widgets.root())
            .unwrap();

        Ok(())
    }

    fn handle_event(&mut self, event: &Event, _widgets: &mut Widgets<WidgetKind>) -> Action {
        match *event {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => Action::Quit,
            _ => Action::Continue,
        }
    }
}

/// A list of types which implement the [`Widget`](crate::widget::Widget) trait.
#[derive(Debug)]
pub enum WidgetKind {
    Root { properties: Properties },
    Tile { properties: Properties },
}

impl WidgetKind {
    /// Creates a new root widget with the given width and height.
    pub fn new_root(width: u32, height: u32) -> Self {
        WidgetKind::Root {
            properties: Properties {
                origin: (0, 0),
                bounds: (width, height),
                color: BACKGROUND_COLOR,
            },
        }
    }

    /// Creates a new image tile of a fixed size located at the given (X, Y) coordinate.
    pub fn new_tile(x: i32, y: i32) -> Self {
        WidgetKind::Tile {
            properties: Properties {
                origin: (x, y),
                bounds: (TILE_WIDTH, TILE_HEIGHT),
                color: TILE_COLOR,
            },
        }
    }
}

impl Widget for WidgetKind {
    fn properties(&self) -> &Properties {
        match *self {
            WidgetKind::Root { ref properties } => properties,
            WidgetKind::Tile { ref properties } => properties,
        }
    }

    fn properties_mut(&mut self) -> &mut Properties {
        match *self {
            WidgetKind::Root { ref mut properties } => properties,
            WidgetKind::Tile { ref mut properties } => properties,
        }
    }

    fn draw(&self, ctx: &mut Context, target: &mut Texture) -> anyhow::Result<()> {
        match *self {
            WidgetKind::Root { ref properties } => {
                let Properties { color, .. } = properties;
                ctx.canvas.with_texture_canvas(target, |texture| {
                    texture.set_draw_color(*color);
                    texture.clear();
                })?
            }
            WidgetKind::Tile { ref properties } => {
                let (x, y) = properties.origin;
                let (width, height) = properties.bounds;
                let rect = Rect::new(x, y, width, height);
                ctx.canvas.with_texture_canvas(target, |texture| {
                    texture.set_draw_color(properties.color);
                    texture.draw_rect(rect).unwrap();
                    texture.clear();
                })?;
            }
        }

        Ok(())
    }
}

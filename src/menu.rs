//! Business logic for the application.

use anyhow::anyhow;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Texture;
use sdl2::ttf::FontStyle;

use crate::app::{Action, Context, Properties, State, Widget, Widgets};
use crate::fetcher::Fetcher;
use crate::schema::{Home, TitleKind};

const HOME_JSON_URL: &str = "https://cd-static.bamgrid.com/dp-117731241344/home.json";

const BACKGROUND_COLOR: Color = Color::RGB(7, 27, 15);
const RIGHT_MARGIN: i32 = 52;
const TOP_MARGIN: i32 = 68;

const FONT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fonts/Cocogoose-Classic-Medium-trial.ttf"
);
const FONT_STYLE: FontStyle = FontStyle::NORMAL;
const LABEL_PADDING: u32 = 18;

const TILE_COLOR: Color = Color::RGB(23, 126, 127);
const TILE_WIDTH: u32 = 500;
const TILE_HEIGHT: u32 = 281;
const TILE_MARGIN: u32 = 28;

/// Contains the state for the main menu.
#[derive(Debug)]
pub struct Menu {
    fetcher: Fetcher,
}

impl Menu {
    /// Creates a new `Menu` application using the given HTTP fetcher.
    pub fn new(fetcher: Fetcher) -> Self {
        Menu { fetcher }
    }
}

impl State<WidgetKind> for Menu {
    fn initialize(&mut self, widgets: &mut Widgets<WidgetKind>) -> anyhow::Result<()> {
        let path = self.fetcher.fetch(HOME_JSON_URL.to_owned())?;
        let json = std::fs::read_to_string(path)?;
        let home_menu: Home = serde_json::from_str(&json)?;
        let (max_width, _) = widgets.get(widgets.root()).properties().bounds;

        let containers = home_menu
            .data
            .get("StandardCollection")
            .ok_or(anyhow!("key `StandardCollection` does not exist"))?
            .containers()
            .ok_or(anyhow!("`StandardCollection` is not a standard collection"))?;

        for (i, row) in containers.iter().enumerate() {
            let (label_id, label_y, label_height) = {
                let text = row
                    .set
                    .text()
                    .title
                    .get(TitleKind::Full)
                    .ok_or_else(|| anyhow!("Full title for collection {} not found", i))?;

                let label = WidgetKind::new_label(
                    text.content.clone(),
                    42,
                    RIGHT_MARGIN,
                    TOP_MARGIN + (i as u32 * (TILE_HEIGHT + 156)) as i32,
                    max_width,
                );

                let (_, y) = label.properties().origin;
                let (_, height) = label.properties().bounds;
                let id = widgets.insert(label, widgets.root()).unwrap();

                (id, y, height)
            };

            for j in 0..11 {
                let _tile_id = widgets
                    .insert(
                        WidgetKind::new_tile(
                            RIGHT_MARGIN + (j * (TILE_WIDTH + TILE_MARGIN)) as i32,
                            label_y + (label_height + LABEL_PADDING) as i32,
                        ),
                        label_id,
                    )
                    .unwrap();
            }
        }

        Ok(())
    }

    fn handle_event(&mut self, event: &Event, _widgets: &mut Widgets<WidgetKind>) -> Action {
        match *event {
            Event::Quit { .. } => return Action::Quit,
            Event::KeyDown {
                keycode: Some(key),
                repeat: false,
                ..
            } => match key {
                Keycode::Escape => return Action::Quit,
                Keycode::Up | Keycode::Down => println!("Panning {:?}", key),
                Keycode::Left | Keycode::Right => println!("Sliding {:?}", key),
                _ => {}
            },
            _ => {}
        }

        Action::Continue
    }
}

/// A list of types which implement the [`Widget`](crate::app::Widget) trait.
#[derive(Debug)]
pub enum WidgetKind {
    Root {
        properties: Properties,
    },
    Label {
        text: String,
        point_size: u16,
        properties: Properties,
    },
    Tile {
        properties: Properties,
    },
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

    /// Creates a new label widget with the given text and properties.
    pub fn new_label(text: String, point_size: u16, x: i32, y: i32, max_width: u32) -> Self {
        // This is a decent height approximation with a bit of extra padding on the bottom.
        let approx_height = (point_size as f32 * 1.333f32) as u32;
        WidgetKind::Label {
            text,
            point_size,
            properties: Properties {
                origin: (x, y),
                bounds: (max_width, approx_height),
                color: Color::WHITE,
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
        match self {
            WidgetKind::Root { properties } => properties,
            WidgetKind::Label { properties, .. } => properties,
            WidgetKind::Tile { properties } => properties,
        }
    }

    fn properties_mut(&mut self) -> &mut Properties {
        match self {
            WidgetKind::Root { properties } => properties,
            WidgetKind::Label { properties, .. } => properties,
            WidgetKind::Tile { properties } => properties,
        }
    }

    fn draw(&self, ctx: &mut Context, target: &mut Texture) -> anyhow::Result<()> {
        match self {
            WidgetKind::Root { properties } => {
                let Properties { color, .. } = properties;
                ctx.canvas.with_texture_canvas(target, |texture| {
                    texture.set_draw_color(*color);
                    texture.clear();
                })?
            }
            WidgetKind::Label {
                properties,
                text,
                point_size,
            } => {
                let text = ctx.textures.render_text(
                    FONT_PATH,
                    *point_size,
                    FONT_STYLE,
                    properties,
                    &text,
                )?;

                let (x, y) = properties.origin;
                let (width, height) = text.bounds;
                ctx.canvas.with_texture_canvas(target, |texture| {
                    texture.set_draw_color(BACKGROUND_COLOR);
                    texture.draw_rect(Rect::new(x, y, width, height)).unwrap();
                    texture.clear();

                    let dst = Rect::new(0, 0, width, height);
                    texture.copy(&text.texture, None, dst).unwrap();
                })?;
            }
            WidgetKind::Tile { properties } => {
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

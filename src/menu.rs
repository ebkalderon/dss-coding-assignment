//! Business logic for the application.

use std::path::PathBuf;
use std::rc::Rc;
use std::task::Poll;

use anyhow::{anyhow, Context as Context_};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Texture;
use sdl2::ttf::FontStyle;
use url::Url;

use crate::app::{Action, Context, Properties, State, Widget, Widgets};
use crate::fetcher::Fetcher;
use crate::schema::{self, Set};

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
const TILE_IMAGE_NAME: &str = "tile";
const TILE_ASPECT_RATIO: &str = "1.78";
const TILE_WIDTH: u32 = 500;
const TILE_HEIGHT: u32 = 281;
const TILE_MARGIN: u32 = 28;

/// Contains the state for the main menu.
#[derive(Debug)]
pub struct Menu {
    fetcher: Rc<Fetcher>,
}

impl Menu {
    /// Creates a new `Menu` application using the given HTTP fetcher.
    pub fn new(f: Fetcher) -> Self {
        let fetcher = Rc::new(f);
        Menu { fetcher }
    }
}

impl State<WidgetKind> for Menu {
    fn initialize(&mut self, widgets: &mut Widgets<WidgetKind>) -> anyhow::Result<()> {
        let (max_width, _) = widgets.get(widgets.root()).properties().bounds;

        let url = HOME_JSON_URL.parse()?;
        let home_menu = download_home_json(url, &self.fetcher)?;
        let rows = get_menu_rows(&home_menu)?;

        for (i, row) in rows.iter().enumerate() {
            let (label_id, label_y, label_height) = {
                let title = get_row_title(row, i)?;

                let label = WidgetKind::new_label(
                    title.to_owned(),
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

            match &row.set {
                Set::Curated { items, .. } => {
                    for (j, tile) in items.iter().enumerate() {
                        let image_url = get_tile_image_url(&tile)?;

                        let _tile_id = widgets
                            .insert(
                                WidgetKind::new_tile(
                                    RIGHT_MARGIN + (j as u32 * (TILE_WIDTH + TILE_MARGIN)) as i32,
                                    label_y + (label_height + LABEL_PADDING) as i32,
                                    image_url.clone(),
                                    self.fetcher.clone(),
                                ),
                                label_id,
                            )
                            .unwrap();
                    }
                }
                Set::Ref { .. } => {} // TODO: Need to implement lazy loading.
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
        image: Thumbnail,
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
    pub fn new_tile(x: i32, y: i32, image_url: Url, fetcher: Rc<Fetcher>) -> Self {
        WidgetKind::Tile {
            image: Thumbnail::Pending(fetcher, image_url),
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
            WidgetKind::Tile { properties, .. } => properties,
        }
    }

    fn properties_mut(&mut self) -> &mut Properties {
        match self {
            WidgetKind::Root { properties } => properties,
            WidgetKind::Label { properties, .. } => properties,
            WidgetKind::Tile { properties, .. } => properties,
        }
    }

    fn draw(&mut self, ctx: &mut Context, target: &mut Texture) -> anyhow::Result<()> {
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
            WidgetKind::Tile { properties, image } => {
                let textures = &mut ctx.textures;
                let thumbnail = image
                    .poll_ready()
                    .transpose()
                    .map(|result| {
                        let path = result?;
                        textures
                            .load_image(&path)
                            .with_context(|| format!("thumbnail {:?} failed to load", path))
                    })
                    .transpose()?;

                let (x, y) = properties.origin;
                let (width, height) = properties.bounds;
                let rect = Rect::new(x, y, width, height);

                ctx.canvas.with_texture_canvas(target, |texture| {
                    texture.set_draw_color(properties.color);
                    texture.draw_rect(rect).unwrap();
                    texture.clear();

                    if let Some(ref thumbnail) = thumbnail {
                        let dst = Rect::new(0, 0, width, height);
                        texture.copy(&thumbnail, None, dst).unwrap();
                    }
                })?;
            }
        }

        Ok(())
    }
}

/// A thumbnail image for a [`WidgetKind::Tile`].
#[derive(Debug)]
pub enum Thumbnail {
    /// Represents a downloaded thumbnail that is cached on disk.
    Ready(PathBuf),
    /// Represents a pending thumbnail that is currently being fetched.
    Pending(Rc<Fetcher>, Url),
}

impl Thumbnail {
    /// Attempts to return the path to the downloaded image file, if it is ready.
    ///
    /// This method does _not_ block if the file is not ready. If the download is still pending,
    /// the current status can be polled again by repeatedly calling this method. If the file from
    /// the requested URL already exists on disk, its path will be returned immediately.
    ///
    /// Returns `Err(_)` if the file at the target URL does not exist, an I/O error occurred, or
    /// the background worker thread was terminated.
    fn poll_ready(&mut self) -> anyhow::Result<Option<&PathBuf>> {
        match *self {
            Thumbnail::Ready(ref path) => Ok(Some(path)),
            Thumbnail::Pending(ref fetcher, ref url) => match fetcher.poll_fetch(url.clone()) {
                Poll::Pending => Ok(None),
                Poll::Ready(result) => {
                    let path = result?;
                    *self = Thumbnail::Ready(path);
                    self.poll_ready()
                }
            },
        }
    }
}

fn download_home_json(_url: Url, _fetcher: &Fetcher) -> anyhow::Result<schema::Home> {
    // FIXME: Due to an apparent upstream API bug (reported via email), we will have to stick to
    // the patched local copy of `home.json` for now.
    // let path = fetcher.fetch(url)?;
    // let json = std::fs::read_to_string(path)?;
    let json = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/home.json"));
    let menu = serde_json::from_str(&json)?;
    Ok(menu)
}

fn get_menu_rows(menu: &schema::Home) -> anyhow::Result<&[schema::Container]> {
    menu.data
        .get("StandardCollection")
        .ok_or(anyhow!("key `StandardCollection` does not exist"))?
        .containers()
        .ok_or(anyhow!("`StandardCollection` is not a standard collection"))
}

fn get_row_title(row: &schema::Container, row_idx: usize) -> anyhow::Result<&str> {
    row.set
        .text()
        .title
        .get(schema::TitleKind::Full)
        .ok_or_else(|| anyhow!("full title for collection {} not found", row_idx))
        .map(|text| text.content.as_str())
}

fn get_tile_image_url(tile: &schema::Collection) -> anyhow::Result<&Url> {
    let tile_name = tile
        .text()
        .title
        .get(schema::TitleKind::Full)
        .map(|text| text.content.as_str())
        .unwrap_or("unknown");

    tile.images()
        .get(TILE_IMAGE_NAME)
        .ok_or_else(|| {
            anyhow!(
                "image named {:?} not found for {:?} tile",
                TILE_IMAGE_NAME,
                tile_name
            )
        })?
        .get(TILE_ASPECT_RATIO)
        .map(|image| &image.url)
        .ok_or_else(|| {
            anyhow!(
                "image aspect ratio {:?} not found for {:?} tile",
                TILE_ASPECT_RATIO,
                tile_name
            )
        })
}

#[cfg(test)]
mod tests {
    use schema::Home;

    use super::*;

    const HOME_JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/home.json"));

    #[test]
    fn gets_menu_rows() {
        let h: Home = serde_json::from_str(HOME_JSON).expect("failed to deserialize `home.json`");
        let _rows = get_menu_rows(&h).expect("failed to get home menu rows");
    }

    #[test]
    fn gets_row_title() {
        let h: Home = serde_json::from_str(HOME_JSON).expect("failed to deserialize `home.json`");
        let rows = get_menu_rows(&h).expect("failed to get home menu rows");
        let (i, first) = rows.iter().enumerate().next().expect("must not be empty");
        let title = get_row_title(&first, i).expect("failed to retrieve home menu rows");
        assert_eq!(title, "New to Disney+");
    }

    #[test]
    fn gets_tile_image_url() {
        let h: Home = serde_json::from_str(HOME_JSON).expect("failed to deserialize `home.json`");
        let rows = get_menu_rows(&h).expect("failed to get home menu rows");
        let first = rows.iter().next().expect("must not be empty");

        let url = match &first.set {
            Set::Ref { .. } => panic!("expected `CuratedSet`, found `SetRef`"),
            Set::Curated { items, .. } => get_tile_image_url(&items[0]).expect("image not found"),
        };

        assert_eq!(
            url.as_str(),
            "https://prod-ripcut-delivery.disney-plus.net/v1/variant/disney/3C33485A3043C22B8C89E131693E8B5B9306DAA4E48612A655560752977728A6/scale?format=jpeg&quality=90&scalingAlgorithm=lanczos3&width=500"
        );
    }
}

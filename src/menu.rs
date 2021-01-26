//! Business logic for the application.

use std::path::PathBuf;
use std::rc::Rc;
use std::task::Poll;

use anyhow::anyhow;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Texture;
use sdl2::ttf::FontStyle;
use url::Url;

use crate::app::{Action, Context, Fullscreen, Properties, State, Widget, WidgetId, Widgets};
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
const LABEL_POINT_SIZE: u16 = 42;
const LABEL_PADDING: u32 = 18;

const TILE_COLOR: Color = Color::RGB(23, 126, 127);
const TILE_IMAGE_NAME: &str = "tile";
const TILE_ASPECT_RATIO: &str = "1.78";
const TILE_WIDTH: u32 = 500;
const TILE_HEIGHT: u32 = 281;
const TILE_MARGIN: u32 = 28;

const ROW_HEIGHT: u32 = TILE_HEIGHT + 156;

const CURSOR_BORDER_COLOR: Color = Color::WHITE;
const CURSOR_BORDER_WIDTH: u8 = 10;
const CURSOR_SCALE_FACTOR: f32 = 1.1;

/// A zero or negative integer offset which marks how far to the right a grid row is scrolled.
///
/// See the documentation for [`Menu::select_tile()`] for more.
type ScrollOffset = isize;

/// Contains the state for the main menu.
#[derive(Debug)]
pub struct Menu {
    fetcher: Rc<Fetcher>,
    rows: Vec<(WidgetId, ScrollOffset)>,
    selected_tile: (usize, usize),
    grid_root: WidgetId,
}

impl Menu {
    /// Creates a new `Menu` application using the given HTTP fetcher.
    #[inline]
    pub fn new(f: Fetcher) -> Self {
        Menu {
            fetcher: Rc::new(f),
            rows: Vec::new(),
            selected_tile: (0, 0),
            grid_root: WidgetId::root(),
        }
    }

    /// Scrolls the entire menu one row up.
    fn move_up(&mut self, widgets: &mut Widgets<WidgetKind>) {
        let (row, column) = self.selected_tile;
        self.select_tile(row.saturating_sub(1), column, widgets);
    }

    /// Scrolls the entire menu one row down.
    fn move_down(&mut self, widgets: &mut Widgets<WidgetKind>) {
        let (row, column) = self.selected_tile;
        self.select_tile(row + 1, column, widgets);
    }

    /// Scrolls the current row one tile to the left.
    fn move_left(&mut self, widgets: &mut Widgets<WidgetKind>) {
        let (row, column) = self.selected_tile;
        self.select_tile(row, column.saturating_sub(1), widgets);
    }

    /// Scrolls the current row one tile to the right.
    fn move_right(&mut self, widgets: &mut Widgets<WidgetKind>) {
        let (row, column) = self.selected_tile;
        self.select_tile(row, column + 1, widgets);
    }

    /// Selects an arbitrary tile from the menu grid, given its row/column position.
    fn select_tile(&mut self, row: usize, column: usize, widgets: &mut Widgets<WidgetKind>) {
        let (cur_row, cur_column) = self.selected_tile;
        let (cur_tile_id, cur_scroll_offset) = self.rows[cur_row];

        if let Some((anchor_id, scroll_offset)) = self.rows.get(row).copied() {
            let tile_ids = widgets.get_children_of(anchor_id);
            let column = find_tile_index(column, cur_scroll_offset, scroll_offset);

            if let Some(tile_id) = tile_ids.get(column) {
                // Deselect the current tile and scale it down, if necessary.
                let (delta_width, delta_height) = {
                    let cur_tile_ids = widgets.get_children_of(cur_tile_id)[cur_column];
                    let mut tile = widgets.get_mut(cur_tile_ids);

                    let (width, height) = tile.bounds();
                    let new_width = (width as f32 * (1.0 / CURSOR_SCALE_FACTOR)) as u32;
                    let new_height = (height as f32 * (1.0 / CURSOR_SCALE_FACTOR)) as u32;

                    let delta_width = width - new_width;
                    let delta_height = height - new_height;

                    // Confirm that this tile is actually scaled up before shrinking it back down.
                    if width != TILE_WIDTH && height != TILE_HEIGHT {
                        tile.set_bounds(new_width, new_height);

                        let (x, y) = tile.origin();
                        let new_x = x + delta_width as i32 / 2;
                        let new_y = y + delta_height as i32 / 2;
                        tile.set_origin(new_x, new_y);
                    }

                    tile.clear_border();

                    (delta_width, delta_height)
                };

                // Select the new tile and scale it up.
                let (new_tile_x, new_tile_y) = {
                    let mut tile = widgets.get_mut(*tile_id);

                    let (width, height) = tile.bounds();
                    tile.set_bounds(width + delta_width, height + delta_height);

                    let (x, y) = tile.origin();
                    let new_x = x - delta_width as i32 / 2;
                    let new_y = y - delta_height as i32 / 2;
                    tile.set_origin(new_x, new_y);

                    tile.set_border(CURSOR_BORDER_COLOR, CURSOR_BORDER_WIDTH);

                    (new_x, new_y)
                };

                // Scroll the entire page up and down, if necessary.
                let (root_x, root_y) = widgets.get(widgets.root()).origin();
                let (root_w, root_h) = widgets.get(widgets.root()).bounds();

                if cur_row > row {
                    let (_, grid_y) = widgets.get(self.grid_root).origin();

                    let should_scroll_up = new_tile_y + (TILE_HEIGHT as i32) < root_h as i32 / 2;
                    let is_not_first_row = grid_y < root_y;

                    if should_scroll_up && is_not_first_row {
                        widgets.translate(self.grid_root, 0, ROW_HEIGHT as i32);
                    }
                } else if cur_row < row {
                    let should_scroll_down = new_tile_y - TILE_HEIGHT as i32 > (root_h as i32) / 2;

                    if should_scroll_down {
                        widgets.translate(self.grid_root, 0, -(ROW_HEIGHT as i32));
                    }
                }

                // Scroll the current row left and right, if necessary.
                if cur_column > column {
                    let (anchor_x, _) = widgets.get(anchor_id).origin();

                    let should_scroll_left = new_tile_x < root_x as i32;
                    let is_not_first_column = anchor_x < root_x;

                    if should_scroll_left && is_not_first_column {
                        widgets.translate(anchor_id, TILE_WIDTH as i32 + TILE_MARGIN as i32, 0);
                        self.rows[cur_row].1 += 1;
                    }
                } else if cur_column < column {
                    let should_scroll_right = new_tile_x + TILE_WIDTH as i32 > root_w as i32;

                    if should_scroll_right {
                        widgets.translate(anchor_id, -(TILE_WIDTH as i32 + TILE_MARGIN as i32), 0);
                        self.rows[cur_row].1 -= 1;
                    }
                }

                self.selected_tile = (row, column);
            }
        }
    }
}

/// Computes the array index of the menu tile widget we want to select using the scroll offsets.
///
/// We want the entire interface to scroll up/down in lockstep, but tiles within the current row
/// should scroll left/right freely. We use the signed integer offsets from both `rows[cur_row]`
/// and `rows[row]` to compute the correct array index for the tile widget we want to select.
#[inline]
const fn find_tile_index(column: usize, scroll_offset: isize, adj_scroll_offset: isize) -> usize {
    (column as isize + scroll_offset - adj_scroll_offset) as usize
}

impl State<WidgetKind> for Menu {
    fn initialize(&mut self, widgets: &mut Widgets<WidgetKind>) -> anyhow::Result<()> {
        let (max_width, _) = widgets.get(widgets.root()).bounds();

        // This is the invisible anchor point to which the entire menu can be scrolled up/down.
        self.grid_root = widgets.insert(WidgetKind::new_anchor(0, 0), widgets.root());

        let url = HOME_JSON_URL.parse()?;
        let home_menu = download_home_json(url, &self.fetcher)?;
        let rows = get_menu_rows(&home_menu)?;
        self.rows.reserve(rows.len());

        for (i, row) in rows.iter().enumerate() {
            let (label_x, label_y, label_height) = {
                let title = get_row_title(row, i)?;

                let label = WidgetKind::new_label(
                    title.to_owned(),
                    LABEL_POINT_SIZE,
                    RIGHT_MARGIN,
                    TOP_MARGIN + (i as u32 * ROW_HEIGHT) as i32,
                    max_width,
                );

                // We affix labels to `grid_root` so that it can scroll up/down as the user presses
                // `UP` and `DOWN`, but remains stationary when the user scrolls left/right.
                let (x, y) = label.origin();
                let (_, height) = label.bounds();
                let _label_id = widgets.insert(label, self.grid_root);

                (x, y, height)
            };

            match &row.set {
                Set::Curated { items, .. } => {
                    // This invisible anchor point is used to scroll the current row of tiles
                    // left/right independently of all the other rows.
                    let row_id =
                        widgets.insert(WidgetKind::new_anchor(label_x, label_y), self.grid_root);

                    // Mark that the current row hasn't been scrolled horizontally by the user yet.
                    // This value comes in handy later in `select_tile()`.
                    let scroll_offset = 0;
                    self.rows.push((row_id, scroll_offset));

                    // Create a row of tiles whose thumbnails are loaded in asynchronously.
                    for (j, tile) in items.iter().enumerate() {
                        let image_url = get_tile_image_url(&tile)?;

                        let tile = WidgetKind::new_tile(
                            RIGHT_MARGIN + (j as u32 * (TILE_WIDTH + TILE_MARGIN)) as i32,
                            label_y + (label_height + LABEL_PADDING) as i32,
                            image_url.clone(),
                            self.fetcher.clone(),
                        );

                        let _tile_id = widgets.insert(tile, row_id);
                    }

                    // Increment the height of `grid_root` so that its dimensions include this row.
                    widgets.get_mut(self.grid_root).properties_mut().bounds.1 += ROW_HEIGHT;
                }
                Set::Ref { .. } => {} // TODO: Need to implement lazy ref set loading.
            }
        }

        // The menu grid is populated, so select the current tile.
        let (row, column) = self.selected_tile;
        self.select_tile(row, column, widgets);

        Ok(())
    }

    fn handle_event(&mut self, event: &Event, widgets: &mut Widgets<WidgetKind>) -> Action {
        match *event {
            Event::Quit { .. } => return Action::Quit,
            Event::KeyDown { keycode, .. } => match keycode {
                Some(Keycode::Up) => self.move_up(widgets),
                Some(Keycode::Down) => self.move_down(widgets),
                Some(Keycode::Left) => self.move_left(widgets),
                Some(Keycode::Right) => self.move_right(widgets),
                Some(Keycode::Escape) => return Action::Quit,
                Some(Keycode::F11) => return Action::Fullscreen(Fullscreen::Toggle),
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
    Anchor {
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
                bounds: (width, height),
                color: BACKGROUND_COLOR,
                ..Default::default()
            },
        }
    }

    /// Creates a new invisible anchor point for other widgets to attach to.
    pub fn new_anchor(x: i32, y: i32) -> Self {
        WidgetKind::Anchor {
            properties: Properties {
                origin: (x, y),
                bounds: (1, 1),
                hidden: true,
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
            },
        }
    }
}

impl Widget for WidgetKind {
    fn properties(&self) -> &Properties {
        match self {
            WidgetKind::Root { properties } => properties,
            WidgetKind::Anchor { properties } => properties,
            WidgetKind::Label { properties, .. } => properties,
            WidgetKind::Tile { properties, .. } => properties,
        }
    }

    fn properties_mut(&mut self) -> &mut Properties {
        match self {
            WidgetKind::Root { properties } => properties,
            WidgetKind::Anchor { properties } => properties,
            WidgetKind::Label { properties, .. } => properties,
            WidgetKind::Tile { properties, .. } => properties,
        }
    }

    fn update(&mut self) {
        if let WidgetKind::Tile { image, properties } = self {
            let prev_state = image.is_ready();
            let next_state = image.poll_ready().transpose().is_some();

            // Redraw tile widgets if the thumbnail is done downloading.
            if prev_state != next_state {
                properties.invalidated = true;
            }
        }
    }

    fn draw(&mut self, ctx: &mut Context, target: &mut Texture) -> anyhow::Result<()> {
        match self {
            WidgetKind::Root { properties } | WidgetKind::Anchor { properties } => {
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

                let (width, height) = text.bounds;
                let rect = Rect::new(0, 0, width, height);
                ctx.canvas.with_texture_canvas(target, |texture| {
                    texture.set_draw_color(BACKGROUND_COLOR);
                    texture.clear();
                    texture.copy(&text.texture, None, rect).unwrap();
                })?;
            }
            WidgetKind::Tile { properties, image } => {
                let textures = &mut ctx.textures;

                // If the thumbnail is ready but resulted in error, or if the file could not be
                // loaded as a texture, just show a blank tile.
                let thumbnail = image
                    .poll_ready()
                    .transpose()
                    .and_then(|result| result.and_then(|path| textures.load_image(&path)).ok());

                let (width, height) = properties.bounds;
                let rect = Rect::new(0, 0, width, height);

                ctx.canvas.with_texture_canvas(target, |texture| {
                    texture.set_draw_color(properties.color);
                    texture.clear();

                    if let Some(ref thumbnail) = thumbnail {
                        texture.copy(&thumbnail, None, rect).unwrap();
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
    /// Represents a thumbnail that is currently being downloaded.
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

    /// Returns `true` if the thumbnail is cached on disk, ready to display.
    #[inline]
    fn is_ready(&self) -> bool {
        match *self {
            Thumbnail::Ready(_) => true,
            _ => false,
        }
    }
}

fn download_home_json(url: Url, fetcher: &Fetcher) -> anyhow::Result<schema::Home> {
    let path = fetcher.fetch(url)?;
    let json = std::fs::read_to_string(path)?;
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

    #[test]
    fn computes_adjacent_tile_index() {
        let requested_column = 4;
        let cur_scroll_offset = -1;
        let adj_scroll_offset = 1;
        assert_eq!(
            find_tile_index(requested_column, cur_scroll_offset, adj_scroll_offset),
            2
        );
    }
}

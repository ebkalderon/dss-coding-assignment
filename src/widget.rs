//! Generic code for implementing UI widgets.

use anyhow::Error;
use fnv::FnvHashMap as HashMap;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;

/// A trait which describes a rectangular UI widget.
pub trait Widget {
    /// Returns the properties of the widget.
    fn properties(&self) -> &Properties;

    /// Renders the widget into a [`Texture`](sdl2::render::Texture) and returns it to the caller.
    fn draw(&self, canvas: &mut Canvas<Window>) -> anyhow::Result<Texture>;
}

/// Contains properties common to all widgets.
#[derive(Debug)]
pub struct Properties {
    /// The top-left (X, Y) coordinate pair denoting the widget's location.
    pub origin: (u32, u32),
    /// Width and height of the widget, in pixels.
    pub bounds: (u32, u32),
    /// Base color of the widget.
    pub color: Color,
}

/// A shared cache of drawable UI widgets.
#[derive(Debug)]
pub struct Widgets<W> {
    cache: HashMap<WidgetId, CacheEntry<W>>,
    next_id: u32,
}

impl<W: Widget> Widgets<W> {
    /// Creates a new [`Widgets`] cache anchored relative to the given `root_widget`.
    pub fn new(root_widget: W) -> Self {
        let mut cache = HashMap::default();
        cache.insert(WidgetId(0), CacheEntry::new(root_widget, WidgetId(0)));
        Widgets { cache, next_id: 1 }
    }

    /// Returns the [`WidgetId`] of the root widget.
    pub fn root(&self) -> WidgetId {
        WidgetId(0)
    }

    /// Inserts a widget into the cache, marked as a child of `parent`.
    ///
    /// Returns `None` if `parent` does not exist, otherwise returns a `Some` containing the
    /// [`WidgetId`] of the inserted widget.
    pub fn insert(&mut self, widget: W, parent: WidgetId) -> Option<WidgetId> {
        let id = WidgetId(self.next_id);

        if let Some(parent_entry) = self.cache.get_mut(&parent) {
            parent_entry.children.push(id);
            self.cache.insert(id, CacheEntry::new(widget, parent));

            self.next_id += 1;
            Some(id)
        } else {
            None
        }
    }

    /// Returns an immutable reference to a widget in the cache.
    pub fn get(&self, id: WidgetId) -> &W {
        &self.cache[&id].widget
    }

    /// Returns an immutable slice containing the children of the widget named `id`.
    pub fn get_children_of(&self, id: WidgetId) -> &[WidgetId] {
        &self.cache[&id].children[..]
    }

    /// Renders all the widgets in the cache to the canvas.
    pub fn draw(&self, canvas: &mut Canvas<Window>) -> anyhow::Result<()> {
        canvas.set_draw_color(Color::RGBA(255, 255, 255, 255));
        canvas.clear();

        self.draw_widget(self.root(), canvas)?;
        canvas.present();

        Ok(())
    }

    fn draw_widget(&self, id: WidgetId, canvas: &mut Canvas<Window>) -> anyhow::Result<()> {
        let widget = self.get(id);
        let (x, y) = widget.properties().origin;
        let (width, height) = widget.properties().bounds;

        let texture = widget.draw(canvas)?;
        let dst = Rect::new(x as i32, y as i32, width, height);
        canvas.copy(&texture, None, dst).map_err(Error::msg)?;

        for child_id in self.get_children_of(id) {
            if *child_id != id {
                self.draw_widget(*child_id, canvas)?;
            }
        }

        Ok(())
    }
}

/// A unique ID referring to a widget stored in a [`Widgets`] cache.
///
/// See the [`Widgets`] documentation for more info.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct WidgetId(u32);

#[derive(Debug)]
struct CacheEntry<W> {
    widget: W,
    parent: WidgetId,
    children: Vec<WidgetId>,
}

impl<W> CacheEntry<W> {
    fn new(widget: W, parent: WidgetId) -> Self {
        CacheEntry {
            widget,
            parent,
            children: Vec::new(),
        }
    }
}

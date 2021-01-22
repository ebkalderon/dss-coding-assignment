//! Generic code for implementing UI widgets.

use std::cell::{Ref, RefCell, RefMut};
use std::fmt::{self, Debug, Formatter};

use anyhow::Error;
use fnv::FnvHashMap as HashMap;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowContext};

/// A trait which describes a rectangular UI widget.
pub trait Widget {
    /// Returns an immutable reference the properties of the widget.
    fn properties(&self) -> &Properties;

    /// Returns a mutable reference to the properties of the widget.
    fn properties_mut(&mut self) -> &mut Properties;

    /// Renders the widget into the given [`Texture`](sdl2::render::Texture).
    fn draw(&self, canvas: &mut Canvas<Window>, texture: &mut Texture) -> anyhow::Result<()>;
}

/// Contains properties common to all widgets.
#[derive(Debug)]
pub struct Properties {
    /// The top-left (X, Y) coordinate pair denoting the widget's location.
    pub origin: (i32, i32),
    /// Width and height of the widget, in pixels.
    pub bounds: (u32, u32),
    /// Base color of the widget.
    pub color: Color,
}

/// A shared cache of drawable UI widgets.
pub struct Widgets<'tc, W> {
    cache: HashMap<WidgetId, CacheEntry<'tc, W>>,
    next_id: u32,
    textures: &'tc TextureCreator<WindowContext>,
}

impl<'tc, W: Widget> Widgets<'tc, W> {
    /// Creates a new [`Widgets`] cache anchored relative to the given `root_widget`.
    pub fn new(root_widget: W, textures: &'tc TextureCreator<WindowContext>) -> Self {
        let mut cache = HashMap::default();
        cache.insert(WidgetId(0), CacheEntry::new(root_widget, WidgetId(0)));
        Widgets {
            cache,
            next_id: 1,
            textures,
        }
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
    ///
    /// # Panics
    ///
    /// Panics if the same widget is already borrowed mutably.
    pub fn get(&self, id: WidgetId) -> Ref<W> {
        self.cache[&id].widget.borrow()
    }

    /// Returns a mutable reference to a widget in the cache.
    ///
    /// # Panics
    ///
    /// Panics if the same widget is already borrowed immutably.
    pub fn get_mut(&self, id: WidgetId) -> RefMut<W> {
        self.cache[&id].widget.borrow_mut()
    }

    /// Returns an immutable slice containing the children of the widget named `id`.
    pub fn get_children_of(&self, id: WidgetId) -> &[WidgetId] {
        &self.cache[&id].children[..]
    }

    /// Applies a delta X/Y translation to a widget and all of its children.
    pub fn translate(&self, id: WidgetId, dx: i32, dy: i32) {
        if dx == 0 && dy == 0 {
            return;
        }

        let mut widget = self.get_mut(id);
        let (x, y) = widget.properties().origin;
        widget.properties_mut().origin = (x + dx, y + dy);

        for child_id in self.get_children_of(id) {
            if *child_id != id {
                self.translate(*child_id, dx, dy);
            }
        }
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

        // Retrieve base widget texture, resizing if bounds have changed.
        let mut widget_texture = self.cache[&id].texture.borrow_mut();
        let texture = widget_texture.create_or_resize(&self.textures, width, height)?;

        // Draw the widget to the texture and copy the texture to the canvas.
        widget.draw(canvas, texture)?;
        let dst = Rect::new(x, y, width, height);
        canvas.copy(&*texture, None, dst).map_err(Error::msg)?;

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

/// A cached widget and its associated metadata.
#[derive(Debug)]
struct CacheEntry<'tc, W> {
    widget: RefCell<W>,
    texture: RefCell<WidgetTexture<'tc>>,
    parent: WidgetId,
    children: Vec<WidgetId>,
}

impl<'tc, W> CacheEntry<'tc, W> {
    /// Creates and returns a new `CacheEntry` with an empty texture.
    fn new(widget: W, parent: WidgetId) -> Self {
        CacheEntry {
            widget: RefCell::new(widget),
            texture: RefCell::new(WidgetTexture::default()),
            parent,
            children: Vec::new(),
        }
    }
}

/// The target texture into which a widget is rendered.
#[derive(Default)]
struct WidgetTexture<'tc> {
    texture: Option<Texture<'tc>>,
    width: u32,
    height: u32,
}

impl<'tc> WidgetTexture<'tc> {
    /// Returns a mutable reference to the texture, ensuring it matches the given dimensions.
    ///
    /// This method does _not_ allocate any memory if the texture already exists and matches the
    /// given dimensions.
    ///
    /// Otherwise, a new texture which matches the given dimensions will be created using the given
    /// [`TextureCreator`](sdl2::render::TextureCreator).
    fn create_or_resize(
        &mut self,
        tc: &'tc TextureCreator<WindowContext>,
        width: u32,
        height: u32,
    ) -> anyhow::Result<&mut Texture<'tc>> {
        if self.texture.is_none() || self.width != width || self.height != height {
            self.width = width;
            self.height = height;
            self.texture = tc
                .create_texture_target(None, width, height)
                .map(Some)
                .map_err(Error::msg)?;
        }

        match self.texture.as_mut() {
            Some(inner) => Ok(inner),
            None => unreachable!(),
        }
    }
}

impl<'tc> Debug for WidgetTexture<'tc> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct(stringify!(WidgetTexture))
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

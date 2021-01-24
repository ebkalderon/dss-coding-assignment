//! Generic code for implementing UI widgets.

use std::cell::{Ref, RefCell, RefMut};
use std::fmt::{self, Debug, Formatter};
use std::path::{Path, PathBuf};

use anyhow::Error;
use fnv::FnvHashMap as HashMap;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::ttf::{FontStyle, Sdl2TtfContext};
use sdl2::video::{Window, WindowContext};

/// A trait which describes a rectangular UI widget.
pub trait Widget {
    /// Returns an immutable reference the properties of the widget.
    fn properties(&self) -> &Properties;

    /// Returns a mutable reference to the properties of the widget.
    fn properties_mut(&mut self) -> &mut Properties;

    /// Renders the widget into the given [`Texture`](sdl2::render::Texture).
    fn draw(&mut self, ctx: &mut Context, target: &mut Texture) -> anyhow::Result<()>;
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

/// A shared context passed to every [`Widget::draw()`] call.
pub struct Context<'a, 'tc> {
    /// Handle to the window canvas.
    pub canvas: &'a mut Canvas<Window>,
    /// Shared texture cache.
    pub textures: &'a mut Textures<'tc>,
}

impl<'a, 'tc> Debug for Context<'a, 'tc> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct(stringify!(Context))
            .field("textures", &self.textures)
            .finish()
    }
}

/// A shared cache of drawable UI widgets.
pub struct Widgets<'tc, W> {
    cache: HashMap<WidgetId, CacheEntry<'tc, W>>,
    next_id: u32,
    textures: Textures<'tc>,
}

impl<'tc, W: Widget> Widgets<'tc, W> {
    /// Creates a new [`Widgets`] cache anchored relative to the given `root_widget`.
    pub(crate) fn new(root_widget: W, textures: Textures<'tc>) -> Self {
        let mut cache = HashMap::default();
        cache.insert(WidgetId(0), CacheEntry::new(root_widget, WidgetId(0)));
        Widgets {
            cache,
            next_id: 1,
            textures,
        }
    }

    /// Returns the unique ID of the root widget.
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
    pub fn draw(&mut self, canvas: &mut Canvas<Window>) -> anyhow::Result<()> {
        canvas.set_draw_color(Color::RGBA(255, 255, 255, 255));
        canvas.clear();

        self.draw_widget(self.root(), canvas)?;
        canvas.present();

        Ok(())
    }

    fn draw_widget(&mut self, id: WidgetId, canvas: &mut Canvas<Window>) -> anyhow::Result<()> {
        let (widget, texture) = self
            .cache
            .get_mut(&id)
            .map(|e| (e.widget.get_mut(), &mut e.texture))
            .unwrap();

        let (x, y) = widget.properties().origin;
        let (width, height) = widget.properties().bounds;

        // Retrieve base widget texture, resizing if bounds have changed.
        let textures = &mut self.textures;
        let target = texture.create_or_resize(textures.creator, width, height)?;

        // Draw the widget to the target texture and copy it to the canvas.
        widget.draw(&mut Context { canvas, textures }, target)?;
        let dst = Rect::new(x, y, width, height);
        canvas.copy(target, None, dst).map_err(Error::msg)?;

        for child_id in self.get_children_of(id).to_vec() {
            if child_id != id {
                self.draw_widget(child_id, canvas)?;
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
    texture: WidgetTexture<'tc>,
    parent: WidgetId,
    children: Vec<WidgetId>,
}

impl<'tc, W> CacheEntry<'tc, W> {
    fn new(widget: W, parent: WidgetId) -> Self {
        CacheEntry {
            widget: RefCell::new(widget),
            texture: WidgetTexture::default(),
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

/// A shared mechanism for caching textures.
///
/// This struct is accessible from the shared [`Context`] passed to every [`Widget::draw()`] call.
pub struct Textures<'tc> {
    creator: &'tc TextureCreator<WindowContext>,
    cache: HashMap<PathBuf, Texture<'tc>>,
    ttf_ctx: Sdl2TtfContext,
}

impl<'tc> Textures<'tc> {
    pub(crate) fn new(creator: &'tc TextureCreator<WindowContext>) -> anyhow::Result<Self> {
        Ok(Textures {
            creator,
            cache: HashMap::default(),
            ttf_ctx: sdl2::ttf::init()?,
        })
    }

    /// Returns a [`Texture`](sdl2::render::Texture) from an image file, caching it in memory.
    ///
    /// Returns `Err` if the image file could not be found at the destination `path`, or if SDL was
    /// unable to load the file successfully.
    pub fn load_image<P: Into<PathBuf>>(&mut self, path: P) -> anyhow::Result<&Texture<'tc>> {
        use sdl2::image::LoadTexture;
        use std::collections::hash_map::Entry;

        match self.cache.entry(path.into()) {
            Entry::Occupied(e) => Ok(e.into_mut()),
            Entry::Vacant(e) => {
                let texture = self.creator.load_texture(e.key()).map_err(Error::msg)?;
                Ok(e.insert(texture))
            }
        }
    }

    /// Renders some text using a TTF font loaded from `path`, caching the font file in memory.
    ///
    /// Returns `Err` if the font file could not be found at the destination `path`, or if SDL was
    /// unable to load the font successfully.
    pub fn render_text<P>(
        &mut self,
        path: P,
        point_size: u16,
        style: FontStyle,
        props: &Properties,
        message: &str,
    ) -> anyhow::Result<Text<'tc>>
    where
        P: AsRef<Path>,
    {
        use sdl2::render::TextureQuery;

        let mut font = self
            .ttf_ctx
            .load_font(path.as_ref(), point_size)
            .map_err(Error::msg)?;

        font.set_style(style);

        let (max_width, _) = props.bounds;
        let surface = font
            .render(message)
            .blended_wrapped(props.color, max_width)
            .map_err(Error::msg)?;

        // NOTE: This is potentially expensive to do every frame. Ideally, we should have some form
        // of caching system which renders and caches glyphs individually for every font, and then
        // assembles finished textures from these cached glyphs. This is much too complex for this
        // toy project, though.
        let texture = self.creator.create_texture_from_surface(&surface)?;
        let TextureQuery { width, height, .. } = texture.query();

        Ok(Text {
            texture,
            bounds: (width, height),
        })
    }
}

impl<'tc> Debug for Textures<'tc> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct(stringify!(Textures))
            .field("cache", &self.cache.keys())
            .finish()
    }
}

/// Contains some rendered text.
///
/// This struct is created by [`Textures::render_text()`]. See its documentation for more.
pub struct Text<'tc> {
    /// Texture containing the rendered text.
    pub texture: Texture<'tc>,
    /// Width and height of the texture, in pixels.
    pub bounds: (u32, u32),
}

impl<'tc> Debug for Text<'tc> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct(stringify!(Text))
            .field("bounds", &self.bounds)
            .finish()
    }
}

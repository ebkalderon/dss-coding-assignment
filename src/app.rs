//! Generic abstraction for UI applications.

pub use self::widget::{Context, Properties, Text, Textures, Widget, WidgetId, Widgets};

use std::time::{Duration, Instant};

use anyhow::Error;
use sdl2::event::Event;
use sdl2::messagebox::{show_simple_message_box, MessageBoxFlag};
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::Sdl;

const TARGET_FRAME_RATE: u16 = 60;
const MESSAGE_BOX_KIND: MessageBoxFlag = MessageBoxFlag::ERROR;

mod widget;

/// An action to take upon receiving an SDL event.
#[derive(Clone, Copy, Debug)]
pub enum Action {
    /// Continue to run the application.
    Continue,
    /// Adjust the window fullscreen state.
    Fullscreen(Fullscreen),
    /// Shut down the application.
    Quit,
}

/// A list of window fullscreen state transitions.
#[derive(Clone, Copy, Debug)]
pub enum Fullscreen {
    /// Switch to fullscreen mode.
    On,
    /// Restore to windowed mode.
    Off,
    /// Toggle between fullscreen and windowed mode.
    Toggle,
}

/// A trait implemented by the main application state.
pub trait State<W: Widget> {
    /// This method is called at initialization time, before any drawing has taken place, and is
    /// responsible for building the initial UI widget layout of the application.
    ///
    /// This trait method is _required_ and is guaranteed to only be called once.
    fn initialize(&mut self, widgets: &mut Widgets<W>) -> anyhow::Result<()>;

    /// This callback is called every time an [SDL event](sdl2::event::Event) is produced from the
    /// window event loop.
    ///
    /// Returns an [`Action`] specifying whether the application should continue to run or quit.
    ///
    /// This trait method is _provided_. If it is not implemented, this method will do nothing and
    /// always return [`Action::Continue`].
    fn handle_event(&mut self, _event: &Event, _widgets: &mut Widgets<W>) -> Action {
        Action::Continue
    }
}

/// Engine which drives the application state and event loop.
#[derive(Debug)]
pub struct App<W, S> {
    state: S,
    root_widget: W,
    error_message_box: Option<&'static str>,
}

impl<W: Widget, S: State<W>> App<W, S> {
    /// Creates a new `App` with the given application [`State`] and root widget.
    #[inline]
    pub fn new(state: S, root_widget: W) -> Self {
        App {
            state,
            root_widget,
            error_message_box: None,
        }
    }

    /// Displays errors in a graphical error message box whenever possible.
    ///
    /// Certain classes of errors, e.g. fatal SDL initialization errors and message box display
    /// errors, naturally cannot be displayed in a message box. The resulting error chain can still
    /// be inspected in its entirety via the return value of [`App::run()`].
    ///
    /// This setting is not enabled by default.
    #[inline]
    pub fn with_error_message_box(mut self, window_title: &'static str) -> Self {
        self.error_message_box = Some(window_title);
        self
    }

    /// Executes the main loop with the given [`Sdl`](sdl2::Sdl) context and
    /// [`Window`](sdl2::video::Window) handle.
    ///
    /// Returns `Ok` when the application has exited successfully, or returns `Err` if the
    /// application failed to initialize or an SDL error was encountered.
    #[inline]
    pub fn run(self, sdl: Sdl, window: Window) -> anyhow::Result<()> {
        let mut canvas = window.into_canvas().accelerated().present_vsync().build()?;

        let error_message_box = self.error_message_box;
        let result = self.main_loop(sdl, &mut canvas);

        if let Some((window_title, error)) = error_message_box.zip(result.as_ref().err()) {
            let message = format!("{:?}", error);
            show_simple_message_box(MESSAGE_BOX_KIND, window_title, &message, canvas.window())?;
        }

        result
    }

    fn main_loop(mut self, sdl: Sdl, canvas: &mut Canvas<Window>) -> anyhow::Result<()> {
        let mut events = sdl.event_pump().map_err(Error::msg)?;

        let texture_creator = canvas.texture_creator();
        let textures = Textures::new(&texture_creator)?;
        let mut widgets = Widgets::new(self.root_widget, textures);

        // Build and populate the `Widgets` cache.
        self.state.initialize(&mut widgets)?;

        'running: loop {
            let start = Instant::now();

            // Handle all pending SDL events.
            for event in events.poll_iter() {
                match self.state.handle_event(&event, &mut widgets) {
                    Action::Continue => {}
                    Action::Fullscreen(f) => fullscreen(f, canvas.window_mut(), &mut widgets)?,
                    Action::Quit => break 'running,
                }
            }

            // Advance the internal state of the widgets.
            widgets.update();

            // Draw the next frame onto the canvas.
            if widgets.is_invalidated() {
                widgets.draw(canvas)?;
            }

            let frame_time = start.elapsed();
            let target_frame_time = Duration::from_secs_f64(1.0 / TARGET_FRAME_RATE as f64);

            // Put the CPU to sleep to save power, if necessary.
            if frame_time < target_frame_time {
                std::thread::sleep(target_frame_time - frame_time);
            }
        }

        Ok(())
    }
}

/// Sets the `window` fullscreen state and scales the root widget bounds to match the new size.
///
/// When fullscreen mode is enabled or toggled on, this function always prefers native fullscreen
/// over borderless desktop fullscreen, unless the SDL window was explicitly initialized with
/// desktop fullscreen.
fn fullscreen<W>(f: Fullscreen, window: &mut Window, widgets: &mut Widgets<W>) -> anyhow::Result<()>
where
    W: Widget,
{
    use sdl2::video::FullscreenType;

    let current = window.fullscreen_state();
    let new_state = match f {
        Fullscreen::On if current == FullscreenType::Desktop => FullscreenType::Desktop,
        Fullscreen::On => FullscreenType::True,
        Fullscreen::Off => FullscreenType::Off,
        Fullscreen::Toggle => match current {
            FullscreenType::True => FullscreenType::Off,
            FullscreenType::Desktop => FullscreenType::Off,
            FullscreenType::Off if current == FullscreenType::Desktop => FullscreenType::Desktop,
            FullscreenType::Off => FullscreenType::True,
        },
    };

    window.set_fullscreen(new_state).map_err(Error::msg)?;
    let (width, height) = window.size();
    widgets.get_mut(widgets.root()).set_bounds(width, height);

    Ok(())
}

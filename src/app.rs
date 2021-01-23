//! Generic abstraction for UI applications.

pub use self::widget::{Context, Properties, Text, Textures, Widget, WidgetId, Widgets};

use std::time::{Duration, Instant};

use anyhow::Error;
use sdl2::event::Event;
use sdl2::video::Window;
use sdl2::Sdl;

const TARGET_FRAME_RATE: u16 = 60;

mod widget;

/// An action to take upon receiving an SDL event.
#[derive(Clone, Copy, Debug)]
pub enum Action {
    /// Continue to run the application.
    Continue,
    /// Shut down the application.
    Quit,
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

/// Entry point for the UI application.
#[derive(Debug)]
pub struct App<W, S> {
    state: S,
    root_widget: W,
}

impl<W: Widget, S: State<W>> App<W, S> {
    /// Creates a new `App` with the given application [`State`] and root widget.
    pub fn new(state: S, root_widget: W) -> Self {
        App { state, root_widget }
    }

    /// Executes the main loop with the given [`Sdl`](sdl2::Sdl) context and
    /// [`Window`](sdl2::video::Window) handle.
    ///
    /// Returns `Ok` when the application has exited successfully, or returns `Err` if the
    /// application failed to initialize or an SDL error was encountered.
    pub fn run(mut self, sdl: Sdl, window: Window) -> anyhow::Result<()> {
        let mut events = sdl.event_pump().map_err(Error::msg)?;
        let mut canvas = window.into_canvas().accelerated().present_vsync().build()?;

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
                    Action::Quit => break 'running,
                }
            }

            // Draw the next frame to the canvas.
            widgets.draw(&mut canvas)?;

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

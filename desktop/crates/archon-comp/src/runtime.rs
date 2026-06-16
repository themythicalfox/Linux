//! The Smithay-backed compositor runtime (only with `smithay-backend`).
//!
//! This is the glue between Wayland/DRM and the tested [`CompositorCore`]: it
//! owns the display, the renderer and the seat, implements the protocol
//! handlers, and feeds client/input events into the core. [`run_winit`] runs the
//! compositor nested inside an existing Wayland/X session — the development and
//! testing path. [`run_udev`] is the bare-metal DRM path used by the ISO
//! session and is the runtime's next build-out (it currently directs the user to
//! the winit backend so the failure is explicit rather than a black screen).
//!
//! The window-management behaviour (tiling, snapping, workspaces, keybindings,
//! Game Mode) all lives in [`CompositorCore`] and is unit tested; this module
//! deliberately stays thin.

use std::sync::Arc;

use smithay::{
    backend::{
        input::{InputEvent, KeyboardKeyEvent},
        renderer::{
            element::{
                surface::{render_elements_from_surface_tree, WaylandSurfaceRenderElement},
                Kind,
            },
            gles::GlesRenderer,
            utils::{draw_render_elements, on_commit_buffer_handler},
            Color32F, Frame, Renderer,
        },
        winit::{self, WinitEvent},
    },
    delegate_compositor, delegate_data_device, delegate_seat, delegate_shm, delegate_xdg_shell,
    input::{keyboard::FilterResult, Seat, SeatHandler, SeatState},
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::{wl_buffer, wl_seat, wl_surface::WlSurface},
            Client, Display, ListeningSocket,
        },
        winit::platform::pump_events::PumpStatus,
    },
    utils::{Rectangle, Serial, Transform},
    wayland::{
        buffer::BufferHandler,
        compositor::{
            with_surface_tree_downward, CompositorClientState, CompositorHandler, CompositorState,
            SurfaceAttributes, TraversalAction,
        },
        selection::{
            data_device::{
                ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler,
            },
            SelectionHandler,
        },
        shell::xdg::{PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState},
        shm::{ShmHandler, ShmState},
    },
};

use crate::CompositorCore;
use archon_config::Config;

/// The compositor's full state: Wayland globals + our window-management core.
pub struct ArchonState {
    compositor_state: CompositorState,
    xdg_shell_state: XdgShellState,
    shm_state: ShmState,
    seat_state: SeatState<Self>,
    data_device_state: DataDeviceState,
    seat: Seat<Self>,

    /// The tested window-management brain. The runtime keeps it in sync with the
    /// set of mapped toplevels and forwards input to it.
    core: CompositorCore,
}

impl BufferHandler for ArchonState {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl XdgShellHandler for ArchonState {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        // Activate and configure the new window, then register it with the core
        // so it joins the active workspace and participates in tiling/focus.
        surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Activated);
        });
        surface.send_configure();
        let area = self.core.work_area;
        let geo = crate::window::Geometry::new(area.x + 64, area.y + 64, area.w / 2, area.h / 2);
        self.core.add_window("xdg", geo);
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {}
    fn grab(&mut self, _surface: PopupSurface, _seat: wl_seat::WlSeat, _serial: Serial) {}
    fn reposition_request(&mut self, _surface: PopupSurface, _positioner: PositionerState, _token: u32) {}
}

impl SelectionHandler for ArchonState {
    type SelectionUserData = ();
}

impl DataDeviceHandler for ArchonState {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}

impl ClientDndGrabHandler for ArchonState {}
impl ServerDndGrabHandler for ArchonState {
    fn send(&mut self, _mime_type: String, _fd: std::os::unix::io::OwnedFd, _seat: Seat<Self>) {}
}

impl CompositorHandler for ArchonState {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);
    }
}

impl ShmHandler for ArchonState {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

impl SeatHandler for ArchonState {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn focus_changed(&mut self, _seat: &Seat<Self>, _focused: Option<&WlSurface>) {}
    fn cursor_image(&mut self, _seat: &Seat<Self>, _image: smithay::input::pointer::CursorImageStatus) {}
}

delegate_xdg_shell!(ArchonState);
delegate_compositor!(ArchonState);
delegate_shm!(ArchonState);
delegate_seat!(ArchonState);
delegate_data_device!(ArchonState);

#[derive(Default)]
struct ClientState {
    compositor_state: CompositorClientState,
}
impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}

/// Run the compositor nested inside an existing Wayland/X session using the
/// winit backend. This is the development/testing path.
pub fn run_winit(config: Config) -> anyhow::Result<()> {
    let mut display: Display<ArchonState> = Display::new()?;
    let dh = display.handle();

    let compositor_state = CompositorState::new::<ArchonState>(&dh);
    let shm_state = ShmState::new::<ArchonState>(&dh, vec![]);
    let mut seat_state = SeatState::new();
    let seat = seat_state.new_wl_seat(&dh, "archon-winit");

    let mut state = ArchonState {
        compositor_state,
        xdg_shell_state: XdgShellState::new::<ArchonState>(&dh),
        shm_state,
        seat_state,
        data_device_state: DataDeviceState::new::<ArchonState>(&dh),
        seat,
        core: CompositorCore::new(&config),
    };

    // Pick an unused wayland socket name and advertise it so clients connect.
    let listener = ListeningSocket::bind_auto("wayland", 1..32)?;
    if let Some(name) = listener.socket_name() {
        std::env::set_var("WAYLAND_DISPLAY", name);
        tracing::info!("ArchonSync listening on {:?}", name);
    }
    let mut clients = Vec::new();

    let (mut backend, mut winit) = winit::init::<GlesRenderer>()
        .map_err(|e| anyhow::anyhow!("failed to init winit backend: {e}"))?;

    let start_time = std::time::Instant::now();
    let keyboard = state
        .seat
        .add_keyboard(Default::default(), 200, 200)
        .map_err(|e| anyhow::anyhow!("failed to add keyboard: {e}"))?;

    // Background clear color from the theme's deepest surface.
    let bg = state.core.theme.bg_deep;
    let clear = Color32F::new(
        bg.r as f32 / 255.0,
        bg.g as f32 / 255.0,
        bg.b as f32 / 255.0,
        1.0,
    );

    loop {
        let status = winit.dispatch_new_events(|event| match event {
            WinitEvent::Resized { size, .. } => {
                state.core.work_area =
                    crate::window::Geometry::new(0, 0, size.w, size.h);
                if state.core.tiling {
                    state.core.relayout();
                }
            }
            WinitEvent::Input(event) => {
                if let InputEvent::Keyboard { event } = event {
                    keyboard.input::<(), _>(
                        &mut state,
                        event.key_code(),
                        event.state(),
                        0.into(),
                        0,
                        |_, _, _| FilterResult::Forward,
                    );
                } else if let InputEvent::PointerMotionAbsolute { .. } = event {
                    if let Some(surface) =
                        state.xdg_shell_state.toplevel_surfaces().iter().next().cloned()
                    {
                        let surface = surface.wl_surface().clone();
                        keyboard.set_focus(&mut state, Some(surface), 0.into());
                    }
                }
            }
            _ => (),
        });

        if let PumpStatus::Exit(_) = status {
            return Ok(());
        }

        let size = backend.window_size();
        let damage = Rectangle::from_size(size);
        {
            let (renderer, mut framebuffer) = backend
                .bind()
                .map_err(|e| anyhow::anyhow!("failed to bind backend: {e}"))?;
            let elements = state
                .xdg_shell_state
                .toplevel_surfaces()
                .iter()
                .flat_map(|surface| {
                    render_elements_from_surface_tree(
                        renderer,
                        surface.wl_surface(),
                        (0, 0),
                        1.0,
                        1.0,
                        Kind::Unspecified,
                    )
                })
                .collect::<Vec<WaylandSurfaceRenderElement<GlesRenderer>>>();

            let mut frame = renderer
                .render(&mut framebuffer, size, Transform::Flipped180)
                .map_err(|e| anyhow::anyhow!("render failed: {e}"))?;
            frame
                .clear(clear, &[damage])
                .map_err(|e| anyhow::anyhow!("clear failed: {e}"))?;
            draw_render_elements(&mut frame, 1.0, &elements, &[damage])
                .map_err(|e| anyhow::anyhow!("draw failed: {e}"))?;
            let _ = frame.finish().map_err(|e| anyhow::anyhow!("finish failed: {e}"))?;

            for surface in state.xdg_shell_state.toplevel_surfaces() {
                send_frames_surface_tree(surface.wl_surface(), start_time.elapsed().as_millis() as u32);
            }

            if let Some(stream) = listener.accept()? {
                let client = display
                    .handle()
                    .insert_client(stream, Arc::new(ClientState::default()))
                    .unwrap();
                clients.push(client);
            }

            display.dispatch_clients(&mut state)?;
            display.flush_clients()?;
        }

        backend
            .submit(Some(&[damage]))
            .map_err(|e| anyhow::anyhow!("submit failed: {e}"))?;
    }
}

/// Bare-metal DRM/udev runtime used by the ISO session.
///
/// The full DRM + libseat + libinput path is the runtime's next build-out. Until
/// then this returns a clear error (rather than a black screen) pointing at the
/// working winit backend, so a misconfigured session fails loudly.
pub fn run_udev(_config: Config) -> anyhow::Result<()> {
    anyhow::bail!(
        "the udev/DRM backend is not built yet; run `archon-comp --backend winit` \
         nested in an existing Wayland/X session to try the compositor"
    )
}

/// Release frame callbacks for a surface tree so clients keep drawing.
fn send_frames_surface_tree(surface: &WlSurface, time: u32) {
    with_surface_tree_downward(
        surface,
        (),
        |_, _, &()| TraversalAction::DoChildren(()),
        |_surf, states, &()| {
            for callback in states
                .cached_state
                .get::<SurfaceAttributes>()
                .current()
                .frame_callbacks
                .drain(..)
            {
                callback.done(time);
            }
        },
        |_, _, &()| true,
    );
}

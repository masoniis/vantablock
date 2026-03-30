use crate::{
    ecs_core::{
        async_loading::LoadingTracker,
        config,
        cross_world_communication::{SimToRenderReceiver, SimToRenderSender},
        frame_sync::FrameSync,
    },
    prelude::*,
    render_world::{
        global_extract::utils::run_extract_schedule, graphics_context::GraphicsContext,
        scheduling::RenderSchedule, textures::load_voxel_texture_assets, RenderWorldInterface,
    },
    simulation_world::{
        input::{
            messages::{RawDeviceMessage, RawWindowMessage},
            resources::DesiredCursorState,
        },
        SimulationSchedule, SimulationWorldInterface,
    },
};
use crossbeam::channel::unbounded;
use futures_lite::future::block_on;
use std::{
    error::Error,
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

/// The main application container, responsible for orchestrating OS
/// events as well as the creation and scheduling of the ECS worlds.
pub struct App {
    /// The window the surface attaches to for rendering.
    ///
    /// The window is an Arc so that the surface can hold a reference with a static
    /// lifetime ensuring the window outlasts the surface the GPU draws to.
    window: Option<Arc<Window>>,

    /// The ECS world responsible for simulation logic
    simulation_world: Option<Arc<Mutex<SimulationWorldInterface>>>,
    /// The ECS world responsible for rendering logic
    render_world: Option<Arc<Mutex<RenderWorldInterface>>>,
    /// A loading tracker necessary to orchestrate async tasks between both worlds.
    loading_tracker: LoadingTracker,

    // World parallelization
    frame_sync: FrameSync,
    render_thread: Option<JoinHandle<()>>,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            simulation_world: None,
            render_world: None,
            loading_tracker: LoadingTracker::default(),
            frame_sync: FrameSync::new(),
            render_thread: None,
        }
    }

    /// Simple utility method to spin up an event loop and run a default app
    pub fn create_and_run() -> Result<(), Box<dyn Error>> {
        let event_loop = EventLoop::new()?;

        let mut app = App::new();

        event_loop.run_app(&mut app)?;
        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            info!("App started/resumed, creating window and renderer...");

            // INFO: --------------------------
            //         setup the window
            // --------------------------------

            let window = Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_title("🅱️oxel")
                            .with_inner_size(LogicalSize::new(1280, 720)),
                    )
                    .unwrap(),
            );

            // INFO: ----------------------------------------
            //         create and initiate the worlds
            // ----------------------------------------------

            // load config
            let app_config = config::load_config();

            // world dependencies that the app must create (due to window)
            let graphics_context = block_on(GraphicsContext::new(window.clone()));
            let (texture_images, texture_registry) =
                load_voxel_texture_assets(&app_config).unwrap();

            let mut simulation_world = SimulationWorldInterface::new(&window, texture_registry);
            let mut render_world = RenderWorldInterface::new(graphics_context, texture_images);

            // add config
            simulation_world.add_resource(app_config.clone());
            render_world.add_resource(app_config);

            // add loading trackers
            simulation_world.add_resource(self.loading_tracker.clone());
            render_world.add_resource(self.loading_tracker.clone());

            // add cross communication channel
            let (sender, receiver) = unbounded();
            let sim_sender_resource = SimToRenderSender(sender);
            let render_receiver_resource = SimToRenderReceiver(receiver);
            simulation_world.add_resource(sim_sender_resource);
            render_world.add_resource(render_receiver_resource);

            info!("Running startup systems...\n\n\n");
            simulation_world.run_schedule(SimulationSchedule::Startup);
            render_world.run_schedule(RenderSchedule::Startup);

            // INFO: ----------------------------------------------
            //         spawn and setup the rendering thread
            // ----------------------------------------------------

            let render_world = Arc::new(Mutex::new(render_world));
            let simulation_world = Arc::new(Mutex::new(simulation_world));

            let render_sync = self.frame_sync.clone();
            let render_world_for_render = render_world.clone();
            let sim_world_for_render = simulation_world.clone();

            let render_thread = thread::Builder::new()
                .name("Render thread".to_string())
                .spawn(move || {
                    #[cfg(feature = "tracy")]
                    tracy_client::set_thread_name!("Render thread");

                    loop {
                        // wait until we are signaled to extract form the sim world
                        render_sync.wait_for_extraction();
                        {
                            let mut sim_guard = sim_world_for_render.lock().unwrap();
                            let mut render_guard = render_world_for_render.lock().unwrap();

                            let _extract_phase_span = info_span!("Extract").entered();

                            // extract schedule needs mutable access to the simulation world
                            run_extract_schedule(
                                &mut sim_guard.borrow(),
                                &mut render_guard.borrow(),
                                RenderSchedule::Extract,
                            );

                            sim_guard.clear_trackers();
                        }
                        render_sync.finish_extraction();

                        // perform rendering now that sim is active again
                        let mut render_world = render_world_for_render.lock().unwrap();
                        {
                            let _render_phase_span = info_span!("Render").entered();
                            render_world.run_schedule(RenderSchedule::Main);
                            render_world.clear_trackers();
                        }
                    }
                })
                .expect("Failed to spawn RenderThread");

            // INFO: ------------------------------
            //         update the app state
            // ------------------------------------

            self.window = Some(window.clone());
            self.simulation_world = Some(simulation_world);
            self.render_world = Some(render_world);
            self.render_thread = Some(render_thread);
        }
    }

    fn device_event(
        &mut self,
        _el: &ActiveEventLoop,
        _id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let Some(simulation_world) = &mut self.simulation_world {
            simulation_world
                .lock()
                .unwrap()
                .send_event(RawDeviceMessage(event.clone()));
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let Some(simulation_world) = &mut self.simulation_world {
            simulation_world
                .lock()
                .unwrap()
                .send_event(RawWindowMessage(event.clone()));

            // NOTE: The events handled here should only be events that rely on the event loop
            // or window. Any other event should be fine to handle within the ECS world itself.
            match event {
                WindowEvent::CloseRequested => {
                    info!("Window close requested, exiting app event loop.");
                    event_loop.exit();
                }
                WindowEvent::RedrawRequested => {
                    if let Some(simulation_world) = self.simulation_world.as_mut() {
                        // wait for the sim signal and run
                        self.frame_sync.wait_for_simulation();
                        {
                            let _main_loop_span = tracing::info_span!("Sim").entered();
                            simulation_world
                                .lock()
                                .unwrap()
                                .run_schedule(SimulationSchedule::Main);

                            // updated cursor if there is a change
                            if let (Some(window), Some(cursor_state)) = (
                                &self.window,
                                simulation_world
                                    .lock()
                                    .unwrap()
                                    .get_resource::<DesiredCursorState>(),
                            ) {
                                window.set_cursor_visible(cursor_state.visible);
                                if let Err(e) = window.set_cursor_grab(cursor_state.grab_mode) {
                                    error!("Failed to set cursor grab mode: {:?}", e);
                                }
                            }
                        }
                        self.frame_sync.finish_simulation();

                        // request the next frame
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    } else {
                        warn!(
                            "Redraw requested but simulation or render world is not initialized."
                        );
                    }
                }
                _ => {}
            }
        }
    }
}

use crate::{
    ecs_core::{
        async_loading::LoadingTracker,
        config,
        cross_world_communication::{SimToRenderReceiver, SimToRenderSender},
    },
    prelude::*,
    render_world::{
        global_extract::utils::run_extract_schedule, graphics_context::GraphicsContext,
        scheduling::RenderSchedule, setup_render_sub_app, textures::load_voxel_texture_assets,
    },
    simulation_world::{
        input::{
            messages::{RawDeviceMessage, RawWindowMessage},
            resources::DesiredCursorState,
        },
        setup_simulation_app,
    },
};
use bevy::app::{App, Startup, SubApp};
use bevy::ecs::schedule::ScheduleLabel;
use crossbeam::channel::unbounded;
use futures_lite::future::block_on;
use std::{error::Error, sync::Arc};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

/// The main application container, responsible for orchestrating OS
/// events as well as the creation and scheduling of the ECS worlds.
pub struct BoxelApp {
    /// The window the surface attaches to for rendering.
    ///
    /// The window is an Arc so that the surface can hold a reference with a static
    /// lifetime ensuring the window outlasts the surface the GPU draws to.
    window: Option<Arc<Window>>,

    /// The ECS app responsible for simulation and main loop
    simulation_app: Option<App>,
    /// A loading tracker necessary to orchestrate async tasks between both worlds.
    loading_tracker: LoadingTracker,
}

impl BoxelApp {
    fn new() -> Self {
        Self {
            window: None,
            simulation_app: None,
            loading_tracker: LoadingTracker::default(),
        }
    }

    /// Simple utility method to spin up an event loop and run a default app
    pub fn create_and_run() -> Result<(), Box<dyn Error>> {
        let event_loop = EventLoop::new()?;

        let mut app = BoxelApp::new();

        event_loop.run_app(&mut app)?;
        Ok(())
    }
}

impl ApplicationHandler for BoxelApp {
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

            let mut simulation_app = setup_simulation_app(&window, texture_registry);

            // INFO: ----------------------------------------------
            //         setup the rendering sub-app
            // ----------------------------------------------------

            let mut sub_app = SubApp::new();
            setup_render_sub_app(&mut sub_app, graphics_context, texture_images);

            // add config
            simulation_app.insert_resource(app_config.clone());
            sub_app.insert_resource(app_config);

            // add loading trackers
            simulation_app.insert_resource(self.loading_tracker.clone());
            sub_app.insert_resource(self.loading_tracker.clone());

            // add cross communication channel
            let (sender, receiver) = unbounded();
            let sim_sender_resource = SimToRenderSender(sender);
            let render_receiver_resource = SimToRenderReceiver(receiver);
            simulation_app.insert_resource(sim_sender_resource);
            sub_app.insert_resource(render_receiver_resource);

            info!("Running startup systems...\n\n\n");
            simulation_app.world_mut().run_schedule(Startup);
            sub_app.world_mut().run_schedule(RenderSchedule::Startup);

            // Attach the SubApp using the bevy::render::RenderApp label
            sub_app.set_extract(move |main_world, render_world| {
                run_extract_schedule(main_world, render_world, RenderSchedule::Extract);
            });

            // Set the schedule that should run on every sub-app update
            sub_app.update_schedule = Some(RenderSchedule::Main.intern());

            simulation_app.insert_sub_app(bevy::render::RenderApp, sub_app);

            // INFO: ------------------------------
            //         update the app state
            // ------------------------------------

            self.window = Some(window.clone());
            self.simulation_app = Some(simulation_app);
        }
    }

    fn device_event(
        &mut self,
        _el: &ActiveEventLoop,
        _id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let Some(simulation_app) = &mut self.simulation_app {
            simulation_app
                .world_mut()
                .write_message(RawDeviceMessage(event.clone()));
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let Some(simulation_app) = &mut self.simulation_app {
            simulation_app
                .world_mut()
                .write_message(RawWindowMessage(event.clone()));

            // NOTE: The events handled here should only be events that rely on the event loop
            // or window. Any other event should be fine to handle within the ECS world itself.
            match event {
                WindowEvent::CloseRequested => {
                    info!("Window close requested, exiting app event loop.");
                    event_loop.exit();
                }
                WindowEvent::RedrawRequested => {
                    let _main_loop_span = tracing::info_span!("Sim").entered();

                    // calling update() automatically runs the Main App, runs our extraction
                    // closure, and then updates the SubApp (Render World).
                    simulation_app.update();

                    // updated cursor if there is a change
                    if let (Some(window), Some(cursor_state)) = (
                        &self.window,
                        simulation_app.world().get_resource::<DesiredCursorState>(),
                    ) {
                        window.set_cursor_visible(cursor_state.visible);
                        if let Err(e) = window.set_cursor_grab(cursor_state.grab_mode) {
                            error!("Failed to set cursor grab mode: {:?}", e);
                        }
                    }

                    // request the next frame
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
                _ => {}
            }
        }
    }
}

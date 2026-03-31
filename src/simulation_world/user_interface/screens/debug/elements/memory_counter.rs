use crate::prelude::*;
use crate::simulation_world::user_interface::{
    components::UiText, screens::debug_screen::MemoryCounterTextElementMarker,
};
use bevy::ecs::prelude::*;
use std::process;
use sysinfo::{Pid, ProcessRefreshKind, System};

#[derive(Resource)]
pub struct SystemInfoResource {
    pub sys: sysinfo::System,
    pid: sysinfo::Pid,
}

impl Default for SystemInfoResource {
    fn default() -> Self {
        let sys = System::new();
        let pid = Pid::from_u32(process::id());
        SystemInfoResource { sys, pid }
    }
}

impl SystemInfoResource {
    pub fn refresh_and_get_memory_string(&mut self) -> String {
        self.sys.refresh_memory();
        self.sys.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::Some(&[self.pid]),
            true,
            ProcessRefreshKind::nothing().with_memory(),
        );

        let process_memory_bytes = self.sys.process(self.pid).map_or(0, |p| p.memory());

        format!("{:.2} MB", process_memory_bytes as f32 / 1024.0 / 1024.0)
    }
}

/// Updates the content of the memory counter text element.
#[instrument(skip_all)]
pub fn update_memory_counter_screen_text(
    // Input
    mut system_info: ResMut<SystemInfoResource>,

    // Output (updated component)
    mut query: Query<&mut UiText, With<MemoryCounterTextElementMarker>>,
) {
    if let Ok(mut ui_text) = query.single_mut() {
        ui_text.content = system_info.refresh_and_get_memory_string();
    } else {
        warn!("Failed to get single UiText with CameraXyzTextMarker");
    }
}

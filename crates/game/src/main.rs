use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{MemoryHints, RenderCreation, WgpuSettings};
use bevy::window::WindowResolution;
use game::TankGamePlugin;

fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                        memory_hints: MemoryHints::MemoryUsage,
                        ..default()
                    })),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Carbon Armor — Stage 3 (Team Battle)".into(),
                        resolution: WindowResolution::new(1280, 720),
                        canvas: Some("#bevy-canvas".into()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: true,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(TankGamePlugin)
        .run();
}

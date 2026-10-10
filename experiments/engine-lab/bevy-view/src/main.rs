use bevy::{
    camera::ScalingMode,
    core_pipeline::tonemapping::Tonemapping,
    mesh::PrimitiveTopology,
    prelude::*,
    render::{
        RenderPlugin,
        render_resource::Face,
        settings::{RenderCreation, WgpuFeatures, WgpuLimits, WgpuSettings},
    },
    window::WindowResolution,
};
use genesis_lab_scene as scene;

#[derive(Resource)]
struct CapturePath(Option<String>);

fn main() -> AppExit {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let capture_path = match args.as_slice() {
        [] => None,
        [flag, path] if flag == "--capture" => Some(path.clone()),
        _ => {
            eprintln!("usage: genesis-lab-bevy [--capture output.png]");
            return AppExit::error();
        }
    };
    App::new()
        .insert_resource(CapturePath(capture_path))
        .insert_resource(ClearColor(Color::linear_rgba(
            scene::BACKGROUND[0],
            scene::BACKGROUND[1],
            scene::BACKGROUND[2],
            1.0,
        )))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Genesis engine lab — Bevy".into(),
                        resolution: WindowResolution::new(scene::WIDTH, scene::HEIGHT)
                            .with_scale_factor_override(1.0),
                        ..default()
                    }),
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                        disabled_features: Some(
                            WgpuFeatures::INDIRECT_FIRST_INSTANCE | WgpuFeatures::PUSH_CONSTANTS,
                        ),
                        constrained_limits: Some(WgpuLimits {
                            max_subgroup_size: u32::MAX,
                            ..default()
                        }),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_systems(Startup, setup)
        .add_systems(Update, capture)
        .run()
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let vertices = scene::settlement();
    let mesh = Mesh::new(PrimitiveTopology::TriangleList, default())
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vertices.iter().map(|v| v.position).collect::<Vec<_>>(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; vertices.len()])
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_COLOR,
            vertices.iter().map(|v| v.color).collect::<Vec<_>>(),
        );
    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            unlit: true,
            cull_mode: Some(Face::Back),
            ..default()
        })),
    ));
    commands.spawn((
        Camera3d::default(),
        Tonemapping::None,
        Msaa::Off,
        bevy::render::view::NoIndirectDrawing,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: scene::VIEW_HEIGHT,
            },
            near: 0.1,
            far: 100.0,
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_translation(Vec3::from(scene::EYE))
            .looking_at(Vec3::from(scene::TARGET), Vec3::Y),
    ));
}

fn capture(
    mut commands: Commands,
    mut frames: Local<u32>,
    path: Res<CapturePath>,
    time: Res<Time>,
    mut exit: MessageWriter<AppExit>,
) {
    use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
    let Some(path) = &path.0 else {
        return;
    };
    *frames += 1;
    if *frames == 60 {
        let path = path.clone();
        commands.spawn(Screenshot::primary_window()).observe(
            move |event: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| match event
                .image
                .clone()
                .try_into_dynamic()
                .map_err(|e| e.to_string())
                .and_then(|image| image.save(&path).map_err(|e| e.to_string()))
            {
                Ok(()) => {
                    exit.write(AppExit::Success);
                }
                Err(error) => {
                    eprintln!("capture failed: {error}");
                    exit.write(AppExit::error());
                }
            },
        );
    }
    if time.elapsed_secs() > 30.0 {
        eprintln!("capture timed out");
        exit.write(AppExit::error());
    }
}

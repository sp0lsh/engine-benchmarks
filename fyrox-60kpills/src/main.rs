//! Fyrox 60k pill benchmark — same workload as pill_demo benchmark scene for performance comparison.
//! Build: cargo run --release
//! Run from project root so "assets" is found (or set working directory to fyrox-demo).
//! Fyrox does not load OBJ; we use procedural cylinder for same count/transforms. Ref: https://github.com/FyroxEngine/Fyrox

use fyrox::{
    asset::Resource,
    core::{
        algebra::{Matrix4, UnitQuaternion, Vector2, Vector3},
        color::Color,
        pool::Handle,
        reflect::prelude::*,
        visitor::prelude::*,
    },
    dpi::{PhysicalPosition, PhysicalSize},
    engine::{executor::Executor, GraphicsContext, GraphicsContextParams},
    event_loop::EventLoop,
    graph::{BaseSceneGraph, SceneGraph},
    gui::{
        brush::Brush,
        message::MessageDirection,
        text::{TextBuilder, TextMessage},
        widget::{WidgetBuilder, WidgetMessage},
        UiNode,
    },
    material::Material,
    plugin::{Plugin, PluginContext, PluginRegistrationContext},
    resource::texture::Texture,
    scene::{
        base::BaseBuilder,
        camera::CameraBuilder,
        light::directional::DirectionalLightBuilder,
        mesh::surface::{SurfaceBuilder, SurfaceData, SurfaceResource},
        mesh::MeshBuilder,
        transform::Transform,
        Scene,
    },
    window::WindowAttributes,
};
use rand::{rngs::StdRng, Rng, SeedableRng};
// use std::io::Write;
use std::{path::Path, time::Instant};

const DEFAULT_PILL_COUNT: usize = 60_000;
const DEFAULT_SPAWN_BATCH: usize = 2000;

#[derive(Default, Debug, Clone, Visit, Reflect)]
pub struct PillSpawn {
    position: Vector3<f32>,
    rotation: UnitQuaternion<f32>,
}

#[derive(Debug, Clone, Visit, Reflect)]
pub struct Game {
    scene: Handle<Scene>,
    pill_count: usize,
    pending_pills: Vec<PillSpawn>,
    pending_index: usize,
    spawn_batch: usize,
    pill_handles: Vec<Handle<fyrox::scene::node::Node>>,
    materials: Vec<Resource<Material>>,
    material_count: usize,
    quality_set: bool,
    fps_text_handles: Vec<Handle<UiNode>>,
    last_frame_ms: f64,
    last_fps: f64,
    #[visit(skip)]
    #[reflect(hidden)]
    last_frame_start: Option<Instant>,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            scene: Handle::NONE,
            pending_pills: Vec::new(),
            pending_index: 0,
            spawn_batch: DEFAULT_SPAWN_BATCH,
            pill_handles: Vec::new(),
            materials: Vec::new(),
            material_count: 1,
            pill_count: 0,
            quality_set: false,
            fps_text_handles: Vec::new(),
            last_frame_ms: 0.0,
            last_fps: 0.0,
            last_frame_start: None,
        }
    }
}

impl Plugin for Game {
    fn register(&self, _context: PluginRegistrationContext) {}

    fn init(&mut self, _scene_path: Option<&str>, ctx: PluginContext) {
        // Create benchmark scene programmatically (camera, light, 60k mesh instances).
        let mut scene = Scene::new();
        // Disable physics systems; benchmark doesn't use them.
        scene
            .graph
            .physics
            .enabled
            .set_value_and_mark_modified(false);
        scene
            .graph
            .physics2d
            .enabled
            .set_value_and_mark_modified(false);

        // Camera at z = -16 looking toward origin (match pill_demo benchmark).
        let mut cam_transform = Transform::default();
        cam_transform.set_position(Vector3::new(0.0, 0.0, -16.0));
        let _camera = CameraBuilder::new(BaseBuilder::new().with_local_transform(cam_transform))
            .build(&mut scene.graph);

        // Four directional lights only.
        let light_rotations = [
            (-45.0f32, 45.0f32, 0.0f32),
            (-20.0f32, -60.0f32, 0.0f32),
            (-60.0f32, 15.0f32, 0.0f32),
            (-35.0f32, 120.0f32, 0.0f32),
        ];
        for (x, y, z) in light_rotations {
            let mut light_transform = Transform::default();
            light_transform.set_rotation(UnitQuaternion::from_euler_angles(
                x.to_radians(),
                y.to_radians(),
                z.to_radians(),
            ));
            DirectionalLightBuilder::new(fyrox::scene::light::BaseLightBuilder::new(
                BaseBuilder::new().with_local_transform(light_transform),
            ))
            .build(&mut scene.graph);
        }

        // Materials (PBR): reuse pill textures and vary base color/metallic/roughness.
        let material_count = std::env::var("FYROX_MATERIAL_COUNT")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(10)
            .max(1);
        let color_texture: Resource<Texture> = ctx
            .resource_manager
            .request("assets/textures/pill_color.png");
        let normal_texture: Resource<Texture> = ctx
            .resource_manager
            .request("assets/textures/pill_normal.png");
        let mut materials = Vec::with_capacity(material_count);
        let mut mat_rng = StdRng::seed_from_u64(7);
        for _ in 0..material_count {
            let mut material = Material::standard();
            material.bind("diffuseTexture", color_texture.clone());
            material.bind("normalTexture", normal_texture.clone());
            let tint = (
                (mat_rng.gen_range(0.2..=0.8) * 255.0) as u8,
                (mat_rng.gen_range(0.2..=0.8) * 255.0) as u8,
                (mat_rng.gen_range(0.2..=0.8) * 255.0) as u8,
            );
            material.set_property(
                "diffuseColor",
                fyrox::core::color::Color::from_rgba(tint.0, tint.1, tint.2, 255),
            );
            material.set_property("metallicFactor", mat_rng.gen_range(0.0..=1.0));
            material.set_property("roughnessFactor", mat_rng.gen_range(0.0..=1.0));
            materials.push(Resource::new_embedded(material));
        }

        // 60k pill-like meshes: procedural cylinder (Fyrox does not load OBJ; same count/transforms).
        // Build the spawn list here so init returns quickly and the window can show.
        let pill_count = std::env::var("FYROX_PILL_COUNT")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(DEFAULT_PILL_COUNT);
        self.pill_count = pill_count;
        let spawn_batch = std::env::var("FYROX_SPAWN_BATCH")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(DEFAULT_SPAWN_BATCH);
        let mut rng = StdRng::seed_from_u64(42);
        let mut pending_pills = Vec::with_capacity(pill_count);
        for _ in 0..pill_count {
            let pos_x = rng.gen_range(-10.0..=20.0);
            let pos_y = rng.gen_range(-10.0..=10.0);
            let pos_z = rng.gen_range(-1.0..=1.0);
            let rot_x = rng.gen_range(-180.0f32..=180.0f32).to_radians();
            pending_pills.push(PillSpawn {
                position: Vector3::new(pos_x, pos_y, pos_z),
                rotation: UnitQuaternion::from_euler_angles(rot_x, 0.0, 0.0),
            });
        }

        self.scene = ctx.scenes.add(scene);
        self.pending_pills = pending_pills;
        self.pending_index = 0;
        self.spawn_batch = spawn_batch;
        self.pill_handles.clear();
        self.materials = materials;
        self.material_count = material_count;
        self.quality_set = false;
        self.last_frame_ms = 0.0;
        self.last_fps = 0.0;
    }

    fn update(&mut self, context: &mut PluginContext) {
        let update_start = Instant::now();
        // Ensure post effects are disabled (tonemapper only).
        if !self.quality_set {
            if let GraphicsContext::Initialized(graphics_context) = context.graphics_context {
                let mut quality = graphics_context.renderer.get_quality_settings();
                quality.use_bloom = false;
                quality.use_ssao = false;
                quality.fxaa = false;
                quality.light_scatter_enabled = false;
                quality.use_parallax_mapping = false;
                quality.point_shadows_enabled = false;
                quality.spot_shadows_enabled = false;
                quality.csm_settings.enabled = false;
                let _ = graphics_context.renderer.set_quality_settings(&quality);
                self.quality_set = true;
            }
        }

        // Create FPS text widget in each UI once graphics (and window UI) exist.
        if self.fps_text_handles.is_empty() {
            for ui in context.user_interfaces.iter_mut() {
                let handle = TextBuilder::new(
                    WidgetBuilder::new()
                        .with_draw_on_top(true)
                        .with_z_index(10_000)
                        .with_foreground(Brush::Solid(Color::opaque(255, 255, 255)).into())
                        .with_margin(fyrox::gui::Thickness::uniform(6.0))
                        .with_desired_position(Vector2::new(6.0, 6.0)),
                )
                .with_font_size(40.0.into())
                .with_shadow(true)
                .with_text("Fyrox, pills: __, update_ms: --  frame_ms: --  FPS: --")
                .build(&mut ui.build_ctx());
                ui.link_nodes(handle, ui.root(), true);
                ui.send_message(WidgetMessage::topmost(handle, MessageDirection::ToWidget));
                self.fps_text_handles.push(handle);
            }
        }

        if let Some(scene_ref) = context.scenes.try_get_mut(self.scene) {
            // Spawn pills in batches to avoid blocking the event loop.
            if self.pending_index < self.pending_pills.len() {
                let root = scene_ref.graph.get_root();
                let cylinder_axis = Matrix4::identity();
                let remaining = self.pending_pills.len() - self.pending_index;
                let batch = self.spawn_batch.min(remaining);
                let start = self.pending_index;
                for (offset, spawn) in self.pending_pills[start..start + batch].iter().enumerate() {
                    let material_index = (start + offset) % self.material_count;
                    let mut transform = Transform::default();
                    transform.set_position(spawn.position);
                    transform.set_rotation(spawn.rotation);
                    let surface = SurfaceBuilder::new(SurfaceResource::new_embedded(
                        SurfaceData::make_cylinder(16, 0.3, 1.0, true, &cylinder_axis),
                    ))
                    .with_material(self.materials[material_index].clone())
                    .build();
                    let pill = MeshBuilder::new(BaseBuilder::new().with_local_transform(transform))
                        .with_surfaces(vec![surface])
                        .build(&mut scene_ref.graph);
                    scene_ref.graph.link_nodes(pill, root);
                    self.pill_handles.push(pill);
                }
                self.pending_index += batch;
                if self.pending_index == self.pending_pills.len() {
                    println!("Spawned {} pills", self.pending_index);
                }
            }

            // Mirror pill_rotation_system: rotate each pill around Y by 90 * dt.
            let dt = context.dt;
            let delta_rot =
                UnitQuaternion::from_axis_angle(&Vector3::y_axis(), 90.0f32.to_radians() * dt);
            for handle in &self.pill_handles {
                if let Some(node) = scene_ref.graph.try_get_mut(*handle) {
                    let rot = **node.local_transform().rotation();
                    node.local_transform_mut().set_rotation(rot * delta_rot);
                }
            }
        }
        let update_ms = update_start.elapsed().as_secs_f64() * 1000.0;
        // let _ = writeln!(std::io::stdout(), "update_ms: {:.3}", update_ms);
        // let _ = std::io::stdout().flush();

        let render_ms = (self.last_frame_ms - update_ms).max(0.0);
        let label = format!(
            "Fyrox, pills: {}/{}  update_ms: {:.1}  frame_ms: {:.1}  render_ms: {:.1}  FPS: {:.1}",
            self.pending_index,
            self.pill_count,
            update_ms,
            self.last_frame_ms,
            render_ms,
            self.last_fps
        );
        for (ui, &handle) in context
            .user_interfaces
            .iter_mut()
            .zip(self.fps_text_handles.iter())
        {
            ui.send_message(TextMessage::text(
                handle,
                MessageDirection::ToWidget,
                label.clone(),
            ));
        }
    }

    fn before_rendering(&mut self, _context: PluginContext) {
        if let Some(t) = self.last_frame_start.take() {
            let frame_ms = t.elapsed().as_secs_f64() * 1000.0;
            let fps = if frame_ms > 0.0 {
                1000.0 / frame_ms
            } else {
                0.0
            };
            self.last_frame_ms = frame_ms;
            self.last_fps = fps;
            // let _ = writeln!(
            //     std::io::stdout(),
            //     "frame_ms: {:.3} (FPS: {:.1})",
            //     frame_ms,
            //     fps
            // );
            // let _ = std::io::stdout().flush();
        }
        self.last_frame_start = Some(Instant::now());
    }

    fn on_scene_loaded(
        &mut self,
        _path: &Path,
        scene: Handle<Scene>,
        _data: &[u8],
        _context: &mut PluginContext,
    ) {
        self.scene = scene;
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut executor = Executor::from_params(
        Some(event_loop),
        GraphicsContextParams {
            window_attributes: WindowAttributes::default()
                .with_title("Fyrox 60k pills")
                .with_inner_size(PhysicalSize::new(1920, 1080))
                .with_position(PhysicalPosition::new(100, 100))
                .with_visible(true)
                .with_active(true),
            vsync: true,
            msaa_sample_count: None,
            graphics_server_constructor: Default::default(),
            named_objects: false,
        },
    );
    executor.add_plugin(Game::default());
    executor.run();
}

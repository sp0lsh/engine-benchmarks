use pill_engine::game::*;
use rand::{thread_rng, Rng};

pub struct PillComponent;

impl Component for PillComponent {}

impl PillTypeMapKey for PillComponent {
    type Storage = ComponentStorage<Self>;
}

pub struct Game {}

impl PillGame for Game {
    fn start(&self, engine: &mut Engine) -> Result<()> {
        let scene = create_scene_benchmark(engine, "Benchmark 60k pills")?;
        engine.set_active_scene(scene)?;
        Ok(())
    }
}

fn create_scene_benchmark(engine: &mut Engine, name: &str) -> Result<SceneHandle> {
    let mut rng = thread_rng();
    let scene = engine.create_scene(name)?;

    engine.register_component::<TransformComponent>(scene)?;
    engine.register_component::<MeshRenderingComponent>(scene)?;
    engine.register_component::<CameraComponent>(scene)?;
    engine.register_component::<PillComponent>(scene)?;

    engine.add_system("PillRotation", pill_rotation_system)?;

    let pill_mesh = Mesh::new("Pill", "models/pill.obj".into());
    let pill_mesh_handle = engine.add_resource(pill_mesh)?;

    let mut materials: Vec<MaterialHandle> = Vec::with_capacity(10);
    for i in 0..10 {
        let mat = Material::builder(&format!("Mat{}", i))
            .color_parameter(
                "tint",
                Color::new(
                    rng.gen_range(0.2..1.0),
                    rng.gen_range(0.2..1.0),
                    rng.gen_range(0.2..1.0),
                ),
            )?
            .build();
        materials.push(engine.add_resource(mat)?);
    }

    let camera = engine.create_entity(scene)?;
    let cam_transform = TransformComponent::builder()
        .position(Vector3f::new(0.0, 0.0, -50.0))
        .build();
    engine.add_component_to_entity(scene, camera, cam_transform)?;
    let cam_component = CameraComponent::builder().enabled(true).fov(60.0).build();
    engine.add_component_to_entity(scene, camera, cam_component)?;

    for i in 0..60000 {
        let entity = engine.create_entity(scene)?;
        let transform = TransformComponent::builder()
            .position(Vector3f::new(
                rng.gen_range(-30.0..30.0),
                rng.gen_range(-20.0..20.0),
                rng.gen_range(-10.0..10.0),
            ))
            .rotation(Vector3f::new(
                rng.gen_range(0.0..360.0),
                rng.gen_range(0.0..360.0),
                rng.gen_range(0.0..360.0),
            ))
            .scale(Vector3f::new(0.3, 0.3, 0.3))
            .build();
        engine.add_component_to_entity(scene, entity, transform)?;

        let mesh_render = MeshRenderingComponent::builder()
            .mesh(&pill_mesh_handle)
            .material(&materials[i % 10])
            .build();
        engine.add_component_to_entity(scene, entity, mesh_render)?;
        engine.add_component_to_entity(scene, entity, PillComponent)?;
    }

    Ok(scene)
}

fn pill_rotation_system(engine: &mut Engine) -> Result<()> {
    let delta_time = engine.get_global_component::<TimeComponent>()?.delta_time;

    for (_, transform, _) in engine.iterate_two_components_mut::<TransformComponent, PillComponent>()? {
        transform.rotate_around_axis(45.0 * delta_time, Vector3f::new(0.0, 1.0, 0.0));
    }

    Ok(())
}

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};
use icarus::TerritoryType::TerritoryTypeSheet;
use kawari::config::get_config;
use physis::{
    common::{Language, Platform},
    layer::{LayerEntryData, LayerGroup, ModelCollisionType, Transformation},
    lvb::Lvb,
    pcb::{Pcb, ResourceNode},
    resource::{Resource, SqPackResource},
};
use recastnavigation_sys::{
    CreateContext, rcAllocCompactHeightfield, rcAllocContourSet, rcAllocHeightfield,
    rcAllocPolyMesh, rcAllocPolyMeshDetail, rcBuildCompactHeightfield, rcBuildContours,
    rcBuildPolyMesh, rcBuildPolyMeshDetail, rcContext, rcCreateHeightfield, rcHeightfield,
    rcRasterizeTriangles,
};

#[derive(Resource)]
struct ZoneToLoad(u16);

fn main() {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();
    let zone_id: u16 = args[1].parse().unwrap();

    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .insert_resource(ZoneToLoad(zone_id))
        .run();
}

/// Walk each node, add it's collision model to the scene.
fn walk_node(
    node: &ResourceNode,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    transform: &Transformation,
    context: *mut rcContext,
    height_field: *mut rcHeightfield,
) {
    if !node.vertices.is_empty() {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        );

        let mut positions = Vec::new();
        for vec in &node.vertices {
            positions.push(vec.clone());
        }

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());

        let mut indices = Vec::new();
        for polygon in &node.polygons {
            let mut vec: Vec<u32> = Vec::from(&polygon.vertex_indices)
                .iter()
                .map(|x| *x as u32)
                .collect();
            assert!(vec.len() == 3);
            indices.append(&mut vec);
        }

        mesh.insert_indices(Indices::U32(indices.clone()));

        // insert into 3d scene
        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(Color::srgb(
                fastrand::f32(),
                fastrand::f32(),
                fastrand::f32(),
            ))),
            Transform {
                translation: Vec3::from_array(transform.translation),
                rotation: Quat::from_euler(
                    EulerRot::XYZ,
                    transform.rotation[0],
                    transform.rotation[1],
                    transform.rotation[2],
                ),
                scale: Vec3::from_array(transform.scale),
            },
        ));

        // Step 2: insert geoemtry into heightfield
        let tile_indices: Vec<i32> = indices.iter().map(|x| *x as i32).collect();
        let tri_area_ids: Vec<u8> = vec![0; tile_indices.len() / 3];

        unsafe {
            assert!(rcRasterizeTriangles(
                context,
                std::mem::transmute::<*const [f32; 3], *const f32>(positions.as_ptr()),
                positions.len() as i32,
                tile_indices.as_ptr(),
                tri_area_ids.as_ptr(),
                tile_indices.len() as i32 / 3,
                height_field,
                1
            ));
        }
    }

    for child in &node.children {
        walk_node(
            &child,
            commands,
            meshes,
            materials,
            transform,
            context,
            height_field,
        );
    }
}

/// Setup 3D scene.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    zone_id: Res<ZoneToLoad>,
) {
    let zone_id = zone_id.0;
    let config = get_config();

    tracing::info!("Generating navmesh for zone {zone_id}!");

    let mut sqpack_resource =
        SqPackResource::from_existing(Platform::Win32, &config.filesystem.game_path);
    let sheet = TerritoryTypeSheet::read_from(&mut sqpack_resource, Language::None).unwrap();
    let Some(row) = sheet.get_row(zone_id as u32) else {
        tracing::error!("Invalid zone id {zone_id}!");
        return;
    };

    // e.g. ffxiv/fst_f1/fld/f1f3/level/f1f3
    let bg_path = row.Bg().into_string().unwrap();

    let path = format!("bg/{}.lvb", &bg_path);
    let lvb_file = sqpack_resource.read(&path).unwrap();
    let lvb = Lvb::from_existing(&lvb_file).unwrap();

    let context;
    let height_field;
    unsafe {
        context = CreateContext(true);

        // Step 1: Create a heightfield
        let size_x = 100;
        let size_z = 100;
        let min_bounds = [-100.0, -100.0, -100.0];
        let max_bounds = [100.0, 100.0, 100.0];
        let cell_size = 10.0;
        let cell_height = 10.0;

        height_field = rcAllocHeightfield();
        assert!(rcCreateHeightfield(
            context,
            height_field,
            size_x,
            size_z,
            min_bounds.as_ptr(),
            max_bounds.as_ptr(),
            cell_size,
            cell_height
        ));
    }

    for path in &lvb.scns[0].header.path_layer_group_resources {
        if path.contains("bg.lgb") {
            tracing::info!("Processing {path}...");

            let lgb_file = sqpack_resource.read(path).unwrap();
            let lgb = LayerGroup::from_existing(&lgb_file);
            let Some(lgb) = lgb else {
                tracing::error!(
                    "Failed to parse {path}, this is most likely a bug in Physis and should be reported somewhere!"
                );
                return;
            };

            // TODO: i think we know which layer is specifically used for navmesh gen, better check that LVB
            for chunk in &lgb.chunks {
                for layer in &chunk.layers {
                    for object in &layer.objects {
                        if let LayerEntryData::BG(bg) = &object.data {
                            if !bg.collision_asset_path.value.is_empty() {
                                tracing::info!("Considering {} for navimesh", object.instance_id);
                                tracing::info!("- Loading {}", bg.collision_asset_path.value);

                                // NOTE: assert is here to find out the unknown
                                assert!(bg.collision_type == ModelCollisionType::Replace);

                                let pcb_file = sqpack_resource
                                    .read(&bg.collision_asset_path.value)
                                    .unwrap();
                                let pcb = Pcb::from_existing(&pcb_file).unwrap();

                                walk_node(
                                    &pcb.root_node,
                                    &mut commands,
                                    &mut meshes,
                                    &mut materials,
                                    &object.transform,
                                    context,
                                    height_field,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    unsafe {
        // Step 3: Build a compact heightfield out of the normal heightfield
        let compact_heightfield = rcAllocCompactHeightfield();
        let walkable_height = 1;
        let walkable_climb = 1;
        assert!(rcBuildCompactHeightfield(
            context,
            walkable_height,
            walkable_climb,
            height_field,
            compact_heightfield
        ));

        // Step 4: Build the contour set from the compact heightfield
        let contour_set = rcAllocContourSet();
        let max_error = 1.0;
        let max_edge_len = 0;
        let build_flags = 0;
        assert!(rcBuildContours(
            context,
            compact_heightfield,
            max_error,
            max_edge_len,
            contour_set,
            build_flags
        ));

        // Step 5: Build the polymesh out of the contour set
        let poly_mesh = rcAllocPolyMesh();
        let nvp = 3;
        assert!(rcBuildPolyMesh(context, contour_set, nvp, poly_mesh));

        // Step 6: Build the polymesh detail
        let poly_mesh_detail = rcAllocPolyMeshDetail();
        let sample_dist = 0.0;
        let sample_max_error = 0.0;
        assert!(rcBuildPolyMeshDetail(
            context,
            poly_mesh,
            compact_heightfield,
            sample_dist,
            sample_max_error,
            poly_mesh_detail
        ));
    }

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(15.0, 15.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

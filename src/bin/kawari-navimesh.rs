use std::ptr::{null, null_mut};

use bevy::{
    asset::RenderAssetUsages,
    color::palettes::{
        css::WHITE,
        tailwind::{BLUE_100, GREEN_100, PINK_100, RED_500},
    },
    pbr::wireframe::{Wireframe, WireframeConfig, WireframePlugin},
    picking::pointer::PointerInteraction,
    prelude::*,
    render::{
        RenderPlugin,
        mesh::{Indices, PrimitiveTopology},
        settings::{RenderCreation, WgpuFeatures, WgpuSettings},
    },
};
use icarus::TerritoryType::TerritoryTypeSheet;
use kawari::{
    config::get_config,
    world::{Navmesh, NavmeshParams},
};
use physis::{
    common::{Language, Platform},
    layer::{LayerEntryData, LayerGroup, ModelCollisionType, Transformation},
    lvb::Lvb,
    model::MDL,
    pcb::{Pcb, ResourceNode},
    resource::{Resource, SqPackResource},
    tera::{PlateModel, Terrain},
};
use recastnavigation_sys::{
    CreateContext, DT_SUCCESS, RC_MESH_NULL_IDX, dtCreateNavMeshData, dtNavMeshCreateParams,
    dtNavMeshQuery, dtNavMeshQuery_findNearestPoly, dtPolyRef, dtQueryFilter,
    rcAllocCompactHeightfield, rcAllocContourSet, rcAllocHeightfield, rcAllocPolyMesh,
    rcAllocPolyMeshDetail, rcBuildCompactHeightfield, rcBuildContours,
    rcBuildContoursFlags_RC_CONTOUR_TESS_WALL_EDGES, rcBuildDistanceField, rcBuildPolyMesh,
    rcBuildPolyMeshDetail, rcBuildRegions, rcCalcGridSize, rcContext, rcCreateHeightfield,
    rcErodeWalkableArea, rcHeightfield, rcMarkWalkableTriangles, rcRasterizeTriangles,
};

#[derive(Resource)]
struct ZoneToLoad(u16);

#[derive(Resource, Default)]
struct NavigationState {
    navmesh: Navmesh,
    path: Vec<Vec3>,
    from_position: Vec3,
    to_position: Vec3,
}

impl NavigationState {
    pub fn calculate_path(&mut self) {
        let start_pos = [
            self.from_position.x,
            self.from_position.y,
            self.from_position.z,
        ];
        let end_pos = [self.to_position.x, self.to_position.y, self.to_position.z];

        self.path = self
            .navmesh
            .calculate_path(start_pos, end_pos)
            .iter()
            .map(|x| Vec3::from_slice(x))
            .collect();
    }
}

unsafe impl Send for NavigationState {}
unsafe impl Sync for NavigationState {}

fn main() {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();
    let zone_id: u16 = args[1].parse().unwrap();

    App::new()
        .add_event::<Navigate>()
        .add_event::<SetOrigin>()
        .add_event::<SetTarget>()
        .add_plugins((
            DefaultPlugins.set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    features: WgpuFeatures::POLYGON_MODE_LINE,
                    ..default()
                }),
                ..default()
            }),
            MeshPickingPlugin,
            WireframePlugin::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, draw_mesh_intersections)
        .insert_resource(WireframeConfig {
            global: false,
            default_color: WHITE.into(),
        })
        .insert_resource(ZoneToLoad(zone_id))
        .insert_resource(NavigationState::default())
        .run();
}

#[derive(Event, Reflect, Clone, Debug)]
struct Navigate();

#[derive(Event, Reflect, Clone, Debug)]
struct SetOrigin(Vec3);

#[derive(Event, Reflect, Clone, Debug)]
struct SetTarget(Vec3);

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
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());

        let mut positions = Vec::new();
        for vec in &node.vertices {
            positions.push(Vec3::from_slice(vec));
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

        mesh.compute_normals();

        let transform = Transform {
            translation: Vec3::from_array(transform.translation),
            rotation: Quat::from_euler(
                EulerRot::XYZ,
                transform.rotation[0],
                transform.rotation[1],
                transform.rotation[2],
            ),
            scale: Vec3::from_array(transform.scale),
        };

        // insert into 3d scene
        commands
            .spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(materials.add(Color::srgb(
                    fastrand::f32(),
                    fastrand::f32(),
                    fastrand::f32(),
                ))),
                transform,
            ))
            .observe(
                |mut trigger: Trigger<Pointer<Click>>,
                 mut navigate_events: EventWriter<Navigate>,
                 mut target_events: EventWriter<SetTarget>,
                 mut origin_events: EventWriter<SetOrigin>| {
                    let click_event: &Pointer<Click> = trigger.event();
                    match click_event.button {
                        PointerButton::Primary => {
                            target_events.write(SetTarget(click_event.hit.position.unwrap()));
                        }
                        PointerButton::Secondary => {
                            origin_events.write(SetOrigin(click_event.hit.position.unwrap()));
                        }
                        PointerButton::Middle => {
                            navigate_events.write(Navigate());
                        }
                    }
                    trigger.propagate(false);
                },
            );

        // Step 2: insert geoemtry into heightfield
        let tile_indices: Vec<i32> = indices.iter().map(|x| *x as i32).collect();
        let mut tri_area_ids: Vec<u8> = vec![0; tile_indices.len() / 3];

        // transform the vertices on the CPU
        let mut tile_vertices: Vec<[f32; 3]> = Vec::new();
        let transform_matrix = transform.compute_matrix();
        for vertex in &positions {
            let transformed_vertex = transform_matrix.transform_point3(*vertex);
            tile_vertices.push([
                transformed_vertex.x,
                transformed_vertex.y,
                transformed_vertex.z,
            ]);
        }

        unsafe {
            let ntris = tile_indices.len() as i32 / 3;

            // mark areas as walkable
            rcMarkWalkableTriangles(
                context,
                45.0,
                std::mem::transmute::<*const [f32; 3], *const f32>(tile_vertices.as_ptr()),
                positions.len() as i32,
                tile_indices.as_ptr(),
                ntris,
                tri_area_ids.as_mut_ptr(),
            );

            assert!(rcRasterizeTriangles(
                context,
                std::mem::transmute::<*const [f32; 3], *const f32>(tile_vertices.as_ptr()),
                positions.len() as i32,
                tile_indices.as_ptr(),
                tri_area_ids.as_ptr(),
                ntris,
                height_field,
                2
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

fn add_plate(
    plate: &PlateModel,
    tera_path: &str,
    sqpack_resource: &mut SqPackResource,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    context: *mut rcContext,
    height_field: *mut rcHeightfield,
) {
    let mdl_path = format!("{}/bgplate/{}", tera_path, plate.filename);
    let mdl_bytes = sqpack_resource.read(&mdl_path).unwrap();
    let mdl = MDL::from_existing(&mdl_bytes).unwrap();

    let lod = &mdl.lods[0];
    for part in &lod.parts {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());

        let mut positions = Vec::new();
        let mut normals = Vec::new();
        for vec in &part.vertices {
            positions.push(Vec3::from_slice(&vec.position));
            normals.push(Vec3::from_slice(&vec.normal));
        }
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.clone());

        mesh.insert_indices(Indices::U16(part.indices.clone()));

        let transform = Transform::from_xyz(plate.position.0, 0.0, plate.position.1);

        // insert into 3d scene
        commands
            .spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(materials.add(Color::srgb(
                    fastrand::f32(),
                    fastrand::f32(),
                    fastrand::f32(),
                ))),
                transform,
            ))
            .observe(
                |mut trigger: Trigger<Pointer<Click>>,
                 mut navigate_events: EventWriter<Navigate>,
                 mut target_events: EventWriter<SetTarget>,
                 mut origin_events: EventWriter<SetOrigin>| {
                    let click_event: &Pointer<Click> = trigger.event();
                    match click_event.button {
                        PointerButton::Primary => {
                            target_events.write(SetTarget(click_event.hit.position.unwrap()));
                        }
                        PointerButton::Secondary => {
                            origin_events.write(SetOrigin(click_event.hit.position.unwrap()));
                        }
                        PointerButton::Middle => {
                            navigate_events.write(Navigate());
                        }
                    }
                    trigger.propagate(false);
                },
            );

        // Step 2: insert geoemtry into heightfield
        let tile_indices: Vec<i32> = part.indices.iter().map(|x| *x as i32).collect();
        let mut tri_area_ids: Vec<u8> = vec![0; tile_indices.len() / 3];

        // transform the vertices on the CPU
        let mut tile_vertices: Vec<[f32; 3]> = Vec::new();
        let transform_matrix = transform.compute_matrix();
        for vertex in &positions {
            let transformed_vertex = transform_matrix.transform_point3(*vertex);
            tile_vertices.push([
                transformed_vertex.x,
                transformed_vertex.y,
                transformed_vertex.z,
            ]);
        }

        unsafe {
            let ntris = tile_indices.len() as i32 / 3;

            // mark areas as walkable
            rcMarkWalkableTriangles(
                context,
                45.0,
                std::mem::transmute::<*const [f32; 3], *const f32>(tile_vertices.as_ptr()),
                positions.len() as i32,
                tile_indices.as_ptr(),
                ntris,
                tri_area_ids.as_mut_ptr(),
            );

            assert!(rcRasterizeTriangles(
                context,
                std::mem::transmute::<*const [f32; 3], *const f32>(tile_vertices.as_ptr()),
                positions.len() as i32,
                tile_indices.as_ptr(),
                tri_area_ids.as_ptr(),
                ntris,
                height_field,
                2
            ));
        }
    }
}

fn get_polygon_at_location(
    query: *const dtNavMeshQuery,
    position: [f32; 3],
    filter: &dtQueryFilter,
) -> (dtPolyRef, [f32; 3]) {
    let extents = [2.0, 4.0, 2.0];

    unsafe {
        let mut nearest_ref = 0;
        let mut nearest_pt = [0.0; 3];
        assert!(
            dtNavMeshQuery_findNearestPoly(
                query,
                position.as_ptr(),
                extents.as_ptr(),
                filter,
                &mut nearest_ref,
                nearest_pt.as_mut_ptr()
            ) == DT_SUCCESS
        );

        return (nearest_ref, nearest_pt);
    }
}

/// Setup 3D scene.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    zone_id: Res<ZoneToLoad>,
    mut navigation_state: ResMut<NavigationState>,
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
    let cell_size = 0.25;
    let cell_height = 0.25;
    unsafe {
        context = CreateContext(true);

        // Step 1: Create a heightfield
        let mut size_x: i32 = 0;
        let mut size_z: i32 = 0;
        let min_bounds = [-100.0, -100.0, -100.0];
        let max_bounds = [100.0, 100.0, 100.0];

        rcCalcGridSize(
            min_bounds.as_ptr(),
            max_bounds.as_ptr(),
            cell_size,
            &mut size_x,
            &mut size_z,
        );

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

    let scene = &lvb.scns[0];

    let tera_bytes = sqpack_resource
        .read(&*format!(
            "{}/bgplate/terrain.tera",
            scene.general.path_terrain
        ))
        .unwrap();
    let tera = Terrain::from_existing(&tera_bytes).unwrap();
    for plate in tera.plates {
        add_plate(
            &plate,
            &scene.general.path_terrain,
            &mut sqpack_resource,
            &mut commands,
            &mut meshes,
            &mut materials,
            context,
            height_field,
        );
    }

    for path in &scene.header.path_layer_group_resources {
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
        let walkable_height = 2;
        let walkable_climb = 1;
        let walkable_radius = 0.5;
        assert!(rcBuildCompactHeightfield(
            context,
            walkable_height,
            walkable_climb,
            height_field,
            compact_heightfield
        ));
        assert!((*compact_heightfield).spanCount > 0);

        assert!(rcErodeWalkableArea(
            context,
            walkable_radius as i32,
            compact_heightfield
        ));

        assert!(rcBuildDistanceField(context, compact_heightfield));

        let border_size = 2;
        let min_region_area = 1;
        let merge_region_area = 0;
        assert!(rcBuildRegions(
            context,
            compact_heightfield,
            border_size,
            min_region_area,
            merge_region_area
        ));

        // Step 4: Build the contour set from the compact heightfield
        let contour_set = rcAllocContourSet();
        let max_error = 1.5;
        let max_edge_len = (12.0 / cell_size) as i32;
        let build_flags = rcBuildContoursFlags_RC_CONTOUR_TESS_WALL_EDGES as i32;
        assert!(rcBuildContours(
            context,
            compact_heightfield,
            max_error,
            max_edge_len,
            contour_set,
            build_flags
        ));
        assert!((*contour_set).nconts > 0);

        // Step 5: Build the polymesh out of the contour set
        let poly_mesh = rcAllocPolyMesh();
        let nvp = 6;
        assert!(rcBuildPolyMesh(context, contour_set, nvp, poly_mesh));
        assert!((*poly_mesh).verts != null_mut());
        assert!((*poly_mesh).nverts > 0);

        let nvp = (*poly_mesh).nvp;
        let cs = (*poly_mesh).cs;
        let ch = (*poly_mesh).ch;
        let orig = (*poly_mesh).bmin;

        // add polymesh to visualization
        {
            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());

            let mut positions = Vec::new();
            for i in 0..(*poly_mesh).nverts as usize {
                let v = (*poly_mesh).verts.wrapping_add(i * 3);
                let x = orig[0] + *v as f32 * cs as f32;
                let y = orig[1] + (*v.wrapping_add(1) + 1) as f32 * ch as f32 + 0.1;
                let z = orig[2] + (*v.wrapping_add(2)) as f32 * cs as f32;

                positions.push(Vec3::new(x, y, z));
            }

            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());

            let mut indices = Vec::new();
            for i in 0..(*poly_mesh).npolys as usize {
                let p = (*poly_mesh).polys.wrapping_add(i * nvp as usize * 2);
                for j in 2..nvp as usize {
                    if *(p.wrapping_add(j)) == RC_MESH_NULL_IDX {
                        break;
                    }

                    indices.push(*p);
                    indices.push(*p.wrapping_add(j - 1));
                    indices.push(*p.wrapping_add(j));
                }
            }

            mesh.insert_indices(Indices::U16(indices.clone()));

            //mesh.compute_normals();

            // insert into 3d scene
            commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(materials.add(Color::srgba(0.0, 0.0, 1.0, 0.5))),
                Pickable::IGNORE,
                Wireframe,
            ));
        }

        let flags =
            std::slice::from_raw_parts_mut((*poly_mesh).flags, (*poly_mesh).npolys as usize);
        for flag in flags {
            *flag = 1;
        }

        // Step 6: Build the polymesh detail
        let poly_mesh_detail = rcAllocPolyMeshDetail();
        let sample_dist = 1.0;
        let sample_max_error = 0.1;
        assert!(rcBuildPolyMeshDetail(
            context,
            poly_mesh,
            compact_heightfield,
            sample_dist,
            sample_max_error,
            poly_mesh_detail
        ));

        let mut create_params = dtNavMeshCreateParams {
            // Polygon Mesh Attributes
            verts: (*poly_mesh).verts,
            vertCount: (*poly_mesh).nverts,
            polys: (*poly_mesh).polys,
            polyFlags: (*poly_mesh).flags,
            polyAreas: (*poly_mesh).areas,
            polyCount: (*poly_mesh).npolys,
            nvp: (*poly_mesh).nvp,

            // Height Detail Attributes
            detailMeshes: (*poly_mesh_detail).meshes,
            detailVerts: (*poly_mesh_detail).verts,
            detailVertsCount: (*poly_mesh_detail).nverts,
            detailTris: (*poly_mesh_detail).tris,
            detailTriCount: (*poly_mesh_detail).ntris,

            // Off-Mesh Connections Attributes
            offMeshConVerts: null(),
            offMeshConRad: null(),
            offMeshConFlags: null(),
            offMeshConAreas: null(),
            offMeshConDir: null(),
            offMeshConUserID: null(),
            offMeshConCount: 0,

            // Tile Attributes
            userId: 0,
            tileX: 0,
            tileY: 0,
            tileLayer: 0,
            bmin: (*poly_mesh).bmin,
            bmax: (*poly_mesh).bmax,

            // General Configuration Attributes
            walkableHeight: walkable_height as f32,
            walkableRadius: walkable_radius,
            walkableClimb: walkable_climb as f32,
            cs: cell_size,
            ch: cell_height,
            buildBvTree: true,
        };

        let mut out_data: *mut u8 = null_mut();
        let mut out_data_size = 0;
        assert!(dtCreateNavMeshData(
            &mut create_params,
            &mut out_data,
            &mut out_data_size
        ));
        assert!(out_data != null_mut());
        assert!(out_data_size > 0);

        navigation_state.navmesh = Navmesh::new(
            NavmeshParams {
                orig: (*poly_mesh).bmin,
                tile_width: (*poly_mesh).bmax[0] - (*poly_mesh).bmin[0],
                tile_height: (*poly_mesh).bmax[2] - (*poly_mesh).bmin[2],
                max_tiles: 1,
                max_polys: (*poly_mesh).npolys,
            },
            Vec::from_raw_parts(out_data, out_data_size as usize, out_data_size as usize),
        );

        // TODO: output in the correct directory
        let serialized_navmesh = navigation_state.navmesh.write_to_buffer().unwrap();
        std::fs::write("test.nvm", &serialized_navmesh).unwrap();
    }

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(55.0, 55.0, 55.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn draw_mesh_intersections(
    pointers: Query<&PointerInteraction>,
    mut gizmos: Gizmos,
    mut navigate_events: EventReader<Navigate>,
    mut origin_events: EventReader<SetOrigin>,
    mut target_events: EventReader<SetTarget>,
    mut navigation_state: ResMut<NavigationState>,
) {
    gizmos.sphere(navigation_state.from_position, 0.05, GREEN_100);
    gizmos.sphere(navigation_state.to_position, 0.05, BLUE_100);

    for pos in &navigation_state.path {
        gizmos.sphere(*pos, 0.05, RED_500);
    }

    for (point, normal) in pointers
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .filter_map(|(_entity, hit)| hit.position.zip(hit.normal))
    {
        gizmos.sphere(point, 0.05, RED_500);
        gizmos.arrow(point, point + normal.normalize() * 0.5, PINK_100);
    }

    for event in origin_events.read() {
        navigation_state.from_position = event.0;
    }

    for event in target_events.read() {
        navigation_state.to_position = event.0;
    }

    for _ in navigate_events.read() {
        navigation_state.calculate_path();
    }
}

use std::ptr::{null, null_mut};

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
    CreateContext, DT_SUCCESS, dtAllocNavMesh, dtAllocNavMeshQuery, dtCreateNavMeshData,
    dtNavMesh_addTile, dtNavMesh_init, dtNavMeshCreateParams, dtNavMeshParams, dtNavMeshQuery,
    dtNavMeshQuery_findNearestPoly, dtNavMeshQuery_findPath, dtNavMeshQuery_init, dtPolyRef,
    dtQueryFilter, dtQueryFilter_dtQueryFilter, rcAllocCompactHeightfield, rcAllocContourSet,
    rcAllocHeightfield, rcAllocPolyMesh, rcAllocPolyMeshDetail, rcBuildCompactHeightfield,
    rcBuildContours, rcBuildContoursFlags_RC_CONTOUR_TESS_WALL_EDGES, rcBuildDistanceField,
    rcBuildPolyMesh, rcBuildPolyMeshDetail, rcBuildRegions, rcCalcGridSize, rcContext,
    rcCreateHeightfield, rcErodeWalkableArea, rcHeightfield, rcMarkWalkableTriangles,
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
        let mut tri_area_ids: Vec<u8> = vec![0; tile_indices.len() / 3];

        unsafe {
            let ntris = tile_indices.len() as i32 / 3;

            // mark areas as walkable
            rcMarkWalkableTriangles(
                context,
                45.0,
                std::mem::transmute::<*const [f32; 3], *const f32>(positions.as_ptr()),
                positions.len() as i32,
                tile_indices.as_ptr(),
                ntris,
                tri_area_ids.as_mut_ptr(),
            );

            assert!(rcRasterizeTriangles(
                context,
                std::mem::transmute::<*const [f32; 3], *const f32>(positions.as_ptr()),
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

fn get_polygon_at_location(
    query: *const dtNavMeshQuery,
    position: [f32; 3],
    filter: &dtQueryFilter,
) -> (dtPolyRef, [f32; 3]) {
    let extents = [3.0, 5.0, 3.0];

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
        assert!(nearest_ref != 0);

        return (nearest_ref, nearest_pt);
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

        let navmesh_params = dtNavMeshParams {
            orig: [0.0; 3],
            tileWidth: 100.0,
            tileHeight: 100.0,
            maxTiles: 1000,
            maxPolys: 1000,
        };

        let navmesh = dtAllocNavMesh();
        assert!(dtNavMesh_init(navmesh, &navmesh_params) == DT_SUCCESS);

        assert!(
            dtNavMesh_addTile(navmesh, out_data, out_data_size, 0, 0, null_mut()) == DT_SUCCESS
        );

        let query = dtAllocNavMeshQuery();
        dtNavMeshQuery_init(query, navmesh, 1024);

        let start_pos = [0.0, 0.0, 0.0];
        let end_pos = [5.0, 0.0, 0.0];

        let mut filter = dtQueryFilter {
            m_areaCost: [0.0; 64],
            m_includeFlags: 0,
            m_excludeFlags: 0,
        };
        dtQueryFilter_dtQueryFilter(&mut filter);

        let (start_poly, start_poly_pos) = get_polygon_at_location(query, start_pos, &filter);
        let (end_poly, end_poly_pos) = get_polygon_at_location(query, end_pos, &filter);

        let mut path = [0; 128];
        let mut path_count = 0;
        dtNavMeshQuery_findPath(
            query,
            start_poly,
            end_poly,
            start_poly_pos.as_ptr(),
            end_poly_pos.as_ptr(),
            &filter,
            path.as_mut_ptr(),
            &mut path_count,
            128,
        ); // TODO: error check
        assert!(path_count > 0);

        dbg!(path);
    }

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(15.0, 15.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

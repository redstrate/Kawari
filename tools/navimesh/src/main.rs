use std::{
    path::PathBuf,
    ptr::{null, null_mut},
};

use glam::{Affine3A, Vec3};
use icarus::RecastNavimesh::RecastNavimeshSheet;
use icarus::TerritoryType::TerritoryTypeSheet;
use kawari::config::get_config;
use kawari_world::{Navmesh, NavmeshParams, NavmeshTile};
use physis::{
    Language,
    layer::{Layer, LayerEntryData, ModelCollisionType, TriggerBoxShape},
    lgb::Lgb,
    lvb::Lvb,
    pcb::{Pcb, Polygon, ResourceNode},
    pcblist::{PcbList, PcbListEntry},
    resource::{ResourceResolver, SqPackResource},
    sgb::Sgb,
};
use recastnavigation_sys::{
    CreateContext, dtCreateNavMeshData, dtNavMeshCreateParams, rcAllocCompactHeightfield,
    rcAllocContourSet, rcAllocHeightfield, rcAllocPolyMesh, rcAllocPolyMeshDetail,
    rcBuildCompactHeightfield, rcBuildContours, rcBuildContoursFlags_RC_CONTOUR_TESS_WALL_EDGES,
    rcBuildDistanceField, rcBuildPolyMesh, rcBuildPolyMeshDetail, rcBuildRegions, rcCalcGridSize,
    rcContext, rcCreateHeightfield, rcErodeWalkableArea, rcFilterLowHangingWalkableObstacles,
    rcFilterWalkableLowHeightSpans, rcHeightfield, rcMarkWalkableTriangles, rcRasterizeTriangles,
};

fn main() {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();
    let zone_id: u16 = args[1].parse().unwrap();
    let destination_path: String = args[2].parse().unwrap();

    let config = get_config();

    tracing::info!("Generating navmesh for zone {zone_id}, writing to {destination_path}!");

    let mut resolver = ResourceResolver::new();
    resolver.add_source(SqPackResource::from_existing(&config.filesystem.game_path));

    let sheet = TerritoryTypeSheet::read_from(&mut resolver, Language::None).unwrap();
    let Some(row) = sheet.row(zone_id as u32) else {
        tracing::error!("Invalid zone id {zone_id}!");
        return;
    };

    // e.g. ffxiv/fst_f1/fld/f1f3/level/f1f3
    let bg_path = row.Bg;
    let name = row.Name;

    let path = format!("bg/{}.lvb", bg_path);
    let lvb = resolver.parsed::<Lvb>(&path).unwrap();

    let navimesh_sheet = RecastNavimeshSheet::read_from(&mut resolver, Language::None).unwrap();
    // Use default settings unless overriden
    let mut navimesh_row = navimesh_sheet
        .row(0)
        .expect("No default row in RecastNavimesh sheet?");
    for (_, row) in navimesh_sheet.into_iter().flatten_subrows() {
        if row.Name == name {
            tracing::info!("Using navimesh settings override for this zone!");
            navimesh_row = row;
        }
    }

    let context;
    unsafe {
        context = CreateContext(true);
    }

    // Determine how many tiles are needed to cover the area.
    let mut min_x = 0f32;
    let mut min_z = 0f32;
    let mut max_x = 0f32;
    let mut max_z = 0f32;

    let scene = &lvb.sections[0];

    let pcblist = resolver
        .parsed::<PcbList>(&format!(
            "{}/collision/list.pcb",
            scene.general.bg_path.value
        ))
        .unwrap();
    min_x = min_x.min(pcblist.bounds.min[0]);
    min_z = min_z.min(pcblist.bounds.min[2]);

    max_x = max_x.max(pcblist.bounds.max[0]);
    max_z = max_z.max(pcblist.bounds.max[2]);

    // TODO: take into account SGBs in bounds checking

    let tile_origin_x = min_x;
    let tile_origin_y = min_z;

    let mut size_x: i32 = 0;
    let mut size_z: i32 = 0;

    let min_y = -100.0;
    let max_y = 100.0;
    let min_bounds = [min_x, min_y, min_z];
    let max_bounds = [max_x, max_y, max_z];

    let cell_size = navimesh_row.CellSize;
    let cell_height = navimesh_row.CellHeight;

    unsafe {
        rcCalcGridSize(
            min_bounds.as_ptr(),
            max_bounds.as_ptr(),
            cell_size,
            &mut size_x,
            &mut size_z,
        );
    }

    let tile_size = navimesh_row.TileSize as i32;
    let tile_width = (size_x + tile_size - 1) / tile_size;
    let tile_height = (size_z + tile_size - 1) / tile_size;

    let walkable_radius = (navimesh_row.AgentRadius / cell_size).ceil() as i32;
    let border_size = walkable_radius + 3;
    let tile_cell_size = tile_size as f32 * cell_size;

    let mut tiles = Vec::new();
    for z in 0..tile_height {
        for x in 0..tile_width {
            // Step 1: Create a heightfield
            unsafe {
                let min_bounds = [
                    (min_x + ((x as f32) * tile_cell_size)) - (border_size as f32 * cell_size),
                    min_y,
                    (min_z + ((z as f32) * tile_cell_size)) - (border_size as f32 * cell_size),
                ];
                let max_bounds = [
                    (min_x + ((x as f32 + 1.0) * tile_cell_size))
                        + (border_size as f32 * cell_size),
                    max_y,
                    (min_z + ((z as f32 + 1.0) * tile_cell_size))
                        + (border_size as f32 * cell_size),
                ];

                let width = tile_size + border_size * 2;
                let height = tile_size + border_size * 2;

                let height_field = rcAllocHeightfield();
                assert!(rcCreateHeightfield(
                    context,
                    height_field,
                    width,
                    height,
                    min_bounds.as_ptr(),
                    max_bounds.as_ptr(),
                    cell_size,
                    cell_height
                ));

                tiles.push(Tile { x, z, height_field });
            }
        }
    }

    let max_slope = navimesh_row.AgentMaxSlope;
    let walkable_climb = (navimesh_row.AgentMaxClimb / cell_size).floor() as i32;
    for entry in pcblist.entries {
        add_plate(
            &entry,
            &scene.general.bg_path.value,
            &mut resolver,
            context,
            &tiles,
            max_slope,
            walkable_climb, // In VX units
        );
    }

    for path in &scene.lgb_paths {
        let lgb = resolver.parsed::<Lgb>(path);
        let lgb = match lgb {
            Ok(lgb) => lgb,
            Err(e) => {
                tracing::error!(
                    "Failed to parse {path}: {e}, this is most likely a bug in Physis and should be reported somewhere!"
                );
                continue;
            }
        };

        // TODO: i think we know which layer is specifically used for navmesh gen, better check that LVB
        for chunk in &lgb.chunks {
            for layer in &chunk.layers {
                // Exclude festival objects which *usually* don't have collision, but this is to exclude non-Global festivals that include non-existent SGBs.
                if layer.header.festival_id != 0 {
                    continue;
                }

                let transform = Affine3A::default();
                walk_layer(
                    &mut resolver,
                    layer,
                    &transform,
                    context,
                    &tiles,
                    max_slope,
                    walkable_climb, // In VX units
                );
            }
        }
    }

    let mut navmesh_tiles = Vec::new();
    let mut max_polys = 0;

    for tile in tiles {
        unsafe {
            let walkable_height = (navimesh_row.AgentHeight / cell_height).ceil() as i32;

            // Step 2b: Some easy filtering
            rcFilterLowHangingWalkableObstacles(context, walkable_climb, tile.height_field);
            // TODO: rcFilterLedgeSpans();
            rcFilterWalkableLowHeightSpans(context, walkable_height, tile.height_field);

            // Step 3: Build a compact heightfield out of the normal heightfield
            let compact_heightfield = rcAllocCompactHeightfield();
            assert!(!tile.height_field.is_null());
            assert!(rcBuildCompactHeightfield(
                context,
                walkable_height, // In VX units
                walkable_climb,  // In VX units
                tile.height_field,
                compact_heightfield
            ));
            if (*compact_heightfield).spanCount == 0 {
                continue;
            }
            assert!((*compact_heightfield).spanCount > 0);

            assert!(rcErodeWalkableArea(
                context,
                walkable_radius, // In VX units
                compact_heightfield
            ));

            assert!(rcBuildDistanceField(context, compact_heightfield));

            let min_region_area = navimesh_row.RegionMinSize;
            let merge_region_area = navimesh_row.RegionMergedSize;
            assert!(rcBuildRegions(
                context,
                compact_heightfield,
                border_size,
                (min_region_area.powi(2)) as i32,   // In VX units
                (merge_region_area.powi(2)) as i32, // In VX units
            ));

            // Step 4: Build the contour set from the compact heightfield
            let contour_set = rcAllocContourSet();
            let max_error = navimesh_row.MaxEdgeError;
            let max_edge_len = (navimesh_row.MaxEdgeLength / cell_size) as i32; // in VX units
            let build_flags = rcBuildContoursFlags_RC_CONTOUR_TESS_WALL_EDGES as i32;
            assert!(rcBuildContours(
                context,
                compact_heightfield,
                max_error,
                max_edge_len,
                contour_set,
                build_flags
            ));
            if (*contour_set).nconts <= 0 {
                continue;
            }
            assert!((*contour_set).nconts > 0);

            // Step 5: Build the polymesh out of the contour set
            let poly_mesh = rcAllocPolyMesh();
            let nvp = navimesh_row.VertsPerPoly as i32;
            assert!(rcBuildPolyMesh(context, contour_set, nvp, poly_mesh));
            assert!(!(*poly_mesh).verts.is_null());
            assert!((*poly_mesh).nverts > 0);

            let flags =
                std::slice::from_raw_parts_mut((*poly_mesh).flags, (*poly_mesh).npolys as usize);
            for flag in flags {
                *flag = 1;
            }

            // Step 6: Build the polymesh detail
            let poly_mesh_detail = rcAllocPolyMeshDetail();
            let sample_dist = cell_size * navimesh_row.DetailMeshSampleDistance;
            let sample_max_error = cell_height * navimesh_row.DetailMeshMaxSampleError;
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
                tileX: tile.x,
                tileY: tile.z,
                tileLayer: 0,
                bmin: (*poly_mesh).bmin,
                bmax: (*poly_mesh).bmax,

                // General Configuration Attributes
                walkableHeight: navimesh_row.AgentHeight,
                walkableRadius: navimesh_row.AgentRadius,
                walkableClimb: navimesh_row.AgentMaxClimb,
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
            assert!(!out_data.is_null());
            assert!(out_data_size > 0);

            let out_data = core::slice::from_raw_parts(out_data, out_data_size as usize);
            let mut data = vec![0; out_data_size as usize];
            data.copy_from_slice(out_data);

            navmesh_tiles.push(NavmeshTile { data });
            max_polys = max_polys.max((*poly_mesh).npolys);
        }
    }

    let navmesh = Navmesh::new(
        zone_id,
        NavmeshParams {
            orig: [tile_origin_x, min_y, tile_origin_y],
            tile_width: tile_size as f32 * cell_size,
            tile_height: tile_size as f32 * cell_size,
            max_tiles: navmesh_tiles.len() as i32,
            max_polys,
        },
        navmesh_tiles,
    );

    let serialized_navmesh = navmesh.write_to_buffer().unwrap();

    let path = PathBuf::from(&destination_path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap(); // create directory structure
    std::fs::write(destination_path, &serialized_navmesh).unwrap();
}

/// Represents the heightfield of a tile.
#[derive(Debug)]
struct Tile {
    x: i32,
    z: i32,
    height_field: *mut rcHeightfield,
}

/// Walk each node, add it's collision model to the scene.
fn walk_node(
    node: &ResourceNode,
    transform: &Affine3A,
    context: *mut rcContext,
    tiles: &[Tile],
    max_slope: f32,
    walkable_climb: i32,
) {
    if !node.vertices.is_empty() {
        let mut indices = Vec::new();
        for polygon in &node.polygons {
            let mut vec: Vec<u32> = Vec::from(&polygon.vertex_indices)
                .iter()
                .map(|x| *x as u32)
                .collect();
            assert!(vec.len() == 3);
            indices.append(&mut vec);
        }

        // Step 2: insert geoemtry into heightfield
        let tile_indices: Vec<i32> = indices.iter().map(|x| *x as i32).collect();
        let ntris = tile_indices.len() as i32 / 3;
        let mut tri_area_ids: Vec<u8> = vec![0; ntris as usize];

        // transform the vertices on the CPU
        let mut tile_vertices: Vec<[f32; 3]> = Vec::new();
        for vertex in &node.vertices {
            let transformed = transform.transform_point3(Vec3::from_slice(vertex));
            tile_vertices.push([transformed.x, transformed.y, transformed.z]);
        }

        for tile in tiles {
            unsafe {
                // mark areas as walkable
                rcMarkWalkableTriangles(
                    context,
                    max_slope,
                    std::mem::transmute::<*const [f32; 3], *const f32>(tile_vertices.as_ptr()),
                    tile_vertices.len() as i32,
                    tile_indices.as_ptr(),
                    ntris,
                    tri_area_ids.as_mut_ptr(),
                );

                assert!(rcRasterizeTriangles(
                    context,
                    std::mem::transmute::<*const [f32; 3], *const f32>(tile_vertices.as_ptr()),
                    tile_vertices.len() as i32,
                    tile_indices.as_ptr(),
                    tri_area_ids.as_ptr(),
                    ntris,
                    tile.height_field,
                    walkable_climb,
                ));
            }
        }
    }

    for child in &node.children {
        walk_node(child, transform, context, tiles, max_slope, walkable_climb);
    }
}

fn add_plate(
    entry: &PcbListEntry,
    tera_path: &str,
    sqpack_resource: &mut ResourceResolver,
    context: *mut rcContext,
    tiles: &[Tile],
    max_slope: f32,
    walkable_climb: i32,
) {
    let pcb_path = format!("{}/collision/tr{:04}.pcb", tera_path, entry.mesh_id,);
    let mdl = sqpack_resource.parsed::<Pcb>(&pcb_path).unwrap();
    let transform = Affine3A::default();
    walk_node(
        &mdl.root_node,
        &transform,
        context,
        tiles,
        max_slope,
        walkable_climb,
    );
}

/// Walk each layer, add it's collision model to the scene.
fn walk_layer(
    resolver: &mut ResourceResolver,
    layer: &Layer,
    transform: &Affine3A,
    context: *mut rcContext,
    tiles: &[Tile],
    max_slope: f32,
    walkable_climb: i32,
) {
    for object in &layer.objects {
        let child_transform: Affine3A = transform * Affine3A::from(object.transform);

        match &object.data {
            LayerEntryData::BgPart(bg_part) => {
                if !bg_part.collision_asset_path.value.is_empty()
                    && bg_part.collision_type == ModelCollisionType::Replace
                {
                    let pcb = resolver
                        .parsed::<Pcb>(&bg_part.collision_asset_path.value)
                        .unwrap();

                    walk_node(
                        &pcb.root_node,
                        &child_transform,
                        context,
                        tiles,
                        max_slope,
                        walkable_climb,
                    );
                }
            }
            LayerEntryData::CollisionBox(collision_box) => {
                if !collision_box.parent_data.enabled {
                    continue;
                }

                if !collision_box.collision_asset_path.value.is_empty() {
                    let pcb = resolver
                        .parsed::<Pcb>(&collision_box.collision_asset_path.value)
                        .unwrap();

                    walk_node(
                        &pcb.root_node,
                        &child_transform,
                        context,
                        tiles,
                        max_slope,
                        walkable_climb,
                    );
                } else {
                    match collision_box.parent_data.trigger_box_shape {
                        TriggerBoxShape::None => unreachable!(),
                        TriggerBoxShape::Box => {
                            // TODO: It might be nice to have this as a helper somewhere?
                            let pcb = Pcb::new_from_vertices(
                                &[
                                    [-1.0, -1.0, 1.0],
                                    [1.0, -1.0, 1.0],
                                    [-1.0, 1.0, 1.0],
                                    [1.0, 1.0, 1.0],
                                    [-1.0, -1.0, -1.0],
                                    [1.0, -1.0, -1.0],
                                    [-1.0, 1.0, -1.0],
                                    [1.0, 1.0, -1.0],
                                ],
                                &[
                                    Polygon {
                                        vertex_indices: [2, 6, 7],
                                        material: 0,
                                    },
                                    Polygon {
                                        vertex_indices: [2, 3, 7],
                                        material: 0,
                                    },
                                    Polygon {
                                        vertex_indices: [0, 4, 5],
                                        material: 0,
                                    },
                                    Polygon {
                                        vertex_indices: [0, 1, 5],
                                        material: 0,
                                    },
                                    Polygon {
                                        vertex_indices: [0, 2, 6],
                                        material: 0,
                                    },
                                    Polygon {
                                        vertex_indices: [0, 4, 6],
                                        material: 0,
                                    },
                                    Polygon {
                                        vertex_indices: [1, 3, 7],
                                        material: 0,
                                    },
                                    Polygon {
                                        vertex_indices: [1, 5, 7],
                                        material: 0,
                                    },
                                    Polygon {
                                        vertex_indices: [0, 2, 3],
                                        material: 0,
                                    },
                                    Polygon {
                                        vertex_indices: [0, 1, 3],
                                        material: 0,
                                    },
                                    Polygon {
                                        vertex_indices: [4, 6, 7],
                                        material: 0,
                                    },
                                    Polygon {
                                        vertex_indices: [4, 5, 7],
                                        material: 0,
                                    },
                                ],
                            );

                            walk_node(
                                &pcb.root_node,
                                &child_transform,
                                context,
                                tiles,
                                max_slope,
                                walkable_climb,
                            );
                        }
                        TriggerBoxShape::Sphere => {
                            tracing::warn!("Sphere collision is not yet supported!")
                        }
                        TriggerBoxShape::Cylinder => {
                            tracing::warn!("Cylinder collision is not yet supported!")
                        }
                        TriggerBoxShape::Plane => {
                            tracing::warn!("Plane collision is not yet supported!")
                        }
                        TriggerBoxShape::Mesh => unreachable!(),
                        TriggerBoxShape::PlaneTwoSided => {
                            tracing::warn!("Plane two-sided collision is not yet supported!")
                        }
                    }
                }
            }
            LayerEntryData::SharedGroup(sgb) => {
                match resolver.parsed::<Sgb>(&sgb.asset_path.value) {
                    Ok(sgb) => {
                        for group in &sgb.sections[0].layer_groups {
                            for layer in &group.layers {
                                walk_layer(
                                    resolver,
                                    layer,
                                    &child_transform,
                                    context,
                                    tiles,
                                    max_slope,
                                    walkable_climb,
                                );
                            }
                        }
                    }
                    Err(err) => {
                        tracing::warn!("Failed to load sgb: {} {err:?}", sgb.asset_path.value);
                    }
                }
            }
            _ => {}
        }
    }
}

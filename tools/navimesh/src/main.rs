use std::{
    path::PathBuf,
    ptr::{null, null_mut},
};

use icarus::RecastNavimesh::RecastNavimeshSheet;
use icarus::TerritoryType::TerritoryTypeSheet;
use kawari::config::get_config;
use kawari_world::{Navmesh, NavmeshParams, NavmeshTile};
use physis::{
    Language,
    layer::{Layer, LayerEntryData, ModelCollisionType, Transformation},
    lgb::Lgb,
    lvb::Lvb,
    pcb::{Pcb, ResourceNode},
    pcblist::{PcbList, PcbListEntry},
    resource::{ResourceResolver, SqPackResource},
    sgb::Sgb,
};
use recastnavigation_sys::{
    CreateContext, dtCreateNavMeshData, dtNavMeshCreateParams, rcAllocCompactHeightfield,
    rcAllocContourSet, rcAllocHeightfield, rcAllocPolyMesh, rcAllocPolyMeshDetail,
    rcBuildCompactHeightfield, rcBuildContours, rcBuildContoursFlags_RC_CONTOUR_TESS_WALL_EDGES,
    rcBuildDistanceField, rcBuildPolyMesh, rcBuildPolyMeshDetail, rcBuildRegions, rcCalcGridSize,
    rcContext, rcCreateHeightfield, rcErodeWalkableArea, rcHeightfield, rcMarkWalkableTriangles,
    rcRasterizeTriangles,
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
    let bg_path = row.Bg();
    let name = row.Name();

    let path = format!("bg/{}.lvb", &bg_path);
    let lvb = resolver.parsed::<Lvb>(&path).unwrap();

    let navimesh_sheet = RecastNavimeshSheet::read_from(&mut resolver, Language::None).unwrap();
    // Use default settings unless overriden
    let mut navimesh_row = navimesh_sheet
        .row(0)
        .expect("No default row in RecastNavimesh sheet?");
    for (_, row) in navimesh_sheet.into_iter().flatten_subrows() {
        // FIXME: Will be called Name in the future.
        if row.Unknown0() == name {
            tracing::info!("Using navimesh settings override for this zone!");
            navimesh_row = row;
        }
    }

    let context;
    unsafe {
        context = CreateContext(true);
    }

    let cell_size = navimesh_row.CellSize();
    let cell_height = navimesh_row.CellHeight();

    let tile_width = navimesh_row.TileSize();
    let tile_height = tile_width;

    let tile_origin_x = -(tile_width * 2.0);
    let tile_origin_y = -(tile_height * 2.0);

    let twcs = tile_width / cell_size;
    let thcs = tile_width / cell_size;

    let mut tiles = Vec::new();
    for z in 0..4 {
        for x in 0..4 {
            // Step 1: Create a heightfield
            unsafe {
                let mut size_x: i32 = 0;
                let mut size_z: i32 = 0;

                let min_bounds = [
                    tile_origin_x + ((x as f32) * twcs),
                    -100.0,
                    tile_origin_y + ((z as f32) * thcs),
                ];
                let max_bounds = [
                    tile_origin_x + ((x as f32 + 1.0) * twcs),
                    100.0,
                    tile_origin_y + ((z as f32 + 1.0) * thcs),
                ];

                rcCalcGridSize(
                    min_bounds.as_ptr(),
                    max_bounds.as_ptr(),
                    cell_size,
                    &mut size_x,
                    &mut size_z,
                );

                let height_field = rcAllocHeightfield();
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

                tiles.push(Tile {
                    x,
                    z,
                    height_field,
                    min_bounds,
                    max_bounds,
                });
            }
        }
    }

    // TODO: the tiles are incredibly inefficient, we loop through each tile and try to rasterize triangles even if they aren't even in said tile
    // while this is "fine" because recast will filter out useless triangles, it wastes so much time

    let scene = &lvb.sections[0];

    let pcblist = resolver
        .parsed::<PcbList>(&format!(
            "{}/collision/list.pcb",
            scene.general.bg_path.value
        ))
        .unwrap();
    let max_slope = navimesh_row.AgentMaxSlope();
    let walkable_climb = navimesh_row.AgentMaxClimb();
    for entry in pcblist.entries {
        add_plate(
            &entry,
            &scene.general.bg_path.value,
            &mut resolver,
            context,
            &tiles,
            max_slope,
            (walkable_climb / cell_size).floor() as i32, // In VX units
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

                let transform = Transformation {
                    translation: [0.0; 3],
                    rotation: [0.0; 3],
                    scale: [1.0; 3],
                };
                walk_layer(
                    &mut resolver,
                    layer,
                    &transform,
                    context,
                    &tiles,
                    max_slope,
                    (walkable_climb / cell_size).floor() as i32, // In VX units
                );
            }
        }
    }

    let mut navmesh_tiles = Vec::new();
    let mut max_polys = 0;

    for tile in tiles {
        unsafe {
            // Step 3: Build a compact heightfield out of the normal heightfield
            let compact_heightfield = rcAllocCompactHeightfield();
            let walkable_height = navimesh_row.AgentHeight();
            let walkable_radius = navimesh_row.AgentRadius();
            assert!(!tile.height_field.is_null());
            assert!(rcBuildCompactHeightfield(
                context,
                (walkable_height / cell_height).ceil() as i32, // In VX units
                (walkable_climb / cell_size).floor() as i32,   // In VX units
                tile.height_field,
                compact_heightfield
            ));
            if (*compact_heightfield).spanCount == 0 {
                continue;
            }
            assert!((*compact_heightfield).spanCount > 0);

            assert!(rcErodeWalkableArea(
                context,
                (walkable_radius / cell_size).ceil() as i32, // In VX units
                compact_heightfield
            ));

            assert!(rcBuildDistanceField(context, compact_heightfield));

            let border_size = 0;
            let min_region_area = navimesh_row.RegionMinSize();
            let merge_region_area = navimesh_row.RegionMergedSize();
            assert!(rcBuildRegions(
                context,
                compact_heightfield,
                border_size,
                (min_region_area / cell_size).ceil() as i32, // In VX units
                (merge_region_area / cell_size).ceil() as i32, // In VX units
            ));

            // Step 4: Build the contour set from the compact heightfield
            let contour_set = rcAllocContourSet();
            let max_error = navimesh_row.MaxEdgeError();
            let max_edge_len = (navimesh_row.MaxEdgeLength() / cell_size) as i32; // in VX units
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
                tracing::warn!("Failed to build contours for a tile, for some reason?");
                continue;
            }
            assert!((*contour_set).nconts > 0);

            // Step 5: Build the polymesh out of the contour set
            let poly_mesh = rcAllocPolyMesh();
            let nvp = navimesh_row.VertsPerPoly() as i32;
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
            let sample_dist = navimesh_row.DetailMeshSampleDistance();
            let sample_max_error = navimesh_row.DetailMeshMaxSampleError();
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
                bmin: tile.min_bounds,
                bmax: tile.max_bounds,

                // General Configuration Attributes
                walkableHeight: walkable_height,
                walkableRadius: walkable_radius,
                walkableClimb: walkable_climb,
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
            orig: [tile_origin_x, 0.0, tile_origin_y],
            tile_width: tile_width / cell_size,
            tile_height: tile_height / cell_height,
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
    min_bounds: [f32; 3],
    max_bounds: [f32; 3],
}

/// Walk each node, add it's collision model to the scene.
fn walk_node(
    node: &ResourceNode,
    transform: &Transformation,
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
        // TODO: compute an actual transformation matrix, we need rotation/scale since porting from Bevy
        let mut tile_vertices: Vec<[f32; 3]> = Vec::new();
        for vertex in &node.vertices {
            tile_vertices.push([
                vertex[0] + transform.translation[0],
                vertex[1] + transform.translation[1],
                vertex[2] + transform.translation[2],
            ]);
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
    let transform = Transformation {
        translation: [0.0; 3],
        rotation: [0.0; 3],
        scale: [1.0; 3],
    };
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
    transform: &Transformation,
    context: *mut rcContext,
    tiles: &[Tile],
    max_slope: f32,
    walkable_climb: i32,
) {
    for object in &layer.objects {
        // FIXME: WRONG WRONG WRONG
        let child_transform = Transformation {
            translation: [
                transform.translation[0] + object.transform.translation[0],
                transform.translation[1] + object.transform.translation[1],
                transform.translation[2] + object.transform.translation[2],
            ],
            rotation: object.transform.rotation,
            scale: object.transform.scale,
        };

        if let LayerEntryData::BG(bg) = &object.data
            && !bg.collision_asset_path.value.is_empty()
        {
            // NOTE: assert is here to find out the unknown
            assert!(bg.collision_type == ModelCollisionType::Replace);

            let pcb = resolver
                .parsed::<Pcb>(&bg.collision_asset_path.value)
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

        if let LayerEntryData::SharedGroup(sgb) = &object.data {
            if let Ok(sgb) = resolver.parsed::<Sgb>(&sgb.asset_path.value) {
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
            } else {
                tracing::warn!("Failed to load sgb: {}", sgb.asset_path.value);
            }
        }
    }
}

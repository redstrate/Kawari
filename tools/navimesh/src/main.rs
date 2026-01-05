use std::{
    path::PathBuf,
    ptr::{null, null_mut},
};

use icarus::TerritoryType::TerritoryTypeSheet;
use kawari::config::get_config;
use kawari_world::{Navmesh, NavmeshParams, NavmeshTile};
use physis::{
    common::Language,
    layer::{LayerEntryData, LayerGroup, ModelCollisionType, Transformation},
    lvb::Lvb,
    model::MDL,
    pcb::{Pcb, ResourceNode},
    resource::{ResourceResolver, SqPackResource},
    tera::{Plate, Terrain},
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
    let Some(row) = sheet.get_row(zone_id as u32) else {
        tracing::error!("Invalid zone id {zone_id}!");
        return;
    };

    // e.g. ffxiv/fst_f1/fld/f1f3/level/f1f3
    let bg_path = row.Bg().into_string().unwrap();

    let path = format!("bg/{}.lvb", &bg_path);
    let lvb = resolver.parsed::<Lvb>(&path).unwrap();

    let context;
    unsafe {
        context = CreateContext(true);
    }

    let cell_size = 0.25;
    let cell_height = 0.25;

    let tile_origin_x = -512.0;
    let tile_origin_y = -512.0;

    let tile_width = 256.0;
    let tile_height = 256.0;

    let mut tiles = Vec::new();
    for z in 0..4 {
        for x in 0..4 {
            // Step 1: Create a heightfield
            unsafe {
                let mut size_x: i32 = 0;
                let mut size_z: i32 = 0;

                let min_bounds = [
                    tile_origin_x + (x as f32 * tile_width),
                    -100.0,
                    tile_origin_y + (z as f32 * tile_height),
                ];
                let max_bounds = [
                    tile_origin_x + ((x as f32 + 1.0) * tile_width),
                    100.0,
                    tile_origin_y + ((z as f32 + 1.0) * tile_height),
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
                    min_bounds,
                    height_field,
                });
            }
        }
    }

    // TODO: the tiles are incredibly inefficient, we loop through each tile and try to rasterize triangles even if they aren't even in said tile
    // while this is "fine" because recast will filter out useless triangles, it wastes so much time

    let scene = &lvb.sections[0];

    let tera = resolver
        .parsed::<Terrain>(&format!(
            "{}/bgplate/terrain.tera",
            scene.general.bg_path.value
        ))
        .unwrap();
    for (i, plate) in tera.plates.iter().enumerate() {
        add_plate(
            &tera,
            (i, plate),
            &scene.general.bg_path.value,
            &mut resolver,
            context,
            &tiles,
        );
    }

    for path in &scene.lgb_paths {
        if path.contains("bg.lgb") {
            tracing::info!("Processing {path}...");

            let lgb = resolver.parsed::<LayerGroup>(path);
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
                    for object in &layer.objects {
                        if let LayerEntryData::BG(bg) = &object.data
                            && !bg.collision_asset_path.value.is_empty()
                        {
                            // NOTE: assert is here to find out the unknown
                            assert!(bg.collision_type == ModelCollisionType::Replace);

                            let pcb = resolver
                                .parsed::<Pcb>(&bg.collision_asset_path.value)
                                .unwrap();

                            walk_node(&pcb.root_node, &object.transform, context, &tiles);
                        }
                    }
                }
            }
        }
    }

    let mut navmesh_tiles = Vec::new();
    let mut max_polys = 0;

    for tile in tiles {
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
                tile.height_field,
                compact_heightfield
            ));
            if (*compact_heightfield).spanCount == 0 {
                continue;
            }
            assert!((*compact_heightfield).spanCount > 0);

            assert!(rcErodeWalkableArea(
                context,
                walkable_radius as i32,
                compact_heightfield
            ));

            assert!(rcBuildDistanceField(context, compact_heightfield));

            let border_size = 0;
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
            if (*contour_set).nconts <= 0 {
                tracing::warn!("Failed to build contours for a tile, for some reason?");
                continue;
            }
            assert!((*contour_set).nconts > 0);

            // Step 5: Build the polymesh out of the contour set
            let poly_mesh = rcAllocPolyMesh();
            let nvp = 6;
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
                tileX: ((tile_origin_x - tile.min_bounds[0]) / tile_width).abs() as i32,
                tileY: ((tile_origin_y - tile.min_bounds[2]) / tile_height).abs() as i32,
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
            assert!(!out_data.is_null());
            assert!(out_data_size > 0);

            navmesh_tiles.push(NavmeshTile {
                data: Vec::from_raw_parts(out_data, out_data_size as usize, out_data_size as usize),
            });
            max_polys = max_polys.max((*poly_mesh).npolys);
        }
    }

    let navmesh = Navmesh::new(
        zone_id,
        NavmeshParams {
            orig: [tile_origin_x, 0.0, tile_origin_y],
            tile_width,
            tile_height,
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
    min_bounds: [f32; 3],
    height_field: *mut rcHeightfield,
}

/// Walk each node, add it's collision model to the scene.
fn walk_node(
    node: &ResourceNode,
    transform: &Transformation,
    context: *mut rcContext,
    tiles: &[Tile],
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
        let mut tri_area_ids: Vec<u8> = vec![0; tile_indices.len() / 3];

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
                let ntris = tile_indices.len() as i32 / 3;

                // mark areas as walkable
                rcMarkWalkableTriangles(
                    context,
                    45.0,
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
                    2
                ));
            }
        }
    }

    for child in &node.children {
        walk_node(child, transform, context, tiles);
    }
}

fn add_plate(
    terrain: &Terrain,
    (plate_index, plate): (usize, &Plate),
    tera_path: &str,
    sqpack_resource: &mut ResourceResolver,
    context: *mut rcContext,
    tiles: &[Tile],
) {
    let mdl_path = format!(
        "{}/bgplate/{}",
        tera_path,
        Terrain::mdl_filename(plate_index)
    );
    let mdl = sqpack_resource.parsed::<MDL>(&mdl_path).unwrap();

    let lod = &mdl.lods[0];
    for part in &lod.parts {
        // Step 2: insert geoemtry into heightfield
        let tile_indices: Vec<i32> = part.indices.iter().map(|x| *x as i32).collect();
        let mut tri_area_ids: Vec<u8> = vec![0; tile_indices.len() / 3];

        // transform the vertices on the CPU
        let mut tile_vertices: Vec<[f32; 3]> = Vec::new();
        for vertex in &part.vertices {
            tile_vertices.push([
                vertex.position[0] + terrain.plate_position(plate)[0],
                vertex.position[1],
                vertex.position[2] + terrain.plate_position(plate)[1],
            ]);
        }

        for tile in tiles {
            unsafe {
                let ntris = tile_indices.len() as i32 / 3;

                // mark areas as walkable
                rcMarkWalkableTriangles(
                    context,
                    45.0,
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
                    2
                ));
            }
        }
    }
}

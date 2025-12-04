use std::{io::Cursor, ptr::null_mut};

use binrw::{BinRead, BinWrite, binrw};
use recastnavigation_sys::{
    DT_SUCCESS, dtAllocNavMesh, dtAllocNavMeshQuery, dtNavMesh, dtNavMesh_addTile, dtNavMesh_init,
    dtNavMeshParams, dtNavMeshQuery, dtNavMeshQuery_findNearestPoly, dtNavMeshQuery_findPath,
    dtNavMeshQuery_findStraightPath, dtNavMeshQuery_init, dtPolyRef, dtQueryFilter,
    dtQueryFilter_dtQueryFilter,
};

#[binrw]
#[brw(little)]
#[derive(Default, Debug, Clone)]
pub struct NavmeshParams {
    pub orig: [f32; 3],
    pub tile_width: f32,
    pub tile_height: f32,
    pub max_tiles: i32,
    pub max_polys: i32,
}

/// Represents a navmesh tile for a zone.
#[binrw]
#[brw(little)]
#[derive(Default, Debug, Clone)]
pub struct NavmeshTile {
    #[br(temp)]
    #[bw(calc = data.len() as u32)]
    data_size: u32,
    #[br(count = data_size)]
    pub data: Vec<u8>,
}

/// Represents a navmesh for a zone.
/// NOTE: We reuse the .nvm file extension used by the retail server. These have no relations to ours.
#[binrw]
#[brw(little)]
#[derive(Default, Debug, Clone)]
pub struct Navmesh {
    /// The zone ID that the navmesh was generated with. Not guaranteed to be valid, it's only here for informative purposes in tools.
    pub zone_id: u16,
    nav_mesh_params: NavmeshParams,
    #[br(temp)]
    #[bw(calc = tiles.len() as u32)]
    tile_count: u32,
    #[br(count = tile_count)]
    tiles: Vec<NavmeshTile>,

    #[bw(ignore)]
    #[br(default)]
    pub navmesh: *mut dtNavMesh,
    #[bw(ignore)]
    #[br(default)]
    navmesh_query: *mut dtNavMeshQuery,
}

// To send the pointers between threads.
unsafe impl Send for Navmesh {}
unsafe impl Sync for Navmesh {}

impl Navmesh {
    /// Creates a new Navmesh.
    pub fn new(zone_id: u16, nav_mesh_params: NavmeshParams, tiles: Vec<NavmeshTile>) -> Self {
        let mut navmesh = Navmesh {
            zone_id,
            nav_mesh_params,
            tiles,
            navmesh: null_mut(),
            navmesh_query: null_mut(),
        };
        navmesh.initialize();
        navmesh
    }

    /// Reads an existing NVM file.
    pub fn from_existing(buffer: &[u8]) -> Option<Self> {
        let mut cursor = Cursor::new(buffer);
        if let Ok(mut navmesh) = Self::read(&mut cursor) {
            navmesh.initialize();
            return Some(navmesh);
        }

        None
    }

    /// Writes to the NVM file format.
    pub fn write_to_buffer(&self) -> Option<Vec<u8>> {
        let mut buffer = Vec::new();

        {
            let mut cursor = Cursor::new(&mut buffer);
            self.write_le(&mut cursor).ok()?;
        }

        Some(buffer)
    }

    /// Initializes Detour data.
    fn initialize(&mut self) {
        let navmesh_params = dtNavMeshParams {
            orig: self.nav_mesh_params.orig,
            tileWidth: self.nav_mesh_params.tile_width,
            tileHeight: self.nav_mesh_params.tile_height,
            maxTiles: self.nav_mesh_params.max_tiles,
            maxPolys: self.nav_mesh_params.max_polys,
        };

        unsafe {
            self.navmesh = dtAllocNavMesh();
            assert!(dtNavMesh_init(self.navmesh, &navmesh_params) == DT_SUCCESS);

            for tile in &mut self.tiles {
                assert!(
                    dtNavMesh_addTile(
                        self.navmesh,
                        tile.data.as_mut_ptr(),
                        tile.data.len() as i32,
                        0,
                        0,
                        null_mut()
                    ) == DT_SUCCESS
                );
            }

            self.navmesh_query = dtAllocNavMeshQuery();
            assert!(!self.navmesh_query.is_null());
            assert!(dtNavMeshQuery_init(self.navmesh_query, self.navmesh, 2048) == DT_SUCCESS);
        }

        // We can clear the tiles, we don't need them anymore.
        self.tiles.clear();
    }

    pub fn calculate_path(&self, start_pos: [f32; 3], end_pos: [f32; 3]) -> Vec<[f32; 3]> {
        unsafe {
            let mut filter = dtQueryFilter {
                m_areaCost: [1.0; 64],
                m_includeFlags: 0xffff,
                m_excludeFlags: 0,
            };
            dtQueryFilter_dtQueryFilter(&mut filter);

            let (start_poly, start_poly_pos) =
                Self::get_polygon_at_location(self.navmesh_query, start_pos, &filter);
            let (end_poly, end_poly_pos) =
                Self::get_polygon_at_location(self.navmesh_query, end_pos, &filter);

            let mut path = [0; 128];
            let mut path_count = 0;
            dtNavMeshQuery_findPath(
                self.navmesh_query,
                start_poly,
                end_poly,
                start_poly_pos.as_ptr(),
                end_poly_pos.as_ptr(),
                &filter,
                path.as_mut_ptr(),
                &mut path_count,
                128,
            ); // TODO: error check

            let mut straight_path = [0.0; 128 * 3];
            let mut straight_path_count = 0;

            // now calculate the positions in the path
            dtNavMeshQuery_findStraightPath(
                self.navmesh_query,
                start_poly_pos.as_ptr(),
                end_poly_pos.as_ptr(),
                path.as_ptr(),
                path_count,
                straight_path.as_mut_ptr(),
                null_mut(),
                null_mut(),
                &mut straight_path_count,
                128,
                0,
            );

            let mut path = Vec::new();
            for pos in straight_path[..straight_path_count as usize * 3].chunks(3) {
                path.push([pos[0], pos[1], pos[2]]);
            }

            path
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

            (nearest_ref, nearest_pt)
        }
    }

    // TODO: There might be more checks desired here, but this suffices for now
    pub fn is_available(&self) -> bool {
        !self.navmesh.is_null()
    }
}

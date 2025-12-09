use kawari::common::ObjectId;

/// Allocates a pool up to `MAX_SIZE`, and can reserve and free indices based on an `ObjectId`.
/// Used to control spawning on the client-side, which needs its spawn indexes manually handled.
///
/// If `START_INDEX` is specified, the allocator will reserve the specified number of indices permanently.
/// This is mostly used for actor spawning, where the player always takes index 0.
///
/// Due to Rust limitations, the `MAX_SIZE` has to exclude the `START_INDEX` by the callee.
/// For example, if you have 100 objects and the first object index is reserved then `MAX_SIZE` should be 99 and `START_INDEX` should be 1.
#[derive(Debug, Clone)]
pub struct SpawnAllocator<const MAX_SIZE: usize, const START_INDEX: usize = 0> {
    pool: [Option<ObjectId>; MAX_SIZE],
}

impl<const MAX_SIZE: usize, const START_INDEX: usize> Default
    for SpawnAllocator<{ MAX_SIZE }, { START_INDEX }>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<const MAX_SIZE: usize, const START_INDEX: usize>
    SpawnAllocator<{ MAX_SIZE }, { START_INDEX }>
{
    pub fn new() -> Self {
        Self {
            pool: [None; MAX_SIZE],
        }
    }

    /// Attempts to reserve a new spawn index, if there isn't a free space returns `None`.
    pub fn reserve(&mut self, object_id: ObjectId) -> Option<u8> {
        for (i, space) in self.pool.iter_mut().enumerate() {
            if space.is_none() {
                *space = Some(object_id);
                return Some(i as u8 + START_INDEX as u8);
            }
        }

        None
    }

    /// Frees the given object from the pool, if it wasn't in the pool returns `None`.
    pub fn free(&mut self, object_id: ObjectId) -> Option<u8> {
        for (i, space) in self.pool.iter_mut().enumerate() {
            if *space == Some(object_id) {
                *space = None;
                return Some(i as u8 + START_INDEX as u8);
            }
        }

        None
    }

    /// Checks if the object exists in the pool.
    pub fn contains(&self, object_id: ObjectId) -> bool {
        for space in &self.pool {
            if *space == Some(object_id) {
                return true;
            }
        }

        false
    }

    /// Frees all objects from the pool.
    pub fn clear(&mut self) {
        self.pool = [None; MAX_SIZE];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate() {
        let mut allocator = SpawnAllocator::<2>::new(); // Can only hold two objects
        assert_eq!(allocator.reserve(ObjectId(0)), Some(0));
        assert_eq!(allocator.reserve(ObjectId(1)), Some(1));
        assert_eq!(allocator.reserve(ObjectId(2)), None); // Reserving a 3rd object should fail

        assert_eq!(allocator.contains(ObjectId(1)), true);
        assert_eq!(allocator.contains(ObjectId(2)), false);

        // Removing the last spot should free it up
        allocator.free(ObjectId(1));
        assert_eq!(allocator.reserve(ObjectId(2)), Some(1));

        // Clearing the pool should allow us to access everything again:
        allocator.clear();
        assert_eq!(allocator.reserve(ObjectId(0)), Some(0));
        assert_eq!(allocator.reserve(ObjectId(1)), Some(1));
        assert_eq!(allocator.reserve(ObjectId(2)), None); // Reserving a 3rd object should fail still
    }

    #[test]
    fn test_starting_index_allocate() {
        let mut allocator = SpawnAllocator::<2, 1>::new();
        assert_eq!(allocator.reserve(ObjectId(0)), Some(1));
        assert_eq!(allocator.reserve(ObjectId(1)), Some(2));
        assert_eq!(allocator.reserve(ObjectId(2)), None);
    }
}

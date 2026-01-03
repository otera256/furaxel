use std::ops::Range;

use bevy::math::UVec3;
use block_mesh::ndshape::Shape;
use itertools::iproduct;
use crate::voxel_world::core::voxel::Voxel;

#[derive(Debug, Clone)]
pub struct Chunk<S: Shape<3, Coord = u32>> {
    pub voxels: Box<[Voxel]>,
    pub shape: S,
}

#[allow(dead_code)]
impl<S: Shape<3, Coord = u32>> Chunk<S> {
    pub fn new_empty(shape: S) -> Self {
        let voxels = vec![Voxel::EMPTY; shape.usize()].into_boxed_slice();
        Self { voxels, shape }
    }
    pub fn new_filled(shape: S, voxel: Voxel) -> Self {
        let voxels = vec![voxel; shape.usize()].into_boxed_slice();
        Self { voxels, shape }
    }

    pub fn new_from_fn<F>(shape: S, mut f: F) -> Self
    where
        F: FnMut(u32, u32, u32) -> Voxel,
    {
        let mut voxels = Vec::with_capacity(shape.usize());
        for (z, y, x) in iproduct!(0..shape.as_array()[2], 0..shape.as_array()[1], 0..shape.as_array()[0]) {
            voxels.push(f(x, y, z));
        }
        Self {
            voxels: voxels.into_boxed_slice(),
            shape,
        }
    }

    #[inline]
    pub fn get_at(&self, pos: UVec3) -> Voxel {
        self.voxels[self.shape.linearize(pos.to_array()) as usize]
    }
    #[inline]
    pub fn get_at_mut(&mut self, pos: UVec3) -> &mut Voxel {
        &mut self.voxels[self.shape.linearize(pos.to_array()) as usize]
    }

    pub fn as_slice(&self) -> &[Voxel] {
        &self.voxels
    }
    pub fn get_range(&self, range_x: Range<u32>, range_y: Range<u32>, range_z: Range<u32>) -> Vec<Voxel> {
        let mut result = Vec::new();
        for (z, y, x) in iproduct!(range_z, range_y, range_x) {
            let voxel = self.get_at(UVec3::new(x, y, z));
            result.push(voxel);
        }
        result
    }
}

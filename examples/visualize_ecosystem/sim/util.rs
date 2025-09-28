use macroquad::prelude::Vec2;
use std::ops::Range;
use crate::sensing;
use crate::params::*;

pub fn dir_from_theta(theta: f32) -> Vec2 { Vec2 { x: theta.cos(), y: theta.sin() } }
// sample_inputs replaced by build_inputs in Phase 3

pub fn grid_index(p: Vec2) -> u32 {
    let nx = (WORLD_W / EXPL_CELL_SIZE).ceil() as u32;
    let ix = (p.x / EXPL_CELL_SIZE).floor().clamp(0.0, nx as f32 - 1.0) as u32;
    let iy = (p.y / EXPL_CELL_SIZE).floor().clamp(0.0, (WORLD_H / EXPL_CELL_SIZE).ceil() as f32 - 1.0) as u32;
    iy * nx + ix
}

pub struct InputRanges {
    pub vision: Range<usize>,
    pub energy: usize,
    pub memory: Range<usize>,
    pub density: Range<usize>,
    pub hearing: Range<usize>,
    pub position: Range<usize>,
}

pub fn input_ranges() -> InputRanges {
    let r= sensing::input_ranges();
    InputRanges { 
        vision: r.vision, 
        energy: r.energy, 
        memory: r.memory, 
        density: r.density, 
        hearing: r.hearing, 
        position: r.position 
    }
}

pub fn mask_inputs(inputs: &mut [f32; INPUTS]) {
    // Zero out disabled modality ranges while keeping the input length/layout stable.
    let r = sensing::input_ranges();
    if !ENABLE_VISION_INPUTS { for i in r.vision { inputs[i] = 0.0; } }
    if !ENABLE_MEMORY_INPUTS { for i in r.memory { inputs[i] = 0.0; } }
    if !ENABLE_DENSITY_INPUTS { for i in r.density { inputs[i] = 0.0; } }
    if !ENABLE_HEARING_INPUTS { for i in r.hearing { inputs[i] = 0.0; } }
}

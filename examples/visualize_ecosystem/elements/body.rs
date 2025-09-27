use crate::glam::Vec2;

#[derive(Clone, Debug)]
pub struct Body {
    pub pos: Vec2,
    pub vel: Vec2,
    pub radius: f32,
}

impl Body{
    pub fn collides(a: &Body, b: &Body) -> bool {
        let delta = b.pos - a.pos;
        let dist2 = delta.length_squared();
        let r = a.radius + b.radius;
        dist2 < r * r
    }
}

pub fn resolve_collision(a: &mut Body, b: &mut Body) {
    let delta = b.pos - a.pos;
    let dist = delta.length();
    if dist == 0.0 { return; }

    let overlap = (a.radius + b.radius) - dist;
    if overlap > 0.0 {
        let correction = delta / dist * (overlap / 2.0);
        a.pos -= correction;
        b.pos += correction;
    }
}

//pub fn handle_agent_food(agent: &mut Body, food: &Body) -> bool {
//    if collides(agent, food) {
//        true
//    } else {
//        false
//    }
//}


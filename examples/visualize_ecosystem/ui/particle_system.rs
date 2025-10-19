use macroquad::prelude::*;

pub struct Particle {
    pos: Vec2,
    vel: Vec2,
    size: f32,
    alpha: f32,
}

impl Particle {
    pub fn new() -> Self {
        Self {
            pos: vec2(
                rand::gen_range(0.0, screen_width()),
                rand::gen_range(0.0, screen_height()),
            ),
            vel: vec2(rand::gen_range(-0.2, 0.2), rand::gen_range(-0.1, 0.1)),
            size: rand::gen_range(1.0, 3.0),
            alpha: rand::gen_range(0.05, 0.2),
        }
    }

    pub fn update(&mut self, mouse_pos: Vec2, pull_target: Option<Vec2>, pull_strength: f32) {
        // Apply velocity
        self.pos += self.vel;

        // Pull effect toward target
        if let Some(target) = pull_target {
            let dir_to_target = target - self.pos;
            let dist = dir_to_target.length();
            if dist > 1.0 {
                let dir = dir_to_target / dist;
                self.vel += dir * pull_strength;
            }
        }

        // Mouse attraction
        let dir_to_mouse = mouse_pos - self.pos;
        let dist = dir_to_mouse.length();
        let interaction_range = 120.0;
        
        if dist < interaction_range {
            let dir = dir_to_mouse / dist.max(1.0);
            let strength = 0.04 * (1.0 - dist / interaction_range);
            self.vel += dir * strength;
        }

        // Damping
        self.vel *= 0.97;

        // Screen wraparound
        if self.pos.x < 0.0 {
            self.pos.x = screen_width();
        }
        if self.pos.x > screen_width() {
            self.pos.x = 0.0;
        }
        if self.pos.y < 0.0 {
            self.pos.y = screen_height();
        }
        if self.pos.y > screen_height() {
            self.pos.y = 0.0;
        }

        // Twinkle effect
        self.alpha = 0.1 + 0.1 * ((get_time() as f32 * 0.8 + self.pos.x * 0.01).sin().abs());
    }

    pub fn draw(&self) {
        // Draw glow
        draw_circle(
            self.pos.x,
            self.pos.y,
            self.size * 2.0,
            Color::new(0.6, 0.8, 1.0, self.alpha * 0.1),
        );
        
        // Draw core
        draw_circle(
            self.pos.x,
            self.pos.y,
            self.size,
            Color::new(1.0, 1.0, 1.0, self.alpha),
        );
    }
}

pub struct ParticleSystem {
    particles: Vec<Particle>,
    pull_active: bool,
    pull_target: Vec2,
    pull_timer: f32,
}

impl ParticleSystem {
    pub fn new(count: usize) -> Self {
        let particles = (0..count).map(|_| Particle::new()).collect();
        
        Self {
            particles,
            pull_active: false,
            pull_target: Vec2::ZERO,
            pull_timer: 0.0,
        }
    }

    pub fn activate_pull(&mut self, target: Vec2, duration: f32) {
        self.pull_target = target;
        self.pull_active = true;
        self.pull_timer = duration;
    }

    pub fn update(&mut self) {
        let mouse_pos = {
            let (mx, my) = mouse_position();
            vec2(mx, my)
        };

        // Update pull timer
        if self.pull_timer > 0.0 {
            self.pull_timer -= get_frame_time();
            if self.pull_timer <= 0.0 {
                self.pull_active = false;
            }
        }

        // Calculate pull strength
        let pull_strength = if self.pull_active && self.pull_timer > 0.0 {
            0.15 * (self.pull_timer / 0.5).min(1.0)
        } else {
            0.0
        };

        let pull_target = if self.pull_active {
            Some(self.pull_target)
        } else {
            None
        };

        // Update all particles
        for particle in &mut self.particles {
            particle.update(mouse_pos, pull_target, pull_strength);
        }
    }

    pub fn draw(&self) {
        for particle in &self.particles {
            particle.draw();
        }
    }

    pub fn reset(&mut self, count: usize) {
        self.particles = (0..count).map(|_| Particle::new()).collect();
        self.pull_active = false;
        self.pull_timer = 0.0;
    }

}
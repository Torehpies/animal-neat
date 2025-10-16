use super::ui_menu::{MenuState, SimConfig, draw_menu};
use macroquad::prelude::*;
mod load_picker;

pub enum MenuResult {
    New(SimConfig),
    Load(String),
    Exit,
}
struct Particle {
    pos: Vec2,
    vel: Vec2,
    size: f32,
    alpha: f32,
}

pub async fn run_main_menu() -> MenuResult {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum MainScreen {
        MainMenu,
        SimOptions,
    }

    let mut screen = MainScreen::MainMenu;
    let mut click_cooldown = 0.0f32;

    let mut particles: Vec<Particle> = (0..100)
        .map(|_| Particle {
            pos: vec2(
                rand::gen_range(0.0, screen_width()),
                rand::gen_range(0.0, screen_height()),
            ),
            vel: vec2(rand::gen_range(-0.2, 0.2), rand::gen_range(-0.1, 0.1)),
            size: rand::gen_range(1.0, 3.0),
            alpha: rand::gen_range(0.05, 0.2),
        })
        .collect();

    let mut particle_pull_active = false;
    let mut pull_target = vec2(0.0, 0.0);
    let mut pull_timer = 0.0f32;

    fn update_and_draw_particles(
        particles: &mut Vec<Particle>,
        particle_pull_active: &mut bool,
        pull_target: &mut Vec2,
        pull_timer: &mut f32,
    ) {
        for p in particles.iter_mut() {
            p.pos += p.vel;

            // Pull effect
            if *particle_pull_active && *pull_timer > 0.0 {
                let dir_to_target = *pull_target - p.pos;
                let dist = dir_to_target.length();
                if dist > 1.0 {
                    let dir = dir_to_target / dist;
                    let strength = 0.15 * (*pull_timer / 0.5).min(1.0);
                    p.vel += dir * strength;
                }
            }

            if *pull_timer > 0.0 {
                *pull_timer -= get_frame_time();
                if *pull_timer <= 0.0 {
                    *particle_pull_active = false;
                }
            }

            // Mouse attraction
            let (mx, my) = mouse_position();
            let dir_to_mouse = vec2(mx - p.pos.x, my - p.pos.y);
            let dist = dir_to_mouse.length();
            if dist < 120.0 {
                let dir = dir_to_mouse / dist.max(1.0);
                let strength = 0.04 * (1.0 - dist / 120.0);
                p.vel += dir * strength;
            }

            // Damping + wrapping
            p.vel *= 0.97;
            if p.pos.x < 0.0 {
                p.pos.x = screen_width();
            }
            if p.pos.x > screen_width() {
                p.pos.x = 0.0;
            }
            if p.pos.y < 0.0 {
                p.pos.y = screen_height();
            }
            if p.pos.y > screen_height() {
                p.pos.y = 0.0;
            }

            // Alpha twinkle + draw
            p.alpha = 0.1 + 0.1 * ((get_time() as f32 * 0.8 + p.pos.x * 0.01).sin().abs());
            draw_circle(
                p.pos.x,
                p.pos.y,
                p.size * 2.0,
                Color::new(0.6, 0.8, 1.0, p.alpha * 0.1),
            );
            draw_circle(p.pos.x, p.pos.y, p.size, Color::new(1.0, 1.0, 1.0, p.alpha));
        }
    }

    loop {
        match screen {
            MainScreen::MainMenu => {
                clear_background(Color::new(0.05, 0.06, 0.10, 1.0));
                update_and_draw_particles(
                    &mut particles,
                    &mut particle_pull_active,
                    &mut pull_target,
                    &mut pull_timer,
                );
                let w = screen_width();
                let h = screen_height();
                let (mx, my) = mouse_position();

                // 🌈 Gentle parallax effect based on mouse movement
                //let parallax_x = (mx / w - 0.5) * 10.0;
                //let parallax_y = (my / h - 0.5) * 10.0;

                // --- Animated background particles ---
                for p in &mut particles {
                    // Move particle
                    p.pos += p.vel;

                    // --- Particle pull effect ---
                    if particle_pull_active && pull_timer > 0.0 {
                        let dir_to_target = pull_target - p.pos;
                        let dist = dir_to_target.length();
                        if dist > 1.0 {
                            let dir = dir_to_target / dist;
                            // Attraction strength decreases as the timer runs out
                            let strength = 0.15 * (pull_timer / 0.5).min(1.0);
                            p.vel += dir * strength;
                        }
                    }

                    if pull_timer > 0.0 {
                        pull_timer -= get_frame_time();
                        if pull_timer <= 0.0 {
                            particle_pull_active = false;
                        }
                    }

                    // --- Mouse interaction ---
                    let (mx, my) = mouse_position();
                    let dir_to_mouse = vec2(mx - p.pos.x, my - p.pos.y);
                    let dist = dir_to_mouse.length();

                    // Only apply attraction if within range (e.g., 120 pixels)
                    let interaction_range = 120.0;
                    if dist < interaction_range {
                        let dir = dir_to_mouse / dist.max(1.0); // prevent divide-by-zero
                        // Strength decreases with distance
                        let strength = 0.04 * (1.0 - dist / interaction_range);
                        p.vel += dir * strength;
                    }

                    // Dampen velocity to prevent chaos
                    p.vel *= 0.97;

                    // --- Screen wraparound ---
                    if p.pos.x < 0.0 {
                        p.pos.x = screen_width();
                    }
                    if p.pos.x > screen_width() {
                        p.pos.x = 0.0;
                    }
                    if p.pos.y < 0.0 {
                        p.pos.y = screen_height();
                    }
                    if p.pos.y > screen_height() {
                        p.pos.y = 0.0;
                    }

                    // --- Twinkle alpha modulation ---
                    p.alpha = 0.1 + 0.1 * ((get_time() as f32 * 0.8 + p.pos.x * 0.01).sin().abs());

                    // --- Draw glow and core ---
                    draw_circle(
                        p.pos.x,
                        p.pos.y,
                        p.size * 2.0,
                        Color::new(0.6, 0.8, 1.0, p.alpha * 0.1),
                    );
                    draw_circle(p.pos.x, p.pos.y, p.size, Color::new(1.0, 1.0, 1.0, p.alpha));
                }

                // Title (centered with glow)
                let title = "NEAT (Animal Ecosystem Simulation using Neuroevolution)";
                let title_size = 46.0;
                let title_width = measure_text(title, None, title_size as u16, 1.0).width;
                let title_x = w * 0.5 - title_width * 0.5;
                let title_y = h * 0.22;
                draw_text(
                    title,
                    title_x + 2.0,
                    title_y + 2.0,
                    title_size,
                    Color::new(0.0, 0.0, 0.0, 0.5),
                );
                draw_text(
                    title,
                    title_x,
                    title_y,
                    title_size,
                    Color::new(0.8, 0.9, 1.0, 1.0),
                );

                // Button parameters
                let btn_w = 340.0;
                let btn_h = 70.0;
                let bx = w * 0.5 - btn_w * 0.5;
                let mut by = h * 0.45;
                let spacing = 24.0;

                // Function for centered button text
                let draw_centered_text =
                    |text: &str, bx: f32, by: f32, bw: f32, bh: f32, size: f32, color: Color| {
                        let tw = measure_text(text, None, size as u16, 1.0).width;
                        let th = measure_text(text, None, size as u16, 1.0).height;
                        draw_text(
                            text,
                            bx + (bw - tw) * 0.5,
                            by + (bh + th) * 0.55, // vertical optical centering
                            size,
                            color,
                        );
                    };

                // --- Simulate button ---
                let simulate_hover = mx >= bx && mx <= bx + btn_w && my >= by && my <= by + btn_h;
                let simulate_color = if simulate_hover {
                    Color::new(0.25, 0.7, 0.35, 1.0)
                } else {
                    Color::new(0.2, 0.55, 0.25, 1.0)
                };
                draw_rectangle(
                    bx + 3.0,
                    by + 3.0,
                    btn_w,
                    btn_h,
                    Color::new(0.0, 0.0, 0.0, 0.25),
                ); // shadow
                draw_rectangle(bx, by, btn_w, btn_h, simulate_color);
                draw_rectangle_lines(bx, by, btn_w, btn_h, 2.0, WHITE);
                draw_centered_text("Simulate", bx, by, btn_w, btn_h, 30.0, WHITE);

                // --- Exit button ---
                by += btn_h + spacing;
                let exit_hover = mx >= bx && mx <= bx + btn_w && my >= by && my <= by + btn_h;
                let exit_color = if exit_hover {
                    Color::new(0.75, 0.25, 0.25, 1.0)
                } else {
                    Color::new(0.55, 0.20, 0.20, 1.0)
                };
                draw_rectangle(
                    bx + 3.0,
                    by + 3.0,
                    btn_w,
                    btn_h,
                    Color::new(0.0, 0.0, 0.0, 0.25),
                );
                draw_rectangle(bx, by, btn_w, btn_h, exit_color);
                draw_rectangle_lines(bx, by, btn_w, btn_h, 2.0, WHITE);
                draw_centered_text("Exit", bx, by, btn_w, btn_h, 30.0, WHITE);

                // --- Click handling ---
                if is_mouse_button_pressed(MouseButton::Left) && click_cooldown <= 0.0 {
                    if simulate_hover || exit_hover {
                        // Figure out which button was clicked
                        let target_center = if simulate_hover {
                            vec2(bx + btn_w / 2.0, by - spacing - btn_h / 2.0) // Simulate button
                        } else {
                            vec2(bx + btn_w / 2.0, by + btn_h / 2.0) // Exit button
                        };

                        // Activate particle pull toward clicked button
                        pull_target = target_center;
                        particle_pull_active = true;
                        pull_timer = 0.5;
                        //click_cooldown = 0.20;

                        // Duration for the pull + fade effect
                        let effect_duration = 0.8;
                        let start_time = get_time();

                        // Fade-out and particle pull animation loop
                        while get_time() - start_time < effect_duration {
                            let t = ((get_time() - start_time) / effect_duration).clamp(0.0, 1.0)
                                as f32;

                            // Re-render full scene (particles, title, buttons)
                            clear_background(Color::new(0.05, 0.06, 0.10, 1.0));

                            // --- Update and draw particles ---
                            for p in &mut particles {
                                p.pos += p.vel;
                                if particle_pull_active {
                                    let dir = pull_target - p.pos;
                                    let dist = dir.length();
                                    if dist > 1.0 {
                                        let dir_n = dir / dist;
                                        let strength = 0.15 * (1.0 - t); // gradually weaken
                                        p.vel += dir_n * strength;
                                    }
                                }
                                p.vel *= 0.97;
                                if p.pos.x < 0.0 {
                                    p.pos.x = screen_width();
                                }
                                if p.pos.x > screen_width() {
                                    p.pos.x = 0.0;
                                }
                                if p.pos.y < 0.0 {
                                    p.pos.y = screen_height();
                                }
                                if p.pos.y > screen_height() {
                                    p.pos.y = 0.0;
                                }

                                draw_circle(
                                    p.pos.x,
                                    p.pos.y,
                                    p.size * 2.0,
                                    Color::new(0.6, 0.8, 1.0, p.alpha * 0.1),
                                );
                                draw_circle(
                                    p.pos.x,
                                    p.pos.y,
                                    p.size,
                                    Color::new(1.0, 1.0, 1.0, p.alpha),
                                );
                            }

                            // --- Redraw title ---
                            let title = "NEAT (Animal Ecosystem Simulation using Neuroevolution)";
                            let title_size = 46.0;
                            let title_width =
                                measure_text(title, None, title_size as u16, 1.0).width;
                            let title_x = w * 0.5 - title_width * 0.5;
                            let title_y = h * 0.22;
                            draw_text(
                                title,
                                title_x + 2.0,
                                title_y + 2.0,
                                title_size,
                                Color::new(0.0, 0.0, 0.0, 0.5),
                            );
                            draw_text(
                                title,
                                title_x,
                                title_y,
                                title_size,
                                Color::new(0.8, 0.9, 1.0, 1.0),
                            );

                            // --- Redraw buttons ---
                            // Simulate
                            draw_rectangle(
                                bx + 3.0,
                                h * 0.45 + 3.0,
                                btn_w,
                                btn_h,
                                Color::new(0.0, 0.0, 0.0, 0.25),
                            );
                            draw_rectangle(
                                bx,
                                h * 0.45,
                                btn_w,
                                btn_h,
                                Color::new(0.2, 0.55, 0.25, 1.0),
                            );
                            draw_rectangle_lines(bx, h * 0.45, btn_w, btn_h, 2.0, WHITE);
                            let text_y = h * 0.45;
                            let tw = measure_text("Simulate", None, 30, 1.0).width;
                            draw_text(
                                "Simulate",
                                bx + (btn_w - tw) * 0.5,
                                text_y + btn_h * 0.65,
                                30.0,
                                WHITE,
                            );

                            // Exit
                            let exit_y = h * 0.45 + btn_h + spacing;
                            draw_rectangle(
                                bx + 3.0,
                                exit_y + 3.0,
                                btn_w,
                                btn_h,
                                Color::new(0.0, 0.0, 0.0, 0.25),
                            );
                            draw_rectangle(
                                bx,
                                exit_y,
                                btn_w,
                                btn_h,
                                Color::new(0.55, 0.20, 0.20, 1.0),
                            );
                            draw_rectangle_lines(bx, exit_y, btn_w, btn_h, 2.0, WHITE);
                            let tw_exit = measure_text("Exit", None, 30, 1.0).width;
                            draw_text(
                                "Exit",
                                bx + (btn_w - tw_exit) * 0.5,
                                exit_y + btn_h * 0.65,
                                30.0,
                                WHITE,
                            );

                            // --- Smooth fade overlay ---
                            draw_rectangle(
                                0.0,
                                0.0,
                                screen_width(),
                                screen_height(),
                                Color::new(0.0, 0.0, 0.0, t * 0.8),
                            );

                            next_frame().await;
                        }

                        // --- Switch screens after fade ---
                        if simulate_hover {
                            screen = MainScreen::SimOptions;
                        } else {
                            return MenuResult::Exit;
                        }

                        click_cooldown = 0.15;
                    }
                }

                // Cooldown decrement
                if click_cooldown > 0.0 {
                    click_cooldown = (click_cooldown - get_frame_time()).max(0.0);
                }

                next_frame().await;
            }

            MainScreen::SimOptions => {
                clear_background(Color::new(0.05, 0.06, 0.10, 1.0));
                update_and_draw_particles(
                    &mut particles,
                    &mut particle_pull_active,
                    &mut pull_target,
                    &mut pull_timer,
                );
                let w = screen_width();
                let h = screen_height();
                let (mx, my) = mouse_position();

                // Title
                let title = "Simulation Options";
                let title_size = 44.0;
                let tw = measure_text(title, None, title_size as u16, 1.0).width;
                let tx = w * 0.5 - tw * 0.5;
                draw_text(
                    title,
                    tx + 2.0,
                    h * 0.18 + 2.0,
                    title_size,
                    Color::new(0.0, 0.0, 0.0, 0.5),
                );
                draw_text(
                    title,
                    tx,
                    h * 0.18,
                    title_size,
                    Color::new(0.8, 0.9, 1.0, 1.0),
                );

                // Button setup
                let btn_w = 420.0;
                let btn_h = 72.0;
                let bx = w * 0.5 - btn_w * 0.5;
                let mut by = h * 0.40;
                let spacing = 22.0;

                let draw_centered_text =
                    |text: &str, bx: f32, by: f32, bw: f32, bh: f32, size: f32, color: Color| {
                        let tw = measure_text(text, None, size as u16, 1.0).width;
                        let th = measure_text(text, None, size as u16, 1.0).height;
                        draw_text(
                            text,
                            bx + (bw - tw) * 0.5,
                            by + (bh + th) * 0.55,
                            size,
                            color,
                        );
                    };

                // --- Create New Simulation ---
                let create_hover = mx >= bx && mx <= bx + btn_w && my >= by && my <= by + btn_h;
                let create_color = if create_hover {
                    Color::new(0.25, 0.7, 0.7, 1.0)
                } else {
                    Color::new(0.2, 0.55, 0.55, 1.0)
                };
                draw_rectangle(
                    bx + 3.0,
                    by + 3.0,
                    btn_w,
                    btn_h,
                    Color::new(0.0, 0.0, 0.0, 0.25),
                );
                draw_rectangle(bx, by, btn_w, btn_h, create_color);
                draw_rectangle_lines(bx, by, btn_w, btn_h, 2.0, WHITE);
                draw_centered_text("Create New Simulation", bx, by, btn_w, btn_h, 28.0, WHITE);

                // --- Load ---
                by += btn_h + spacing;
                let load_hover = mx >= bx && mx <= bx + btn_w && my >= by && my <= by + btn_h;
                let load_color = if load_hover {
                    Color::new(0.7, 0.65, 0.3, 1.0)
                } else {
                    Color::new(0.55, 0.5, 0.25, 1.0)
                };
                draw_rectangle(
                    bx + 3.0,
                    by + 3.0,
                    btn_w,
                    btn_h,
                    Color::new(0.0, 0.0, 0.0, 0.25),
                );
                draw_rectangle(bx, by, btn_w, btn_h, load_color);
                draw_rectangle_lines(bx, by, btn_w, btn_h, 2.0, WHITE);
                draw_centered_text("Load Saved Simulation", bx, by, btn_w, btn_h, 28.0, WHITE);

                // --- Back ---
                by += btn_h + 32.0;
                let back_w = 180.0;
                let back_h = 50.0;
                let back_x = w * 0.5 - back_w * 0.5;
                let back_hover =
                    mx >= back_x && mx <= back_x + back_w && my >= by && my <= by + back_h;
                let back_color = if back_hover {
                    Color::new(0.45, 0.45, 0.45, 1.0)
                } else {
                    Color::new(0.35, 0.35, 0.35, 1.0)
                };
                draw_rectangle(
                    back_x + 3.0,
                    by + 3.0,
                    back_w,
                    back_h,
                    Color::new(0.0, 0.0, 0.0, 0.25),
                );
                draw_rectangle(back_x, by, back_w, back_h, back_color);
                draw_rectangle_lines(back_x, by, back_w, back_h, 2.0, WHITE);
                draw_centered_text("Back", back_x, by, back_w, back_h, 24.0, WHITE);

                // Click handling
                if is_mouse_button_pressed(MouseButton::Left) && click_cooldown <= 0.0 {
                    if create_hover {
                        let mut menu_state = MenuState::new();
                        let config = loop {
                            if let Some(cfg) = draw_menu(&mut menu_state) {
                                break cfg;
                            }
                            next_frame().await;
                        };
                        return MenuResult::New(config);
                    }

                    if load_hover {
                        //click_cooldown = 0.20;
                        // Wait for cooldown
                        let cooldown_start = get_time();
                        while get_time() - cooldown_start < 0.20 {
                            next_frame().await;
                        }

                        if let Some(path) = load_picker::pick_snapshot().await {
                            return MenuResult::Load(path);
                        }
                        // Reset cooldown after picker closes
                        click_cooldown = 0.15;
                    }

                    if back_hover {
                        //click_cooldown = 0.20;
                        let cooldown_start = get_time();
                        while get_time() - cooldown_start < 0.20 {
                            next_frame().await;
                        }

                        // ⬇️ Reset particles when going back
                        particles = (0..100)
                            .map(|_| Particle {
                                pos: vec2(
                                    rand::gen_range(0.0, screen_width()),
                                    rand::gen_range(0.0, screen_height()),
                                ),
                                vel: vec2(rand::gen_range(-0.2, 0.2), rand::gen_range(-0.1, 0.1)),
                                size: rand::gen_range(1.0, 3.0),
                                alpha: rand::gen_range(0.05, 0.2),
                            })
                            .collect();

                        particle_pull_active = false;
                        pull_timer = 0.0;

                        screen = MainScreen::MainMenu;
                        click_cooldown = 0.15;
                    }
                }

                // Cooldown decrement
                if click_cooldown > 0.0 {
                    click_cooldown = (click_cooldown - get_frame_time()).max(0.0);
                }

                next_frame().await;
            }
        }
    }
}

use super::ui_menu::{MenuState, SimConfig, draw_menu};
use macroquad::prelude::*;
mod load_picker;
pub mod particle_system;

use particle_system::ParticleSystem;

pub enum MenuResult {
    New(SimConfig),
    Load(String),
    Exit,
}

// UI Drawing Helpers
fn draw_centered_text(text: &str, x: f32, y: f32, w: f32, h: f32, size: f32, color: Color) {
    let tw = measure_text(text, None, size as u16, 1.0).width;
    let th = measure_text(text, None, size as u16, 1.0).height;
    draw_text(text, x + (w - tw) * 0.5, y + (h + th) * 0.55, size, color);
}

fn draw_button(x: f32, y: f32, w: f32, h: f32, text: &str, color: Color, font_size: f32) {
    draw_rectangle(x + 3.0, y + 3.0, w, h, Color::new(0.0, 0.0, 0.0, 0.25));
    draw_rectangle(x, y, w, h, color);
    draw_rectangle_lines(x, y, w, h, 2.0, WHITE);
    draw_centered_text(text, x, y, w, h, font_size, WHITE);
}

fn draw_title(title: &str, size: f32, x: f32, y: f32) {
    draw_text(
        title,
        x + 2.0,
        y + 2.0,
        size,
        Color::new(0.0, 0.0, 0.0, 0.5),
    );
    draw_text(title, x, y, size, Color::new(0.8, 0.9, 1.0, 1.0));
}

pub async fn run_main_menu() -> MenuResult {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum MainScreen {
        MainMenu,
        SimOptions,
    }

    let mut screen = MainScreen::MainMenu;
    let mut click_cooldown = 0.0;
    let mut particle_system = ParticleSystem::new(100);

    loop {
        match screen {
            MainScreen::MainMenu => {
                clear_background(Color::new(0.05, 0.06, 0.10, 1.0));

                particle_system.update();
                particle_system.draw();

                let w = screen_width();
                let h = screen_height();
                let (mx, my) = mouse_position();

                // Title
                let title = "NEAT (Animal Ecosystem Simulation using Neuroevolution)";
                let title_size = 46.0;
                let title_width = measure_text(title, None, title_size as u16, 1.0).width;
                let title_x = w * 0.5 - title_width * 0.5;
                let title_y = h * 0.22;
                draw_title(title, title_size, title_x, title_y);

                // Button parameters
                let btn_w = 340.0;
                let btn_h = 70.0;
                let bx = w * 0.5 - btn_w * 0.5;
                let by_simulate = h * 0.45;
                let by_exit = by_simulate + btn_h + 24.0;

                // Hover detection
                let simulate_hover =
                    mx >= bx && mx <= bx + btn_w && my >= by_simulate && my <= by_simulate + btn_h;
                let exit_hover =
                    mx >= bx && mx <= bx + btn_w && my >= by_exit && my <= by_exit + btn_h;

                // Draw buttons
                let simulate_color = if simulate_hover {
                    Color::new(0.25, 0.7, 0.35, 1.0)
                } else {
                    Color::new(0.2, 0.55, 0.25, 1.0)
                };
                draw_button(
                    bx,
                    by_simulate,
                    btn_w,
                    btn_h,
                    "Simulate",
                    simulate_color,
                    30.0,
                );

                let exit_color = if exit_hover {
                    Color::new(0.75, 0.25, 0.25, 1.0)
                } else {
                    Color::new(0.55, 0.20, 0.20, 1.0)
                };
                draw_button(bx, by_exit, btn_w, btn_h, "Exit", exit_color, 30.0);

                // Click handling
                if is_mouse_button_pressed(MouseButton::Left) && click_cooldown <= 0.0 {
                    if simulate_hover || exit_hover {
                        let target_center = if simulate_hover {
                            vec2(bx + btn_w / 2.0, by_simulate + btn_h / 2.0)
                        } else {
                            vec2(bx + btn_w / 2.0, by_exit + btn_h / 2.0)
                        };

                        particle_system.activate_pull(target_center, 0.5);
                        click_cooldown = 0.20;

                        let effect_duration = 0.8;
                        let start_time = get_time();

                        while get_time() - start_time < effect_duration {
                            let t = ((get_time() - start_time) / effect_duration).clamp(0.0, 1.0)
                                as f32;

                            clear_background(Color::new(0.05, 0.06, 0.10, 1.0));

                            particle_system.update();
                            particle_system.draw();

                            draw_title(title, title_size, title_x, title_y);
                            draw_button(
                                bx,
                                by_simulate,
                                btn_w,
                                btn_h,
                                "Simulate",
                                Color::new(0.2, 0.55, 0.25, 1.0),
                                30.0,
                            );
                            draw_button(
                                bx,
                                by_exit,
                                btn_w,
                                btn_h,
                                "Exit",
                                Color::new(0.55, 0.20, 0.20, 1.0),
                                30.0,
                            );

                            draw_rectangle(
                                0.0,
                                0.0,
                                screen_width(),
                                screen_height(),
                                Color::new(0.0, 0.0, 0.0, t * 0.8),
                            );

                            next_frame().await;
                        }

                        if simulate_hover {
                            screen = MainScreen::SimOptions;
                        } else {
                            return MenuResult::Exit;
                        }

                        click_cooldown = 0.15;
                    }
                }

                if click_cooldown > 0.0 {
                    click_cooldown = (click_cooldown - get_frame_time()).max(0.0);
                }

                next_frame().await;
            }

            MainScreen::SimOptions => {
                clear_background(Color::new(0.05, 0.06, 0.10, 1.0));

                particle_system.update();
                particle_system.draw();

                let w = screen_width();
                let h = screen_height();
                let (mx, my) = mouse_position();

                // Title
                let title = "Simulation Options";
                let title_size = 44.0;
                let tw = measure_text(title, None, title_size as u16, 1.0).width;
                let tx = w * 0.5 - tw * 0.5;
                draw_title(title, title_size, tx, h * 0.18);

                // Button setup
                let btn_w = 420.0;
                let btn_h = 72.0;
                let bx = w * 0.5 - btn_w * 0.5;
                let by_create = h * 0.40;
                let by_load = by_create + btn_h + 22.0;
                let by_back = by_load + btn_h + 32.0;

                // Hover detection
                let create_hover =
                    mx >= bx && mx <= bx + btn_w && my >= by_create && my <= by_create + btn_h;
                let load_hover =
                    mx >= bx && mx <= bx + btn_w && my >= by_load && my <= by_load + btn_h;

                let back_w = 180.0;
                let back_h = 50.0;
                let back_x = w * 0.5 - back_w * 0.5;
                let back_hover = mx >= back_x
                    && mx <= back_x + back_w
                    && my >= by_back
                    && my <= by_back + back_h;

                // Draw buttons
                let create_color = if create_hover {
                    Color::new(0.25, 0.7, 0.7, 1.0)
                } else {
                    Color::new(0.2, 0.55, 0.55, 1.0)
                };
                draw_button(
                    bx,
                    by_create,
                    btn_w,
                    btn_h,
                    "Create New Simulation",
                    create_color,
                    28.0,
                );

                let load_color = if load_hover {
                    Color::new(0.7, 0.65, 0.3, 1.0)
                } else {
                    Color::new(0.55, 0.5, 0.25, 1.0)
                };
                draw_button(
                    bx,
                    by_load,
                    btn_w,
                    btn_h,
                    "Load Saved Simulation",
                    load_color,
                    28.0,
                );

                let back_color = if back_hover {
                    Color::new(0.45, 0.45, 0.45, 1.0)
                } else {
                    Color::new(0.35, 0.35, 0.35, 1.0)
                };
                draw_button(back_x, by_back, back_w, back_h, "Back", back_color, 24.0);

                // Click handling
                if is_mouse_button_pressed(MouseButton::Left) && click_cooldown <= 0.0 {
                    if create_hover {
                        let target_center = vec2(bx + btn_w / 2.0, by_create + btn_h / 2.0);
                        particle_system.activate_pull(target_center, 0.5);
                        click_cooldown = 0.20;

                        let effect_duration = 0.8;
                        let start_time = get_time();

                        while get_time() - start_time < effect_duration {
                            let t = ((get_time() - start_time) / effect_duration).clamp(0.0, 1.0)
                                as f32;

                            clear_background(Color::new(0.05, 0.06, 0.10, 1.0));

                            particle_system.update();
                            particle_system.draw();

                            draw_title(title, title_size, tx, h * 0.18);
                            draw_button(
                                bx,
                                by_create,
                                btn_w,
                                btn_h,
                                "Create New Simulation",
                                Color::new(0.2, 0.55, 0.55, 1.0),
                                28.0,
                            );
                            draw_button(
                                bx,
                                by_load,
                                btn_w,
                                btn_h,
                                "Load Saved Simulation",
                                Color::new(0.55, 0.5, 0.25, 1.0),
                                28.0,
                            );
                            draw_button(
                                back_x,
                                by_back,
                                back_w,
                                back_h,
                                "Back",
                                Color::new(0.35, 0.35, 0.35, 1.0),
                                24.0,
                            );

                            draw_rectangle(
                                0.0,
                                0.0,
                                screen_width(),
                                screen_height(),
                                Color::new(0.0, 0.0, 0.0, t * 0.8),
                            );

                            next_frame().await;
                        }

                        let mut menu_state = MenuState::new();
                        loop {
                            if let Some(result) = draw_menu(&mut menu_state).await {
                                match result {
                                    crate::ui_menu::MenuResult::Config(cfg) => {
                                        return MenuResult::New(cfg);
                                    }
                                    crate::ui_menu::MenuResult::Back => {
                                        break;
                                    }
                                }
                            }
                            next_frame().await;
                        }
                        particle_system.reset(100);
                        click_cooldown = 0.15;
                    }

                    if load_hover {
                        let target_center = vec2(bx + btn_w / 2.0, by_load + btn_h / 2.0);
                        particle_system.activate_pull(target_center, 0.5);
                        click_cooldown = 0.20;

                        let effect_duration = 0.8;
                        let start_time = get_time();

                        while get_time() - start_time < effect_duration {
                            let t = ((get_time() - start_time) / effect_duration).clamp(0.0, 1.0)
                                as f32;

                            clear_background(Color::new(0.05, 0.06, 0.10, 1.0));

                            particle_system.update();
                            particle_system.draw();

                            draw_title(title, title_size, tx, h * 0.18);
                            draw_button(
                                bx,
                                by_create,
                                btn_w,
                                btn_h,
                                "Create New Simulation",
                                Color::new(0.2, 0.55, 0.55, 1.0),
                                28.0,
                            );
                            draw_button(
                                bx,
                                by_load,
                                btn_w,
                                btn_h,
                                "Load Saved Simulation",
                                Color::new(0.55, 0.5, 0.25, 1.0),
                                28.0,
                            );
                            draw_button(
                                back_x,
                                by_back,
                                back_w,
                                back_h,
                                "Back",
                                Color::new(0.35, 0.35, 0.35, 1.0),
                                24.0,
                            );

                            draw_rectangle(
                                0.0,
                                0.0,
                                screen_width(),
                                screen_height(),
                                Color::new(0.0, 0.0, 0.0, t * 0.8),
                            );

                            next_frame().await;
                        }

                        if let Some(path) = load_picker::pick_snapshot().await {
                            return MenuResult::Load(path);
                        }
                        particle_system.reset(100);
                        click_cooldown = 0.15;
                    }

                    if back_hover {
                        let target_center = vec2(back_x + back_w / 2.0, by_back + back_h / 2.0);
                        particle_system.activate_pull(target_center, 0.5);
                        click_cooldown = 0.20;

                        let effect_duration = 0.8;
                        let start_time = get_time();

                        while get_time() - start_time < effect_duration {
                            let t = ((get_time() - start_time) / effect_duration).clamp(0.0, 1.0)
                                as f32;

                            clear_background(Color::new(0.05, 0.06, 0.10, 1.0));

                            particle_system.update();
                            particle_system.draw();

                            draw_title(title, title_size, tx, h * 0.18);
                            draw_button(
                                bx,
                                by_create,
                                btn_w,
                                btn_h,
                                "Create New Simulation",
                                Color::new(0.2, 0.55, 0.55, 1.0),
                                28.0,
                            );
                            draw_button(
                                bx,
                                by_load,
                                btn_w,
                                btn_h,
                                "Load Saved Simulation",
                                Color::new(0.55, 0.5, 0.25, 1.0),
                                28.0,
                            );
                            draw_button(
                                back_x,
                                by_back,
                                back_w,
                                back_h,
                                "Back",
                                Color::new(0.35, 0.35, 0.35, 1.0),
                                24.0,
                            );

                            draw_rectangle(
                                0.0,
                                0.0,
                                screen_width(),
                                screen_height(),
                                Color::new(0.0, 0.0, 0.0, t * 0.8),
                            );

                            next_frame().await;
                        }

                        particle_system.reset(100);
                        screen = MainScreen::MainMenu;
                        click_cooldown = 0.15;
                        continue;
                    }
                }

                if click_cooldown > 0.0 {
                    click_cooldown = (click_cooldown - get_frame_time()).max(0.0);
                }

                next_frame().await;
            }
        }
    }
}
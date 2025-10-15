use macroquad::prelude::*;
use super::ui_menu::{SimConfig, MenuState, draw_menu};
mod load_picker;

pub enum MenuResult {
    New(SimConfig),
    Load(String),
    Exit,
}

pub async fn run_main_menu() -> MenuResult {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum MainScreen {
        MainMenu,
        SimOptions,
    }

    let mut screen = MainScreen::MainMenu;

    loop {
        match screen {
            MainScreen::MainMenu => {
                clear_background(Color::new(0.05, 0.06, 0.10, 1.0));
                let w = screen_width();
                let h = screen_height();
                let (mx, my) = mouse_position();

                // Title (centered with glow)
                let title = "NEAT (Animal Ecosystem Simulation using Neuroevolution)";
                let title_size = 46.0;
                let title_width = measure_text(title, None, title_size as u16, 1.0).width;
                let title_x = w * 0.5 - title_width * 0.5;
                let title_y = h * 0.22;
                draw_text(title, title_x + 2.0, title_y + 2.0, title_size, Color::new(0.0, 0.0, 0.0, 0.5));
                draw_text(title, title_x, title_y, title_size, Color::new(0.8, 0.9, 1.0, 1.0));

                // Button parameters
                let btn_w = 340.0;
                let btn_h = 70.0;
                let bx = w * 0.5 - btn_w * 0.5;
                let mut by = h * 0.45;
                let spacing = 24.0;

                // Function for centered button text
                let draw_centered_text = |text: &str, bx: f32, by: f32, bw: f32, bh: f32, size: f32, color: Color| {
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
                draw_rectangle(bx + 3.0, by + 3.0, btn_w, btn_h, Color::new(0.0, 0.0, 0.0, 0.25)); // shadow
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
                draw_rectangle(bx + 3.0, by + 3.0, btn_w, btn_h, Color::new(0.0, 0.0, 0.0, 0.25));
                draw_rectangle(bx, by, btn_w, btn_h, exit_color);
                draw_rectangle_lines(bx, by, btn_w, btn_h, 2.0, WHITE);
                draw_centered_text("Exit", bx, by, btn_w, btn_h, 30.0, WHITE);

                // Click handling
                if is_mouse_button_pressed(MouseButton::Left) {
                    if simulate_hover {
                        screen = MainScreen::SimOptions;
                    } else if exit_hover {
                        return MenuResult::Exit;
                    }
                }

                next_frame().await;
            }

            MainScreen::SimOptions => {
                clear_background(Color::new(0.05, 0.06, 0.10, 1.0));
                let w = screen_width();
                let h = screen_height();
                let (mx, my) = mouse_position();

                // Title
                let title = "Simulation Options";
                let title_size = 44.0;
                let tw = measure_text(title, None, title_size as u16, 1.0).width;
                let tx = w * 0.5 - tw * 0.5;
                draw_text(title, tx + 2.0, h * 0.18 + 2.0, title_size, Color::new(0.0, 0.0, 0.0, 0.5));
                draw_text(title, tx, h * 0.18, title_size, Color::new(0.8, 0.9, 1.0, 1.0));

                // Button setup
                let btn_w = 420.0;
                let btn_h = 72.0;
                let bx = w * 0.5 - btn_w * 0.5;
                let mut by = h * 0.40;
                let spacing = 22.0;

                let draw_centered_text = |text: &str, bx: f32, by: f32, bw: f32, bh: f32, size: f32, color: Color| {
                    let tw = measure_text(text, None, size as u16, 1.0).width;
                    let th = measure_text(text, None, size as u16, 1.0).height;
                    draw_text(text, bx + (bw - tw) * 0.5, by + (bh + th) * 0.55, size, color);
                };

                // --- Create New Simulation ---
                let create_hover = mx >= bx && mx <= bx + btn_w && my >= by && my <= by + btn_h;
                let create_color = if create_hover {
                    Color::new(0.25, 0.7, 0.7, 1.0)
                } else {
                    Color::new(0.2, 0.55, 0.55, 1.0)
                };
                draw_rectangle(bx + 3.0, by + 3.0, btn_w, btn_h, Color::new(0.0, 0.0, 0.0, 0.25));
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
                draw_rectangle(bx + 3.0, by + 3.0, btn_w, btn_h, Color::new(0.0, 0.0, 0.0, 0.25));
                draw_rectangle(bx, by, btn_w, btn_h, load_color);
                draw_rectangle_lines(bx, by, btn_w, btn_h, 2.0, WHITE);
                draw_centered_text("Load Saved Simulation", bx, by, btn_w, btn_h, 28.0, WHITE);

                // --- Back ---
                by += btn_h + 32.0;
                let back_w = 180.0;
                let back_h = 50.0;
                let back_x = w * 0.5 - back_w * 0.5;
                let back_hover = mx >= back_x && mx <= back_x + back_w && my >= by && my <= by + back_h;
                let back_color = if back_hover {
                    Color::new(0.45, 0.45, 0.45, 1.0)
                } else {
                    Color::new(0.35, 0.35, 0.35, 1.0)
                };
                draw_rectangle(back_x + 3.0, by + 3.0, back_w, back_h, Color::new(0.0, 0.0, 0.0, 0.25));
                draw_rectangle(back_x, by, back_w, back_h, back_color);
                draw_rectangle_lines(back_x, by, back_w, back_h, 2.0, WHITE);
                draw_centered_text("Back", back_x, by, back_w, back_h, 24.0, WHITE);

                // Click handling
                if is_mouse_button_pressed(MouseButton::Left) {
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
                        if let Some(path) = load_picker::pick_snapshot().await {
                            return MenuResult::Load(path);
                        }
                    }

                    if back_hover {
                        screen = MainScreen::MainMenu;
                    }
                }

                next_frame().await;
            }
        }
    }
}

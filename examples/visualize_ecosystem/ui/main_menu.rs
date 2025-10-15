use macroquad::prelude::*;

// Sibling module `menu.rs` defines `SimConfig`, `MenuState` and `draw_menu`.
use super::ui_menu::{SimConfig, MenuState, draw_menu};
mod load_picker;

/// Run the main menu flow. Returns Some(SimConfig) when the user chooses to create a
/// new simulation (or confirms settings). Returns None if the user chose Exit.
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
                clear_background(Color::new(0.05, 0.05, 0.08, 1.0));
                let w = screen_width();
                let h = screen_height();

                // Title
                let title = "NEAT (Animal Ecosystem Simulation using Neuroevolution)";
                let title_size = 48.0;
                let title_x = w * 0.5 - (title.len() as f32 * (title_size * 0.18));
                draw_text(title, title_x, h * 0.22, title_size, WHITE);

                // Buttons
                let btn_w = 360.0;
                let btn_h = 80.0;
                let bx = w * 0.5 - btn_w * 0.5;
                let by = h * 0.45;

                // Simulate button
                draw_rectangle(bx, by, btn_w, btn_h, Color::new(0.2, 0.6, 0.25, 1.0));
                draw_rectangle_lines(bx, by, btn_w, btn_h, 2.0, BLACK);
                draw_text("Simulate", bx + 28.0, by + 50.0, 32.0, BLACK);

                // Exit button
                let by2 = by + btn_h + 20.0;
                draw_rectangle(bx, by2, btn_w, btn_h, Color::new(0.6, 0.2, 0.2, 1.0));
                draw_rectangle_lines(bx, by2, btn_w, btn_h, 2.0, BLACK);
                draw_text("Exit", bx + 28.0, by2 + 50.0, 32.0, BLACK);

                // Click handling
                if is_mouse_button_pressed(MouseButton::Left) {
                    let (mx, my) = mouse_position();
                    if mx >= bx && mx <= bx + btn_w && my >= by && my <= by + btn_h {
                        screen = MainScreen::SimOptions;
                    } else if mx >= bx && mx <= bx + btn_w && my >= by2 && my <= by2 + btn_h {
                        return MenuResult::Exit; // user chose Exit
                    }
                }

                next_frame().await;
            }

            MainScreen::SimOptions => {
                clear_background(Color::new(0.05, 0.05, 0.08, 1.0));
                let w = screen_width();
                let h = screen_height();

                draw_text("Simulation Options", w * 0.5 - 180.0, h * 0.20, 40.0, WHITE);

                let btn_w = 420.0;
                let btn_h = 72.0;
                let bx = w * 0.5 - btn_w * 0.5;
                let mut by = h * 0.40;

                // Create New Simulation
                draw_rectangle(bx, by, btn_w, btn_h, Color::new(0.2, 0.55, 0.55, 1.0));
                draw_rectangle_lines(bx, by, btn_w, btn_h, 2.0, BLACK);
                draw_text("Create New Simulation", bx + 22.0, by + 46.0, 28.0, BLACK);

                // Load (not implemented yet)
                by += btn_h + 18.0;
                draw_rectangle(bx, by, btn_w, btn_h, Color::new(0.5, 0.5, 0.2, 1.0));
                draw_rectangle_lines(bx, by, btn_w, btn_h, 2.0, BLACK);
                draw_text("Load", bx + 22.0, by + 46.0, 28.0, BLACK);

                // Back button
                by += btn_h + 28.0;
                let back_w = 160.0;
                let back_x = w * 0.5 - back_w * 0.5;
                draw_rectangle(back_x, by, back_w, 48.0, Color::new(0.4, 0.4, 0.4, 1.0));
                draw_rectangle_lines(back_x, by, back_w, 48.0, 2.0, BLACK);
                draw_text("Back", back_x + 34.0, by + 34.0, 24.0, BLACK);

                if is_mouse_button_pressed(MouseButton::Left) {
                    let (mx, my) = mouse_position();
                    // Create New
                    let create_by = h * 0.40;
                    if mx >= bx && mx <= bx + btn_w && my >= create_by && my <= create_by + btn_h {
                        // Forward to existing menu UI (re-use MenuState/draw_menu)
                        let mut menu_state = MenuState::new();
                        let config = loop {
                            if let Some(cfg) = draw_menu(&mut menu_state) {
                                break cfg;
                            }
                            next_frame().await;
                        };
                        return MenuResult::New(config);
                    }

                    // Load clicked -> show file list of snapshots
                    let load_by = h * 0.40 + btn_h + 18.0;
                    if mx >= bx && mx <= bx + btn_w && my >= load_by && my <= load_by + btn_h {
                        // Open the mouse-driven picker UI
                        if let Some(path) = load_picker::pick_snapshot().await {
                            return MenuResult::Load(path);
                        }
                    }

                    // Back
                    if mx >= back_x && mx <= back_x + back_w && my >= by && my <= by + 48.0 {
                        screen = MainScreen::MainMenu;
                    }
                }

                next_frame().await;
            }
        }
    }
}

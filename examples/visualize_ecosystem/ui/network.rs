use macroquad::prelude::*;
use neat::neat::node_gene::NodeType;
use neat::neat::genome::Genome;

pub fn draw_network_panel(area: Rect, genome: &Genome) {
    // Compute layout: inputs (left), hidden (middle), outputs (right)
    // Collect nodes by type and sort by id for stability
    let mut inputs: Vec<u32> = Vec::new();
    let mut hiddens: Vec<u32> = Vec::new();
    let mut outputs: Vec<u32> = Vec::new();
    for (id, node) in &genome.nodes {
        match node.node_type {
            NodeType::Input => inputs.push(*id),
            NodeType::Hidden => hiddens.push(*id),
            NodeType::Output => outputs.push(*id),
            NodeType::Bias => {} // not used in this setup
        }
    }
    inputs.sort_unstable();
    hiddens.sort_unstable();
    outputs.sort_unstable();

    // Positions
    let left_x = area.x + 40.0;
    let right_x = area.x + area.w - 40.0;
    let mid_x = (left_x + right_x) * 0.5;
    let top_y = area.y + 24.0;
    let bot_y = area.y + area.h - 24.0;

    let mut pos: std::collections::HashMap<u32, (f32, f32)> = std::collections::HashMap::new();
    let place_col = |ids: &Vec<u32>, x: f32, pos: &mut std::collections::HashMap<u32, (f32, f32)>| {
        let n = ids.len().max(1) as f32;
        for (i, id) in ids.iter().enumerate() {
            let t = if n <= 1.0 { 0.5 } else { i as f32 / (n - 1.0) };
            let y = top_y * (1.0 - t) + bot_y * t;
            pos.insert(*id, (x, y));
        }
    };
    place_col(&inputs, left_x, &mut pos);
    place_col(&hiddens, mid_x, &mut pos);
    place_col(&outputs, right_x, &mut pos);

    // Draw connections first
    for conn in &genome.connections {
        if !conn.enabled { continue; }
        if let (Some(&(x1, y1)), Some(&(x2, y2))) = (pos.get(&conn.in_node_id), pos.get(&conn.out_node_id)) {
            let w = (conn.weight.abs() * 2.0).clamp(1.0, 4.0);
            let col = if conn.weight >= 0.0 { Color::new(0.2, 0.9, 0.3, 0.85) } else { Color::new(0.95, 0.25, 0.25, 0.85) };
            draw_line(x1, y1, x2, y2, w, col);
        }
    }
    // Draw nodes on top
    let draw_nodes = |ids: &Vec<u32>, color: Color| {
        for id in ids {
            if let Some(&(x, y)) = pos.get(id) {
                draw_circle(x, y, 3.0, color);
                draw_circle_lines(x, y, 3.0, 1.5, BLACK);
            }
        }
    };
    draw_nodes(&inputs, Color::new(0.2, 0.6, 1.0, 1.0));
    draw_nodes(&hiddens, Color::new(0.8, 0.8, 0.85, 1.0));
    draw_nodes(&outputs, Color::new(1.0, 0.6, 0.2, 1.0));

    // Legends
    let legend_y = area.y + 16.0;
    let mut lx = area.x + 8.0;
    let legend = |lx: &mut f32, label: &str, col: Color| {
        draw_circle(*lx + 8.0, legend_y, 6.0, col);
        draw_circle_lines(*lx + 8.0, legend_y, 6.0, 1.0, BLACK);
        draw_text(label, *lx + 18.0, legend_y + 4.0, 14.0, LIGHTGRAY);
        *lx += 90.0;
    };
    legend(&mut lx, "Inputs", Color::new(0.2, 0.6, 1.0, 1.0));
    legend(&mut lx, "Hidden", Color::new(0.8, 0.8, 0.85, 1.0));
    legend(&mut lx, "Outputs", Color::new(1.0, 0.6, 0.2, 1.0));
}

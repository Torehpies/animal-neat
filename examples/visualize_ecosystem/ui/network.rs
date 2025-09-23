use macroquad::prelude::*;
use neat::neat::node_gene::NodeType;
use neat::neat::genome::Genome;

pub fn draw_network_panel(area: Rect, genome: &Genome) {
    // Compute layered layout based on topological depth:
    // depth 0 = inputs, depth max = outputs, hidden nodes placed by (1 + max depth of incoming)
    // Collect nodes by type and sort by id for stability
    let mut inputs: Vec<u32> = Vec::new();
    let mut hiddens: Vec<u32> = Vec::new();
    let mut outputs: Vec<u32> = Vec::new();
    for (id, node) in &genome.nodes {
        match node.node_type {
            NodeType::Input => inputs.push(*id),
            NodeType::Hidden => hiddens.push(*id),
            NodeType::Output => outputs.push(*id),
            NodeType::Bias => {} // not shown
        }
    }
    inputs.sort_unstable();
    hiddens.sort_unstable();
    outputs.sort_unstable();

    // Build incoming adjacency for enabled edges
    use std::collections::HashMap;
    let mut incoming: HashMap<u32, Vec<u32>> = HashMap::new();
    for conn in &genome.connections {
        if !conn.enabled { continue; }
        incoming.entry(conn.out_node_id).or_default().push(conn.in_node_id);
    }

    // Depth computation (memoized DFS on DAG)
    use std::collections::HashMap as Map;
    let mut depth: Map<u32, usize> = Map::new();
    let mut visiting: std::collections::HashSet<u32> = std::collections::HashSet::new();
    // Inputs at depth 0
    for &id in &inputs { depth.insert(id, 0); }

    fn node_depth(
        id: u32,
        incoming: &HashMap<u32, Vec<u32>>,
        depth: &mut Map<u32, usize>,
        visiting: &mut std::collections::HashSet<u32>,
    ) -> usize {
        if let Some(&d) = depth.get(&id) { return d; }
        if visiting.contains(&id) { return 0; } // guard (shouldn't happen in acyclic graphs)
        visiting.insert(id);
        let d = if let Some(ins) = incoming.get(&id) {
            let mut max_d = 0usize;
            for &u in ins {
                let dep = node_depth(u, incoming, depth, visiting);
                if dep > max_d { max_d = dep; }
            }
            max_d.saturating_add(1)
        } else { 1 };
        visiting.remove(&id);
        depth.insert(id, d);
        d
    }

    // Compute depths for hidden and outputs
    for &id in &hiddens { let _ = node_depth(id, &incoming, &mut depth, &mut visiting); }
    for &id in &outputs { let _ = node_depth(id, &incoming, &mut depth, &mut visiting); }

    // Ensure outputs sit at the rightmost layer
    let max_depth = outputs
        .iter()
        .map(|&id| *depth.get(&id).unwrap_or(&0))
        .max()
        .unwrap_or(1)
        .max(1);

    // Group nodes by depth bucket
    let mut by_layer: Vec<Vec<u32>> = vec![Vec::new(); max_depth + 1];
    for &id in &inputs { by_layer[0].push(id); }
    for &id in &hiddens { let d = (*depth.get(&id).unwrap_or(&1)).min(max_depth-1); by_layer[d].push(id); }
    for &id in &outputs { by_layer[max_depth].push(id); }
    for layer in &mut by_layer { layer.sort_unstable(); }

    // Positions across columns
    let left_x = area.x + 40.0;
    let right_x = area.x + area.w - 40.0;
    let top_y = area.y + 24.0;
    let bot_y = area.y + area.h - 24.0;

    let mut pos: std::collections::HashMap<u32, (f32, f32)> = std::collections::HashMap::new();
    for (d, ids) in by_layer.iter().enumerate() {
        if ids.is_empty() { continue; }
        let t = if max_depth == 0 { 0.0 } else { d as f32 / (max_depth as f32) };
        let x = left_x * (1.0 - t) + right_x * t;
        let n = ids.len().max(1) as f32;
        for (i, id) in ids.iter().enumerate() {
            let ty = if n <= 1.0 { 0.5 } else { i as f32 / (n - 1.0) };
            let y = top_y * (1.0 - ty) + bot_y * ty;
            pos.insert(*id, (x, y));
        }
    }

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

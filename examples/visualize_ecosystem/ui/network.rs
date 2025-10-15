use macroquad::prelude::*;
use neat::neat::node_gene::NodeType;
use crate::params::{COMMUNICATION_ENABLED, HEARING_SECTORS, ENABLE_HEARING_INPUTS};
use crate::sensing;
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

    // If communication is disabled, hide the last output node (assumes canonical order: [turn, speed, call])
    let mut hidden_outputs: std::collections::HashSet<u32> = std::collections::HashSet::new();
    if !COMMUNICATION_ENABLED && outputs.len() >= 3 {
        if let Some(&call_id) = outputs.get(2) {
            hidden_outputs.insert(call_id);
        }
    }

    // If hearing inputs are disabled (or zero sectors), hide the hearing input nodes by id range.
    // Assumes input node IDs are contiguous starting at 0 in the same order as sensing::input_ranges().
    let mut hidden_inputs: std::collections::HashSet<u32> = std::collections::HashSet::new();
    if HEARING_SECTORS == 0 || !ENABLE_HEARING_INPUTS {
        let r = sensing::input_ranges();
        if r.hearing.end > r.hearing.start {
            for id in r.hearing.start as u32..r.hearing.end as u32 {
                hidden_inputs.insert(id);
            }
        }
    }

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
        if hidden_outputs.contains(&conn.in_node_id) || hidden_outputs.contains(&conn.out_node_id) { continue; }
        if hidden_inputs.contains(&conn.in_node_id) || hidden_inputs.contains(&conn.out_node_id) { continue; }
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
    if hidden_inputs.is_empty() {
        draw_nodes(&inputs, Color::new(0.2, 0.6, 1.0, 1.0));
    } else {
        let visible_in: Vec<u32> = inputs.iter().copied().filter(|id| !hidden_inputs.contains(id)).collect();
        draw_nodes(&visible_in, Color::new(0.2, 0.6, 1.0, 1.0));
    }

    // Draw short labels for each input node
    let input_label = |idx: usize| -> String {
        let r = sensing::input_ranges();
        if r.vision.contains(&idx) {
            let i = idx - r.vision.start;
            let ray = i / 5;
            let cat = match i % 5 { 0 => "P", 1 => "C", 2 => "S", 3 => "O", _ => "W" };
            return format!("V R{}:{}", ray, cat);
        }
        if idx == r.energy { return "Energy".to_string(); }
        if r.memory.contains(&idx) {
            let j = idx - r.memory.start;
            let name = match j { 
                0 => "Mem Food.x", 1 => "Mem Food.y", 
                2 => "Mem Same.x", 3 => "Mem Same.y", 
                4 => "Mem Other.x", 5 => "Mem Other.y",
                _ => "Mem" 
            };
            return name.to_string();
        }
        if r.hearing.contains(&idx) { // May be empty when hearing disabled
            let j = idx - r.hearing.start;
            let sector = ["L", "F", "R"][j.min(2)];
            return format!("H:{}", sector);
        }
        if r.position.contains(&idx) {
            let j = idx - r.position.start;
            let name = if j == 0 { "Pos.x" } else { "Pos.y" };
            return name.to_string();
        }
        format!("In{}", idx)
    };

    let draw_input_labels = |ids: &Vec<u32>| {
        // Larger font; place the label just to the left of the node (about 10px gap)
        let fs_px: f32 = 16.0;
        let fs: u16 = 16;
        for id in ids {
            if hidden_inputs.contains(id) { continue; }
            if let Some(&(x, y)) = pos.get(id) {
                let label = input_label(*id as usize);
                let tw = measure_text(&label, None, fs, 1.0).width;
                let tx = x - 10.0 - tw; // 10px left of the circle edge
                let ty = y + 5.0; // slight vertical offset
                // Foreground
                draw_text(&label, tx, ty, fs_px, LIGHTGRAY);
            }
        }
    };
    if hidden_inputs.is_empty() { draw_input_labels(&inputs); } else {
        let visible_in: Vec<u32> = inputs.iter().copied().filter(|id| !hidden_inputs.contains(id)).collect();
        draw_input_labels(&visible_in);
    }
    draw_nodes(&hiddens, Color::new(0.8, 0.8, 0.85, 1.0));
    if hidden_outputs.is_empty() {
        draw_nodes(&outputs, Color::new(1.0, 0.6, 0.2, 1.0));
    } else {
        // Draw only visible outputs (exclude hidden call)
        let visible: Vec<u32> = outputs.iter().copied().filter(|id| !hidden_outputs.contains(id)).collect();
        draw_nodes(&visible, Color::new(1.0, 0.6, 0.2, 1.0));
    }

    // Legends
    let legend_y = area.y + 16.0;
    // Position the legend beside the input nodes column for better alignment
    let mut lx = left_x + 12.0;
    let legend = |lx: &mut f32, label: &str, col: Color| {
        draw_circle(*lx + 8.0, legend_y, 6.0, col);
        draw_circle_lines(*lx + 8.0, legend_y, 6.0, 1.0, BLACK);
        draw_text(label, *lx + 18.0, legend_y + 4.0, 14.0, LIGHTGRAY);
        *lx += 90.0;
    };
    legend(&mut lx, "Inputs", Color::new(0.2, 0.6, 1.0, 1.0));
    legend(&mut lx, "Hidden", Color::new(0.8, 0.8, 0.85, 1.0));
    let label = if COMMUNICATION_ENABLED { "Outputs (3)" } else { "Outputs (2)" };
    legend(&mut lx, label, Color::new(1.0, 0.6, 0.2, 1.0));
}

/// Draw the same network layout but color nodes by their live activation values in [-1,1].
/// Pass a map from node id to activation value (as produced by Genome::evaluate_with_activations_slice).
pub fn draw_network_panel_activations(area: Rect, genome: &Genome, activations: &std::collections::HashMap<u32, f32>) {
    // Collect nodes by type
    let mut inputs: Vec<u32> = Vec::new();
    let mut hiddens: Vec<u32> = Vec::new();
    let mut outputs: Vec<u32> = Vec::new();
    for (id, node) in &genome.nodes {
        match node.node_type {
            NodeType::Input => inputs.push(*id),
            NodeType::Hidden => hiddens.push(*id),
            NodeType::Output => outputs.push(*id),
            NodeType::Bias => {}
        }
    }
    inputs.sort_unstable(); hiddens.sort_unstable(); outputs.sort_unstable();

    // Hide optional IO
    let mut hidden_outputs: std::collections::HashSet<u32> = std::collections::HashSet::new();
    if !COMMUNICATION_ENABLED && outputs.len() >= 3 { if let Some(&call_id) = outputs.get(2) { hidden_outputs.insert(call_id); } }
    let mut hidden_inputs: std::collections::HashSet<u32> = std::collections::HashSet::new();
    if HEARING_SECTORS == 0 || !ENABLE_HEARING_INPUTS {
        let r = sensing::input_ranges();
        if r.hearing.end > r.hearing.start { for id in r.hearing.start as u32..r.hearing.end as u32 { hidden_inputs.insert(id); } }
    }

    // Incoming adjacency
    use std::collections::HashMap;
    let mut incoming: HashMap<u32, Vec<u32>> = HashMap::new();
    for conn in &genome.connections { if !conn.enabled { continue; } incoming.entry(conn.out_node_id).or_default().push(conn.in_node_id); }
    // Depth via memoized DFS
    let mut depth: HashMap<u32, usize> = HashMap::new();
    let mut visiting: std::collections::HashSet<u32> = std::collections::HashSet::new();
    for &id in &inputs { depth.insert(id, 0); }
    fn node_depth(id: u32, incoming: &HashMap<u32, Vec<u32>>, depth: &mut HashMap<u32, usize>, visiting: &mut std::collections::HashSet<u32>) -> usize {
        if let Some(&d) = depth.get(&id) { return d; }
        if visiting.contains(&id) { return 0; }
        visiting.insert(id);
        let d = if let Some(ins) = incoming.get(&id) { ins.iter().map(|u| node_depth(*u, incoming, depth, visiting)).max().unwrap_or(0) + 1 } else { 1 };
        visiting.remove(&id);
        depth.insert(id, d);
        d
    }
    for &id in &hiddens { let _ = node_depth(id, &incoming, &mut depth, &mut visiting); }
    for &id in &outputs { let _ = node_depth(id, &incoming, &mut depth, &mut visiting); }
    let max_depth = outputs.iter().map(|&id| *depth.get(&id).unwrap_or(&0)).max().unwrap_or(1).max(1);
    let mut by_layer: Vec<Vec<u32>> = vec![Vec::new(); max_depth + 1];
    for &id in &inputs { by_layer[0].push(id); }
    for &id in &hiddens { let d = (*depth.get(&id).unwrap_or(&1)).min(max_depth-1); by_layer[d].push(id); }
    for &id in &outputs { by_layer[max_depth].push(id); }
    for layer in &mut by_layer { layer.sort_unstable(); }

    // Positions
    let left_x = area.x + 40.0;
    let right_x = area.x + area.w - 40.0;
    let top_y = area.y + 24.0;
    let bot_y = area.y + area.h - 24.0;
    let mut pos: std::collections::HashMap<u32, (f32, f32)> = std::collections::HashMap::new();
    for (d, ids) in by_layer.iter().enumerate() { if ids.is_empty() { continue; } let t = if max_depth == 0 { 0.0 } else { d as f32 / (max_depth as f32) }; let x = left_x * (1.0 - t) + right_x * t; let n = ids.len().max(1) as f32; for (i, id) in ids.iter().enumerate() { let ty = if n <= 1.0 { 0.5 } else { i as f32 / (n - 1.0) }; let y = top_y * (1.0 - ty) + bot_y * ty; pos.insert(*id, (x, y)); } }

    // Draw connections (by weight only for simplicity)
    for conn in &genome.connections {
        if !conn.enabled { continue; }
        if hidden_outputs.contains(&conn.in_node_id) || hidden_outputs.contains(&conn.out_node_id) { continue; }
        if hidden_inputs.contains(&conn.in_node_id) || hidden_inputs.contains(&conn.out_node_id) { continue; }
        if let (Some(&(x1, y1)), Some(&(x2, y2))) = (pos.get(&conn.in_node_id), pos.get(&conn.out_node_id)) {
            let w = (conn.weight.abs() * 2.0).clamp(1.0, 4.0);
            let col = if conn.weight >= 0.0 { Color::new(0.2, 0.9, 0.3, 0.75) } else { Color::new(0.95, 0.25, 0.25, 0.75) };
            draw_line(x1, y1, x2, y2, w, col);
        }
    }

    // Color map for activations in [-1,1]: blue (neg) -> gray (0) -> red (pos)
    let act_color = |v: f32| -> Color {
        let v = v.clamp(-1.0, 1.0);
        if v >= 0.0 {
            let t = v; // 0..1
            Color::new(0.4 + 0.6*t, 0.4 - 0.2*t, 0.4 - 0.3*t, 1.0)
        } else {
            let t = -v; // 0..1
            Color::new(0.4 - 0.3*t, 0.4 - 0.2*t, 0.4 + 0.6*t, 1.0)
        }
    };

    // Draw nodes with activation fill; outline by type
    // Inputs: place numeric label to the right of the node to avoid overlap, larger font with shadow
    let draw_input_nodes = |ids: &Vec<u32>, outline: Color| {
        for id in ids {
            if hidden_inputs.contains(id) { continue; }
            if let Some(&(x, y)) = pos.get(id) {
                let v = *activations.get(id).unwrap_or(&0.0);
                let col = act_color(v);
                // Numeric label to the left of the node ("before" the circle)
                let txt = format!("{:.2}", v);
                let fs_px: f32 = 12.0; let fs: u16 = 12;
                let tw = measure_text(&txt, None, fs, 1.0).width;
                let tx = x - 9.0 - tw; // just to the left of the circle
                let ty = y + fs_px * 0.35; // vertically centered-ish
                // Shadow first, then foreground
                draw_text(&txt, tx + 1.0, ty + 1.0, fs_px, BLACK);
                draw_text(&txt, tx, ty, fs_px, LIGHTGRAY);
                // Draw node on top of the label
                draw_circle(x, y, 6.0, col);
                draw_circle_lines(x, y, 6.0, 1.5, outline);
            }
        }
    };
    // Hidden/other nodes: keep centered but make slightly larger with shadow for readability
    let draw_other_nodes = |ids: &Vec<u32>, outline: Color| {
        for id in ids {
            if hidden_outputs.contains(id) { continue; }
            if let Some(&(x, y)) = pos.get(id) {
                let v = *activations.get(id).unwrap_or(&0.0);
                let col = act_color(v);
                draw_circle(x, y, 6.0, col);
                draw_circle_lines(x, y, 6.0, 1.5, outline);
                let txt = format!("{:.2}", v);
                let fs_px: f32 = 12.0; let fs: u16 = 12;
                let tw = measure_text(&txt, None, fs, 1.0).width;
                let tx = x - tw * 0.5;
                let ty = y - 8.0;
                // Shadow
                draw_text(&txt, tx + 1.0, ty + 1.0, fs_px, BLACK);
                // Foreground
                draw_text(&txt, tx, ty, fs_px, LIGHTGRAY);
            }
        }
    };
    draw_input_nodes(&inputs, BLACK);
    draw_other_nodes(&hiddens, BLACK);
    // For outputs, use orange outline
    for id in outputs.iter() {
        if hidden_outputs.contains(id) { continue; }
        if let Some(&(x,y)) = pos.get(id) {
            let v = *activations.get(id).unwrap_or(&0.0);
            let col = act_color(v);
            draw_circle(x, y, 7.0, col);
            draw_circle_lines(x, y, 7.0, 2.0, Color::new(1.0, 0.6, 0.2, 1.0));
            let txt = format!("{:.2}", v);
            let fs_px: f32 = 12.0; let fs: u16 = 12;
            let tw = measure_text(&txt, None, fs, 1.0).width;
            let tx = x - tw*0.5;
            let ty = y - 9.0;
            // Shadow
            draw_text(&txt, tx + 1.0, ty + 1.0, fs_px, BLACK);
            // Foreground
            draw_text(&txt, tx, ty, fs_px, LIGHTGRAY);
        }
    }

    // Legend for activations
    let legend_y = area.y + 14.0;
    // Negative sample
    let neg = act_color(-1.0); draw_circle(area.x + 10.0, legend_y, 6.0, neg); draw_text("-1", area.x + 20.0, legend_y + 4.0, 12.0, LIGHTGRAY);
    let zero = act_color(0.0); draw_circle(area.x + 60.0, legend_y, 6.0, zero); draw_text("0", area.x + 70.0, legend_y + 4.0, 12.0, LIGHTGRAY);
    let pos = act_color(1.0); draw_circle(area.x + 100.0, legend_y, 6.0, pos); draw_text("+1", area.x + 110.0, legend_y + 4.0, 12.0, LIGHTGRAY);
}

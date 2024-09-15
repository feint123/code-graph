use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::{fs::read_dir, path::Path};

use eframe::egui::{CollapsingHeader, Ui};
use egui::{emath, Color32, Pos2, Rect, Stroke, Vec2};
use lang::{CQuery, JavaQuery, JsQuery, RustQuery, SymbolQuery};
use lazy_static::lazy_static;
use tree_sitter::Node;
use tree_sitter::Parser;
use uuid::Uuid;

pub mod lang;

#[derive(Clone, PartialEq)]
pub enum TreeEvent {
    Clicked(String),
    None,
}
#[derive(Debug, Clone, PartialEq)]
pub enum TreeType {
    File,
    Directory,
}

#[derive(Clone, Default, Debug)]
pub struct Tree {
    pub label: String,
    full_path: String,
    select_path: String,
    children: Vec<Tree>,
    tree_type: Option<TreeType>,
    clicked: bool,
}

impl Tree {
    pub fn new(name: &str, full_path: &str, tree_type: TreeType) -> Self {
        Self {
            label: name.to_owned(),
            full_path: full_path.to_owned(),
            children: vec![],
            tree_type: Some(tree_type),
            clicked: false,
            select_path: "".to_owned(),
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) -> TreeEvent {
        let root_name = self.label.clone();
        self.ui_impl(ui, 0, root_name.as_str())
    }
}

impl Tree {
    fn ui_impl(&mut self, ui: &mut Ui, depth: usize, name: &str) -> TreeEvent {
        let tree_type = self.tree_type.clone().unwrap_or(TreeType::File);
        if self.children.len() > 0 || tree_type == TreeType::Directory {
            return CollapsingHeader::new(name)
                .default_open(depth < 1)
                .show(ui, |ui| self.children_ui(ui, depth))
                .body_returned
                .unwrap_or(TreeEvent::None);
        } else {
            let full_path = self.full_path.clone();
            if ui
                .selectable_value(&mut self.select_path, full_path, name)
                .clicked()
            {
                return TreeEvent::Clicked(self.full_path.to_string());
            }
            return TreeEvent::None;
        }
    }

    pub fn clicked(&self) -> bool {
        if self.clicked {
            return true;
        } else if self.children.len() > 0 {
            for child in &self.children {
                if child.clicked() {
                    return true;
                }
            }
        }
        return false;
    }

    fn children_ui(&mut self, ui: &mut Ui, depth: usize) -> TreeEvent {
        for ele in &mut self.children {
            let name = ele.label.clone();
            let event = ele.ui_impl(ui, depth + 1, &name);
            if let TreeEvent::Clicked(_) = event {
                return event;
            }
        }
        TreeEvent::None
    }
}
pub fn recursion_dir(root_path: &Path, pathes: &mut Vec<PathBuf>, mut root_tree: Tree) -> Tree {
    if root_path.is_dir() {
        for entry in read_dir(root_path).expect("Error read Dir") {
            let dir_entry = entry.expect("Error");
            let path_buf = dir_entry.path();
            let is_dir = path_buf.is_dir();
            let tree_type = if is_dir {
                TreeType::Directory
            } else {
                TreeType::File
            };
            let mut tree = Tree::new(
                path_buf.file_name().unwrap().to_str().unwrap(),
                path_buf.as_os_str().to_str().unwrap(),
                tree_type,
            );
            if path_buf.is_dir() {
                tree = recursion_dir(path_buf.as_path(), pathes, tree);
            } else if path_buf.is_file() {
                pathes.push(path_buf);
            }
            root_tree.children.push(tree);
        }
    }
    return root_tree;
}
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum CodeBlockType {
    FUNCTION,
    METHOD,
    STRUCT,
    IMPL,
    CLASS,
    CONST,
    NORMAL,
    CALL,
}
#[derive(Debug, Clone)]
pub struct CodeNode {
    id: String,
    // 标签
    pub label: String,
    // 代码内容
    pub block: String,
    // 文件定位 LineNumber
    pub file_location: usize,
    // 文件路径
    pub file_path: String,
    // 等级
    level: usize,
    // block
    block_type: CodeBlockType,
    // position
    position: Pos2,
    visiable: bool,
}

impl Default for CodeNode {
    fn default() -> Self {
        Self {
            block_type: CodeBlockType::NORMAL,
            id: "".to_owned(),
            label: "".to_owned(),
            block: "".to_owned(),
            file_location: 0,
            level: 0,
            file_path: "".to_owned(),
            position: Pos2::ZERO,
            visiable: true,
        }
    }
}

impl CodeNode {
    pub fn new(
        id: &str,
        label: &str,
        block: &str,
        file_location: usize,
        block_type: CodeBlockType,
        level: usize,
    ) -> Self {
        Self {
            id: id.to_owned(),
            label: label.to_owned(),
            block: block.to_owned(),
            file_location: file_location.to_owned(),
            file_path: "".to_owned(),
            block_type,
            position: Pos2::new(0.0, 0.0),
            level,
            visiable: true,
        }
    }
}
#[derive(Clone, Copy)]
pub struct CodeNodeIndex(usize);

pub struct Edge {
    from: usize,
    to: usize,
}

pub struct Graph {
    nodes: Vec<CodeNode>,
    edges: Vec<Edge>,
    focus_node: Option<CodeNodeIndex>,
}

lazy_static! {
    static ref GRAPH_THEME: HashMap<eframe::Theme, HashMap<CodeBlockType, egui::Color32>> = {
        let mut dark_block_type_map = HashMap::new();
        dark_block_type_map.insert(CodeBlockType::NORMAL, egui::Color32::DARK_GRAY);
        dark_block_type_map.insert(CodeBlockType::FUNCTION, egui::Color32::DARK_BLUE);
        dark_block_type_map.insert(CodeBlockType::STRUCT, egui::Color32::from_rgb(204, 112, 0));
        dark_block_type_map.insert(CodeBlockType::CONST, egui::Color32::from_rgb(204, 112, 0));
        dark_block_type_map.insert(CodeBlockType::CLASS, egui::Color32::DARK_GREEN);
        let mut light_block_type_map = HashMap::new();
        light_block_type_map.insert(CodeBlockType::NORMAL, egui::Color32::LIGHT_GRAY);
        light_block_type_map.insert(CodeBlockType::FUNCTION, egui::Color32::LIGHT_BLUE);
        light_block_type_map.insert(CodeBlockType::STRUCT, egui::Color32::LIGHT_YELLOW);
        light_block_type_map.insert(CodeBlockType::CONST, egui::Color32::LIGHT_YELLOW);
        light_block_type_map.insert(CodeBlockType::CLASS, egui::Color32::LIGHT_GREEN);
        let mut m = HashMap::new();
        m.insert(eframe::Theme::Dark, dark_block_type_map);
        m.insert(eframe::Theme::Light, light_block_type_map);
        m
    };
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: vec![],
            edges: vec![],
            focus_node: None,
        }
    }

    pub fn get_focus_idx(&mut self) -> Option<CodeNodeIndex> {
        return self.focus_node;
    }

    pub fn add_node(&mut self, node: CodeNode) -> CodeNodeIndex {
        let index = self.nodes.len();
        self.nodes.push(node);
        return CodeNodeIndex(index);
    }

    pub fn add_edge(&mut self, from: CodeNodeIndex, to: CodeNodeIndex) {
        self.edges.push(Edge {
            from: from.0,
            to: to.0,
        })
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
        self.focus_node = None;
    }
    /**
     * 对节点进行布局
     */
    pub fn layout(&mut self, ui: &mut Ui, start_point: Option<Vec2>) {
        let (_, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click());
        let mut sum_height = 0.0;
        let mut start_p = Vec2::new(ui.available_width() / 2.0, 32.0);
        if let Some(point) = start_point {
            start_p = point;
        }
        // 直线布局
        for (index, node) in self
            .nodes
            .iter_mut()
            .filter(|node| node.visiable)
            .enumerate()
        {
            let text_size = painter
                .layout_no_wrap(
                    node.label.clone(),
                    egui::FontId::default(),
                    egui::Color32::WHITE,
                )
                .size();
            node.position = Pos2::new(
                start_p.x + node.level as f32 * 20.0,
                index as f32 * 16.0 + sum_height + start_p.y,
            );
            sum_height += text_size.y;
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) -> egui::Response {
        let (response, painter) =
            ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());

        let focus_stroke_color;
        let stroke_color;
        let text_color;
        let grid_color;
        let block_type_map;

        if ui.ctx().style().visuals.dark_mode {
            stroke_color = egui::Color32::LIGHT_GRAY;
            text_color = egui::Color32::WHITE;
            focus_stroke_color = egui::Color32::LIGHT_BLUE;
            grid_color = Color32::from_gray(50);
            block_type_map = GRAPH_THEME.get(&eframe::Theme::Dark).unwrap();
        } else {
            focus_stroke_color = egui::Color32::BLUE;
            stroke_color = egui::Color32::DARK_GRAY;
            text_color = egui::Color32::DARK_GRAY;
            grid_color = Color32::from_gray(220);
            block_type_map = GRAPH_THEME.get(&eframe::Theme::Light).unwrap();
        }

        // 获取可用区域
        let rect = ui.max_rect();

        // 定义网格参数
        let cell_size = 10.0; // 网格单元格大小
        let stroke = Stroke::new(0.5, grid_color); // 线条宽度和颜色

        // 绘制垂直线
        let mut x = rect.left();
        while x <= rect.right() {
            let line = [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())];
            painter.line_segment(line, stroke);
            x += cell_size;
        }

        // 绘制水平线
        let mut y = rect.top();
        while y <= rect.bottom() {
            let line = [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)];
            painter.line_segment(line, stroke);
            y += cell_size;
        }

        let to_screen = emath::RectTransform::from_to(
            Rect::from_min_size(Pos2::ZERO, response.rect.size()),
            response.rect,
        );
        let mut node_size_list = vec![];

        // 绘制节点
        for (index, node) in self.nodes.iter_mut().enumerate() {
            let node_pos = to_screen.transform_pos(node.position);
            let text_size = painter
                .layout_no_wrap(
                    node.label.clone(),
                    egui::FontId::default(),
                    egui::Color32::WHITE,
                )
                .size();

            node_size_list.push(text_size + Vec2::new(16.0, 8.0));
            if node.visiable {
                let rect = egui::Rect::from_min_size(
                    node_pos,
                    egui::vec2(text_size.x + 16.0, text_size.y + 8.0),
                );
                let fill_color = block_type_map
                    .get(&node.block_type)
                    .copied()
                    .unwrap_or(egui::Color32::DARK_GRAY);

                painter.rect(rect, 5.0, fill_color, Stroke::new(1.0, stroke_color));

                painter.text(
                    node_pos + Vec2::new(8.0, 4.0),
                    egui::Align2::LEFT_TOP,
                    &node.label,
                    egui::FontId::default(),
                    text_color,
                );

                let point_id = response.id.with(&node.id);

                let node_response = ui.interact(rect, point_id, egui::Sense::click_and_drag());
                if node_response.dragged() {
                    // 更新节点位置
                    node.position += node_response.drag_delta();
                }
                if node_response.clicked() {
                    self.focus_node = Some(CodeNodeIndex(index));
                }
                if let Some(f_node) = self.focus_node {
                    if f_node.0 == index {
                        // ui.ctx().request_repaint();
                        // let time = ui.input(|i| i.time);
                        painter.rect(
                            rect,
                            5.0,
                            egui::Color32::TRANSPARENT,
                            Stroke::new(2.5, focus_stroke_color),
                        );
                    }
                }
            }

            if response.dragged() {
                // 更新节点位置
                node.position += response.drag_delta();
            }
        }

        // 绘制边
        for edge in &self.edges {
            if !self.nodes[edge.to].visiable || !self.nodes[edge.from].visiable {
                continue;
            }
            let from = to_screen.transform_pos(self.nodes[edge.from].position)
                + Vec2::new(0.0, node_size_list[edge.from].y / 2.0);
            let to = to_screen.transform_pos(self.nodes[edge.to].position)
                + Vec2::new(0.0, node_size_list[edge.to].y / 2.0);
            painter.line_segment(
                [from, from + Vec2::new(-10.0, 0.0)],
                (1.0, egui::Color32::GRAY),
            );
            painter.line_segment(
                [from + Vec2::new(-10.0, 0.0), Pos2::new(from.x - 10.0, to.y)],
                (1.0, egui::Color32::GRAY),
            );
            painter.line_segment(
                [Pos2::new(from.x - 10.0, to.y), to],
                (1.0, egui::Color32::GRAY),
            );
        }
        // 绘制伸缩
        if self.nodes.len() > 0 {
            let mut level_queue = VecDeque::new();
            level_queue.push_back(0);
            while let Some(node_index) = level_queue.pop_front() {
                let mut sub_nodes = vec![];
                for edge in &self.edges {
                    if edge.from == node_index {
                        level_queue.push_back(edge.to);
                        sub_nodes.push(edge.to);
                    }
                }
                if !sub_nodes.is_empty() && self.nodes[node_index].visiable {
                    let from = to_screen.transform_pos(self.nodes[node_index].position)
                        + Vec2::new(0.0, node_size_list[node_index].y / 2.0);
                    let tree_point = from + Vec2::new(-10.0, 0.0);
                    painter.circle_filled(tree_point, 5.0, stroke_color);
                    let point_id = response
                        .id
                        .with(format!("edge-{}", self.nodes[node_index].id));

                    let node_response = ui.interact(
                        egui::Rect::from_center_size(tree_point, Vec2::new(10.0, 10.0)),
                        point_id,
                        egui::Sense::click(),
                    );
                    if !self.nodes[sub_nodes[0]].visiable {
                        painter.circle_stroke(
                            tree_point,
                            7.0,
                            Stroke::new(2.0, focus_stroke_color),
                        );
                    }
                    if node_response.clicked() {
                        let mut change_visiable_queue = VecDeque::new();
                        let visiable = !self.nodes[sub_nodes[0]].visiable;
                        for index in sub_nodes {
                            change_visiable_queue.push_back(index);
                        }
                        while let Some(visiable_index) = change_visiable_queue.pop_front() {
                            self.nodes[visiable_index].visiable = visiable;
                            for edge in &self.edges {
                                if edge.from == visiable_index {
                                    change_visiable_queue.push_back(edge.to);
                                }
                            }
                        }
                        self.layout(ui, Some(self.nodes[0].position.to_vec2()));
                    }
                }
            }
        }
        self.draw_minimap(ui, &node_size_list, &response, block_type_map);
        response
    }

    fn draw_minimap(
        &self,
        ui: &mut Ui,
        rect_size: &Vec<Vec2>,
        response: &egui::Response,
        color_map: &HashMap<CodeBlockType, Color32>,
    ) {
        let minimap_size = Vec2::new(200.0, 150.0); // 缩略图大小
        let minimap_margin = 10.0; // 缩略图与画布边缘的间距

        // 计算缩略图位置(右下角)
        let minimap_pos = Pos2::new(
            response.rect.right() - minimap_size.x - minimap_margin,
            response.rect.bottom() - minimap_size.y - minimap_margin,
        );

        let minimap_rect = Rect::from_min_size(minimap_pos, minimap_size);

        // 绘制缩略图背景
        ui.painter()
            .rect_filled(minimap_rect, 0.0, ui.visuals().extreme_bg_color);

        // 计算缩放比例
        let scale_x = minimap_size.x / response.rect.width();
        let scale_y = minimap_size.y / response.rect.height();
        let scale = scale_x.min(scale_y);

        for (index, node) in self.nodes.iter().enumerate() {
            if node.visiable {
                // 检查节点是否在可视区域内

                let mut minimap_node_pos = minimap_pos + (node.position.to_vec2() * scale);
                let mut node_size = rect_size[index] * scale;
                if minimap_node_pos.x < minimap_rect.min.x {
                    node_size.x = node_size.x - (minimap_rect.min.x - minimap_node_pos.x);
                    minimap_node_pos.x = minimap_rect.min.x;
                }
                if minimap_node_pos.y < minimap_rect.min.y {
                    node_size.y = node_size.y - (minimap_rect.min.y - minimap_node_pos.y);
                    minimap_node_pos.y = minimap_rect.min.y;
                }
                if minimap_node_pos.x + node_size.x > minimap_rect.max.x {
                    node_size.x = minimap_rect.max.x - minimap_node_pos.x;
                }
                if minimap_node_pos.y + node_size.y > minimap_rect.max.y {
                    node_size.y = minimap_rect.max.y - minimap_node_pos.y;
                }
                let node_rect = Rect::from_min_size(minimap_node_pos, node_size);

                let fill_color = color_map
                    .get(&node.block_type)
                    .copied()
                    .unwrap_or(egui::Color32::DARK_GRAY);

                ui.painter().rect_filled(node_rect, 0.0, fill_color);
            }
        }
        // 绘制缩略图边框
        ui.painter().rect_stroke(
            minimap_rect,
            0.0,
            Stroke::new(1.0, ui.visuals().text_color()),
        );
    }

    pub fn get_node(&mut self, index: CodeNodeIndex) -> CodeNode {
        let default_node = CodeNode::default();
        let node = self.nodes.get(index.0).unwrap_or(&default_node);
        return node.clone();
    }

    pub fn node_index(&mut self, node_id: &str) -> CodeNodeIndex {
        for (index, node) in self.nodes.iter().enumerate() {
            if node.id == node_id {
                return CodeNodeIndex(index);
            }
        }
        CodeNodeIndex(0)
    }
}

pub fn valid_file_extention(extension: &str) -> bool {
    return vec!["rs", "c", "h", "java", "js", "jsx"].contains(&extension);
}

pub fn get_symbol_query(extention: &str) -> Box<dyn SymbolQuery> {
    match extention {
        "rs" => Box::new(RustQuery),
        "java" => Box::new(JavaQuery),
        "c" | "h" => Box::new(CQuery),
        "js" | "jsx" => Box::new(JsQuery),
        _ => Box::new(RustQuery),
    }
}

pub fn fetch_calls(path: &str, code: &str, symbol_query: Box<dyn SymbolQuery>) -> Vec<CodeNode> {
    let mut parser = Parser::new();
    parser
        .set_language(&symbol_query.get_lang())
        .expect("Error load Rust grammer");
    let tree = parser.parse(code, None).unwrap();
    let root_node = tree.root_node();
    recursion_call(root_node, path, code, &symbol_query)
}

pub fn recursion_call(
    node: Node,
    path: &str,
    code: &str,
    symbol_query: &Box<dyn SymbolQuery>,
) -> Vec<CodeNode> {
    let mut nodes = vec![];
    let code_node = symbol_query.get_call(code, &node);
    if let Some(mut node) = code_node {
        node.file_path = path.to_string();
        nodes.push(node);
    }

    for child in node.children(&mut node.walk()) {
        let sub_nodes = recursion_call(child, path, code, symbol_query);
        if sub_nodes.len() > 0 {
            for sub_node in sub_nodes {
                nodes.push(sub_node);
            }
        }
    }
    return nodes;
}
/**
* 打印大纲
*/
pub fn fetch_symbols(
    path: &str,
    code: &str,
    symbol_query: Box<dyn SymbolQuery>,
    graph: &mut Graph,
) {
    let mut parser = Parser::new();
    parser
        .set_language(&symbol_query.get_lang())
        .expect("Error load Rust grammer");
    let tree = parser.parse(code, None).unwrap();
    let root_node = tree.root_node();
    let root_code_node = CodeNode::new(
        format!("{}", Uuid::new_v4()).as_str(),
        path,
        code,
        0,
        CodeBlockType::NORMAL,
        0,
    );
    graph.add_node(root_code_node);
    recursion_outline(
        root_node,
        CodeNodeIndex(0),
        path,
        code,
        1,
        &symbol_query,
        graph,
    );
}

pub fn recursion_outline(
    node: Node,
    parent_id: CodeNodeIndex,
    path: &str,
    code: &str,
    level: usize,
    symbol_query: &Box<dyn SymbolQuery>,
    graph: &mut Graph,
) {
    let mut current_id = parent_id;
    let code_node = symbol_query.get_definition(code, &node);
    let mut level = level;
    if let Some(mut node) = code_node {
        node.file_path = path.to_string();
        node.level = level;
        let index = graph.add_node(node);
        current_id = index;
        graph.add_edge(parent_id, index);
        level += 1;
    }

    for child in node.children(&mut node.walk()) {
        recursion_outline(child, current_id, path, code, level, symbol_query, graph)
    }
}

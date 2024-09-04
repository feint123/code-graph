use std::path::PathBuf;
use std::{fs::read_dir, path::Path};

use eframe::egui::{CollapsingHeader, Ui};
use egui::{emath, Color32, Pos2, Rect, Stroke, Vec2};
use tree_sitter::Node;
use tree_sitter::{Language, Parser};
use uuid::Uuid;

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
#[derive(Debug, Clone)]
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
    pub fn layout(&mut self, ui: &mut Ui) {
        let (_, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click());
        let mut sum_height = 0.0;
        // 直线布局
        for (index, node) in self.nodes.iter_mut().enumerate() {
            let text_size = painter
                .layout_no_wrap(
                    node.label.clone(),
                    egui::FontId::default(),
                    egui::Color32::WHITE,
                )
                .size();
            node.position = Pos2::new(
                ui.available_size().x / 2.0 + node.level as f32 * 20.0,
                index as f32 * 16.0 + sum_height + 32.0,
            );
            sum_height += text_size.y;
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) -> egui::Response {
        let (response, painter) =
            ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
        // 获取可用区域
        let rect = ui.max_rect();

        // 定义网格参数
        let cell_size = 10.0; // 网格单元格大小
        let color = Color32::from_gray(220); // 网格线颜色
        let stroke = Stroke::new(0.5, color); // 线条宽度和颜色

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

            let rect = egui::Rect::from_min_size(
                node_pos,
                egui::vec2(text_size.x + 16.0, text_size.y + 8.0),
            );
            let fill_color = match node.block_type {
                CodeBlockType::NORMAL => egui::Color32::LIGHT_GRAY,
                CodeBlockType::FUNCTION => egui::Color32::LIGHT_BLUE,
                CodeBlockType::STRUCT => egui::Color32::LIGHT_YELLOW,
                CodeBlockType::CLASS => egui::Color32::LIGHT_GREEN,
                _ => egui::Color32::LIGHT_GRAY,
            };

            painter.rect(rect, 5.0, fill_color, Stroke::NONE);

            painter.text(
                node_pos + Vec2::new(8.0, 4.0),
                egui::Align2::LEFT_TOP,
                &node.label,
                egui::FontId::default(),
                egui::Color32::DARK_GRAY,
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
                        Stroke::new(2.5, egui::Color32::BLUE),
                    );
                }
            }

            if response.dragged() {
                // 更新节点位置
                node.position += response.drag_delta();
            }
        }

        // 绘制边
        for edge in &self.edges {
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
        response
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
    return vec!["rs", "c", "h", "java"].contains(&extension);
}

pub fn get_symbol_query(extention: &str) -> Box<dyn SymbolQuery> {
    match extention {
        "rs" => Box::new(RustQuery),
        "java" => Box::new(JavaQuery),
        "c" => Box::new(CQuery),
        "h" => Box::new(CQuery),
        _ => Box::new(RustQuery),
    }
}

pub trait SymbolQuery {
    fn get_call(&self, code: &str, node: &Node) -> Option<CodeNode>;
    fn get_lang(&self) -> Language;
    fn get_definition(&self, code: &str, node: &Node) -> Option<CodeNode>;
}
pub struct RustQuery;
pub struct CQuery;
pub struct JavaQuery;

impl SymbolQuery for CQuery {
    fn get_call(&self, code: &str, node: &Node) -> Option<CodeNode> {
        let node_type = node.kind();

        if node_type == "call_expression" {
            let block_text = &code[node.byte_range()];
            let fe = node.child_by_field_name("function");
            if let Some(fe) = fe {
                let fi = fe.child_by_field_name("field");
                if let Some(fi) = fi {
                    let label = &code[fi.byte_range()];
                    return Some(CodeNode::new(
                        format!("{}", Uuid::new_v4()).as_str(),
                        label,
                        block_text,
                        fi.start_position().row + 1,
                        CodeBlockType::CALL,
                        0,
                    ));
                } else {
                    let label = &code[fe.byte_range()];
                    return Some(CodeNode::new(
                        format!("{}", Uuid::new_v4()).as_str(),
                        label,
                        block_text,
                        fe.start_position().row + 1,
                        CodeBlockType::CALL,
                        0,
                    ));
                }
            }
        }
        None
    }

    fn get_lang(&self) -> Language {
        tree_sitter_c::language()
    }

    fn get_definition(&self, code: &str, node: &Node) -> Option<CodeNode> {
        let node_type = node.kind();
        let definition_list = [("function_definition", "compound_statement")];
        for (root_type, end_type) in definition_list {
            if node_type == root_type {
                let mut output = String::new();
                for child in node.children(&mut node.walk()) {
                    if child.kind() == end_type {
                        break;
                    } else {
                        let node_text = &code[child.byte_range()];
                        output.push_str(node_text);
                        output.push(' ');
                    }
                }
                let block_type = match root_type {
                    "function_definition" => CodeBlockType::FUNCTION,
                    "struct_item" => CodeBlockType::STRUCT,
                    _ => CodeBlockType::NORMAL,
                };
                let block_text = &code[node.byte_range()];
                return Some(CodeNode::new(
                    format!("{}", Uuid::new_v4()).as_str(),
                    output.as_str().split("(").next().unwrap_or("bad symbol"),
                    block_text,
                    node.start_position().row + 1,
                    block_type,
                    0,
                ));
            }
        }

        None
    }
}

impl SymbolQuery for JavaQuery {
    fn get_call(&self, code: &str, node: &Node) -> Option<CodeNode> {
        let node_type = node.kind();

        if node_type == "method_invocation" {
            let block_text = &code[node.byte_range()];
            let fe = node.child_by_field_name("name");
            if let Some(fe) = fe {
                let label = &code[fe.byte_range()];
                return Some(CodeNode::new(
                    format!("{}", Uuid::new_v4()).as_str(),
                    label,
                    block_text,
                    fe.start_position().row + 1,
                    CodeBlockType::CALL,
                    0,
                ));
            }
        }
        None
    }

    fn get_lang(&self) -> Language {
        tree_sitter_java::language()
    }

    fn get_definition(&self, code: &str, node: &Node) -> Option<CodeNode> {
        let node_type = node.kind();
        let definition_list = [
            ("class_declaration", "class_body"),
            ("method_declaration", "formal_parameters"),
            ("interface_declaration", "interface_body"),
        ];
        for (root_type, end_type) in definition_list {
            if node_type == root_type {
                let mut output = String::new();
                for child in node.children(&mut node.walk()) {
                    if child.kind() == end_type {
                        break;
                    } else {
                        let node_text = &code[child.byte_range()];

                        output.push_str(node_text);

                        output.push(' ');
                    }
                }
                let block_type = match root_type {
                    "method_declaration" => CodeBlockType::FUNCTION,
                    "class_declaration" => CodeBlockType::CLASS,
                    "interface_declaration" => CodeBlockType::CLASS,
                    _ => CodeBlockType::NORMAL,
                };
                let block_text = &code[node.byte_range()];
                return Some(CodeNode::new(
                    format!("{}", Uuid::new_v4()).as_str(),
                    output.as_str(),
                    block_text,
                    node.start_position().row + 1,
                    block_type,
                    0,
                ));
            }
        }

        None
    }
}

impl SymbolQuery for RustQuery {
    fn get_lang(&self) -> Language {
        tree_sitter_rust::language()
    }

    // call_expression 下 identifier 和 field_identifier
    fn get_call(&self, code: &str, node: &Node) -> Option<CodeNode> {
        let node_type = node.kind();

        if node_type == "call_expression" {
            let block_text = &code[node.byte_range()];
            let fe = node.child_by_field_name("function");
            if let Some(fe) = fe {
                let fi = fe.child_by_field_name("field");
                if let Some(fi) = fi {
                    let label = &code[fi.byte_range()];
                    return Some(CodeNode::new(
                        format!("{}", Uuid::new_v4()).as_str(),
                        label,
                        block_text,
                        fi.start_position().row + 1,
                        CodeBlockType::CALL,
                        0,
                    ));
                } else {
                    let label = &code[fe.byte_range()];
                    return Some(CodeNode::new(
                        format!("{}", Uuid::new_v4()).as_str(),
                        label,
                        block_text,
                        fe.start_position().row + 1,
                        CodeBlockType::CALL,
                        0,
                    ));
                }
            }
        }
        None
    }

    fn get_definition(&self, code: &str, node: &Node) -> Option<CodeNode> {
        let node_type = node.kind();
        let definition_list = [
            ("function_item", "parameters"),
            ("impl_item", "declaration_list"),
            ("struct_item", "field_declaration_list"),
            ("trait_item", "declaration_list"),
            ("function_signature_item", "parameters"),
        ];
        for (root_type, end_type) in definition_list {
            if node_type == root_type {
                let mut output = String::new();
                for child in node.children(&mut node.walk()) {
                    if child.kind() == end_type {
                        break;
                    } else {
                        let node_text = &code[child.byte_range()];

                        output.push_str(node_text);

                        output.push(' ');
                    }
                }
                let block_type = match root_type {
                    "function_item" => CodeBlockType::FUNCTION,
                    "struct_item" => CodeBlockType::STRUCT,
                    "function_signature_item" => CodeBlockType::FUNCTION,
                    "trait_item" => CodeBlockType::CLASS,
                    "impl_item" => CodeBlockType::CLASS,
                    _ => CodeBlockType::NORMAL,
                };
                let block_text = &code[node.byte_range()];
                return Some(CodeNode::new(
                    format!("{}", Uuid::new_v4()).as_str(),
                    output.as_str(),
                    block_text,
                    node.start_position().row + 1,
                    block_type,
                    0,
                ));
            }
        }

        None
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
        0,
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
    if let Some(mut node) = code_node {
        node.file_path = path.to_string();
        node.level = level;
        let index = graph.add_node(node);
        current_id = index;
        graph.add_edge(parent_id, index);
    }

    for child in node.children(&mut node.walk()) {
        recursion_outline(
            child,
            current_id,
            path,
            code,
            level + 1,
            symbol_query,
            graph,
        )
    }
}

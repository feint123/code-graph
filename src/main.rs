use std::{
    ffi::OsStr,
    fs::{self},
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::{self, Receiver},
    thread::{self},
};

use code_graph::{
    fetch_calls, fetch_symbols, get_symbol_query, recursion_dir, valid_file_extention, CodeNode,
    Graph, Tree, TreeEvent, TreeType,
};
use eframe::egui::{self};
use egui::{text::LayoutJob, FontId, Rounding, TextFormat, Ui, Vec2, Widget};
use font_kit::{family_name::FamilyName, properties::Properties, source::SystemSource};
use rfd::{FileDialog, MessageDialog};
use serde::{Deserialize, Serialize};

fn main() -> eframe::Result {
    let mut options = eframe::NativeOptions::default();
    options.persist_window = true;
    eframe::run_native(
        "Code Graph",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            // 修改一些基本配置
            cc.egui_ctx.style_mut(|style| {
                style.spacing.button_padding = Vec2::new(8.0, 2.0);
            });
            let system_source = SystemSource::new();
            let mut fonts = egui::FontDefinitions::default();
            // 尝试加载系统默认字体
            if let Ok(font) = system_source.select_best_match(
                &[
                    FamilyName::Title("Source Han Mono SC".to_string()),
                    FamilyName::Title("PingFang SC".to_string()),
                    FamilyName::Title("Microsoft YaHei".to_string()),
                ],
                &Properties::new(),
            ) {
                if let Ok(font_data) = font.load() {
                    fonts.font_data.insert(
                        "system_font".to_owned(),
                        egui::FontData::from_owned(font_data.copy_font_data().unwrap().to_vec()),
                    );
                }
            }
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "system_font".to_owned());

            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("system_font".to_owned());
            // cc.egui_ctx.set_debug_on_hover(true);
            cc.egui_ctx.set_fonts(fonts);
            let mut my_app = MyApp::default();
            if let Some(storage) = cc.storage {
                if let Some(app_state) = storage.get_string("app_state") {
                    let app_state = serde_json::from_str::<AppState>(&app_state);
                    if let Ok(app_state) = app_state {
                        my_app.project_root_path =
                            Some(Path::new(&app_state.root_path).to_path_buf());
                        my_app.editor = app_state.editor;
                    }
                }
            }
            Ok(Box::new(my_app))
        }),
    )
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
enum Editor {
    VSCode,
    Zed,
    Idea,
}
#[derive(Debug, Serialize, Deserialize)]
struct AppState {
    editor: Editor,
    root_path: String,
}
struct MyApp {
    tree: Tree,
    code: String,
    current_node: CodeNode,
    call_nodes: Vec<CodeNode>,
    filter_call_nodes: Vec<CodeNode>,
    project_root_path: Option<PathBuf>,
    root_path: String,
    graph: Graph,
    editor: Editor,
    rx: Option<Receiver<(Tree, Vec<CodeNode>)>>,
    debug: DebugInfo,
}
#[derive(Default, Debug)]
struct DebugInfo {
    fps: f32,
    enable: bool,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            code: "".to_owned(),
            current_node: CodeNode::default(),
            call_nodes: vec![],
            filter_call_nodes: vec![],
            tree: Tree::new("", "", TreeType::File),
            project_root_path: None,
            root_path: "".to_owned(),
            graph: Graph::new(),
            editor: Editor::VSCode,
            rx: None,
            debug: DebugInfo::default(),
        }
    }
}

impl MyApp {
    fn side_panel(&mut self, ui: &mut Ui) {
        if self.tree.label.is_empty() {
            ui.label("这里什么也没有");
        } else {
            if let TreeEvent::Clicked(name) = self.tree.ui(ui) {
                let path = Path::new(&name);
                let ext = path.extension().unwrap_or(OsStr::new("")).to_str().unwrap();
                if valid_file_extention(ext) {
                    self.code = fs::read_to_string(path).unwrap();
                    self.current_node = CodeNode::default();
                    self.graph.clear();
                    // 解析代码，生成图
                    fetch_symbols(&name, &self.code, get_symbol_query(ext), &mut self.graph);
                    // 布局
                    self.graph.layout(ui, None);
                } else {
                    MessageDialog::new()
                        .set_title("提示")
                        .set_description("不受支持的文件类型")
                        .show();
                }
            }
        }
    }
    fn open_editor(&self, file_path: &str, line_number: usize) {
        let command = match self.editor {
            Editor::Zed => "zed",
            Editor::VSCode => "code",
            Editor::Idea => "idea",
        };
        let args = match self.editor {
            Editor::Zed => vec![format!("{}:{}", file_path, line_number)],
            Editor::VSCode => vec!["-g".to_owned(), format!("{}:{}", file_path, line_number)],
            Editor::Idea => vec![
                "-l".to_owned(),
                format!("{}", line_number),
                format!("{}", file_path),
            ],
        };
        // 执行shell 命令
        let _ = Command::new(command).args(args).output().is_err_and(|err| {
            let message_dialog = rfd::MessageDialog::new();
            message_dialog
                .set_title("打开失败")
                .set_description(err.to_string())
                .show();
            return true;
        }); // 传递命令行参数
    }
    fn right_panel(&mut self, ui: &mut Ui) {
        ui.add_space(10.0);
        egui::Grid::new("param_grid")
            .num_columns(2)
            .spacing([10.0, 10.0])
            .show(ui, |ui| {
                ui.label("选择编辑器");
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_source("choose editor")
                        .selected_text(format!("{:?}", self.editor))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.editor, Editor::Idea, "Idea");
                            ui.selectable_value(&mut self.editor, Editor::VSCode, "VSCode");
                            ui.selectable_value(&mut self.editor, Editor::Zed, "Zed");
                        });
                    ui.add_space(4.0);
                    if self.get_normal_button("打开").ui(ui).clicked() {
                        self.open_editor(
                            &self.current_node.file_path,
                            self.current_node.file_location,
                        );
                    }
                });

                ui.end_row();
            });

        ui.add_space(10.0);
        egui::CollapsingHeader::new("调用列表")
            .default_open(true)
            .show(ui, |ui| {
                for node in &self.filter_call_nodes {
                    let mut job = LayoutJob::default();
                    job.append(
                        node.block.replace("\n", " ").replace(" ", "").as_str(),
                        0.0,
                        TextFormat {
                            color: ui.style().visuals.text_color(),
                            ..Default::default()
                        },
                    );
                    job.append(
                        format!("\n{}:{}", node.file_path, node.file_location).as_str(),
                        0.0,
                        TextFormat {
                            font_id: FontId::monospace(8.0),
                            ..Default::default()
                        },
                    );
                    if egui::Button::new(job)
                        .rounding(Rounding::same(8.0))
                        .min_size(egui::Vec2::new(ui.available_width(), 0.0))
                        .ui(ui)
                        .clicked()
                    {
                        self.open_editor(&node.file_path, node.file_location);
                    }
                }
            });

        ui.add_space(10.0);
        egui::CollapsingHeader::new("代码预览")
            .default_open(true)
            .show(ui, |ui| {
                let language = "rs";
                let theme = egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx());
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui_extras::syntax_highlighting::code_view_ui(
                        ui,
                        &theme,
                        &self.current_node.block,
                        language,
                    );
                });
            });
        ui.add_space(10.0);
    }

    fn draw_debug_info(&self, ctx: &egui::Context) {
        let painter = ctx.debug_painter();

        // 绘制帧率
        painter.text(
            egui::pos2(10.0, 10.0),
            egui::Align2::LEFT_TOP,
            format!("FPS: {:.1}", self.debug.fps),
            egui::FontId::default(),
            egui::Color32::GREEN,
        );

        // 可以添加更多调试信息...
        // 例如，内存使用、对象数量等
    }

    fn get_normal_button(&mut self, text: &str) -> egui::Button {
        return egui::Button::new(text).rounding(Rounding::same(5.0));
    }
}

impl eframe::App for MyApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string(
            "app_state",
            serde_json::to_string(&AppState {
                editor: self.editor.clone(),
                root_path: self.root_path.clone(),
            })
            .unwrap(),
        );
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.debug.enable {
            let time = ctx.input(|i| i.unstable_dt);
            self.debug.fps = 1.0 / time;
            self.draw_debug_info(ctx);
        }
        egui::SidePanel::left("side_panel")
            .resizable(true)
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label("文件列表");
                    // let file_icon = egui::include_image!("../assets/folder.png");
                    let open_file_button = ui.add(self.get_normal_button("选择"));
                    if open_file_button.clicked() {
                        // 打开系统目录
                        if let Some(path) = FileDialog::new().pick_folder() {
                            self.root_path = path.as_os_str().to_str().unwrap().to_string();
                            self.project_root_path = Some(path);
                        }
                    }
                    open_file_button.on_hover_text("选择项目目录");
                });
                if let Some(dir_path) = &self.project_root_path {
                    // 清除图里的数据
                    self.graph.clear();
                    let new_tree = Tree::new(
                        dir_path.as_os_str().to_str().unwrap(),
                        dir_path.as_os_str().to_str().unwrap(),
                        TreeType::Directory,
                    );
                    let dir_path = dir_path.clone();
                    let (tx, rx) = mpsc::channel();
                    self.rx = Some(rx);
                    // 在后台线程中执行耗时任务
                    thread::spawn(move || {
                        let mut pathes = vec![];
                        let result = recursion_dir(&dir_path, &mut pathes, new_tree);
                        let call_node_list = pathes
                            .iter()
                            .map(|path_buffer| {
                                let ext = path_buffer
                                    .extension()
                                    .unwrap_or(OsStr::new(""))
                                    .to_str()
                                    .unwrap();
                                let name = path_buffer.as_os_str().to_str().unwrap();
                                if valid_file_extention(ext) {
                                    let code = fs::read_to_string(path_buffer).unwrap_or("".into());
                                    return fetch_calls(&name, &code, get_symbol_query(ext));
                                }
                                return vec![];
                            })
                            .flatten()
                            .collect::<Vec<CodeNode>>();
                        // 解析获取文件中说有使用了符号的代码
                        tx.send((result, call_node_list)).unwrap();
                    });
                    self.project_root_path = None
                }

                if let Some(rx) = &self.rx {
                    if let Ok(result) = rx.try_recv() {
                        self.tree = result.0;
                        self.call_nodes = result.1;
                        self.rx = None;
                    } else {
                        ui.spinner();
                    }
                }

                ui.add_space(10.0);
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.set_min_height(ui.available_height());
                    self.side_panel(ui);
                });
            });
        egui::SidePanel::right("right_panel")
            .min_width(240.0)
            .resizable(true)
            .show_separator_line(false)
            .show(ctx, |ui| {
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.set_min_height(ui.available_height());
                    self.right_panel(ui);
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                let response = self.graph.ui(ui);
                if let Some(focue_node) = self.graph.get_focus_idx() {
                    self.current_node = self.graph.get_node(focue_node);
                    self.filter_call_nodes.clear();
                    for node in &self.call_nodes {
                        let current_label = &self.current_node.label;
                        for ele in current_label.split(" ") {
                            if ele == node.label {
                                self.filter_call_nodes.push(node.clone());
                            }
                        }
                    }
                }
                response
            });
        });
    }
}

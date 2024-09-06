use tree_sitter::{Language, Node};
use uuid::Uuid;

use crate::{CodeBlockType, CodeNode};

pub trait SymbolQuery {
    fn get_call(&self, code: &str, node: &Node) -> Option<CodeNode>;
    fn get_lang(&self) -> Language;
    fn get_definition(&self, code: &str, node: &Node) -> Option<CodeNode>;
}
pub struct RustQuery;
pub struct CQuery;
pub struct JavaQuery;
pub struct JsQuery;

impl SymbolQuery for JsQuery {
    fn get_call(&self, code: &str, node: &Node) -> Option<CodeNode> {
        let node_type = node.kind();

        if node_type == "call_expression" {
            let block_text = &code[node.byte_range()];
            let fe = node.child_by_field_name("function");
            if let Some(fe) = fe {
                let fi = fe.child_by_field_name("property");
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
        tree_sitter_javascript::language()
    }

    fn get_definition(&self, code: &str, node: &Node) -> Option<CodeNode> {
        let node_type = node.kind();
        let definition_list = [
            ("function_declaration", "formal_parameters"),
            ("class_declaration", "class_body"),
            ("method_definition", "formal_parameters"),
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
                    "function_declaration" => CodeBlockType::FUNCTION,
                    "method_definition" => CodeBlockType::FUNCTION,
                    "class_declaration" => CodeBlockType::CLASS,
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
        if node_type == "lexical_declaration" {
            if node.parent().is_some() && node.parent().unwrap().grammar_name() == "program" {
                let mut output = String::new();
                let kind_node = node.child_by_field_name("kind");
                if let Some(kind_node) = kind_node {
                    output.push_str(&code[kind_node.byte_range()]);
                }
                for child in node.children(&mut node.walk()) {
                    if "variable_declarator" == child.kind() {
                        let name = child.child_by_field_name("name");
                        if let Some(name) = name {
                            output.push_str(" ");
                            output.push_str(&code[name.byte_range()]);
                        }
                    }
                }
                let block_type = CodeBlockType::CONST;
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

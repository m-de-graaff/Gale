//! AST → formatted GaleX source code printer.
//!
//! Walks every AST node and emits formatted text with consistent
//! indentation (2 spaces), brace placement, and spacing.

use crate::ast::*;

/// Internal printer state.
struct Printer {
    buf: String,
    indent: usize,
    at_line_start: bool,
}

const INDENT: &str = "  ";
#[allow(dead_code)]
const MAX_INLINE_LEN: usize = 80;

impl Printer {
    fn new() -> Self {
        Self {
            buf: String::with_capacity(4096),
            indent: 0,
            at_line_start: true,
        }
    }

    fn write(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        if self.at_line_start {
            for _ in 0..self.indent {
                self.buf.push_str(INDENT);
            }
            self.at_line_start = false;
        }
        self.buf.push_str(s);
    }

    fn writeln(&mut self, s: &str) {
        self.write(s);
        self.buf.push('\n');
        self.at_line_start = true;
    }

    fn newline(&mut self) {
        self.buf.push('\n');
        self.at_line_start = true;
    }

    fn indent(&mut self) {
        self.indent += 1;
    }

    fn dedent(&mut self) {
        self.indent = self.indent.saturating_sub(1);
    }

    fn finish(self) -> String {
        self.buf
    }
}

/// Pretty-print a full program.
pub fn print_program(program: &Program) -> String {
    let mut p = Printer::new();
    let mut first = true;
    for item in &program.items {
        if !first {
            p.newline();
        }
        print_item(item, &mut p);
        first = false;
    }
    // Ensure trailing newline
    let mut result = p.finish();
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

// ── Items ──────────────────────────────────────────────────────────────

fn print_item(item: &Item, p: &mut Printer) {
    match item {
        Item::Use(decl) => print_use(decl, p),
        Item::Out(decl) => {
            p.write("out ");
            print_item(&decl.inner, p);
        }
        Item::FnDecl(decl) => print_fn_decl(decl, p),
        Item::GuardDecl(decl) => print_guard(decl, p),
        Item::StoreDecl(decl) => print_store(decl, p),
        Item::ActionDecl(decl) => print_action(decl, p),
        Item::QueryDecl(decl) => print_query(decl, p),
        Item::ChannelDecl(decl) => print_channel(decl, p),
        Item::TypeAlias(decl) => {
            p.write(&format!("type {} = ", decl.name));
            print_type(&decl.ty, p);
            p.newline();
        }
        Item::EnumDecl(decl) => print_enum(decl, p),
        Item::TestDecl(decl) => print_test(decl, p),
        Item::ComponentDecl(decl) => print_component("ui", decl, p),
        Item::LayoutDecl(decl) => print_layout(decl, p),
        Item::ApiDecl(decl) => print_api(decl, p),
        Item::MiddlewareDecl(decl) => print_middleware(decl, p),
        Item::EnvDecl(decl) => print_env(decl, p),
        Item::ServerBlock(block) => print_boundary("server", block, p),
        Item::ClientBlock(block) => print_boundary("client", block, p),
        Item::SharedBlock(block) => print_boundary("shared", block, p),
        Item::Stmt(stmt) => print_stmt(stmt, p),
    }
}

fn print_use(decl: &UseDecl, p: &mut Printer) {
    p.write("use ");
    match &decl.imports {
        ImportKind::Default(name) => p.write(name),
        ImportKind::Named(names) => {
            p.write("{ ");
            for (i, name) in names.iter().enumerate() {
                if i > 0 {
                    p.write(", ");
                }
                p.write(name);
            }
            p.write(" }");
        }
        ImportKind::Star => p.write("*"),
    }
    p.writeln(&format!(" from \"{}\"", decl.path));
}

fn print_fn_decl(decl: &FnDecl, p: &mut Printer) {
    if decl.is_async {
        p.write("async ");
    }
    p.write(&format!("fn {}(", decl.name));
    print_params(&decl.params, p);
    p.write(")");
    if let Some(ref ret) = decl.ret_ty {
        p.write(" -> ");
        print_type(ret, p);
    }
    p.write(" ");
    print_block(&decl.body, p);
    p.newline();
}

fn print_guard(decl: &GuardDecl, p: &mut Printer) {
    p.writeln(&format!("guard {} {{", decl.name));
    p.indent();
    for field in &decl.fields {
        p.write(&format!("{}: ", field.name));
        print_type(&field.ty, p);
        for v in &field.validators {
            p.write(&format!(".{}", v.name));
            if !v.args.is_empty() {
                p.write("(");
                for (i, arg) in v.args.iter().enumerate() {
                    if i > 0 {
                        p.write(", ");
                    }
                    print_expr(arg, p);
                }
                p.write(")");
            } else {
                p.write("()");
            }
        }
        p.newline();
    }
    p.dedent();
    p.writeln("}");
}

fn print_store(decl: &StoreDecl, p: &mut Printer) {
    p.writeln(&format!("store {} {{", decl.name));
    p.indent();
    for member in &decl.members {
        match member {
            StoreMember::Signal(stmt) => print_stmt(stmt, p),
            StoreMember::Derive(stmt) => print_stmt(stmt, p),
            StoreMember::Method(fn_decl) => print_fn_decl(fn_decl, p),
        }
    }
    p.dedent();
    p.writeln("}");
}

fn print_action(decl: &ActionDecl, p: &mut Printer) {
    p.write(&format!("action {}(", decl.name));
    print_params(&decl.params, p);
    p.write(")");
    if let Some(ref ret) = decl.ret_ty {
        p.write(" -> ");
        print_type(ret, p);
    }
    p.write(" ");
    print_block(&decl.body, p);
    p.newline();
}

fn print_query(decl: &QueryDecl, p: &mut Printer) {
    p.write(&format!("query {} = ", decl.name));
    print_expr(&decl.url_pattern, p);
    if let Some(ref ret) = decl.ret_ty {
        p.write(" -> ");
        print_type(ret, p);
    }
    p.newline();
}

fn print_channel(decl: &ChannelDecl, p: &mut Printer) {
    p.write(&format!("channel {}", decl.name));
    if !decl.params.is_empty() {
        p.write("(");
        print_params(&decl.params, p);
        p.write(")");
    }
    let dir = match decl.direction {
        ChannelDirection::ServerToClient => " -> ",
        ChannelDirection::ClientToServer => " <- ",
        ChannelDirection::Bidirectional => " <-> ",
    };
    p.write(dir);
    print_type(&decl.msg_ty, p);
    if decl.handlers.is_empty() {
        p.newline();
    } else {
        p.writeln(" {");
        p.indent();
        for handler in &decl.handlers {
            p.write(&format!("on {}(", handler.event));
            print_params(&handler.params, p);
            p.write(") ");
            print_block(&handler.body, p);
            p.newline();
        }
        p.dedent();
        p.writeln("}");
    }
}

fn print_enum(decl: &EnumDecl, p: &mut Printer) {
    p.writeln(&format!("enum {} {{", decl.name));
    p.indent();
    for (i, variant) in decl.variants.iter().enumerate() {
        p.write(variant.as_str());
        if i < decl.variants.len() - 1 {
            p.write(",");
        }
        p.newline();
    }
    p.dedent();
    p.writeln("}");
}

fn print_test(decl: &TestDecl, p: &mut Printer) {
    p.write(&format!("test \"{}\" ", decl.name));
    print_block(&decl.body, p);
    p.newline();
}

fn print_component(kind: &str, decl: &ComponentDecl, p: &mut Printer) {
    p.write(&format!("{kind} {}", decl.name));
    if !decl.props.is_empty() {
        p.write("(");
        print_params(&decl.props, p);
        p.write(")");
    }
    p.writeln(" {");
    p.indent();
    // Statements
    for stmt in &decl.body.stmts {
        print_stmt(stmt, p);
    }
    if !decl.body.stmts.is_empty() && !decl.body.template.is_empty() {
        p.newline();
    }
    // Template
    print_template_nodes(&decl.body.template, p);
    // Head
    if let Some(ref head) = decl.body.head {
        p.newline();
        print_head(head, p);
    }
    p.dedent();
    p.writeln("}");
}

fn print_layout(decl: &LayoutDecl, p: &mut Printer) {
    p.write(&format!("layout {}", decl.name));
    if !decl.props.is_empty() {
        p.write("(");
        print_params(&decl.props, p);
        p.write(")");
    }
    p.writeln(" {");
    p.indent();
    for stmt in &decl.body.stmts {
        print_stmt(stmt, p);
    }
    if !decl.body.stmts.is_empty() && !decl.body.template.is_empty() {
        p.newline();
    }
    print_template_nodes(&decl.body.template, p);
    if let Some(ref head) = decl.body.head {
        p.newline();
        print_head(head, p);
    }
    p.dedent();
    p.writeln("}");
}

fn print_api(decl: &ApiDecl, p: &mut Printer) {
    p.writeln(&format!("api {} {{", decl.name));
    p.indent();
    for handler in &decl.handlers {
        p.write(&format!("{}", handler.method).to_lowercase());
        for param in &handler.path_params {
            p.write(&format!("[{param}]"));
        }
        if !handler.params.is_empty() {
            p.write("(");
            print_params(&handler.params, p);
            p.write(")");
        }
        if let Some(ref ret) = handler.ret_ty {
            p.write(" -> ");
            print_type(ret, p);
        }
        p.write(" ");
        print_block(&handler.body, p);
        p.newline();
    }
    p.dedent();
    p.writeln("}");
}

fn print_middleware(decl: &MiddlewareDecl, p: &mut Printer) {
    p.write("middleware ");
    match &decl.target {
        MiddlewareTarget::Global => {}
        MiddlewareTarget::PathPrefix(path) => p.write(&format!("for \"{path}\" ")),
        MiddlewareTarget::Resource(name) => p.write(&format!("for {name} ")),
    }
    p.write(&format!("{}(", decl.name));
    print_params(&decl.params, p);
    p.write(") ");
    print_block(&decl.body, p);
    p.newline();
}

fn print_env(decl: &EnvDecl, p: &mut Printer) {
    p.writeln("env {");
    p.indent();
    for var in &decl.vars {
        p.write(&format!("{}: ", var.key));
        print_type(&var.ty, p);
        for v in &var.validators {
            p.write(&format!(".{}", v.name));
            if !v.args.is_empty() {
                p.write("(");
                for (i, arg) in v.args.iter().enumerate() {
                    if i > 0 {
                        p.write(", ");
                    }
                    print_expr(arg, p);
                }
                p.write(")");
            } else {
                p.write("()");
            }
        }
        if let Some(ref default) = var.default {
            p.write(" = ");
            print_expr(default, p);
        }
        p.newline();
    }
    p.dedent();
    p.writeln("}");
}

fn print_boundary(keyword: &str, block: &BoundaryBlock, p: &mut Printer) {
    p.writeln(&format!("{keyword} {{"));
    p.indent();
    for item in &block.items {
        print_item(item, p);
    }
    p.dedent();
    p.writeln("}");
}

fn print_head(head: &HeadBlock, p: &mut Printer) {
    p.writeln("head {");
    p.indent();
    for field in &head.fields {
        p.write(&format!("{}: ", field.key));
        print_expr(&field.value, p);
        p.newline();
    }
    p.dedent();
    p.writeln("}");
}

// ── Statements ─────────────────────────────────────────────────────────

fn print_stmt(stmt: &Stmt, p: &mut Printer) {
    match stmt {
        Stmt::Let {
            name, ty_ann, init, ..
        } => {
            p.write(&format!("let {name}"));
            if let Some(ty) = ty_ann {
                p.write(": ");
                print_type(ty, p);
            }
            p.write(" = ");
            print_expr(init, p);
            p.newline();
        }
        Stmt::Mut {
            name, ty_ann, init, ..
        } => {
            p.write(&format!("mut {name}"));
            if let Some(ty) = ty_ann {
                p.write(": ");
                print_type(ty, p);
            }
            p.write(" = ");
            print_expr(init, p);
            p.newline();
        }
        Stmt::Signal {
            name, ty_ann, init, ..
        } => {
            p.write(&format!("signal {name}"));
            if let Some(ty) = ty_ann {
                p.write(": ");
                print_type(ty, p);
            }
            p.write(" = ");
            print_expr(init, p);
            p.newline();
        }
        Stmt::Derive { name, init, .. } => {
            p.write(&format!("derive {name} = "));
            print_expr(init, p);
            p.newline();
        }
        Stmt::Frozen { name, init, .. } => {
            p.write(&format!("frozen {name} = "));
            print_expr(init, p);
            p.newline();
        }
        Stmt::RefDecl { name, ty_ann, .. } => {
            p.write(&format!("ref {name}: "));
            print_type(ty_ann, p);
            p.newline();
        }
        Stmt::FnDecl(decl) => print_fn_decl(decl, p),
        Stmt::If {
            condition,
            then_block,
            else_branch,
            ..
        } => {
            p.write("if ");
            print_expr(condition, p);
            p.write(" ");
            print_block(then_block, p);
            if let Some(branch) = else_branch {
                match branch {
                    ElseBranch::Else(block) => {
                        p.write(" else ");
                        print_block(block, p);
                    }
                    ElseBranch::ElseIf(stmt) => {
                        p.write(" else ");
                        print_stmt(stmt, p);
                        return; // ElseIf handles its own newline
                    }
                }
            }
            p.newline();
        }
        Stmt::For {
            binding,
            index,
            iterable,
            body,
            ..
        } => {
            p.write(&format!("for {binding}"));
            if let Some(idx) = index {
                p.write(&format!(", {idx}"));
            }
            p.write(" in ");
            print_expr(iterable, p);
            p.write(" ");
            print_block(body, p);
            p.newline();
        }
        Stmt::Return { value, .. } => {
            p.write("return");
            if let Some(val) = value {
                p.write(" ");
                print_expr(val, p);
            }
            p.newline();
        }
        Stmt::Effect { body, .. } => {
            p.write("effect ");
            print_block(body, p);
            p.newline();
        }
        Stmt::Watch {
            target,
            next_name,
            prev_name,
            body,
            ..
        } => {
            p.write("watch ");
            print_expr(target, p);
            p.write(&format!(" as ({next_name}, {prev_name}) "));
            print_block(body, p);
            p.newline();
        }
        Stmt::ExprStmt { expr, .. } => {
            print_expr(expr, p);
            p.newline();
        }
        Stmt::Block(block) => {
            print_block(block, p);
            p.newline();
        }
    }
}

fn print_block(block: &Block, p: &mut Printer) {
    p.writeln("{");
    p.indent();
    for stmt in &block.stmts {
        print_stmt(stmt, p);
    }
    p.dedent();
    p.write("}");
}

// ── Expressions ────────────────────────────────────────────────────────

fn print_expr(expr: &Expr, p: &mut Printer) {
    match expr {
        Expr::IntLit { value, .. } => p.write(&value.to_string()),
        Expr::FloatLit { value, .. } => p.write(&value.to_string()),
        Expr::StringLit { value, .. } => p.write(&format!("\"{value}\"")),
        Expr::BoolLit { value, .. } => p.write(if *value { "true" } else { "false" }),
        Expr::NullLit { .. } => p.write("null"),
        Expr::RegexLit { pattern, flags, .. } => p.write(&format!("/{pattern}/{flags}")),
        Expr::TemplateLit { parts, .. } => {
            p.write("`");
            for part in parts {
                match part {
                    TemplatePart::Text(t) => p.write(t),
                    TemplatePart::Expr(e) => {
                        p.write("${");
                        print_expr(e, p);
                        p.write("}");
                    }
                }
            }
            p.write("`");
        }
        Expr::Ident { name, .. } => p.write(name),
        Expr::BinaryOp {
            left, op, right, ..
        } => {
            print_expr(left, p);
            p.write(&format!(" {} ", format_binop(op)));
            print_expr(right, p);
        }
        Expr::UnaryOp { op, operand, .. } => {
            p.write(match op {
                UnaryOp::Neg => "-",
                UnaryOp::Not => "!",
            });
            print_expr(operand, p);
        }
        Expr::Ternary {
            condition,
            then_expr,
            else_expr,
            ..
        } => {
            print_expr(condition, p);
            p.write(" ? ");
            print_expr(then_expr, p);
            p.write(" : ");
            print_expr(else_expr, p);
        }
        Expr::NullCoalesce { left, right, .. } => {
            print_expr(left, p);
            p.write(" ?? ");
            print_expr(right, p);
        }
        Expr::FnCall { callee, args, .. } => {
            print_expr(callee, p);
            p.write("(");
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    p.write(", ");
                }
                print_expr(arg, p);
            }
            p.write(")");
        }
        Expr::MemberAccess { object, field, .. } => {
            print_expr(object, p);
            p.write(&format!(".{field}"));
        }
        Expr::OptionalChain { object, field, .. } => {
            print_expr(object, p);
            p.write(&format!("?.{field}"));
        }
        Expr::IndexAccess { object, index, .. } => {
            print_expr(object, p);
            p.write("[");
            print_expr(index, p);
            p.write("]");
        }
        Expr::ArrayLit { elements, .. } => {
            p.write("[");
            for (i, el) in elements.iter().enumerate() {
                if i > 0 {
                    p.write(", ");
                }
                print_expr(el, p);
            }
            p.write("]");
        }
        Expr::ObjectLit { fields, .. } => {
            p.write("{ ");
            for (i, f) in fields.iter().enumerate() {
                if i > 0 {
                    p.write(", ");
                }
                p.write(&format!("{}: ", f.key));
                print_expr(&f.value, p);
            }
            p.write(" }");
        }
        Expr::ArrowFn { params, body, .. } => {
            if params.len() == 1 && params[0].ty_ann.is_none() {
                p.write(params[0].name.as_str());
            } else {
                p.write("(");
                print_params(params, p);
                p.write(")");
            }
            p.write(" => ");
            match body {
                ArrowBody::Expr(e) => print_expr(e, p),
                ArrowBody::Block(b) => print_block(b, p),
            }
        }
        Expr::Spread { expr: inner, .. } => {
            p.write("...");
            print_expr(inner, p);
        }
        Expr::Range { start, end, .. } => {
            print_expr(start, p);
            p.write("..");
            print_expr(end, p);
        }
        Expr::Pipe { left, right, .. } => {
            print_expr(left, p);
            p.write(" |> ");
            print_expr(right, p);
        }
        Expr::Await { expr: inner, .. } => {
            p.write("await ");
            print_expr(inner, p);
        }
        Expr::Assign {
            target, op, value, ..
        } => {
            print_expr(target, p);
            p.write(match op {
                AssignOp::Assign => " = ",
                AssignOp::AddAssign => " += ",
                AssignOp::SubAssign => " -= ",
            });
            print_expr(value, p);
        }
        Expr::Assert { expr: inner, .. } => {
            p.write("assert ");
            print_expr(inner, p);
        }
        Expr::EnvAccess { key, .. } => {
            p.write(&format!("env.{key}"));
        }
    }
}

fn format_binop(op: &BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Eq => "==",
        BinOp::NotEq => "!=",
        BinOp::Lt => "<",
        BinOp::Gt => ">",
        BinOp::LtEq => "<=",
        BinOp::GtEq => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
        BinOp::DotDot => "..",
    }
}

// ── Types ──────────────────────────────────────────────────────────────

fn print_type(ty: &TypeAnnotation, p: &mut Printer) {
    match ty {
        TypeAnnotation::Named { name, .. } => p.write(name),
        TypeAnnotation::Array { element, .. } => {
            print_type(element, p);
            p.write("[]");
        }
        TypeAnnotation::Union { types, .. } => {
            for (i, t) in types.iter().enumerate() {
                if i > 0 {
                    p.write(" | ");
                }
                print_type(t, p);
            }
        }
        TypeAnnotation::Optional { inner, .. } => {
            print_type(inner, p);
            p.write("?");
        }
        TypeAnnotation::StringLiteral { value, .. } => p.write(&format!("\"{value}\"")),
        TypeAnnotation::Function { params, ret, .. } => {
            p.write("fn(");
            for (i, param) in params.iter().enumerate() {
                if i > 0 {
                    p.write(", ");
                }
                print_type(param, p);
            }
            p.write(") -> ");
            print_type(ret, p);
        }
        TypeAnnotation::Tuple { elements, .. } => {
            p.write("(");
            for (i, el) in elements.iter().enumerate() {
                if i > 0 {
                    p.write(", ");
                }
                print_type(el, p);
            }
            p.write(")");
        }
        TypeAnnotation::Object { fields, .. } => {
            p.write("{ ");
            for (i, f) in fields.iter().enumerate() {
                if i > 0 {
                    p.write(", ");
                }
                p.write(f.name.as_str());
                if f.optional {
                    p.write("?");
                }
                p.write(": ");
                print_type(&f.ty, p);
            }
            p.write(" }");
        }
    }
}

// ── Template ───────────────────────────────────────────────────────────

fn print_template_nodes(nodes: &[TemplateNode], p: &mut Printer) {
    for node in nodes {
        print_template_node(node, p);
    }
}

fn print_template_node(node: &TemplateNode, p: &mut Printer) {
    match node {
        TemplateNode::Element {
            tag,
            attributes,
            directives,
            children,
            ..
        } => {
            p.write(&format!("<{tag}"));
            print_attrs(attributes, directives, p);
            p.writeln(">");
            if !children.is_empty() {
                p.indent();
                print_template_nodes(children, p);
                p.dedent();
            }
            p.writeln(&format!("</{tag}>"));
        }
        TemplateNode::SelfClosing {
            tag,
            attributes,
            directives,
            ..
        } => {
            p.write(&format!("<{tag}"));
            print_attrs(attributes, directives, p);
            p.writeln(" />");
        }
        TemplateNode::Text { value, .. } => {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                p.writeln(trimmed);
            }
        }
        TemplateNode::ExprInterp { expr, .. } => {
            p.write("{");
            print_expr(expr, p);
            p.writeln("}");
        }
        TemplateNode::When {
            condition,
            body,
            else_branch,
            ..
        } => {
            p.write("when ");
            print_expr(condition, p);
            p.writeln(" {");
            p.indent();
            print_template_nodes(body, p);
            p.dedent();
            p.write("}");
            if let Some(branch) = else_branch {
                match branch {
                    WhenElse::Else(nodes) => {
                        p.writeln(" else {");
                        p.indent();
                        print_template_nodes(nodes, p);
                        p.dedent();
                        p.writeln("}");
                    }
                    WhenElse::ElseWhen(node) => {
                        p.write(" else ");
                        print_template_node(node, p);
                    }
                }
            } else {
                p.newline();
            }
        }
        TemplateNode::Each {
            binding,
            index,
            iterable,
            body,
            empty,
            ..
        } => {
            p.write(&format!("each {binding}"));
            if let Some(idx) = index {
                p.write(&format!(", {idx}"));
            }
            p.write(" in ");
            print_expr(iterable, p);
            p.writeln(" {");
            p.indent();
            print_template_nodes(body, p);
            p.dedent();
            p.write("}");
            if let Some(empty_nodes) = empty {
                p.writeln(" empty {");
                p.indent();
                print_template_nodes(empty_nodes, p);
                p.dedent();
                p.writeln("}");
            } else {
                p.newline();
            }
        }
        TemplateNode::Suspend { body, .. } => {
            p.writeln("suspend {");
            p.indent();
            print_template_nodes(body, p);
            p.dedent();
            p.writeln("}");
        }
        TemplateNode::Slot { name, .. } => {
            if let Some(n) = name {
                p.writeln(&format!("<slot name=\"{n}\" />"));
            } else {
                p.writeln("<slot />");
            }
        }
    }
}

fn print_attrs(attributes: &[Attribute], directives: &[Directive], p: &mut Printer) {
    for attr in attributes {
        p.write(&format!(" {}", attr.name));
        match &attr.value {
            AttrValue::String(val) => p.write(&format!("=\"{val}\"")),
            AttrValue::Expr(expr) => {
                p.write("={");
                print_expr(expr, p);
                p.write("}");
            }
            AttrValue::Bool => {} // bare attribute
        }
    }
    for dir in directives {
        match dir {
            Directive::Bind { field, expr, .. } => {
                if let Some(e) = expr {
                    if let Expr::Ident { name, .. } = e.as_ref() {
                        p.write(&format!(" bind:{field}={{{name}}}"));
                    } else {
                        p.write(&format!(" bind:{field}={{...}}"));
                    }
                } else {
                    p.write(&format!(" bind:{field}"));
                }
            }
            Directive::On {
                event,
                modifiers,
                handler,
                ..
            } => {
                p.write(&format!(" on:{event}"));
                for m in modifiers {
                    p.write(&format!(".{m}"));
                }
                p.write("={");
                print_expr(handler, p);
                p.write("}");
            }
            Directive::Class {
                name, condition, ..
            } => {
                p.write(&format!(" class:{name}"));
                if !matches!(condition, Expr::BoolLit { value: true, .. }) {
                    p.write("={");
                    print_expr(condition, p);
                    p.write("}");
                }
            }
            Directive::Ref { name, .. } => p.write(&format!(" ref:{name}")),
            Directive::Transition { kind, .. } => p.write(&format!(" transition:{kind}")),
            Directive::Key { expr, .. } => {
                p.write(" key={");
                print_expr(expr, p);
                p.write("}");
            }
            Directive::Into { slot, .. } => p.write(&format!(" into:{slot}")),
            Directive::FormAction { action, .. } => {
                p.write(" form:action={");
                print_expr(action, p);
                p.write("}");
            }
            Directive::FormGuard { guard, .. } => {
                p.write(" form:guard={");
                print_expr(guard, p);
                p.write("}");
            }
            Directive::FormError { field, .. } => {
                p.write(&format!(" form:error=\"{field}\""));
            }
            Directive::Prefetch { mode, .. } => {
                p.write(&format!(" prefetch=\"{mode}\""));
            }
        }
    }
}

// ── Params ─────────────────────────────────────────────────────────────

fn print_params(params: &[Param], p: &mut Printer) {
    for (i, param) in params.iter().enumerate() {
        if i > 0 {
            p.write(", ");
        }
        p.write(param.name.as_str());
        if let Some(ref ty) = param.ty_ann {
            p.write(": ");
            print_type(ty, p);
        }
        if let Some(ref default) = param.default {
            p.write(" = ");
            print_expr(default, p);
        }
    }
}

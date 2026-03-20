//! GaleX Abstract Syntax Tree — complete node definitions.
//!
//! This module defines every AST node type for the GaleX language.
//! The parser (Phase 9) will produce these nodes from the token stream.
//! The type checker (Phase 10) will walk these nodes to infer and verify types.
//!
//! Every node carries a [`Span`] for error reporting.

use crate::span::Span;
use smol_str::SmolStr;

// ── Program ────────────────────────────────────────────────────────────

/// A complete GaleX source file.
#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
    pub span: Span,
}

/// A top-level item in a GaleX file.
#[derive(Debug, Clone)]
pub enum Item {
    /// `use X from "path"` or `use { A, B } from "path"`
    Use(UseDecl),
    /// `out ...` — exported declaration
    Out(OutDecl),
    /// `fn name(...) -> T { ... }`
    FnDecl(FnDecl),
    /// `guard Name { ... }`
    GuardDecl(GuardDecl),
    /// `store Name { ... }`
    StoreDecl(StoreDecl),
    /// `action name(...) -> T { ... }`
    ActionDecl(ActionDecl),
    /// `query name = ... -> T`
    QueryDecl(QueryDecl),
    /// `channel name(...) <-> T { ... }`
    ChannelDecl(ChannelDecl),
    /// `type Name = ...`
    TypeAlias(TypeAliasDecl),
    /// `enum Name { ... }`
    EnumDecl(EnumDecl),
    /// `test "name" { ... }`
    TestDecl(TestDecl),
    /// `out ui Name(...) { ... }`
    ComponentDecl(ComponentDecl),
    /// `out layout Name(...) { ... }`
    LayoutDecl(LayoutDecl),
    /// `out api ResourceName { get() {...}, post(body: T) {...}, ... }`
    ApiDecl(ApiDecl),
    /// `middleware name(req: Request, next: Next) -> Response { ... }`
    MiddlewareDecl(MiddlewareDecl),
    /// `env { KEY: type.validator(), ... }`
    EnvDecl(EnvDecl),
    /// `server { ... }`
    ServerBlock(BoundaryBlock),
    /// `client { ... }`
    ClientBlock(BoundaryBlock),
    /// `shared { ... }`
    SharedBlock(BoundaryBlock),
    /// Any statement at the top level
    Stmt(Stmt),
}

// ── Declarations ───────────────────────────────────────────────────────

/// `use X from "path"` or `use { A, B } from "path"`
#[derive(Debug, Clone)]
pub struct UseDecl {
    pub imports: ImportKind,
    pub path: SmolStr,
    pub span: Span,
}

/// What is being imported.
#[derive(Debug, Clone)]
pub enum ImportKind {
    /// `use Foo from "..."`
    Default(SmolStr),
    /// `use { Foo, Bar } from "..."`
    Named(Vec<SmolStr>),
    /// `use * from "..."`
    Star,
}

/// `out <visibility> <inner>`
#[derive(Debug, Clone)]
pub struct OutDecl {
    pub inner: Box<Item>,
    pub span: Span,
}

/// `fn name(params) -> ret { body }`
#[derive(Debug, Clone)]
pub struct FnDecl {
    pub name: SmolStr,
    pub params: Vec<Param>,
    pub ret_ty: Option<TypeAnnotation>,
    pub body: Block,
    pub is_async: bool,
    pub span: Span,
}

/// `guard Name { field: type.validator() ... }`
#[derive(Debug, Clone)]
pub struct GuardDecl {
    pub name: SmolStr,
    pub fields: Vec<GuardFieldDecl>,
    pub span: Span,
}

/// A single field in a guard declaration.
#[derive(Debug, Clone)]
pub struct GuardFieldDecl {
    pub name: SmolStr,
    pub ty: TypeAnnotation,
    pub validators: Vec<ValidatorCall>,
    pub span: Span,
}

/// A chained validator call: `.email()`, `.min(2)`, `.max(100)`
#[derive(Debug, Clone)]
pub struct ValidatorCall {
    pub name: SmolStr,
    pub args: Vec<Expr>,
    pub span: Span,
}

/// `store Name { signals, derives, methods }`
#[derive(Debug, Clone)]
pub struct StoreDecl {
    pub name: SmolStr,
    pub members: Vec<StoreMember>,
    pub span: Span,
}

/// A member inside a store declaration.
#[derive(Debug, Clone)]
pub enum StoreMember {
    Signal(Stmt),
    Derive(Stmt),
    Method(FnDecl),
}

/// `action name(params) -> ret { body }`
#[derive(Debug, Clone)]
pub struct ActionDecl {
    pub name: SmolStr,
    pub params: Vec<Param>,
    pub ret_ty: Option<TypeAnnotation>,
    pub body: Block,
    pub span: Span,
}

/// `query name = url_pattern -> ret_ty`
#[derive(Debug, Clone)]
pub struct QueryDecl {
    pub name: SmolStr,
    pub url_pattern: Expr,
    pub ret_ty: Option<TypeAnnotation>,
    pub span: Span,
}

/// `channel name(params) <direction> MsgType { handlers }`
#[derive(Debug, Clone)]
pub struct ChannelDecl {
    pub name: SmolStr,
    pub params: Vec<Param>,
    pub direction: ChannelDirection,
    pub msg_ty: TypeAnnotation,
    pub handlers: Vec<ChannelHandler>,
    pub span: Span,
}

/// `on connect(emit) { ... }` or `on receive(msg) { ... }`
#[derive(Debug, Clone)]
pub struct ChannelHandler {
    pub event: SmolStr,
    pub params: Vec<Param>,
    pub body: Block,
    pub span: Span,
}

/// Channel data flow direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelDirection {
    /// `->` server to client
    ServerToClient,
    /// `<-` client to server
    ClientToServer,
    /// `<->` bidirectional
    Bidirectional,
}

/// `type Name = TypeAnnotation`
#[derive(Debug, Clone)]
pub struct TypeAliasDecl {
    pub name: SmolStr,
    pub ty: TypeAnnotation,
    pub span: Span,
}

/// `enum Name { Variant, Variant, ... }`
#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub name: SmolStr,
    pub variants: Vec<SmolStr>,
    pub span: Span,
}

/// `test "description" { body }`
#[derive(Debug, Clone)]
pub struct TestDecl {
    pub name: SmolStr,
    pub body: Block,
    pub span: Span,
}

/// `out ui Name(props) { template_body }`
#[derive(Debug, Clone)]
pub struct ComponentDecl {
    pub name: SmolStr,
    pub props: Vec<Param>,
    pub body: ComponentBody,
    pub span: Span,
}

/// The body of a component — mix of code and template nodes.
#[derive(Debug, Clone)]
pub struct ComponentBody {
    pub stmts: Vec<Stmt>,
    pub template: Vec<TemplateNode>,
    /// Optional `head { ... }` block for setting page metadata.
    pub head: Option<HeadBlock>,
    pub span: Span,
}

/// `out layout Name(props) { template_body }`
///
/// A layout wraps pages — its template MUST contain a `<slot/>` node
/// where page content will be injected during SSR.
#[derive(Debug, Clone)]
pub struct LayoutDecl {
    pub name: SmolStr,
    pub props: Vec<Param>,
    pub body: ComponentBody,
    pub span: Span,
}

// ── API routes ─────────────────────────────────────────────────────────

/// HTTP methods supported by API route handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::Get => write!(f, "GET"),
            HttpMethod::Post => write!(f, "POST"),
            HttpMethod::Put => write!(f, "PUT"),
            HttpMethod::Patch => write!(f, "PATCH"),
            HttpMethod::Delete => write!(f, "DELETE"),
        }
    }
}

/// A single HTTP method handler within an API resource block.
///
/// ```text
/// get[id](params: SearchParams) -> User { ... }
///  ^   ^       ^                    ^       ^
///  |   |       |                    |       body
///  |   |       params               ret_ty
///  |   path_params
///  method
/// ```
#[derive(Debug, Clone)]
pub struct ApiHandler {
    /// The HTTP method (GET, POST, PUT, PATCH, DELETE).
    pub method: HttpMethod,
    /// Path parameters: `get[id]` → `vec!["id"]`, `get[year][month]` → `vec!["year", "month"]`.
    pub path_params: Vec<SmolStr>,
    /// Typed parameters (query params for GET, request body for POST/PUT/PATCH).
    pub params: Vec<Param>,
    /// Return type annotation.
    pub ret_ty: Option<TypeAnnotation>,
    /// Handler body.
    pub body: Block,
    pub span: Span,
}

/// `out api ResourceName { handlers... }`
///
/// A REST resource with multiple HTTP method handlers. Route paths are
/// derived from the resource name using kebab-case with an `/api/` prefix.
#[derive(Debug, Clone)]
pub struct ApiDecl {
    pub name: SmolStr,
    pub handlers: Vec<ApiHandler>,
    pub span: Span,
}

// ── Middleware ──────────────────────────────────────────────────────────

/// Where a middleware applies in the route hierarchy.
#[derive(Debug, Clone)]
pub enum MiddlewareTarget {
    /// Applies to all routes (no `for` clause).
    Global,
    /// Applies to routes under a path prefix: `for "/api"`.
    PathPrefix(SmolStr),
    /// Applies to a named resource or component: `for Users`.
    Resource(SmolStr),
}

/// `middleware name(req: Request, next: Next) -> Response { ... }`
///
/// A middleware function that intercepts requests/responses in the
/// route handling pipeline. The body can inspect/modify the request,
/// call `next(req)` to continue the chain, and inspect/modify the
/// response before returning it.
#[derive(Debug, Clone)]
pub struct MiddlewareDecl {
    pub name: SmolStr,
    /// What routes this middleware applies to.
    pub target: MiddlewareTarget,
    /// Parameters — typically `(req: Request, next: Next)`.
    pub params: Vec<Param>,
    /// Middleware body.
    pub body: Block,
    pub span: Span,
}

// ── Environment variables ───────────────────────────────────────────────

/// A single env var definition: `KEY: type.validator()`
#[derive(Debug, Clone)]
pub struct EnvVarDef {
    /// The environment variable name, e.g. `"DATABASE_URL"`.
    pub key: SmolStr,
    /// The declared type (`string`, `int`, `float`, `bool`).
    pub ty: TypeAnnotation,
    /// Validators applied to the value (e.g. `nonEmpty()`, `min(1)`).
    pub validators: Vec<ValidatorCall>,
    /// Default value if the variable is not set in the environment.
    pub default: Option<Expr>,
    pub span: Span,
}

/// `env { vars... }`
///
/// Declares expected environment variables with types, validators, and
/// optional defaults. Variables are loaded at startup, validated with
/// fail-fast semantics, and exposed as typed fields on a static `Env` struct.
#[derive(Debug, Clone)]
pub struct EnvDecl {
    pub vars: Vec<EnvVarDef>,
    pub span: Span,
}

/// `server { ... }` / `client { ... }` / `shared { ... }`
#[derive(Debug, Clone)]
pub struct BoundaryBlock {
    pub items: Vec<Item>,
    pub span: Span,
}

// ── Statements ─────────────────────────────────────────────────────────

/// A statement.
#[derive(Debug, Clone)]
pub enum Stmt {
    /// `let name: ty = init`
    Let {
        name: SmolStr,
        ty_ann: Option<TypeAnnotation>,
        init: Expr,
        span: Span,
    },
    /// `mut name: ty = init`
    Mut {
        name: SmolStr,
        ty_ann: Option<TypeAnnotation>,
        init: Expr,
        span: Span,
    },
    /// `signal name: ty = init`
    Signal {
        name: SmolStr,
        ty_ann: Option<TypeAnnotation>,
        init: Expr,
        span: Span,
    },
    /// `derive name = expr`
    Derive {
        name: SmolStr,
        init: Expr,
        span: Span,
    },
    /// `frozen name = expr`
    Frozen {
        name: SmolStr,
        init: Expr,
        span: Span,
    },
    /// `ref name: Type`
    RefDecl {
        name: SmolStr,
        ty_ann: TypeAnnotation,
        span: Span,
    },
    /// Inline function declaration (also a statement)
    FnDecl(FnDecl),
    /// `if condition { ... } else { ... }`
    If {
        condition: Expr,
        then_block: Block,
        else_branch: Option<ElseBranch>,
        span: Span,
    },
    /// `for binding in iterable { ... }`
    For {
        binding: SmolStr,
        index: Option<SmolStr>,
        iterable: Expr,
        body: Block,
        span: Span,
    },
    /// `return expr`
    Return { value: Option<Expr>, span: Span },
    /// `effect { ... }`
    Effect {
        body: Block,
        cleanup: Option<Block>,
        span: Span,
    },
    /// `watch expr as (next, prev) { ... }`
    Watch {
        target: Expr,
        next_name: SmolStr,
        prev_name: SmolStr,
        body: Block,
        span: Span,
    },
    /// Expression used as statement
    ExprStmt { expr: Expr, span: Span },
    /// `{ ... }` block
    Block(Block),
}

/// An else branch: either `else { ... }` or `else if ... { ... }`.
#[derive(Debug, Clone)]
pub enum ElseBranch {
    Else(Block),
    ElseIf(Box<Stmt>),
}

/// A block of statements.
#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

// ── Expressions ────────────────────────────────────────────────────────

/// An expression.
#[derive(Debug, Clone)]
pub enum Expr {
    /// `42`
    IntLit { value: i64, span: Span },
    /// `3.14`
    FloatLit { value: f64, span: Span },
    /// `"hello"`
    StringLit { value: SmolStr, span: Span },
    /// `true` / `false`
    BoolLit { value: bool, span: Span },
    /// `null`
    NullLit { span: Span },
    /// `/pattern/flags`
    RegexLit {
        pattern: SmolStr,
        flags: SmolStr,
        span: Span,
    },
    /// `` `Hello, ${name}!` ``
    TemplateLit {
        parts: Vec<TemplatePart>,
        span: Span,
    },
    /// A variable or type name reference
    Ident { name: SmolStr, span: Span },
    /// `left op right`
    BinaryOp {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
        span: Span,
    },
    /// `op operand` (prefix)
    UnaryOp {
        op: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },
    /// `condition ? then : else_`
    Ternary {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
        span: Span,
    },
    /// `left ?? right`
    NullCoalesce {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    /// `callee(args...)`
    FnCall {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    /// `object.field`
    MemberAccess {
        object: Box<Expr>,
        field: SmolStr,
        span: Span,
    },
    /// `object?.field`
    OptionalChain {
        object: Box<Expr>,
        field: SmolStr,
        span: Span,
    },
    /// `object[index]`
    IndexAccess {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    /// `[elem, elem, ...]`
    ArrayLit { elements: Vec<Expr>, span: Span },
    /// `{ key: value, ... }`
    ObjectLit {
        fields: Vec<ObjectFieldExpr>,
        span: Span,
    },
    /// `(params) => body`
    ArrowFn {
        params: Vec<Param>,
        ret_ty: Option<TypeAnnotation>,
        body: ArrowBody,
        span: Span,
    },
    /// `...expr`
    Spread { expr: Box<Expr>, span: Span },
    /// `start..end`
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        span: Span,
    },
    /// `left |> right`
    Pipe {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    /// `await expr`
    Await { expr: Box<Expr>, span: Span },
    /// `target = value` or `target += value`
    Assign {
        target: Box<Expr>,
        op: AssignOp,
        value: Box<Expr>,
        span: Span,
    },
    /// `assert expr` (inside test blocks)
    Assert { expr: Box<Expr>, span: Span },
    /// `env.KEY`
    EnvAccess { key: SmolStr, span: Span },
}

/// A segment of a template literal.
#[derive(Debug, Clone)]
pub enum TemplatePart {
    /// Static text segment.
    Text(SmolStr),
    /// `${expr}` interpolation.
    Expr(Expr),
}

/// `{ key: value }` in an object literal.
#[derive(Debug, Clone)]
pub struct ObjectFieldExpr {
    pub key: SmolStr,
    pub value: Expr,
    pub span: Span,
}

/// The body of an arrow function — either an expression or a block.
#[derive(Debug, Clone)]
pub enum ArrowBody {
    Expr(Box<Expr>),
    Block(Block),
}

// ── Operators ──────────────────────────────────────────────────────────

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,    // +
    Sub,    // -
    Mul,    // *
    Div,    // /
    Mod,    // %
    Eq,     // ==
    NotEq,  // !=
    Lt,     // <
    Gt,     // >
    LtEq,   // <=
    GtEq,   // >=
    And,    // &&
    Or,     // ||
    DotDot, // ..
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg, // -x
    Not, // !x
}

/// Assignment operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,    // =
    AddAssign, // +=
    SubAssign, // -=
}

// ── Template nodes ─────────────────────────────────────────────────────

/// A node in the component template.
#[derive(Debug, Clone)]
pub enum TemplateNode {
    /// `<tag attrs...>children</tag>`
    Element {
        tag: SmolStr,
        attributes: Vec<Attribute>,
        directives: Vec<Directive>,
        children: Vec<TemplateNode>,
        span: Span,
    },
    /// `<tag attrs... />`
    SelfClosing {
        tag: SmolStr,
        attributes: Vec<Attribute>,
        directives: Vec<Directive>,
        span: Span,
    },
    /// `"text content"`
    Text { value: SmolStr, span: Span },
    /// `{expr}` interpolation
    ExprInterp { expr: Expr, span: Span },
    /// `when condition { body } else { ... }`
    When {
        condition: Expr,
        body: Vec<TemplateNode>,
        else_branch: Option<WhenElse>,
        span: Span,
    },
    /// `each item, index in list { body } empty { fallback }`
    Each {
        binding: SmolStr,
        index: Option<SmolStr>,
        iterable: Expr,
        body: Vec<TemplateNode>,
        empty: Option<Vec<TemplateNode>>,
        span: Span,
    },
    /// `suspend fallback={...} { body }`
    Suspend {
        fallback: Option<Box<TemplateNode>>,
        body: Vec<TemplateNode>,
        span: Span,
    },
    /// `slot name { default_content }`
    Slot {
        name: Option<SmolStr>,
        default: Option<Vec<TemplateNode>>,
        span: Span,
    },
}

/// Else branch of a `when` block.
#[derive(Debug, Clone)]
pub enum WhenElse {
    /// `else { nodes }`
    Else(Vec<TemplateNode>),
    /// `else when condition { nodes } ...`
    ElseWhen(Box<TemplateNode>),
}

/// An HTML attribute: `name="value"` or `name={expr}`.
#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: SmolStr,
    pub value: AttrValue,
    pub span: Span,
}

/// The value of an attribute.
#[derive(Debug, Clone)]
pub enum AttrValue {
    /// `"string"`
    String(SmolStr),
    /// `{expr}`
    Expr(Expr),
    /// Boolean attribute (no value): `disabled`
    Bool,
}

/// A template directive.
#[derive(Debug, Clone)]
pub enum Directive {
    /// `bind:field={expr}` — two-way binding between DOM property and signal/variable.
    /// `field` is the DOM property (e.g. `value`), `expr` is the source (e.g. signal name).
    Bind {
        field: SmolStr,
        expr: Option<Box<Expr>>,
        span: Span,
    },
    /// `on:event.modifier={handler}`
    On {
        event: SmolStr,
        modifiers: Vec<SmolStr>,
        handler: Expr,
        span: Span,
    },
    /// `class:name={condition}`
    Class {
        name: SmolStr,
        condition: Expr,
        span: Span,
    },
    /// `ref:name`
    Ref { name: SmolStr, span: Span },
    /// `transition:type={config}`
    Transition {
        kind: SmolStr,
        config: Option<Expr>,
        span: Span,
    },
    /// `key={expr}`
    Key { expr: Expr, span: Span },
    /// `into:slot`
    Into { slot: SmolStr, span: Span },
    /// `form:action={action}`
    FormAction { action: Expr, span: Span },
    /// `form:guard={guard}`
    FormGuard { guard: Expr, span: Span },
    /// `form:error field="name"`
    FormError { field: SmolStr, span: Span },
    /// `prefetch="mode"`
    Prefetch { mode: SmolStr, span: Span },
}

// ── Type annotations ───────────────────────────────────────────────────

/// A type annotation as written in source code.
///
/// These are resolved to [`TypeId`](crate::types::ty::TypeId) during type checking.
#[derive(Debug, Clone)]
pub enum TypeAnnotation {
    /// A named type: `string`, `int`, `User`, `HTMLElement`
    Named { name: SmolStr, span: Span },
    /// `T[]` — array type
    Array {
        element: Box<TypeAnnotation>,
        span: Span,
    },
    /// `T | U` — union type
    Union {
        types: Vec<TypeAnnotation>,
        span: Span,
    },
    /// `T?` — optional type (sugar for `T | null`)
    Optional {
        inner: Box<TypeAnnotation>,
        span: Span,
    },
    /// `"primary" | "ghost"` — string literal union
    StringLiteral { value: SmolStr, span: Span },
    /// `fn(params) -> ret` — function type
    Function {
        params: Vec<TypeAnnotation>,
        ret: Box<TypeAnnotation>,
        span: Span,
    },
    /// `(T, U, V)` — tuple type
    Tuple {
        elements: Vec<TypeAnnotation>,
        span: Span,
    },
    /// `{ key: Type, ... }` — object type
    Object {
        fields: Vec<ObjectTypeField>,
        span: Span,
    },
}

/// A field in an object type annotation.
#[derive(Debug, Clone)]
pub struct ObjectTypeField {
    pub name: SmolStr,
    pub ty: TypeAnnotation,
    pub optional: bool,
    pub span: Span,
}

// ── Function parameters ────────────────────────────────────────────────

/// A function or component parameter.
#[derive(Debug, Clone)]
pub struct Param {
    pub name: SmolStr,
    pub ty_ann: Option<TypeAnnotation>,
    pub default: Option<Expr>,
    pub span: Span,
}

// ── Head block ─────────────────────────────────────────────────────────

/// `head { title: "...", description: "...", og: { ... } }`
#[derive(Debug, Clone)]
pub struct HeadBlock {
    pub fields: Vec<HeadField>,
    pub span: Span,
}

/// A field inside a `head` block.
#[derive(Debug, Clone)]
pub struct HeadField {
    pub key: SmolStr,
    pub value: Expr,
    pub span: Span,
}

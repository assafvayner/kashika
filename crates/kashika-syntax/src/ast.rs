use kashika_diag::Span;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct File {
    pub stmts: Vec<Stmt>,
    /// Where the first statement starts, whether or not it parsed.
    pub first_stmt: Option<Span>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Diagram(Diagram),
    Style(StyleDef),
    Box(Node),
    Group(Group),
    Junction(Junction),
    Text(Text),
    Line(Line),
    Import(Import),
    Shape(ShapeDef),
    Image(Image),
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Diagram(d) => d.span,
            Stmt::Style(s) => s.span,
            Stmt::Box(n) => n.span,
            Stmt::Group(g) => g.node.span,
            Stmt::Junction(j) => j.span,
            Stmt::Text(t) => t.span,
            Stmt::Line(l) => l.span,
            Stmt::Import(i) => i.span,
            Stmt::Shape(s) => s.span,
            Stmt::Image(i) => i.span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagram {
    pub title: String,
    pub size: (f64, f64),
    pub attrs: Vec<Attr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StyleDef {
    pub name: String,
    pub attrs: Vec<Attr>,
    pub span: Span,
}

/// A `box`, or the header of a `group`.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: Ident,
    pub label: Option<String>,
    pub at: Point,
    pub size: (f64, f64),
    pub attrs: Vec<Attr>,
    /// A box's `{ }` text block; always `None` on a group header.
    pub block: Option<Vec<BlockText>>,
    pub span: Span,
}

/// A `text` item inside a box's text block. `at` is in the box-local frame.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockText {
    pub content: String,
    pub at: Option<Point>,
    pub attrs: Vec<Attr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub node: Node,
    pub children: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Junction {
    pub id: Ident,
    pub at: Point,
    pub attrs: Vec<Attr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    pub id: Option<Ident>,
    pub content: String,
    pub at: Point,
    pub attrs: Vec<Attr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub id: Option<Ident>,
    pub start: Point,
    pub segs: Vec<Seg>,
    pub attrs: Vec<Attr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    pub path: String,
    pub path_span: Span,
    /// The `as <ns>` namespace.
    pub ns: Option<Ident>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShapeDef {
    pub name: Ident,
    pub body: ShapeBody,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShapeBody {
    /// `shape <name> { path "..." [outline N] [text-area (x,y,w,h)] }`; each value keeps its span.
    Paths {
        path: (String, Span),
        outline: Option<(f64, Span)>,
        text_area: Option<([f64; 4], Span)>,
    },
    /// `shape <name> from "<file.svg>" [attrs]`.
    From {
        file: String,
        file_span: Span,
        attrs: Vec<Attr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub id: Ident,
    pub file: String,
    pub file_span: Span,
    pub at: Point,
    pub size: (f64, f64),
    pub attrs: Vec<Attr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attr {
    pub key: Ident,
    /// The `t` of `label@t=...`.
    pub at_t: Option<f64>,
    pub value: Value,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Flag,
    Num(f64),
    Str(String),
    Color(String),
    /// An identifier, or a `:`/`,`/`.`-joined list of them, e.g. `cross:l2,near` or `aws.s3`.
    Word(String),
    Pair(f64, f64),
    /// `side@t`, accepted only as the value of `pin`.
    AnchorAt(String, f64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Point {
    Lit {
        x: Coord,
        y: Coord,
        span: Span,
    },
    Ref {
        r: Ref,
        offset: Option<(f64, f64)>,
        span: Span,
    },
}

impl Point {
    pub fn span(&self) -> Span {
        match self {
            Point::Lit { span, .. } | Point::Ref { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Coord {
    Num(f64),
    Axis { r: Ref, axis: Axis },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ref {
    pub id: Ident,
    pub anchor: Option<Anchor>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Anchor {
    pub name: Ident,
    pub t: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Seg {
    pub kind: SegKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum SegKind {
    To(Point),
    H(f64),
    V(f64),
    X(Target),
    Y(Target),
    Hv(Point),
    Vh(Point),
    Curve { side: Side, amount: f64, to: Point },
    CurveVia { c1: Point, c2: Point, to: Point },
}

#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum Target {
    Num(f64),
    Point(Point),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

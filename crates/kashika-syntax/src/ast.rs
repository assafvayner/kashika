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
    /// An identifier, or a `:`/`,`-joined list of them, e.g. `cross:l2,near`.
    Word(String),
    Pair(f64, f64),
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

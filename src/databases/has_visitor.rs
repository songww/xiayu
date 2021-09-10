pub trait HasVisitor<'a> {
    type Visitor: crate::visitors::Visitor<'a>;
    fn visitor() -> Self::Visitor;
}

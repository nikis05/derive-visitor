use std::any::Any;

use syn::Type;

enum Op {
    Enter,
    Exit,
}

trait Visitor {
    fn drive(&mut self, item: &dyn Any, op: Op);
}

trait Walk: Any {
    fn walk<V: Visitor>(&self, visitor: &mut V);
}

impl<T: Walk> Walk for Vec<T> {
    fn walk<V: Visitor>(&self, visitor: &mut V) {
        self.iter().for_each(|item| item.walk(visitor))
    }
}

// #[derive(Walk)]
struct Schema {
    definitions: Vec<Definition>,
    // #[walk(with="reverse_vec_walker")]
    something: Vec<Definition>,
}

impl Walk for Schema {
    fn walk<V: Visitor>(&self, visitor: &mut V) {
        visitor.drive(self, Op::Enter);
        Walk::walk(&self.definitions, visitor);
        reverse_vec_walker(&self.something, visitor);
        visitor.drive(self, Op::Exit);
    }
}

// #[derive(Walk)]
enum Definition {
    Type(TypeDefinition),
    // #[walk(skip)]
    Directive(DirectiveDefinition),
    Complex {
        foo: TypeDefinition,
        bar: TypeDefinition,
    },
}

impl Walk for Definition {
    fn walk<V: Visitor>(&self, visitor: &mut V) {
        visitor.drive(self, Op::Enter);
        match self {
            Self::Type(item) => Walk::walk(item, visitor),
            Self::Complex { foo, bar } => Walk::walk(foo, visitor),
            Self::Directive(_) => {}
        }
        visitor.drive(self, Op::Exit);
    }
}

// #[derive(Walk)]
struct TypeDefinition;

impl Walk for TypeDefinition {
    fn walk<V: Visitor>(&self, visitor: &mut V) {
        visitor.drive(self, Op::Enter);
        visitor.drive(self, Op::Exit);
    }
}

// #[derive(Walk)]
struct DirectiveDefinition;

impl Walk for DirectiveDefinition {
    fn walk<V: Visitor>(&self, visitor: &mut V) {
        visitor.drive(self, Op::Enter);
        visitor.drive(self, Op::Exit);
    }
}

// #[derive(Visitor)]
// #[visitor(TypeDefinition, Schema(enter))]
struct SchemaVisitor;

impl Visitor for SchemaVisitor {
    fn drive(&mut self, item: &dyn std::any::Any, op: Op) {
        if let Some(downcast) = <dyn Any>::downcast_ref::<TypeDefinition>(item) {
            match op {
                Op::Enter => self.enter_type_definition(downcast),
                Op::Exit => self.exit_type_definition(downcast),
            }
        }
    }
}

impl SchemaVisitor {
    fn enter_type_definition(&mut self, type_definition: &TypeDefinition) {}
    fn exit_type_definition(&mut self, type_definition: &TypeDefinition) {}
}

fn reverse_vec_walker<T: Walk, V: Visitor>(item: &Vec<T>, visitor: &mut V) {
    item.iter().rev().for_each(|item| item.walk(visitor))
}

fn test() {
    let mut visitor = SchemaVisitor;
    let schema = Schema {
        definitions: vec![],
        something: vec![],
    };
    schema.walk(&mut visitor);
}

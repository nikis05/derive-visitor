use derive_visitor::{Visitor, Walk};

fn main() {
    println!("Hello, world!");
}

#[derive(Walk)]
struct Schema {
    definitions: Vec<Definition>,
    #[walk(with = "reverse_vec_walker")]
    something: Vec<Definition>,
}

#[derive(Walk)]
enum Definition {
    Type(TypeDefinition),
    // #[walk(skip)]
    Directive(DirectiveDefinition),
    Complex {
        foo: TypeDefinition,
        bar: TypeDefinition,
    },
}
#[derive(Walk)]
struct TypeDefinition;

#[derive(Walk)]
struct DirectiveDefinition;

#[derive(Visitor)]
#[visitor(
    TypeDefinition,
    Schema(enter),
    DirectiveDefinition(enter = "my_enter_directive", exit = "my_exit_directive")
)]
struct SchemaVisitor;

impl SchemaVisitor {
    fn enter_type_definition(&mut self, type_definition: &TypeDefinition) {}
    fn exit_type_definition(&mut self, type_definition: &TypeDefinition) {}
    fn enter_schema(&mut self, schema: &Schema) {}
    fn my_enter_directive(&mut self, directive: &DirectiveDefinition) {}
    fn my_exit_directive(&mut self, directive: &DirectiveDefinition) {}
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

This crate derives [visitor pattern](https://rust-unofficial.github.io/patterns/patterns/behavioural/visitor.html)
for arbitrary data structures. This pattern is particularly useful when dealing with complex nested data structures,
abstract trees and hierarchies of all kinds.

The main building blocks of this crate are two derivable traits:

- [Visitor](https://docs.rs/derive-visitor/0.1.1/derive_visitor/trait.Visitor.html) implementations walk through a data structures and accumulates some information;
- [Drive](https://docs.rs/derive-visitor/0.1.1/derive_visitor/trait.Drive.html) implementations are data structures that know how to drive a visitor through themselves.

Please refer to these traits' documentation for more details.

## Example

```rust
use derive_visitor::{Visitor, Drive};

#[derive(Drive)]
struct Directory {
    #[drive(skip)]
    name: String,
    items: Vec<DirectoryItem>,
}

#[derive(Drive)]
enum DirectoryItem {
    File(File),
    Directory(Directory),
}

#[derive(Drive)]
struct File {
    #[drive(skip)]
    name: String,
}

#[derive(Visitor, Default)]
#[visitor(File(enter), Directory(enter))]
struct Counter {
    files: u32,
    directories: u32
}

impl Counter {
    fn enter_file(&mut self, _file: &File) {
        self.files += 1;
    }
    fn enter_directory(&mut self, _directory: &Directory) {
        self.directories += 1;
    }
}

let mut counter = Counter::default();

let example_directory = Directory {
    name: "root".into(),
    items: vec![
        DirectoryItem::Directory(
            Directory {
                name: "home".into(),
                items: vec![
                    DirectoryItem::File(File { name: "README.md".into() }),
                    DirectoryItem::File(File { name: "Star Wars.mov".into() })
                ]
            }
        ),
        DirectoryItem::Directory(
            Directory { name: "downloads".into(), items: vec![] }
        )
    ],
};

example_directory.drive(&mut counter);

assert_eq!(counter.files, 2);
assert_eq!(counter.directories, 3);
```

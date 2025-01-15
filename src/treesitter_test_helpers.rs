#![cfg(test)]

use tree_sitter::Node;

pub fn find_item_nodes<'tree>(root: Node<'tree>, kind: &str) -> Vec<Node<'tree>> {
    let mut cursor = root.walk();
    root.children(&mut cursor)
        .filter(|node| node.kind() == kind)
        .collect()
}

pub fn find_item_node<'tree>(root: Node<'tree>, kind: &str) -> Node<'tree> {
    let nodes = find_item_nodes(root, kind);
    match nodes.len() {
        1 => nodes[0],
        0 => panic!("No node found with kind {}", kind),
        _ => panic!("Multiple nodes found with kind {}", kind),
    }
}

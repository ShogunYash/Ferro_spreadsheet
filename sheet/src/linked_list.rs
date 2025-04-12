// linked_list.rs

#[derive(Debug, Clone)]
pub struct Node {
    pub key: i32,
    pub next: Option<Box<Node>>,
}

impl Node {
    // Create a new node
    pub fn new(key: i32) -> Self {
        Node { key, next: None }
    }

    // Prepend a new node to the list
    pub fn prepend(list: Option<Box<Node>>, key: i32) -> Option<Box<Node>> {
        Some(Box::new(Node {
            key,
            next: list,
        }))
    }

    // Remove a node with the given key
    pub fn remove(head: &mut Option<Box<Node>>, key: i32) -> bool {
        if let Some(node) = head {
            if node.key == key {
                *head = node.next.take();
                return true;
            }
            
            let mut current = head;
            while let Some(node) = current {
                // Check if the next node is the one we want to remove
                if let Some(next) = &node.next {
                    if next.key == key {
                        // Take the next node's next and replace it
                        let mut removed = node.next.take().unwrap();
                        node.next = removed.next.take();
                        return true;
                    }
                }
                current = &mut node.next;
            }
        }
        false
    }
}
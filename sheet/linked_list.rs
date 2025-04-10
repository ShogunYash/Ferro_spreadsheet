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

    // Append a new node to the end of the list
    pub fn append(mut head: &mut Option<Box<Node>>, key: i32) {
        match head {
            None => {
                *head = Some(Box::new(Node::new(key)));
            }
            Some(node) => {
                let mut current = node;
                while let Some(ref mut next_node) = current.next {
                    current = next_node;
                }
                current.next = Some(Box::new(Node::new(key)));
            }
        }
    }

    // Remove a node with the given key
    pub fn remove(head: &mut Option<Box<Node>>, key: i32) -> bool {
        if let Some(node) = head {
            if node.key == key {
                *head = node.next.take();
                return true;
            }
    
            let mut current = node;
            while let Some(mut next_node) = current.next.take() {
                if next_node.key == key {
                    current.next = next_node.next.take();
                    return true;
                } else {
                    current.next = Some(next_node); // re-attach the node if not removed
                    current = current.next.as_mut().unwrap(); // safe unwrap since we just put it back
                }
            }
        }
        false
    }
    

    // Check if a key exists in the list
    pub fn contains(head: &Option<Box<Node>>, key: i32) -> bool {
        let mut current = head;
        while let Some(node) = current {
            if node.key == key {
                return true;
            }
            current = &node.next;
        }
        false
    }
    
    // Find node with the given key
    pub fn find(head: &Option<Box<Node>>, key: i32) -> Option<&Node> {
        let mut current = head;
        while let Some(node) = current {
            if node.key == key {
                return Some(node);
            }
            current = &node.next;
        }
        None
    }

    // Convert the linked list to a vector for testing or debugging
    pub fn to_vec(&self) -> Vec<i32> {
        let mut result = vec![self.key];
        let mut current = &self.next;
        while let Some(node) = current {
            result.push(node.key);
            current = &node.next;
        }
        result
    }
    
    // Get the length of the linked list
    pub fn len(head: &Option<Box<Node>>) -> usize {
        let mut count = 0;
        let mut current = head;
        
        while let Some(node) = current {
            count += 1;
            current = &node.next;
        }
        
        count
    }
    
    // Create a linked list from a vector
    pub fn from_vec(values: &[i32]) -> Option<Box<Node>> {
        if values.is_empty() {
            return None;
        }
        
        let mut head = Some(Box::new(Node::new(values[0])));
        for &value in &values[1..] {
            Node::append(&mut head, value);
        }
        
        head
    }
}
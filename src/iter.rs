use std::iter::Peekable;
use crate::node::{Node, NodeEdgesValueIter};

// At this point, dfs is only used when iterating through labels
// values iteration not currently supported
pub type LabelsIter<'a, K, V> = NodeDFSIter<'a, K, V>;
type ItemsIter<'a, K, V> = Peekable<Box<NodeEdgesValueIter<'a, K, V>>>;


// Wraps two variants into a single unified iteration enum type
// Single type allows for iteration regardless if type is a Node ref or Iter type of Nodes
#[derive(Debug)]
enum IterType<'a, K, V> {
    Item(&'a Node<K, V>),
    Iter(ItemsIter<'a, K, V>),
}

// Handles DFS iteration using a stack and top level element
#[derive(Debug)]
pub struct NodeDFSIter<'a, K, V> {
    current: Option<IterType<'a, K, V>>,
    unvisited: Vec<IterType<'a, K, V>>,
}

impl<'a, K: 'a, V: 'a> Default for NodeDFSIter<'a, K, V> {
    fn default() -> Self {
        NodeDFSIter {
            current: None,
            unvisited: vec![],
        }
    }
}

// NodeDFSIter methods

impl<'a, K: 'a, V: 'a> NodeDFSIter<'a, K, V> {
    pub fn new(node: &'a Node<K, V>) -> NodeDFSIter<'a, K, V> {
        NodeDFSIter {
            current: Some(IterType::Item(node)),
            unvisited: Vec::new(),
        }
    }

    pub fn empty() -> NodeDFSIter<'a, K, V> {
        NodeDFSIter::default()
    }

    // Helper method to add an iter of nodes
    fn add_iter(&mut self, mut iter: ItemsIter<'a, K, V>) {
        if let Some(n) = iter.next() {
            self.current = Some(IterType::Item(n));
            // Peek to ensure another element available in order to push
            if let Some(_) = iter.peek() {
                self.unvisited.push(IterType::Iter(iter))
            }
        }
    }
}


impl<'a, K: 'a, V: 'a> Iterator for NodeDFSIter<'a, K, V> {
    type Item = &'a [u8];
    fn next(&mut self) -> Option<Self::Item> {
        let mut iter: ItemsIter<K, V>;

        // Loop handles producing concrete next value
        // even if literal next type is node or node iter
        loop {
            match self.current.take() {
                // if stack empty switch to current
                None => match self.unvisited.pop() {
                    Some(last) => self.current = Some(last),
                    None => break None,
                },
                // Handle current if node item
                Some(IterType::Item(n)) => {
                    iter = Box::new(n.edges_values_iter()).peekable();
                    self.add_iter(iter);

                    // Don't add root label
                    if n.label().is_some() {
                        break n.label()
                    }
                },
                // Handle current if node iter
                Some(IterType::Iter(iter)) => {
                    self.add_iter(iter)
                }
            }
        }
    }
}

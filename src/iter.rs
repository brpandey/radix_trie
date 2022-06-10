use std::iter::Peekable;
use crate::node::{Node, NodeEdgesValueIter};

pub type LabelsIter<'a, K, V> = NodeDFSIter<'a, K, V>;
type ItemsIter<'a, K, V> = Peekable<Box<NodeEdgesValueIter<'a, K, V>>>;

#[derive(Debug)]
enum IterType<'a, K, V> {
    Item(&'a Node<K, V>),
    Iter(ItemsIter<'a, K, V>),
}

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


    fn add_iter(&mut self, mut iter: ItemsIter<'a, K, V>) {
        if let Some(n) = iter.next() {
            self.current = Some(IterType::Item(n));
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

        loop {
            match self.current.take() {
                None => match self.unvisited.pop() {
                    Some(last) => self.current = Some(last),
                    None => break None,
                },
                Some(IterType::Item(n)) => {
                    iter = Box::new(n.edges_values_iter()).peekable();
                    self.add_iter(iter);
                    if n.label().is_some() {
                        break n.label()
                    }
                },
                Some(IterType::Iter(iter)) => {
                    self.add_iter(iter)
                }
            }
        }
    }
}

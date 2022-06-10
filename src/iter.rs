use std::iter::Peekable;
use crate::node::{Node, NodeEdgesValueIter};

pub type LabelsIter<'a, K> = NodeDFSIter<'a, K>;
type ItemsIter<'a, K> = Peekable<Box<NodeEdgesValueIter<'a, K>>>;

#[derive(Debug)]
enum IterType<'a, K> {
    Item(&'a Node<K>),
    Iter(ItemsIter<'a, K>),
}

pub struct NodeDFSIter<'a, K> {
    current: Option<IterType<'a, K>>,
    unvisited: Vec<IterType<'a, K>>,
}

impl<'a, K: 'a> Default for NodeDFSIter<'a, K> {
    fn default() -> Self {
        NodeDFSIter {
            current: None,
            unvisited: vec![],
        }
    }
}

// NodeDFSIter methods

impl<'a, K: 'a> NodeDFSIter<'a, K> {
    pub fn new(node: &'a Node<K>) -> NodeDFSIter<'a, K> {
        NodeDFSIter {
            current: Some(IterType::Item(node)),
            unvisited: Vec::new(),
        }
    }

    pub fn empty() -> NodeDFSIter<'a, K> {
        NodeDFSIter::default()
    }


    fn add_iter(&mut self, mut iter: ItemsIter<'a, K>) {
        if let Some(n) = iter.next() {
            self.current = Some(IterType::Item(n));
            if let Some(_) = iter.peek() {
                self.unvisited.push(IterType::Iter(iter))
            }
        }
    }
}

impl<'a, K: 'a> Iterator for NodeDFSIter<'a, K> {
    type Item = &'a [u8];
    fn next(&mut self) -> Option<Self::Item> {
        let mut iter: ItemsIter<K>;

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

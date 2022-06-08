use std::iter::Peekable;
use crate::node::{Node, NodeEdgesValueIter};

pub type LabelsIter<'a> = NodeDFSIter<'a>;
type ItemsIter<'a> = Peekable<Box<NodeEdgesValueIter<'a>>>;

#[derive(Debug)]
enum IterType<'a>{
    Item(&'a Node),
    Iter(ItemsIter<'a>),
}

pub struct NodeDFSIter<'a> {
    current: Option<IterType<'a>>,
    unvisited: Vec<IterType<'a>>,
}

impl<'a> Default for NodeDFSIter<'a> {
    fn default() -> Self {
        NodeDFSIter {
            current: None,
            unvisited: vec![],
        }
    }
}


impl<'a> NodeDFSIter<'a> {
    pub fn new(node: &'a Node) -> NodeDFSIter<'a> {
        NodeDFSIter {
            current: Some(IterType::Item(node)),
            unvisited: Vec::new(),
        }
    }

    pub fn empty() -> NodeDFSIter<'a> {
        NodeDFSIter::default()
    }
}


// NodeDFSIter methods

impl<'a> NodeDFSIter<'a> {
    fn add_iter(&mut self, mut iter: ItemsIter<'a>) {
        if let Some(n) = iter.next() {
            self.current = Some(IterType::Item(n));
            if let Some(_) = iter.peek() {
                self.unvisited.push(IterType::Iter(iter))
            }
        }
    }
}

impl<'a> Iterator for NodeDFSIter<'a> {
    type Item = &'a [u8];
    fn next(&mut self) -> Option<Self::Item> {
        let mut iter: ItemsIter;

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

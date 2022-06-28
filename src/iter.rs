#![allow(dead_code)]

use std::iter::Peekable;
use crate::node::{Node, NodeEdgesValueIter, NodeEdgesValueIterMut};
use crate::macros::enum_extract;

// At this point, dfs is the basis for all iteration types
// Iteration types are implemented as new types (kudos Haskell)
// around a dfs ref nodes or ref mut nodes struct

pub struct LabelsIter<'a, K, V> (NodeDFSIter<'a, K, V>);
pub struct ValuesIter<'a, K, V>(NodeDFSIter<'a, K, V>);
pub struct ValuesIterMut<'a, K, V>(NodeDFSIterMut<'a, K, V>);
pub struct IntoIter<'a, K, V>(NodeDFSIterMut<'a, K, V>);

type ItemsIter<'a, K, V> = Peekable<NodeEdgesValueIter<'a, K, V>>;
type ItemsIterMut<'a, K, V> = Peekable<NodeEdgesValueIterMut<'a, K, V>>;


#[derive(Copy, Clone, Debug)]
enum IterationType {
    Labels,
    Values,
    ValuesMut,
    ValuesOwned,
}

#[derive(Debug)]
enum NextType<'a, V> {
    LabelRef(Option<&'a [u8]>),
    // LabelsMutRef not supported as would violate Trie integrity just like HashMap, BTreeMap, etc..
    ValueRef(Option<&'a V>),
    ValueRefMut(Option<&'a mut V>),
    ValueOwned(Option<V>),
}


// Wraps variants into a single unified iteration enum type
// Single type allows for iteration regardless if type is a Node ref or Iter type of Nodes
// Note: Mut variants not included in a separate mut variants enum to decrease fragmentation

#[derive(Debug)]
enum IterUnified<'a, K, V> {
    Item(&'a Node<K, V>),
    Iter(ItemsIter<'a, K, V>),
    ItemMut(&'a mut Node<K, V>),
    IterMut(ItemsIterMut<'a, K, V>),
}

/*-----------------------------------------------------------------------*/
// Handles DFS iteration using a stack and top level element
#[derive(Debug)]
pub struct NodeDFSIter<'a, K, V> {
    current: Option<IterUnified<'a, K, V>>,
    unvisited: Vec<IterUnified<'a, K, V>>,
}

// Handles DFS mut iteration using a stack and top level element
#[derive(Debug)]
pub struct NodeDFSIterMut<'a, K, V> {
    current: Option<IterUnified<'a, K, V>>,
    unvisited: Vec<IterUnified<'a, K, V>>,
}


impl<'a, K: 'a, V: 'a> Default for NodeDFSIter<'a, K, V> {
    fn default() -> Self {
        NodeDFSIter {
            current: None,
            unvisited: vec![],
        }
    }
}

impl<'a, K: 'a, V: 'a> Default for NodeDFSIterMut<'a, K, V> {
    fn default() -> Self {
        NodeDFSIterMut {
            current: None,
            unvisited: vec![],
        }
    }
}

/*-----------------------------------------------------------------------*/
// NodeDFSIter methods

impl<'a, K: 'a, V: 'a> NodeDFSIter<'a, K, V> {
    pub fn new(node: &'a Node<K, V>) -> NodeDFSIter<'a, K, V> {
        NodeDFSIter {
            current: Some(IterUnified::Item(node)),
            unvisited: Vec::new(),
        }
    }

    // Helper method to add an iter of nodes
    fn add_iter(&mut self, mut iter: ItemsIter<'a, K, V>) {
        if let Some(n) = iter.next() {
            self.current = Some(IterUnified::Item(n));
            // Peek to ensure another element available in order to push
            if iter.peek().is_some() {
                self.unvisited.push(IterUnified::Iter(iter))
            }
        }
    }

    fn next(&mut self, itype: IterationType) -> Option<NextType<'a, V>> {
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
                Some(IterUnified::Item(n)) => {
                    iter = n.edges_values_iter().peekable();
                    self.add_iter(iter);

                    match itype {
                        IterationType::Labels => {
                            // Don't add root label which is none
                            if n.label().is_some() {
                                break Some(NextType::LabelRef(n.label()))
                            }
                        },
                        IterationType::Values => {
                            // Only pass nodes that have values
                            if n.value().is_some() {
                                break Some(NextType::ValueRef(n.value()))
                            }
                        },
                        _ => unreachable!()
                    }
                },
                // Handle current if node iter
                Some(IterUnified::Iter(iter)) => {
                    self.add_iter(iter)
                },
                _ => unreachable!()
            }
        }
    }
}



/*-----------------------------------------------------------------------*/
// Handle mut dfs iterations
impl<'a, K: 'a, V: 'a> NodeDFSIterMut<'a, K, V> {
    pub fn new(node: &'a mut Node<K, V>) -> NodeDFSIterMut<'a, K, V> {
        NodeDFSIterMut {
            current: Some(IterUnified::ItemMut(node)),
            unvisited: Vec::new(),
        }
    }

    // Helper method to add an iter of nodes
    fn add_iter(&mut self, mut iter: ItemsIterMut<'a, K, V>) {
        if let Some(n) = iter.next() {
            self.current = Some(IterUnified::ItemMut(n));
            // Peek to ensure another element available in order to push
            if iter.peek().is_some() {
                self.unvisited.push(IterUnified::IterMut(iter))
            }
        }
    }

    fn next(&mut self, itype: IterationType) -> Option<NextType<'a, V>> {
        let iter: ItemsIterMut<K, V>;

        // Loop handles producing concrete next value
        // even if literal next type is node or node iter
        loop {
            let temp = self.current.take();
            match temp {
                // if stack empty switch to current
                None => match self.unvisited.pop() {
                    Some(last) => self.current = Some(last),
                    None => break None,
                },
                // Handle current if node item
                Some(IterUnified::ItemMut(n)) => {

                    /*-------------------------------------------------------------------------------------------------*/
                    //TODO
                    //Can refactor the NodeDFSIter data structure
                    //the fields: current, unvisited can be reducing down to a single stack field unvisted
                    //since vector implements the Extend trait which takes an iterator as an argument to
                    //extend the stack!

                    // Hack for now, to get around borrow checker concerns about exclusive mutable access!
                    // Had to mark node struct fields: "value" and "edges" as pub(crate)  -- not completely ideal
                    // Borrow checker is smart enough to know that different struct fields can be re-borrowed (as mutable)
                    // In that mutable access (a write) to one won't affect another
                    /*-------------------------------------------------------------------------------------------------*/

                    let edges = &mut n.edges;
                    iter = edges.values_mut().peekable();
                    self.add_iter(iter);

                    let v = &mut n.value;

                    match itype {
                        IterationType::ValuesMut => {
                            break Some(NextType::ValueRefMut(v.as_deref_mut())) // n.value_mut()))
                        },
                        IterationType::ValuesOwned => {
                            break Some(NextType::ValueOwned(v.take().map(|b| *b))) // n.take_value()))
                        },
                        _ => unreachable!()
                    }
                },
                // Handle current if node iter mut
                Some(IterUnified::IterMut(iter)) => {
                    self.add_iter(iter)
                },
                _ => unreachable!()
            }
        }
    }
}



/*-----------------------------------------------------------------------*/
// Default trait implementations for custom iter types
impl<'a, K: 'a, V: 'a> Default for LabelsIter<'a, K, V> {
    fn default() -> Self {
        LabelsIter(NodeDFSIter::default())
    }
}

impl<'a, K: 'a, V: 'a> Default for ValuesIter<'a, K, V> {
    fn default() -> Self {
        ValuesIter(NodeDFSIter::default())
    }
}

impl<'a, K: 'a, V: 'a> Default for ValuesIterMut<'a, K, V> {
    fn default() -> Self {
        ValuesIterMut(NodeDFSIterMut::default())
    }
}

impl<'a, K: 'a, V: 'a> Default for IntoIter<'a, K, V> {
    fn default() -> Self {
        IntoIter(NodeDFSIterMut::default())
    }
}

/*-----------------------------------------------------------------------*/
// Implementations for custom iterator types which leverage base iterator
impl<'a, K: 'a, V: 'a> LabelsIter<'a, K, V> {
    pub fn new(node: &'a Node<K, V>) -> LabelsIter<'a, K, V> {
        LabelsIter(NodeDFSIter::new(node))
    }
}

impl<'a, K: 'a, V: 'a> ValuesIter<'a, K, V> {
    pub fn new(node: &'a Node<K, V>) -> ValuesIter<'a, K, V> {
        ValuesIter(NodeDFSIter::new(node))
    }
}

impl<'a, K: 'a, V: 'a> ValuesIterMut<'a, K, V> {
    pub fn new(node: &'a mut Node<K, V>) -> ValuesIterMut<'a, K, V> {
        ValuesIterMut(NodeDFSIterMut::new(node))
    }
}

impl<'a, K: 'a, V: 'a> IntoIter<'a, K, V> {
    pub fn new(node: &'a mut Node<K, V>) -> IntoIter<'a, K, V> {
        IntoIter(NodeDFSIterMut::new(node))
    }
}

/*-----------------------------------------------------------------------*/
// Iterator trait impl for custom iterator types which leverage base iterator
impl<'a, K: 'a, V: 'a> Iterator for LabelsIter<'a, K, V> {
    type Item = &'a [u8];
    fn next(&mut self) -> Option<&'a [u8]> {
        let result = self.0.next(IterationType::Labels);
        result.and_then(|r| enum_extract!(r, NextType::LabelRef))
    }
}

impl<'a, K: 'a, V: 'a> Iterator for ValuesIter<'a, K, V> {
    type Item = &'a V;
    fn next(&mut self) -> Option<Self::Item> {
        let result = self.0.next(IterationType::Values);
        result.and_then(|r| enum_extract!(r, NextType::ValueRef))
    }
}


impl<'a, K: 'a, V: 'a> Iterator for ValuesIterMut<'a, K, V> {
    type Item = &'a mut V;
    fn next(&mut self) -> Option<&'a mut V> {
        let result = self.0.next(IterationType::ValuesMut);
        result.and_then(|r| enum_extract!(r, NextType::ValueRefMut))
    }
}


impl<'a, K, V> Iterator for IntoIter<'_, K, V> {
    type Item = V;
    fn next(&mut self) -> Option<Self::Item> {
        let result = self.0.next(IterationType::ValuesOwned);
        result.and_then(|r| enum_extract!(r, NextType::ValueOwned))
    }
}


/*
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
                Some(IterUnified::Item(n)) => {
                    iter = Box::new(n.edges_values_iter()).peekable();
                    self.add_iter(iter);

                    // Don't add root label
                    if n.label().is_some() {
                        break n.label()
                    }
                },
                // Handle current if node iter
                Some(IterUnified::Iter(iter)) => {
                    self.add_iter(iter)
                }
            }
        }
    }
}

 */


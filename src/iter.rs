#![allow(dead_code)]

use crate::node::{Node};
use crate::macros::enum_extract;

// At this point, dfs is the basis for all iteration types
// Iteration types are implemented as new types (kudos Haskell)
// around a dfs ref nodes or ref mut nodes struct

#[derive(Clone, Debug)]
pub struct LabelsIter<'a, K, V> (NodeDFSIter<'a, K, V>);

#[derive(Clone, Debug)]
pub struct ValuesIter<'a, K, V>(NodeDFSIter<'a, K, V>);

#[derive(Debug)]
pub struct ValuesIterMut<'a, K, V>(NodeDFSIterMut<'a, K, V>);

#[derive(Clone, Debug)]
pub struct LeafPairsIter<'a, K, V> (NodeDFSIter<'a, K, V>);

#[derive(Debug)]
pub struct LeafPairsIterMut<'a, K, V> (NodeDFSIterMut<'a, K, V>);

#[derive(Clone, Debug)]
pub struct IntoIter<K, V>(NodeDFSIterOwned<K, V>);

#[derive(Copy, Clone, Debug)]
enum IterationType {
    Labels,
    Values,
    ValuesMut,
    LabelsValues,
    LabelsValuesMut,
    ValuesOwned,
}

#[derive(Debug)]
enum NextType<'a, V> {
    LabelRef(Option<&'a [u8]>),
    // LabelsMutRef not supported as would violate Trie integrity just like HashMap, BTreeMap, etc..
    ValueRef(Option<&'a V>),
    ValueRefMut(Option<&'a mut V>),
    ValueOwned(Option<V>),
    LeafPairRef(Option<(&'a [u8], &'a V)>),
    LeafPairRefMut(Option<(&'a [u8], &'a mut V)>),
}


// Wraps variants into a single unified iteration enum type
// NOTE: These wrappers aren't entirely necessary now that the variants have been removed

#[derive(Clone, Debug)]
enum IterUnified<'a, K, V> {
    Item(&'a Node<K, V>),
}

#[derive(Debug)]
enum IterUnifiedMut<'a, K, V> {
    ItemMut(&'a mut Node<K, V>),
}


/*-----------------------------------------------------------------------*/
// Handles DFS iteration using a stack and total size
#[derive(Clone, Debug)]
pub struct NodeDFSIter<'a, K, V> {
    stack: Vec<IterUnified<'a, K, V>>,
    size: usize,
}

// Handles DFS mut iteration using a stack and total size
#[derive(Debug)]
pub struct NodeDFSIterMut<'a, K, V> {
    stack: Vec<IterUnifiedMut<'a, K, V>>,
    size: usize,
}

// Handles DFS iteration by value using a stack and total size
#[derive(Clone, Debug)]
pub struct NodeDFSIterOwned<K, V> {
    stack: Vec<Node<K, V>>,
}

impl<'a, K: 'a, V: 'a> Default for NodeDFSIter<'a, K, V> {
    fn default() -> Self {
        NodeDFSIter {
            stack: vec![],
            size: 0,
        }
    }
}

impl<'a, K: 'a, V: 'a> Default for NodeDFSIterMut<'a, K, V> {
    fn default() -> Self {
        NodeDFSIterMut {
            stack: vec![],
            size: 0,
        }
    }
}

impl<K, V> Default for NodeDFSIterOwned<K, V> {
    fn default() -> Self {
        NodeDFSIterOwned {
            stack: vec![],
        }
    }
}

//-----------------------------------------------------------------------
// NodeDFSIter methods

impl<'a, K: 'a, V: 'a> NodeDFSIter<'a, K, V> {

    pub fn new(node: &'a Node<K, V>, size: usize) -> NodeDFSIter<'a, K, V> {
        NodeDFSIter {
            stack: vec![IterUnified::Item(node)],
            size,
        }
    }

    // Next method leverages vector's extend trait implementation to add an entire iteration
    // of outgoing edge nodes instead of having to handle the case of specific item or iter
    fn next(&mut self, itype: IterationType) -> Option<NextType<'a, V>> {
        loop {
            match self.stack.pop() {
                None => break None,
                Some(IterUnified::Item(n)) => {
                    self.stack.extend(n.edges_values_iter().map(|b| IterUnified::Item(&*b)));

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
                        IterationType::LabelsValues => {
                            // Pass leaf nodes that have a label and a value
                            if n.label().is_some() && n.value().is_some() {
                                break Some(NextType::LeafPairRef(Some((n.label().unwrap(), n.value().unwrap()))))
                            }
                        },
                        _ => unreachable!()
                    }
                },
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.size, Some(self.size))
    }
}

/*-----------------------------------------------------------------------*/
// Handle mut dfs iterations
impl<'a, K: 'a, V: 'a> NodeDFSIterMut<'a, K, V> {
    pub fn new(node: &'a mut Node<K, V>, size: usize) -> NodeDFSIterMut<'a, K, V> {
        NodeDFSIterMut {
            stack: vec![IterUnifiedMut::ItemMut(node)],
            size,
        }
    }

    // Next method leverages vector's extend trait implementation to add an entire iteration
    // of outgoing edge nodes instead of having to handle the case of specific item or iter
    fn next(&mut self, itype: IterationType) -> Option<NextType<'a, V>> {
        loop {
            match self.stack.pop() {
                None => break None,
                Some(IterUnifiedMut::ItemMut(n)) => {

                    /*-------------------------------------------------------------------------------------------------*/
                    //TODO
                    // Hack for now, to get around borrow checker concerns about exclusive mutable access!
                    // Had to mark node struct fields: "value" and "edges" as pub(crate)  -- not completely ideal
                    // Borrow checker is smart enough to know that different struct fields can be re-borrowed (as mutable)
                    // In that mutable access (a write) to one won't affect another
                    /*-------------------------------------------------------------------------------------------------*/

                    let edges = &mut n.edges;

                    self.stack.extend(edges.values_mut().map(|b| IterUnifiedMut::ItemMut(&mut *b)));

                    let v = &mut n.value;
                    let label = n.label.as_deref();

                    match itype {
                        IterationType::ValuesMut => {
                            if v.is_some() {
                                break Some(NextType::ValueRefMut(v.as_deref_mut())) // n.value_mut()))
                            }
                        },
                        IterationType::LabelsValuesMut => {
                            // Pass leaf nodes that have a label and a value
                            // Supply both ref label, ref mut value
                            if label.is_some() && v.is_some() {
                                break Some(NextType::LeafPairRefMut(Some((label.unwrap(), v.as_deref_mut().unwrap()))))
                            }
                        },
                        _ => unreachable!()
                    }
                },
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.size, Some(self.size))
    }
}



//-----------------------------------------------------------------------
// NodeDFSIter methods

impl<K, V> NodeDFSIterOwned<K, V> {
    pub fn new(node: Node<K, V>) -> NodeDFSIterOwned<K, V> {
        NodeDFSIterOwned {
            stack: vec![node],
        }
    }

    // Next method leverages vector's extend trait implementation to add an entire iteration
    // of outgoing edge nodes instead of having to handle the case of specific item or iter
    fn next(&mut self, itype: IterationType) -> Option<NextType<V>> {
        loop {
            match self.stack.pop() {
                None => break None,
                Some(mut n) => {

                    /*-------------------------------------------------------------------------------------------------*/
                    //TODO
                    // Hack for now, to get around borrow checker concerns about exclusive mutable access!
                    // Had to mark node struct fields: "value" and "edges" as pub(crate)  -- not completely ideal
                    // Borrow checker is smart enough to know that different struct fields can be re-borrowed (as mutable)
                    // In that mutable access (a write) to one won't affect another
                    /*-------------------------------------------------------------------------------------------------*/

//                    let edges = &mut n.edges;
                    self.stack.extend(n.edges.into_values().map(|b| *b));
                    let v = &mut n.value;

                    match itype {
                        IterationType::ValuesOwned => {
                            if v.is_some() {
                                break Some(NextType::ValueOwned(v.take().map(|b| *b))) // n.take_value()))
                            }
                        },
                        _ => unreachable!()
                    }
                },
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

impl<'a, K: 'a, V: 'a> Default for LeafPairsIter<'a, K, V> {
    fn default() -> Self {
        LeafPairsIter(NodeDFSIter::default())
    }
}

impl<'a, K: 'a, V: 'a> Default for LeafPairsIterMut<'a, K, V> {
    fn default() -> Self {
        LeafPairsIterMut(NodeDFSIterMut::default())
    }
}

impl<K, V> Default for IntoIter<K, V> {
    fn default() -> Self {
        IntoIter(NodeDFSIterOwned::default())
    }
}

/*-----------------------------------------------------------------------*/
// Implementations for custom iterator types which leverage base iterator
impl<'a, K: 'a, V: 'a> LabelsIter<'a, K, V> {
    pub fn new(node: &'a Node<K, V>, size: usize) -> LabelsIter<'a, K, V> {
        LabelsIter(NodeDFSIter::new(node, size))
    }
}

impl<'a, K: 'a, V: 'a> ValuesIter<'a, K, V> {
    pub fn new(node: &'a Node<K, V>, size: usize) -> ValuesIter<'a, K, V> {
        ValuesIter(NodeDFSIter::new(node, size))
    }
}

impl<'a, K: 'a, V: 'a> ValuesIterMut<'a, K, V> {
    pub fn new(node: &'a mut Node<K, V>, size: usize) -> ValuesIterMut<'a, K, V> {
        ValuesIterMut(NodeDFSIterMut::new(node, size))
    }
}

impl<'a, K: 'a, V: 'a> LeafPairsIter<'a, K, V> {
    pub fn new(node: &'a Node<K, V>, size: usize) -> LeafPairsIter<'a, K, V> {
        LeafPairsIter(NodeDFSIter::new(node, size))
    }
}

impl<'a, K: 'a, V: 'a> LeafPairsIterMut<'a, K, V> {
    pub fn new(node: &'a mut Node<K, V>, size: usize) -> LeafPairsIterMut<'a, K, V> {
        LeafPairsIterMut(NodeDFSIterMut::new(node, size))
    }
}

impl<K, V> IntoIter<K, V> {
    pub fn new(node: Node<K, V>) -> IntoIter<K, V> {
        IntoIter(NodeDFSIterOwned::new(node))
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

impl<'a, K: 'a, V: 'a> Iterator for LeafPairsIter<'a, K, V> {
    type Item = (&'a [u8], &'a V);
    fn next(&mut self) -> Option<(&'a [u8], &'a V)> {
        let result = self.0.next(IterationType::LabelsValues);
        result.and_then(|r| enum_extract!(r, NextType::LeafPairRef))
    }
}

impl<'a, K: 'a, V: 'a> Iterator for LeafPairsIterMut<'a, K, V> {
    type Item = (&'a [u8], &'a mut V);
    fn next(&mut self) -> Option<(&'a [u8], &'a mut V)> {
        let result = self.0.next(IterationType::LabelsValuesMut);
        result.and_then(|r| enum_extract!(r, NextType::LeafPairRefMut))
    }
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = V;
    fn next(&mut self) -> Option<Self::Item> {
        let result = self.0.next(IterationType::ValuesOwned);
        result.and_then(|r| enum_extract!(r, NextType::ValueOwned))
    }
}


/*
// Original next implementation before using Extend trait

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


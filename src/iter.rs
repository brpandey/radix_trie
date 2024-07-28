#![allow(dead_code)]

use crate::node::Node;
use crate::macros::enum_extract;

// Iteration types are implemented as new types (kudos Haskell)
// around a base iter type

#[derive(Clone, Debug)]
pub struct LabelsIter<'a, K, V>(BaseIter<'a, K, V>);

#[derive(Clone, Debug)]
pub struct ValuesIter<'a, K, V>(BaseIter<'a, K, V>);

#[derive(Debug)]
pub struct ValuesIterMut<'a, K, V>(BaseIterMut<'a, K, V>);

#[derive(Clone, Debug)]
pub struct LeafPairsIter<'a, K, V>(BaseIter<'a, K, V>);

#[derive(Debug)]
pub struct LeafPairsIterMut<'a, K, V>(BaseIterMut<'a, K, V>);

#[derive(Clone, Debug)]
pub struct IntoIter<K, V>(BaseIterOwned<K, V>);

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

/*-----------------------------------------------------------------------*/
// Handles DFS iteration using a stack and total size
#[derive(Clone, Debug)]
pub struct BaseIter<'a, K, V> {
    stack: Vec<&'a Node<K, V>>,
    size: usize,
}

// Handles DFS mut iteration using a stack and total size
#[derive(Debug)]
pub struct BaseIterMut<'a, K, V> {
    stack: Vec<&'a mut Node<K, V>>,
    size: usize,
}

// Handles DFS iteration by value using a stack and total size
#[derive(Clone, Debug)]
pub struct BaseIterOwned<K, V> {
    stack: Vec<Node<K, V>>,
}

impl<'a, K: 'a, V: 'a> Default for BaseIter<'a, K, V> {
    fn default() -> Self {
        BaseIter {
            stack: vec![],
            size: 0,
        }
    }
}

impl<'a, K: 'a, V: 'a> Default for BaseIterMut<'a, K, V> {
    fn default() -> Self {
        BaseIterMut {
            stack: vec![],
            size: 0,
        }
    }
}

impl<K, V> Default for BaseIterOwned<K, V> {
    fn default() -> Self {
        BaseIterOwned {
            stack: vec![],
        }
    }
}

//-----------------------------------------------------------------------
// BaseIter methods

impl<'a, K: 'a, V: 'a> BaseIter<'a, K, V> {
    pub fn new(node: &'a Node<K, V>, size: usize) -> BaseIter<'a, K, V> {
        BaseIter {
            stack: vec![node],
            size,
        }
    }

    // Next method leverages vector's extend trait implementation to add an entire iteration
    // of outgoing edge nodes instead of having to handle the case of specific item or iter
    fn next(&mut self, itype: IterationType) -> Option<NextType<'a, V>> {
        loop {
            match self.stack.pop() {
                None => break None,
                Some(n) => {
                    let view = n.node_view();
                    self.stack.extend(view.edges.map(|b| &**b));

                    match itype {
                        IterationType::Labels => {
                            // Don't add root label which is none
                            if view.label.is_some() {
                                break Some(NextType::LabelRef(view.label))
                            }
                        },
                        IterationType::Values => {
                            // Only pass data that have actual value
                            if view.value.is_some() {
                                break Some(NextType::ValueRef(view.value))
                            }
                        },
                        IterationType::LabelsValues => {
                            // Pass leaf data that has a label and a value
                            if view.label.is_some() && view.value.is_some() {
                                break Some(NextType::LeafPairRef(Some((view.label.unwrap(), view.value.unwrap()))))
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
impl<'a, K: 'a, V: 'a> BaseIterMut<'a, K, V> {
    pub fn new(node: &'a mut Node<K, V>, size: usize) -> BaseIterMut<'a, K, V> {
        BaseIterMut {
            stack: vec![node],
            size,
        }
    }

    // Next method leverages vector's extend trait implementation to add an entire iteration
    // of outgoing edge nodes instead of having to handle the case of specific item or iter
    fn next(&mut self, itype: IterationType) -> Option<NextType<'a, V>> {
        loop {
            match self.stack.pop() {
                None => break None,
                Some(n) => {
                    // Mutable view type w/ accesible fields avoids concerns about exclusive mutable access to node
                    let view_mut = n.node_view_mut();
                    self.stack.extend(view_mut.edges.map(|b| &mut **b));

                    match itype {
                        IterationType::ValuesMut => {
                            if view_mut.value.is_some() {
                                break Some(NextType::ValueRefMut(view_mut.value))
                            }
                        },
                        IterationType::LabelsValuesMut => {
                            // Pass leaf data that has a label and a value
                            // Supply both ref label, ref mut value
                            if view_mut.label.is_some() && view_mut.value.is_some() {
                                break Some(NextType::LeafPairRefMut(Some((view_mut.label.unwrap(), view_mut.value.unwrap()))))
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
// BaseIter methods

impl<K, V> BaseIterOwned<K, V> {
    pub fn new(node: Node<K, V>) -> BaseIterOwned<K, V> {
        BaseIterOwned {
            stack: vec![node],
        }
    }

    // Next method leverages vector's extend trait implementation to add an entire iteration
    // of outgoing edge nodes instead of having to handle the case of specific item or iter
    fn next(&mut self, itype: IterationType) -> Option<NextType<V>> {
        loop {
            match self.stack.pop() {
                None => break None,
                Some(n) => {
                    let view_owned = n.node_view_owned();
                    self.stack.extend(view_owned.edges.map(|b| *b));

                    match itype {
                        IterationType::ValuesOwned => {
                            if view_owned.value.is_some() {
                                break Some(NextType::ValueOwned(view_owned.value))
                            }
                        },
                        _ => unreachable!()
                    }
                },
            }
        }
    }
}


// Macro to implement Default trait for given type using inner type
macro_rules! derive_default {
    ($type:ident, $inner:ident) => {

        impl<'a, K: 'a, V: 'a> Default for $type<'a, K, V> {
            fn default() -> Self {
                $type($inner::default())
            }
        }
    };
}

/*-----------------------------------------------------------------------*/
// Default trait implementations for custom iter types

derive_default!(LabelsIter, BaseIter);
derive_default!(ValuesIter, BaseIter);
derive_default!(ValuesIterMut, BaseIterMut);
derive_default!(LeafPairsIter, BaseIter);
derive_default!(LeafPairsIterMut, BaseIterMut);

impl<K, V> Default for IntoIter<K, V> {
    fn default() -> Self {
        IntoIter(BaseIterOwned::default())
    }
}


/*-----------------------------------------------------------------------*/
// Implementations for custom iterator types which leverage base iterator
impl<'a, K: 'a, V: 'a> LabelsIter<'a, K, V> {
    pub fn new(node: &'a Node<K, V>, size: usize) -> LabelsIter<'a, K, V> {
        LabelsIter(BaseIter::new(node, size))
    }
}

impl<'a, K: 'a, V: 'a> ValuesIter<'a, K, V> {
    pub fn new(node: &'a Node<K, V>, size: usize) -> ValuesIter<'a, K, V> {
        ValuesIter(BaseIter::new(node, size))
    }
}

impl<'a, K: 'a, V: 'a> ValuesIterMut<'a, K, V> {
    pub fn new(node: &'a mut Node<K, V>, size: usize) -> ValuesIterMut<'a, K, V> {
        ValuesIterMut(BaseIterMut::new(node, size))
    }
}

impl<'a, K: 'a, V: 'a> LeafPairsIter<'a, K, V> {
    pub fn new(node: &'a Node<K, V>, size: usize) -> LeafPairsIter<'a, K, V> {
        LeafPairsIter(BaseIter::new(node, size))
    }
}

impl<'a, K: 'a, V: 'a> LeafPairsIterMut<'a, K, V> {
    pub fn new(node: &'a mut Node<K, V>, size: usize) -> LeafPairsIterMut<'a, K, V> {
        LeafPairsIterMut(BaseIterMut::new(node, size))
    }
}

impl<K, V> IntoIter<K, V> {
    pub fn new(node: Node<K, V>) -> IntoIter<K, V> {
        IntoIter(BaseIterOwned::new(node))
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

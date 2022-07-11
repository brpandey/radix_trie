use crate::node::Node;

use std::collections::hash_map::{Keys, Values, ValuesMut, IntoValues};

/*-----------------------------------------------------------------------------*/
// Auxiliary data structures that provide views into Node mainly used by Iter,
// generated when necessary

pub struct NodeView<'a, K, V> {
    pub(crate) label: Option<&'a [u8]>,
    pub(crate) value: Option<&'a V>,
    pub(crate) edges: Values<'a, u8, Box<Node<K, V>>>,
    pub(crate) keys: Keys<'a, u8, Box<Node<K, V>>>,
}

// Borrow checker is smart enough to know that different struct fields can be re-borrowed (as mutable)
// In that mutable access (a write) to one won't affect another
pub struct NodeViewMut<'a, K, V> {
    pub(crate) label: Option<&'a [u8]>, // not allowed to modify label - just a shared ref
    pub(crate) value: Option<&'a mut V>,
    pub(crate) edges: ValuesMut<'a, u8, Box<Node<K, V>>>,
}

pub struct NodeViewOwned<K, V> {
    pub(crate) value: Option<V>,
    pub(crate) edges: IntoValues<u8, Box<Node<K, V>>>
}

/*-----------------------------------------------------------------------------*/

impl<'a, K, V> NodeView<'a, K, V> {
    pub(super) fn new(label: Option<&'a [u8]>, value: Option<&'a V>,
               edges: Values<'a, u8, Box<Node<K, V>>>, keys: Keys<'a, u8, Box<Node<K, V>>>) -> Self {
        NodeView {
            label,
            value,
            edges,
            keys,
        }
    }
}


impl<'a, K, V> NodeViewMut<'a, K, V> {
    pub(super) fn new(label: Option<&'a [u8]>, value: Option<&'a mut V>,
               edges: ValuesMut<'a, u8, Box<Node<K, V>>>) -> Self {
        NodeViewMut {
            label,
            value,
            edges,
        }
    }
}

impl<'a, K, V> NodeViewOwned<K, V> {
    pub(super) fn new(value: Option<V>, edges: IntoValues<u8, Box<Node<K, V>>>) -> Self {
        NodeViewOwned {
            value,
            edges,
        }
    }
}


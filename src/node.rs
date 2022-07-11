pub mod view;

use std::mem;
use std::ops::Deref;
use std::fmt;
use std::borrow::Cow;
use std::marker::PhantomData;
use std::collections::HashMap;

use crate::delete::{Playback, Cursor, capture};
use crate::iter::{LabelsIter, ValuesIter, ValuesIterMut, IntoIter, LeafPairsIter, LeafPairsIterMut};
use crate::traverse::{TraverseType, TraverseResult, KeyMatch, SuffixType, traverse_match, traverse};
use crate::node::view::{NodeView, NodeViewMut, NodeViewOwned};

// A key is not actually stored in the Trie but instead a Vec<u8>
// The trie is accessed via anything the implements the trait AsRef<[u8]>
// To link the traits and generics involved, K is in fact a zero-sized PhantomData type
// To prevent the unused K from affecting the drop check anaylsis it is wrapped in an fn() (just like Empty Iterator)

#[derive(Clone, PartialEq, Eq)]
pub struct Node<K, V> {
    label: Option<Vec<u8>>,
    value: Option<Box<V>>,
    tag: NodeType,
    edges: HashMap<u8, Box<Node<K, V>>>,
    key: PhantomData<fn() -> K>,  // from Empty Iterator
}

impl<K, V> Default for Node<K, V> {
    fn default() -> Self {
        Node {
            label: None,
            value: None,
            tag: NodeType::default(),
            edges: HashMap::new(),
            key: PhantomData,
        }
    }
}

impl<K, V> fmt::Debug for Node<K, V> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Node")
            .field("label", &self.label.as_deref())
            .field("value", &format_args!(".."))
            //.field("value", &self.value.as_deref())
            .field("tag", &self.tag)
            .field("edges", &self.edges)
            //.field("key", &format_args!("_"))
            .finish()
    }
}

// A key node contains a value and inner node does not
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum NodeType {
    Key,
    Inner,
}

impl Default for NodeType {
    fn default() -> Self { NodeType::Inner }
}

// Define type which reflects outgoing edges number
#[derive(Debug, PartialEq)]
pub enum EdgeType {
    Single, // Exactly 1
    Branching(usize),  // 2 or more
}

impl<K, V> Node<K, V> {
    pub fn new(label: Option<Vec<u8>>, tag: NodeType, value: Option<Box<V>>) -> Self {
        Node {
            label,
            value,
            tag,
            edges: HashMap::new(),
            key: PhantomData,
        }
    }


    // Returns ref to key fragment label associated with node
    #[inline]
    pub(crate) fn label(&self) -> Option<&[u8]> {
        self.label.as_deref()
    }

    #[inline]
    pub fn is_key(&self) -> bool {
        self.tag == NodeType::Key
    }

    #[inline]
    pub(crate) fn edge_type(&self) -> Option<EdgeType> {
        match self.edges.len() {
            0 => None,
            1 => Some(EdgeType::Single),
            len => Some(EdgeType::Branching(len)),
        }
    }

    #[allow(clippy::borrowed_box)]
    #[inline]
    pub(crate) fn lookup_edge(&self, first: u8) -> Option<&Box<Node<K, V>>> {
        self.edges.get(&first)
    }

    #[allow(clippy::borrowed_box)]
    #[inline]
    pub(crate) fn lookup_edge_mut(&mut self, first: u8) -> Option<&mut Box<Node<K, V>>> {
        self.edges.get_mut(&first)
    }

    // Retrieves value associated with prefix token
    pub fn search(&self, prefix: &[u8]) -> Option<&'_ V> {
        let current: &Node<K, V> = self;
        let result: TraverseResult<K, V> = traverse(current, prefix, TraverseType::Search)?;

        match result {
            TraverseResult::Terminal(true, n) => n.value.as_deref(),
            _ => None,
        }
    }

    // Helper function to insert bridge node which provides a fork to contain an existing node
    // And create space for a new key fragment
    fn insert_bridge(&mut self, byte_key: u8, common: Cow<[u8]>, suffix_edge: Cow<[u8]>) -> &mut Box<Node<K, V>> {
        if common.is_empty() || suffix_edge.is_empty() {
            unreachable!();
        }

        let mut bridge_node = Box::new(Node::new(Some(common.into_owned()), NodeType::Inner, None));
        let mut old_node = self.edges.remove(&byte_key).unwrap();
        let next_byte_key = suffix_edge[0];

        // replace previous key with the edge suffix value (as the common prefix goes in the bridge node)
        old_node.label.replace(suffix_edge.into_owned());
        bridge_node.edges.insert(next_byte_key, old_node);

        self.edges.insert(byte_key, bridge_node);
        self.edges.get_mut(&byte_key).unwrap()
    }

    #[inline]
    fn next_helper(&mut self, key: u8) -> Option<& '_ mut Node<K, V>> {
        self.lookup_edge_mut(key).map(|box_ref| &mut **box_ref)
    }

    // If value already present return it and replace it
    // If value not already present, insert it creating new intermediate
    // nodes as necessary

    pub fn insert(&mut self, token: Cow<[u8]>, value: V) -> Option<V> {
        let mut current: &mut Node<K, V> = self;
        let mut temp_box: &mut Box<Node<K, V>>;
        let mut nav_token: &[u8] = token.deref();

        let (mut input_label, mut interior_label1, mut interior_label2): (Cow<[u8]>, Cow<[u8]>, Cow<[u8]>);

        if token.is_empty() {
            return None
        }

        loop {
            // To insert a new node, token slices are matched until we find a hole (None) so to speak,
            // A different cow is created on each iteration, to signal our intent to delay memory allocation
            // until absolutely necessary. Granted this is not a normal COW use case as we don't benefit from Deref
            // despite whether its borrowed or owned..

            input_label = Cow::from(nav_token);

            match traverse_match(current, nav_token) {
                // Success match with no leftovers, done searching
                Some(KeyMatch {next: _, common: _ , leftover: SuffixType::Empty, edge_key}) => {
                    current = current.next_helper(edge_key).unwrap();
                    break
                },
                Some(KeyMatch {next: _, common: _, leftover: SuffixType::OnlyToken(sufxt), edge_key}) => {
                    nav_token = sufxt;
                    current = current.next_helper(edge_key).unwrap();
                },
                Some(KeyMatch {next: _, common, leftover: SuffixType::OnlyEdge(sufxe), edge_key}) => {
                    interior_label1 = common.to_owned().into();
                    interior_label2 = sufxe.to_owned().into();

                    temp_box = current.insert_bridge(edge_key, interior_label1, interior_label2);
                    current = &mut **temp_box;

                    break // no more token leftovers
                },
                Some(KeyMatch {next: _, common, leftover: SuffixType::BothEdgeToken(sufxe, sufxt), edge_key}) => {
                    interior_label1 = common.to_owned().into();
                    interior_label2 = sufxe.to_owned().into();

                    temp_box = current.insert_bridge(edge_key, interior_label1, interior_label2);
                    current = &mut **temp_box;

                    nav_token = sufxt;
                },
                None => {
                    // Match not found hence create new node and write new label
                    let key = input_label[0];
                    let label = Some(input_label.into_owned());

                    current.edges.insert(key, Box::new(Node::new(label, NodeType::Key, None)));
                    current = &mut **current.edges.get_mut(&key).unwrap();
                    break

                }
            };
        }

        // With the iteration finished, a current node as a key node indicates
        // it was previously inserted, hence grab old value and replace with new boxed_value

        let boxed_value = Box::new(value);

        match current.tag {
            NodeType::Inner => {
                current.tag = NodeType::Key;
                current.value.replace(boxed_value);
                None // not returning anything since this is a new key node
            },
            NodeType::Key => {
                let new_node = Node::new(current.label.take(), NodeType::Key, Some(boxed_value));
                let mut old_node = mem::replace(current, new_node);
                let _old = mem::replace(&mut current.edges, old_node.edges);
                old_node.value.take().map(|bx| *bx) // return Some without Box wrapper around V
            }
        }
    }

    // Removes node from tree either by unmarking node as a key node, pruning trie or compressing nodes
    // or a combination of both.  Relies on a generated delete plan for guidance when making
    // modifications to trie
    pub fn remove(&mut self, prefix: &[u8]) -> Option<V> {
        let mut current: &mut Node<K, V> = self;
        let mut item: Playback;
        let mut counter: u32 = 0;
        let mut temp: &mut Box<Node<K, V>>;
        let mut temp_box: Box<Node<K, V>>;
        let mut value: Option<V> = None;

        let mut replay = capture(current, prefix)?;

        // As long as replay plan isn't empty follow the plan
        while !replay.is_empty() {
            item = replay.pop().unwrap();

            match item {
                // continue iterating
                Playback::Keep(Cursor::Link(i, edge_key)) if i == counter => {
                    temp = current.edges.get_mut(&edge_key).unwrap();
                    current = &mut **temp;
                },
                // perform special pass through compression
                Playback::Merge(Cursor::DoubleLink(i, child_key, merge_grandchild_key)) if i == counter => {
                    temp_box = current.handle_passthrough(child_key, merge_grandchild_key);
                    current = &mut *temp_box;
                },
                // remove edge and keep iterating
                Playback::Prune(Cursor::Link(i, edge_key)) if i == counter => {
                    temp_box = current.edges.remove(&edge_key).unwrap();
                    current = &mut *temp_box;
                },
                // unmark tag and grab value
                Playback::Unmark(Cursor::Node(i)) if i == counter => {
                    current.tag = NodeType::Inner;
                    value = current.value.take().map(|bx| *bx) // return Some without Box wrapper around V;
                },
                _ => {
                    unreachable!()
                }
            }

            counter += 1;
        }

        value
    }

    // Helper function to merge a passthrough node and its replacement to save space
    // Restores the tree's integrity after a delete by combining once separate labels
    fn handle_passthrough(&mut self, edge_key: u8, merge_key: u8) -> Box<Node<K, V>> {
        let current = self;

        /*
        Merge before prune

        Let's say x is the current node, which has a branch to its child y,
        y is the pass through node that has a branch to its child z,
        z is the node to ultimately delete
        y' is the other sibling node to z and is y's other child node,
        y' replaces parent y with concatening of both labels, while preserving y's child edges

        A) Merge before prune
                                          [prune]
                x  ----> y (pass through) ------> z (delete)
                          \
                           \
                            \-------------------> y'

                becomes

                x  ----> y" (merged)

        or

        B) Merge no prune

        The deleted/unmarked node becomes the passthrough node

                x  ----> y (delete) ------> y'

                becomes

                x  ----> y" (merged)

         */

        let mut passthrough = current.edges.remove(&edge_key).unwrap(); // y
        let mut merged = passthrough.edges.remove(&merge_key).unwrap(); // remove y' from y
        let mut la = passthrough.label.take().unwrap();

        // Put in place new label that combines both labels la and lb
        let lb = &mut merged.label.take().unwrap();
        la.append(lb);
        merged.label.replace(la);

        // Here we perform the actual "compression" effect by inserting y' into y's old spot
        current.edges.insert(edge_key, merged);

        passthrough
    }
}

// Node functionality related to Iter

impl<K, V> Node<K, V> {
    pub(crate) fn iter(&self, size: usize) -> LeafPairsIter<'_, K, V> {
        LeafPairsIter::new(self, size)
    }

    pub(crate) fn iter_mut(&mut self, size: usize) -> LeafPairsIterMut<'_, K, V> {
        LeafPairsIterMut::new(self, size)
    }

    pub(crate) fn labels(&self, size: usize) -> LabelsIter<'_, K, V> {
        LabelsIter::new(self, size)
    }

    pub(crate) fn values(&self, size: usize) -> ValuesIter<'_, K, V> {
        ValuesIter::new(self, size)
    }

    pub(crate) fn values_mut(&mut self, size: usize) -> ValuesIterMut<'_, K, V> {
        ValuesIterMut::new(self, size)
    }

    /*-----------------------------------------------------------------------------*/
    // View structs are used to get around multiple mutable reborrow concerns
    // when mostly used with iter when a node is being mutably borrowed,
    // and also to eliminate getter methods clutter

    pub(crate) fn node_view(&self) -> NodeView<'_, K, V> {
        NodeView::new(
            self.label.as_deref(),
            self.value.as_deref(),
            self.edges.values(),
            self.edges.keys(),
        )
    }

    pub(crate) fn node_view_mut(&mut self) -> NodeViewMut<'_, K, V> {
        NodeViewMut::new(
            self.label.as_deref(),
            self.value.as_deref_mut(),
            self.edges.values_mut(),
        )
    }

    pub(crate) fn node_view_owned(mut self) -> NodeViewOwned<K, V> {
        NodeViewOwned::new(
            self.value.take().map(|b| *b),
            self.edges.into_values(),
        )
    }
}

impl <K, V> IntoIterator for Node<K, V> {
    type Item = V;
    type IntoIter = IntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter::new(self)
    }
}


use std::mem;
use std::marker::PhantomData;

use std::collections::{HashMap};

use crate::iter::LabelsIter;
use crate::delete::{Playback, Cursor, capture};
use crate::traverse::{TraverseType, TraverseResult, KeyMatch, SuffixType, traverse_match, traverse};

#[derive(Clone, Debug, PartialEq, Eq)]
// #[derive(Debug)] - define custom Debug?

// Since generics and traits work hand in hand and we want to use the trait AsRef<[u8]>
// for our Trie, since we don't actually store a key type K in the node but instead a Vec<u8>, we
// simulate that we store a K with the zero-sized unused field key as a PhantomData type
// To prevent the unused K from affecting the drop check anaylsis it is wrapped in an fn() (just like Empty Iterator)

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

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum NodeType {
    Key,
    Inner,
}

impl Default for NodeType {
    fn default() -> Self { NodeType::Inner }
}

#[derive(Debug, PartialEq)]
pub enum EdgeType {
    Single, // Exactly 1
    Branching(usize),  // 2 or more
}

pub type NodeEdgesValueIter<'a, K, V> = std::collections::hash_map::Values<'a, u8, Box<Node<K, V>>>;
pub type NodeEdgesKeyIter<'a, K, V> = std::collections::hash_map::Keys<'a, u8, Box<Node<K, V>>>;


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

    pub(crate) fn label(&self) -> Option<&[u8]> {
        self.label.as_deref()
    }

    pub fn is_key(&self) -> bool {
        self.tag == NodeType::Key
    }

    pub(crate) fn edge_type(&self) -> Option<EdgeType> {
        match self.edges.len() {
            0 => None,
            1 => Some(EdgeType::Single),
            len => Some(EdgeType::Branching(len)),
        }
    }

    pub(crate) fn edges_keys_iter(&self) -> NodeEdgesKeyIter<'_, K, V> {
        self.edges.keys()
    }

    pub(crate) fn edges_values_iter(&self) -> NodeEdgesValueIter<'_, K, V> {
        self.edges.values()
    }

    pub(crate) fn lookup_edge(&self, first: u8) -> Option<&Box<Node<K, V>>> {
        self.edges.get(&first)
    }

    pub(crate) fn lookup_edge_mut(&mut self, first: u8) -> Option<&mut Box<Node<K, V>>> {
        self.edges.get_mut(&first)
    }

    pub fn search(&self, prefix: &[u8]) -> Option<&'_ V> 
    {
        let current: &Node<K, V> = self;
        let result: TraverseResult<K, V> = traverse(current, prefix, TraverseType::Search)?;

        match result {
            TraverseResult::Terminal(true, n) => n.value.as_deref(),
            _ => None,
        }
    }


    // If value already present return it and replace it
    // If value not already present, insert it creating new intermediate
    // nodes as necessary

    fn insert_bridge(&mut self, byte_key: u8, common: Vec<u8>, suffix_edge: Vec<u8>) -> &mut Box<Node<K, V>> {
        if common.is_empty() || suffix_edge.is_empty() {
            unreachable!();
        }

        let mut bridge_node = Box::new(Node::new(Some(common), NodeType::Inner, None));
        let mut old_node = self.edges.remove(&byte_key).unwrap();

        let next_byte_key = suffix_edge[0];

        // replace previous key with the edge suffix value (as the common prefix goes in the bridge node)
        old_node.label.replace(suffix_edge);

        bridge_node.edges.insert(next_byte_key, old_node);

        self.edges.insert(byte_key, bridge_node);
        self.edges.get_mut(&byte_key).unwrap()
    }

    fn next_helper(&mut self, key: u8) -> Option<& '_ mut Node<K, V>>{
        self.lookup_edge_mut(key).map(|box_ref| &mut **box_ref)
    }

    pub fn insert(&mut self, prefix: &[u8], value: V) -> Option<V>
    {
        let mut current: &mut Node<K, V> = self;
        let mut temp_box: &mut Box<Node<K, V>>;

        if prefix.is_empty() {
            return None
        }

        let mut nav_token: &[u8] = prefix;

        loop {
            match traverse_match(current, nav_token) {
                // Success match with no leftovers, done searching
                Some(KeyMatch {next: _, common: _ , leftover: SuffixType::Empty, edge_key}) => {
                    //                    current = next;

                    //println!("11 Traverse match Edge key is {:?}, current node is {:#?}", edge_key, current);

                    current = current.next_helper(edge_key).unwrap();
                    break
                },
                Some(KeyMatch {next: _, common: _, leftover: SuffixType::OnlyToken(sufx), edge_key}) => {
                    //                    current = next;
                    //println!("1 Traverse match Edge key is {:?}, current node is {:#?}", edge_key, current);
                    current = current.next_helper(edge_key).unwrap();

                    nav_token = sufx
                },
                Some(KeyMatch {next: _, common, leftover: SuffixType::OnlyEdge(sufx), edge_key}) => {
                    //                    temp = current.insert_bridge(nav_token[0], common, sufx);
                    let c = common.to_owned();
                    let s = sufx.to_owned();
                    temp_box = current.insert_bridge(edge_key, c, s);
                    current = &mut **temp_box;
                    break // no more token leftovers
                },
                Some(KeyMatch {next: _, common, leftover: SuffixType::BothEdgeToken(sufxe, sufxt), edge_key}) => {
                    //temp = current.insert_bridge(nav_token[0], common, sufxe);
                    let c = common.to_owned();
                    let s = sufxe.to_owned();
                    temp_box = current.insert_bridge(edge_key, c, s);
                    nav_token = sufxt;
                    current = &mut **temp_box
                },
                None => {
                    let key = nav_token[0];
                    let label = Some(nav_token.to_owned());
                    current.edges.insert(key, Box::new(Node::new(label, NodeType::Key, None)));
                    current = &mut **current.edges.get_mut(&key).unwrap();
                    break;
                }
            };
        }

        // As we have finished iterating through, the prefix mark the node properly
        // if a node is marked already as a Key Node, (indicating it was previously
        // inserted), grab old value out and replace with new boxed node)

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

    pub fn remove(&mut self, prefix: &[u8]) -> Option<V>
    {
        let mut current: &mut Node<K, V> = self;
        let mut item: Playback;
        let mut counter: u32 = 0;
        let mut temp: &mut Box<Node<K, V>>;
        let mut temp_box: Box<Node<K, V>>;
        let mut value: Option<V> = None;

        //println!("xx1");

        let mut replay = capture(&current, prefix)?;

        //println!("xx2, replay stack is {:?}", replay);

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

        // y
        let mut passthrough = current.edges.remove(&edge_key).unwrap();

        //println!("xxp2 passthrough is {:?}", passthrough);

        // merge key is key to y'
        // remove y' from y
        let mut merged = passthrough.edges.remove(&merge_key).unwrap();

        //println!("xxp2.3 merged is now {:?}", &merged);

        let mut la = passthrough.label.take().unwrap();
        let lb = &mut merged.label.take().unwrap();
        la.append(lb);

        //          let lab = la.zip(lb).map(|(&mut v1, &mut v2)| v1.append(v2)).unwrap();
        merged.label.replace(la);

        // Here we perform the actual compression by inserting y' into y's old spot
        current.edges.insert(edge_key, merged);

        //            println!("xxp2.5 removed passthrough is now {:?}", passthrough);
        //println!("xxp2.7 current is now {:?}", &current);
        passthrough
    }
}

impl<K, V> Node<K, V> {
    pub(crate) fn iter(&self) -> LabelsIter<'_, K, V> {
        LabelsIter::new(self)
    }
}

impl <'a, K, V> IntoIterator for &'a Node<K, V> {
    type Item = &'a [u8];
    type IntoIter = LabelsIter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}


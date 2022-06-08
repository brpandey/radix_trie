use std::mem;
use std::str;
use std::collections::{HashMap};

use crate::iter::LabelsIter;
use crate::delete::{Playback, Cursor, capture};
use crate::traverse::{TraverseType, TraverseResult, KeyMatch, SuffixType, traverse_match, traverse};

#[derive(Clone, Debug, PartialEq, Eq)]
// #[derive(Debug)] - define custom Debug?
pub struct Node {
    label: Option<Vec<u8>>,
    value: Option<i32>,
    tag: NodeType,
    edges: HashMap<u8, Box<Node>>,
}

impl Default for Node {
    fn default() -> Self {
        Node {
            label: None,
            value: None,
            tag: NodeType::default(),
            edges: HashMap::new()
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

pub type NodeEdgesValueIter<'a> = std::collections::hash_map::Values<'a, u8, Box<Node>>;
pub type NodeEdgesKeyIter<'a> = std::collections::hash_map::Keys<'a, u8, Box<Node>>;

impl Node {
    pub fn new(label: Option<Vec<u8>>, tag: NodeType, value: Option<i32>) -> Self {
        Node {
            label,
            value,
            tag,
            edges: HashMap::new(),
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

    pub(crate) fn edges_keys_iter(&self) -> NodeEdgesKeyIter<'_> {
        self.edges.keys()
    }

    pub(crate) fn edges_values_iter(&self) -> NodeEdgesValueIter<'_> {
        self.edges.values()
    }

    pub(crate) fn lookup_edge(&self, first: u8) -> Option<&Box<Node>> {
        self.edges.get(&first)
    }

    pub(crate) fn lookup_edge_mut(&mut self, first: u8) -> Option<&mut Box<Node>> {
        self.edges.get_mut(&first)
    }

    pub fn search(&self, prefix: &str) -> Option<&'_ i32> {
        let current: &Node = self;
        let result: TraverseResult = traverse(current, prefix.as_bytes(), TraverseType::Search)?;

        match result {
            TraverseResult::Terminal(true, n) => n.value.as_ref(),
            _ => None,
        }
    }


    // If value already present return it and replace it
    // If value not already present, insert it creating new intermediate
    // nodes as necessary

    pub fn insert_bridge(&mut self, byte_key: u8, common: Vec<u8>, suffix_edge: Vec<u8>) -> &mut Box<Node> {
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

    fn next_helper(&mut self, key: u8) -> Option<& '_ mut Node>{
        self.lookup_edge_mut(key).map(|box_ref| &mut **box_ref)
    }

    pub fn insert(&mut self, prefix: &str, value: Option<i32>) -> Option<i32> {
        let mut current: &mut Node = self;
        let mut temp_box: &mut Box<Node>;

        if prefix.is_empty() {
            return None
        }

        let mut nav_token: &[u8] = prefix.as_bytes();

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
        match current.tag {
            NodeType::Inner => {
                current.tag = NodeType::Key;
                current.value = value;
                None
            },
            NodeType::Key => {
                let new_node = Node::new(current.label.take(), NodeType::Key, value);
                let old_node = mem::replace(current, new_node);
                let _old = mem::replace(&mut current.edges, old_node.edges);
                old_node.value
            }
        }
    }

    pub fn remove(&mut self, prefix: &str) -> Option<i32> {
        let mut current: &mut Node = self;
        let mut item: Playback;
        let mut counter: u32 = 0;
        let mut temp: &mut Box<Node>;
        let mut temp_box: Box<Node>;
        let mut value: Option<i32> = None;

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
                    value = current.value.take();
                },
                _ => {
                    unreachable!()
                }
            }

            counter += 1;
        }

        value
    }

    fn handle_passthrough(&mut self, edge_key: u8, merge_key: u8) -> Box<Node> {
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

impl Node {
    pub(crate) fn iter(&self) -> LabelsIter<'_> {
        LabelsIter::new(self)
    }
}

impl <'a> IntoIterator for &'a Node {
    type Item = &'a [u8];
    type IntoIter = LabelsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}


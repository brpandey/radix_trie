use std::collections::VecDeque;

use crate::node::{Node};
use crate::traverse::{TraverseItem, TraverseType, TraverseResult, traverse};
use crate::macros::enum_extract;

// Finds the longest path that corresponds to the prefix token, one that fully captures
// the token or part of it (should it not fully reside in trie) and return it as an iterator
pub fn longest_prefix<'a, 'b, K, V>(node: &'a Node<K, V>, prefix: &'b [u8]) -> Option<impl Iterator<Item = &'a u8>> { // Option<String> {
    let value: TraverseResult<K, V> =  traverse(node, prefix, TraverseType::FoldOrPartial)?;
    let mut stack = enum_extract!(value, TraverseResult::Stack);

    // store label iterators
    let mut prefixes: Vec<_>;
    let mut result = None;

    // Iteratively pop off all stack elements until we find a value that is a NodeType::Key
    // If so, we prematurely drain the stack to collect all the remaining prefix tokens of the antecedents
    // in reverse order starting from 1.., and reasssemble this then longest prefix

    let mut last_label;

    while !stack.is_empty() {
        let TraverseItem{node, next_key: _, label, level} = stack.pop().unwrap();

        last_label = label;

        if node.is_key() {
            prefixes = Vec::with_capacity(level as usize);

            // Ignore root label so start with 1
            prefixes = stack.drain(1..).fold(prefixes, |mut acc, TraverseItem{node: _, next_key: _, label, level: _}| {
                if label.is_some() {
                    acc.push(label.unwrap().iter());
                    //acc.extend(label.unwrap().to_owned())
                }
                acc}
            );

            // Add in last label of key node
            prefixes.push(last_label.unwrap().iter());
            let p = prefixes.into_iter().flat_map(|it| it);
            result = Some(p);

            //prefixes.extend(last_label.unwrap().to_owned());
            //result = Some(prefixes)
        }
    }

    result
}

// Find all prefix keys which have the same common prefix
pub fn all_keys<'a, 'b, K, V>(node: &'a Node<K, V>, prefix: &'b [u8]) -> Option<Vec<Vec<u8>>> {
    // Grab node where the prefix search ends
    let result: TraverseResult<K, V> = traverse(node, prefix, TraverseType::Search)?;

    // If prefix is contained in the middle of a label e.g. partial terminal, that's fine
    // Just take that terminal node's label which is prefix + edge_suffix as the
    // starting point for the common prefix for all keys
    let mut leftover = None;

    let current =
        match result {
            TraverseResult::Terminal(_, n) => n,
            TraverseResult::PartialTerminal(_, n, extra) => {
                leftover = Some(extra);
                n
            },
            _ => unreachable!(),
        };

    let mut result: Vec<Vec<u8>> = Vec::new();
    let mut backlog: VecDeque<(&Node<K, V>, Vec<u8>)> = VecDeque::new();

    let mut child: &Node<K, V>;
    let mut child_bytes: Vec<u8>;
    let mut label_slice: &[u8];

    let mut seed = prefix.to_vec();

    // add in the leftover suffix edge if only a partial match was achieved
    match leftover {
        None => backlog.push_back((current, seed)),
        Some(extra) => {
            seed.extend_from_slice(extra);
            backlog.push_back((current, seed))
        },
    }

    // Using BFS to construct the matching keys with the shared common prefix
    while !backlog.is_empty() {
        let (current, bytes) = backlog.pop_front().unwrap();

        for boxed_child_node_ref in current.edges_values_iter() {
            child = &**boxed_child_node_ref;

            // Since bytes is being accessed by other node child siblings, clone it
            child_bytes = bytes.clone();
            label_slice = child.label().unwrap();

            // preallocate extra space and then add
            child_bytes.reserve(label_slice.len());
            child_bytes.extend(label_slice.iter());

            // update the prefix token for this node, along with node ref in backlog
            backlog.push_back((child, child_bytes))
        }

        if current.is_key() {
            result.push(bytes)
        }
    }

    Some(result)
}

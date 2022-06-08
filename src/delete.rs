use std::collections::HashSet;

use crate::node::{Node, EdgeType};
use crate::traverse::{traverse, TraverseItem, TraverseType, TraverseResult};
use crate::macros::enum_extract;

type DeletePlan = Vec<Playback>;

#[derive(Debug, PartialEq)]
pub enum Cursor {
    Node(u32),
    Link(u32, u8),
    DoubleLink(u32, u8, u8),
}

#[derive(Debug, PartialEq)]
pub enum Playback {
    Unmark(Cursor),
    Prune(Cursor),
    MergeTemp(u8),
    Merge(Cursor),
    Keep(Cursor),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Status {
    Deleted,
    DeletedPruned,
    Merged,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Prune,
    Merge,
    Noop,
}

// In order to delete a node without using back or parent links we create a replay stack which
// gives us the required info to delete a node or prune nodes while iterating a single mutable pointer
// starting from the trie root node
// (While Rust supports recursion but not tail recursion this explicit stack somewhat likens
// to a call stack with no limitations of potentially blowing the call stack)
pub fn capture(current: &Node, prefix: &str) -> Option<DeletePlan> {
    let mut replay: Vec<Playback> = Vec::new();
    let mut status: HashSet<Status> = HashSet::new();
    let mut action: Action = Action::Noop;

    // Essentially reduce the multiple node immutable refs stack into
    // a replay stack which just has copy values / semantics

    // Take the dfs stack (with the terminal node on top)
    // and convert it into a replay stack

    let result: TraverseResult =  traverse(current, prefix.as_bytes(), TraverseType::Fold)?;
    let mut stack = enum_extract!(result, TraverseResult::Stack);

    //prepopulated stack given prefix and trie
    if let Some(TraverseItem{node, next_key: _, label: _, level}) = stack.pop() {
        if node.is_key() {
            replay.push(Playback::Unmark(Cursor::Node(level)));

            //println!("unmark here, node to unmark is {:?}", &node);

            status.insert(Status::Deleted);

            // No child edges then can easily prune, otherwise if single we have a passthrough

            match node.edge_type() {
                None => action = Action::Prune,
                Some(EdgeType::Single) => {
                    // record key that will be used as the merge key / merge node
                    // when we merge the passthrough node's label with the merge node
                    let merge_key = node.edges_keys_iter().copied().collect::<Vec<u8>>().pop().unwrap();

                    let item = Playback::MergeTemp(merge_key);
                    replay.push(item);

                    action = Action::Merge
                },
                _ => (),
            }
        }
    }

    // Essentially we work backwards from the node we want to delete
    while !stack.is_empty() {
        let TraverseItem{node, next_key, label: _, level} = stack.pop().unwrap();

        match action {
            Action::Prune => {
                // We can only prune a level above the node that needs deleting
                let info = Cursor::Link(level, next_key);
                let item = Playback::Prune(info);
                replay.push(item);

                // Only prune once since when we insert everything is already compressed,
                // only have to prune the outgoing edge to the node to delete

                status.remove(&Status::Deleted);
                status.insert(Status::DeletedPruned);
            },
            Action::Merge => {
                match replay.pop() {
                    Some(Playback::MergeTemp(merge_key)) => {
                        let info = Cursor::DoubleLink(level, next_key, merge_key);
                        let item = Playback::Merge(info);

                        replay.push(item);
                        status.insert(Status::Merged);
                    },
                    _ => unreachable!()
                }
            },
            Action::Noop => {
                replay.push(Playback::Keep(Cursor::Link(level, next_key)))
            },
        }

        action = Action::Noop;

        //println!("capture status is {:?}, ode is {:?}", &status, &node);

        // passthrough is available once to be compressed after a single prune sequence
        if status.contains(&Status::DeletedPruned) && status.len() == 1 &&
            !node.is_key() && node.edge_type().unwrap() == EdgeType::Branching(2) {

                // record key that will be used as the merge key / merge node
                // when we merge the passthrough node's label with the merge node
                let mut set = node.edges_keys_iter().collect::<HashSet<_>>();
                set.remove(&next_key);
                let merge_key = set.into_iter().copied().collect::<Vec<u8>>().pop().unwrap();

                let item = Playback::MergeTemp(merge_key);
                replay.push(item);

                action = Action::Merge
            }
    }

    // replay is empty or predicate is false, return None
    Some(replay).filter(|r| !r.is_empty())

}

use crate::node::Node;

pub(crate) type TraverseStack<'a, K, V> = Vec<TraverseItem<'a, K, V>>;

#[derive(Debug, Copy, Clone)]
pub(crate) enum TraverseType {
    Search,
    Fold,
    FoldOrPartial,  // If full key doesn't completely exist in tree, grab a partial fold (longest prefix)
}

// KeyMatch represents the match state after a token match with an interior label
#[derive(Debug)]
pub(crate) struct KeyMatch<'a, 'b, K: 'a, V: 'a> {
    pub(crate) next: &'a Node<K, V>,
    pub(crate) common: &'a [u8],
    pub(crate) leftover: SuffixType<'a, 'b>,
    pub(crate) edge_key: u8,
}

// Defines the leftover suffix that wasn't matched with the interior label
#[derive(Debug)]
pub(crate) enum SuffixType<'a, 'b> {
    Empty,
    OnlyToken(&'b [u8]), // edge found in larger token, match
    BothEdgeToken(&'a [u8], &'b [u8]), // edge and token both contain smaller substring match
    OnlyEdge(&'a [u8]), // edge and token-prefix match
}

// Defines Stack item struct type
#[derive(Debug)]
pub(crate) struct TraverseItem<'a, K: 'a, V: 'a>{
    pub(crate) node: &'a Node<K, V>,
    pub(crate) next_key: u8,
    pub(crate) label: Option<&'a [u8]>,
    pub(crate) level: u32,
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) enum TraverseResult<'a, K: 'a, V: 'a> {
    Stack(Vec<TraverseItem<'a, K, V>>),
    PartialTerminal(bool, &'a Node<K, V>, &'a [u8]), // If match prefix matches some of the terminal's label
    Terminal(bool, &'a Node<K, V>),
}

impl<'a, 'b, K: 'a, V: 'a> KeyMatch<'a, 'b, K, V> {
    pub fn new(next: &'a Node<K, V>, common: &'a [u8], leftover: SuffixType<'a, 'b>, edge_key: u8) -> Self {
        KeyMatch {
            next,
            common,
            leftover,
            edge_key
        }
    }
}

impl<'a, 'b> SuffixType<'a, 'b> {
    pub fn new(edge_suffix: &'a [u8], token_suffix: &'b [u8]) -> Self {
        match (edge_suffix.len(), token_suffix.len()) {
            (0, 0) => SuffixType::Empty,
            (0, t) if t > 0 => SuffixType::OnlyToken(token_suffix),
            (e, 0) if e > 0 => SuffixType::OnlyEdge(edge_suffix),
            (e, t) if t > 0 && e > 0 => SuffixType::BothEdgeToken(edge_suffix, token_suffix),
            _ => unreachable!(),
        }
    }
}

// Matches token and relevant interior label
pub(crate) fn traverse_match<'a, 'b, K, V>(node: &'a Node<K, V>, token: &'b [u8]) -> Option<KeyMatch<'a, 'b, K, V>> {
    let mut index = 0;
    let next_node: &Node<K, V>;
    let edge_key = token[0];

    if let Some(box_ref) = node.lookup_edge(edge_key) {
        next_node = &**box_ref;

        //iterate through both byte slice values using zip to find
        //common prefix index
        for (c1, c2) in token.iter().zip(next_node.label().unwrap().iter()) {
            if c1 == c2 {
                index += 1;
            } else {
                break;
            }
        }

        //use common prefix to extract remaining suffixes as well as the actual common prefix
        let (common, edge_suffix) = next_node.label().unwrap().split_at(index);
        let (_, token_suffix) = token.split_at(index);

        // No match case
        if common.is_empty() {
            return None
        }

        let leftover = SuffixType::new(edge_suffix, token_suffix);
        Some(KeyMatch::new(next_node, common, leftover, edge_key))
    } else {
        None
    }
}

// Iterates through trie matching interior labels, accumulating a result
pub(crate) fn traverse<'a, 'b, K, V>(node: &'a Node<K, V>, token: &'b [u8], traverse_type: TraverseType) -> Option<TraverseResult<'a, K, V>> {
    let mut stack: TraverseStack<K, V> = Vec::new();
    let mut current: &Node<K, V> = node;
    let mut level: u32 = 0;
    let mut partial_terminal = None;

    // Seed node stack with root node
    // Populate the stack by iterating through token byte chunks at each matching node level
    let mut nav_token = token;

    // Stack is populated from the start node to the end node,
    // hence the end node's data is on top when loop is finished

    stack.push(TraverseItem{
        node: current, next_key: Default::default(), label: None, level,
    });

    loop {
        level += 1;
        match traverse_match(current, nav_token) {
            // Success match with no leftovers, done searching
            Some(KeyMatch {next, common: _, leftover: SuffixType::Empty, ..}) => {
                current = next;

                traverse_fold_helper(current, level, &mut stack, traverse_type);
                break
            },
            Some(KeyMatch {next, common: _, leftover: SuffixType::OnlyToken(sufx), ..}) => {
                current = next;
                nav_token = sufx;

                traverse_fold_helper(current, level, &mut stack, traverse_type);
            },
            Some(KeyMatch {next, common: _, leftover: SuffixType::OnlyEdge(sufx), ..}) => {
                // if search key is found as a prefix of one of the labels, we return the node type and node.
                // essentially, if the trie wasn't compressed it would correspond
                // to a pass through node

                match traverse_type {
                    TraverseType::Search => {
                        partial_terminal = Some(sufx);
                        current = next;
                        break;
                    },
                    TraverseType::FoldOrPartial => break,
                    TraverseType::Fold => return None,
                }
            },
            // These KeyMatch types indicate the prefix token is not found (completely or even partially) in the trie yet
            Some(_) => return None,
            None => {
                match traverse_type {
                    TraverseType::FoldOrPartial if !stack.is_empty() => break,
                    _ => return None,
                }
            }
        }
    }

    // Handle edge cases if we found search key but in middle of a label or
    // Complete prefix not found in trie for fold
    let value =
        match traverse_type {
            TraverseType::Search => {
                if let Some(sufx) = partial_terminal {
                    TraverseResult::PartialTerminal(current.is_key(), current, sufx)
                } else {
                    TraverseResult::Terminal(current.is_key(), current)
                }
            },
            TraverseType::FoldOrPartial => TraverseResult::Stack(stack),
            TraverseType::Fold => TraverseResult::Stack(stack)
        };

    Some(value)
}

// Helper function to push traverse info onto stack 
fn traverse_fold_helper<'a, 's, K, V>(node: &'a Node<K, V>, level: u32,
                                    stack: &'s mut TraverseStack<'a, K, V>, traverse_type: TraverseType) {
    match traverse_type {
        TraverseType::Fold | TraverseType::FoldOrPartial => {
            if let Some(common) = node.label() {
                // Grab top element on stack, if present,
                // set prior next_key given that it is available as the current label's first byte
                if let Some(item) = stack.last_mut() {
                    item.next_key = *common.get(0).unwrap()
                }
            }

            stack.push(
                TraverseItem{node, next_key: Default::default(), label: node.label(), level}
            );
        },
        _ => (),
    }
}



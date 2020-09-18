//! Simple grapha library
//!
//! ##Overview
//! Simple implementation to find all possible routes between two node
//! in a directed graph

#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::{ clone::*, collections::btree_set, vec};

pub type Routes<N> = vec::Vec<N>;

pub fn find_all_routes<N, FN, IN>(
  start: &N, target: &N,
  mut successors: FN,
  max_depth: usize) -> vec::Vec<Routes<N>>
where
  N: Eq + Ord + Clone,
  FN: FnMut(&N) -> IN,
  IN: IntoIterator<Item = N>,
{
  if start == target {
    return vec![];
  }

  let mut visited = btree_set::BTreeSet::new();
  visited.insert(start.clone());
  do_find_all_routes(start, target, max_depth, &mut successors, &mut visited)
}

fn do_find_all_routes<N, FN, IN>(start: &N, target: &N,
                                 max_depth: usize,
                                 successors: &mut FN,
                                 visited_nodes: &mut btree_set::BTreeSet<N>) -> vec::Vec<Routes<N>>
where
  N: Eq + Ord + Clone,
  FN: FnMut(&N) -> IN,
  IN: IntoIterator<Item = N>,
{
  if start == target || max_depth == 0 {
    return vec![];
  }

  let all_successors = successors(start);
  // find valid successors, skip already visited nodes
  let valid_successors: vec::Vec<_> = all_successors.into_iter()
    .filter(|n| !visited_nodes.contains(n)).collect();

  valid_successors.into_iter().map(|node| {
    if target.clone() != node.clone() {
      visited_nodes.insert(node.clone());
      let routes = do_find_all_routes::<N, FN, IN>(&node, target, max_depth - 1,
                                                   successors, visited_nodes);
      visited_nodes.remove(&node);
      routes.into_iter().map(|mut r| {
        r.insert(0, node.clone());
        r
      }).collect()
    } else {
      vec![vec![node.clone()]]
    }
  }).filter(|r| !r.is_empty()).flatten().collect()
}

#[cfg(test)]
mod tests {
  use sp_std::collections::btree_map::*;
  use sp_std::{ clone::*, vec,};
  use super::*;

  pub fn format_routes<T>(routes: &vec::Vec<T>) -> String
  where
    T: sp_std::fmt::Debug {
    let mut s: String = "".to_owned();

    for r in routes {
      let f = format!("{:?},", r);
      s  = s + &f;
    }

    s
  }

  type Pos = (u32, u32);

  fn get_succssors(pos: &Pos, edges: &BTreeMap<Pos, vec::Vec<Pos>>) -> vec::Vec<Pos>{
    edges.get(pos).unwrap_or(&vec![]).clone()
  }

  #[test]
  fn test_simple_route() {
    let mut edges = BTreeMap::new();
    edges.insert((0, 0), vec![(0, 1), (1, 0)]);

    let routes = find_all_routes(&(0, 0), &(1, 0), |p| get_succssors(p, &edges), 2);
    assert_eq!(routes.len(), 1, "only one routes");
    let route = &routes[0];
    assert_eq!(route.len(), 1);
    assert_eq!(route[0], (1, 0));
    // limit max depth to 1
    let routes = find_all_routes(&(0, 0), &(1, 0), |p| get_succssors(p, &edges), 1);
    assert_eq!(routes.len(), 1, "only one routes");
    let route = &routes[0];
    assert_eq!(route.len(), 1);
    assert_eq!(route[0], (1, 0));

    let routes = find_all_routes(&(0, 0), &(1, 1), |p| get_succssors(p, &edges), 5);
    assert_eq!(routes.len(), 0, "no routes to (1, 1)");
  }

  #[test]
  fn test_complex_routes() {
    let mut edges = BTreeMap::new();
    edges.insert((0, 0), vec![(0, 1), (1, 0)]);
    edges.insert((1, 0), vec![(1, 1)]);
    edges.insert((1, 1), vec![(2, 1)]);
    edges.insert((2, 1), vec![(2, 2)]);
    edges.insert((2, 2), vec![(3, 2)]);

    let routes = find_all_routes(&(0, 0), &(0, 1), |p| get_succssors(p, &edges), 5);
    assert_eq!(routes.len(), 1);
    assert_eq!(format_routes(&routes[0]), "(0, 1),");

    let routes = find_all_routes(&(0, 0), &(3, 2), |p| get_succssors(p, &edges), 5);
    assert_eq!(routes.len(), 1);
    assert_eq!(format_routes(&routes[0]), "(1, 0),(1, 1),(2, 1),(2, 2),(3, 2),");

    // limit the max depth, will not found the route
    let routes = find_all_routes(&(0, 0), &(3, 2), |p| get_succssors(p, &edges), 4);
    assert_eq!(routes.len(), 0);

    edges.insert((0, 1), vec![(1, 1)]);
    let routes = find_all_routes(&(0, 0), &(3, 2), |p| get_succssors(p, &edges), 5);
    assert_eq!(routes.len(), 2);
    assert_eq!(format_routes(&routes[0]), "(0, 1),(1, 1),(2, 1),(2, 2),(3, 2),");
    assert_eq!(format_routes(&routes[1]), "(1, 0),(1, 1),(2, 1),(2, 2),(3, 2),");

    // insert a short route
    edges.insert((0, 0), vec![(0, 1), (1, 0), (2, 2)]);
    let routes = find_all_routes(&(0, 0), &(3, 2), |p| get_succssors(p, &edges), 5);
    assert_eq!(routes.len(), 3);
    assert_eq!(format_routes(&routes[0]), "(0, 1),(1, 1),(2, 1),(2, 2),(3, 2),");
    assert_eq!(format_routes(&routes[1]), "(1, 0),(1, 1),(2, 1),(2, 2),(3, 2),");
    assert_eq!(format_routes(&routes[2]), "(2, 2),(3, 2),");

    // limit max depth, check only returns the short route
    let routes = find_all_routes(&(0, 0), &(3, 2), |p| get_succssors(p, &edges), 2);
    assert_eq!(routes.len(), 1);
    assert_eq!(format_routes(&routes[0]), "(2, 2),(3, 2),");

    let routes = find_all_routes(&(0, 0), &(3, 2), |p| get_succssors(p, &edges), 4);
    assert_eq!(routes.len(), 1);
    assert_eq!(format_routes(&routes[0]), "(2, 2),(3, 2),");

    let routes = find_all_routes(&(0, 0), &(3, 2), |p| get_succssors(p, &edges), 2);
    assert_eq!(routes.len(), 1);
    assert_eq!(format_routes(&routes[0]), "(2, 2),(3, 2),");

    let routes = find_all_routes(&(0, 0), &(3, 2), |p| get_succssors(p, &edges), 1);
    assert_eq!(routes.len(), 0);
  }
}

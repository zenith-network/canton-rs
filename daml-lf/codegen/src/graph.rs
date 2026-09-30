//! Supporting graph types

#![allow(dead_code, reason = "Keeping some methods for future use if needed")]

use std::mem;

/// Square matrix (`N x N`) of `Copy + Eq` elements
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SquareMatrix<T: Copy + Eq> {
    matrix: Vec<Vec<T>>,
}

impl<T: Copy + Eq> SquareMatrix<T> {
    /// Create new empty matrix of size `0`
    pub const fn new() -> Self {
        Self { matrix: Vec::new() }
    }

    /// Create new matrix `size x size` filled with `value`
    pub fn new_with_size(size: usize, value: T) -> Self {
        Self {
            matrix: vec![vec![value; size]; size],
        }
    }

    /// Create new matrix `size x size` filled with default value of `T`
    pub fn new_with_size_default(size: usize) -> Self
    where
        T: Default,
    {
        Self::new_with_size(size, T::default())
    }

    /// Get size of matrix
    pub const fn size(&self) -> usize {
        self.matrix.len()
    }

    /// Increment the size of the matrix by 1, inserting `value` into all new elements
    pub fn increment_size(&mut self, value: T) {
        for line in &mut self.matrix {
            line.push(value);
        }
        self.matrix.push(vec![value; self.size() + 1]);
    }

    /// Increment the size of the matrix by 1, inserting default value of `T` into all new elements
    pub fn increment_size_with_default(&mut self)
    where
        T: Default,
    {
        self.increment_size(T::default());
    }

    /// Get element `M[x][y]`
    ///
    /// Panics if indices are out of bounds.
    pub fn get(&self, x: usize, y: usize) -> T {
        self.matrix[x][y]
    }

    /// Set element `M[x][y]` to `value`
    ///
    /// Returns old value. Panics if indices are out of bounds.
    pub fn set(&mut self, x: usize, y: usize, value: T) -> T {
        mem::replace(&mut self.matrix[x][y], value)
    }
}

/// Directed edge of the graph
///
/// Represents different kinds of dependency
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Edge {
    /// Reference to a name
    ///
    /// Contract ID, impl interface, List, GenMap, TextMap
    Ref,

    /// Ownership of the value
    ///
    /// Raw field, Optional
    Inline,
}

/// Directed graph of `Copy + Eq` non-duplicating elements
///
/// Maintains the matrix of inline dependencies
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiGraph<T: Copy + Eq> {
    /// Contained of the nodes
    ///
    /// Internal invariant: nodes here are unique
    nodes: Vec<T>,

    /// Assymetric matrix of edges
    ///
    /// Internal invariant: if `N = size(nodes)`, then this is `N x N`
    edges: SquareMatrix<Option<Edge>>,

    /// Assymetric matrix `M`, where `M[x][y]` is set, if there is an inline path from `x` to `y`
    ///
    /// If `M[x][x]` is set, then `x` is "self-referring".
    ///
    /// This matrix is maintained during build process, so save time on traversals at the generation
    /// phase.
    ///
    /// Internal invariant: if `N = size(nodes)`, then this is `N x N`
    inline_paths: SquareMatrix<bool>,
}

impl<T: Copy + PartialEq + Eq> DiGraph<T> {
    /// Create new empty graph
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: SquareMatrix::new(),
            inline_paths: SquareMatrix::new(),
        }
    }

    /// Create graph with given nodes and no edges
    ///
    /// Duplicating entries will be skipped.
    pub fn from_roots(roots: Vec<T>) -> Self {
        let mut nodes = Vec::with_capacity(roots.len());
        for root in roots {
            if !nodes.contains(&root) {
                nodes.push(root);
            }
        }

        let size = nodes.len();
        Self {
            nodes,
            edges: SquareMatrix::new_with_size_default(size),
            inline_paths: SquareMatrix::new_with_size_default(size),
        }
    }

    /// Return all nodes in the graph as a slice
    pub fn nodes(&self) -> &[T] {
        &self.nodes
    }

    /// Return matrix of edges
    pub fn edges(&self) -> &SquareMatrix<Option<Edge>> {
        &self.edges
    }

    /// Return matrix of inline relations
    ///
    /// The purpose of this matrix is to establish ownership relations between types for the code
    /// generation phase and answer a question: do we need to `Box` a field type or not.
    pub fn inline_matrix(&self) -> &SquareMatrix<bool> {
        &self.inline_paths
    }

    /// Return `true` if there is an inline path from A to B
    ///
    /// Panics if indices are out of bounds
    pub fn inline_path_exists(&self, idx_a: usize, idx_b: usize) -> bool {
        self.inline_paths.get(idx_a, idx_b)
    }

    /// Add a node, returning it's index
    ///
    /// Nodes are checked for equality, so if node already exists in the graph, it won't be added
    /// again. Instead the old index is returned.
    pub fn add_node(&mut self, node: T) -> usize {
        if let Some(idx) = self.lookup_node(node) {
            return idx;
        }

        self.nodes.push(node);

        self.edges.increment_size_with_default();
        self.inline_paths.increment_size_with_default();

        self.nodes.len() - 1
    }

    /// Return node with given index, if some
    pub fn get_node(&self, idx: usize) -> Option<T> {
        self.nodes.get(idx).copied()
    }

    /// Lookup node by value
    pub fn lookup_node(&self, node: T) -> Option<usize> {
        self.nodes.iter().position(|n| n == &node)
    }

    /// Insert or replace an edge between existing nodes.
    ///
    /// Panics if nodes don't exist (indices are out of bounds).
    ///
    /// Return old edge, if some.
    pub fn add_edge(&mut self, idx_a: usize, idx_b: usize, edge: Edge) -> Option<Edge> {
        let old_edge = self.edges.set(idx_a, idx_b, Some(edge));

        match (old_edge, edge) {
            (Some(Edge::Inline), Edge::Ref) => self.rebuild_inline_paths(),
            (old_edge, Edge::Inline) if old_edge != Some(Edge::Inline) => {
                self.add_inline_path(idx_a, idx_b);
            }
            _ => {}
        }

        old_edge
    }

    /// Return edge, if some
    ///
    /// Panics if nodes don't exist (indices are out of bounds).
    pub fn get_edge(&self, idx_a: usize, idx_b: usize) -> Option<Edge> {
        self.edges.get(idx_a, idx_b)
    }

    /// Add an inline edge to the existing transitive closure.
    fn add_inline_path(&mut self, idx_a: usize, idx_b: usize) {
        if self.inline_paths.get(idx_a, idx_b) {
            return;
        }

        let size = self.nodes.len();
        let predecessors: Vec<_> = (0..size)
            .filter(|&idx| idx == idx_a || self.inline_paths.get(idx, idx_a))
            .collect();
        let successors: Vec<_> = (0..size)
            .filter(|&idx| idx == idx_b || self.inline_paths.get(idx_b, idx))
            .collect();

        for predecessor in predecessors {
            for &successor in &successors {
                self.inline_paths.set(predecessor, successor, true);
            }
        }
    }

    /// Rebuild the transitive closure of all inline edges.
    fn rebuild_inline_paths(&mut self) {
        let size = self.nodes.len();
        let mut inline_paths = SquareMatrix::new_with_size(size, false);

        for idx_a in 0..size {
            for idx_b in 0..size {
                if self.edges.get(idx_a, idx_b) == Some(Edge::Inline) {
                    inline_paths.set(idx_a, idx_b, true);
                }
            }
        }

        for intermediate in 0..size {
            for idx_a in 0..size {
                if !inline_paths.get(idx_a, intermediate) {
                    continue;
                }
                for idx_b in 0..size {
                    if inline_paths.get(intermediate, idx_b) {
                        inline_paths.set(idx_a, idx_b, true);
                    }
                }
            }
        }

        self.inline_paths = inline_paths;
    }
}

#[cfg(test)]
mod tests {
    use super::Edge;

    #[test]
    fn test_edge_ord() {
        assert!(Edge::Inline > Edge::Ref);
    }

    mod square_matrix {
        use super::super::SquareMatrix;

        use pretty_assertions::assert_eq;
        use rstest::rstest;

        #[test]
        fn new_creates_empty_matrix() {
            let matrix = SquareMatrix::<u32>::new();

            assert_eq!(matrix.size(), 0);
            assert_eq!(matrix.matrix, Vec::<Vec<u32>>::new());
        }

        #[rstest]
        #[case::empty(0, 7, vec![])]
        #[case::one(1, 7, vec![vec![7]])]
        #[case::multiple(3, 7, vec![vec![7; 3]; 3])]
        fn new_with_size_fills_square_matrix(
            #[case] size: usize,
            #[case] value: u32,
            #[case] expected: Vec<Vec<u32>>,
        ) {
            let matrix = SquareMatrix::new_with_size(size, value);

            assert_eq!(matrix.size(), size);
            assert_eq!(matrix.matrix, expected);
        }

        #[test]
        fn new_with_size_default_fills_with_zeroes() {
            let matrix = SquareMatrix::<u32>::new_with_size_default(2);

            assert_eq!(matrix.matrix, vec![vec![0; 2]; 2]);
        }

        #[test]
        fn increment_size_extends_rows_and_adds_row() {
            let mut matrix = SquareMatrix::new_with_size(2, 1_u32);
            matrix.set(0, 1, 9);

            matrix.increment_size(7);

            assert_eq!(matrix.size(), 3);
            assert_eq!(
                matrix.matrix,
                vec![vec![1, 9, 7], vec![1, 1, 7], vec![7, 7, 7]]
            );
        }

        #[test]
        fn increment_empty_matrix_with_default_creates_zero_cell() {
            let mut matrix = SquareMatrix::<u32>::new();

            matrix.increment_size_with_default();

            assert_eq!(matrix.matrix, vec![vec![0]]);
        }

        #[test]
        fn get_and_set_access_cells_and_set_returns_old_value() {
            let mut matrix = SquareMatrix::new_with_size(2, 3_u32);

            assert_eq!(matrix.get(1, 0), 3);
            assert_eq!(matrix.set(1, 0, 8), 3);
            assert_eq!(matrix.get(1, 0), 8);
            assert_eq!(matrix.get(0, 1), 3);
        }

        #[rstest]
        #[case::x(2, 0)]
        #[case::y(0, 2)]
        #[should_panic]
        fn get_panics_when_an_index_is_out_of_bounds(#[case] x: usize, #[case] y: usize) {
            SquareMatrix::new_with_size(2, 0_u32).get(x, y);
        }

        #[rstest]
        #[case::x(2, 0)]
        #[case::y(0, 2)]
        #[should_panic]
        fn set_panics_when_an_index_is_out_of_bounds(#[case] x: usize, #[case] y: usize) {
            SquareMatrix::new_with_size(2, 0_u32).set(x, y, 1);
        }
    }

    mod gen_graph {
        use super::super::{DiGraph, Edge, SquareMatrix};

        use pretty_assertions::assert_eq;
        use rstest::{fixture, rstest};

        #[fixture]
        fn graph() -> DiGraph<u32> {
            DiGraph::from_roots(vec![10, 20, 30])
        }

        fn assert_inline_paths<const N: usize>(graph: &DiGraph<u32>, expected: [[bool; N]; N]) {
            for (idx_a, row) in expected.iter().enumerate() {
                for (idx_b, expected) in row.iter().enumerate() {
                    assert_eq!(graph.inline_path_exists(idx_a, idx_b), *expected);
                }
            }

            let expected_matrix = SquareMatrix {
                matrix: expected.iter().map(|row| row.to_vec()).collect(),
            };
            assert_eq!(graph.inline_matrix(), &expected_matrix);
        }

        #[test]
        fn new_creates_empty_graph() {
            let graph = DiGraph::<u32>::new();

            assert!(graph.nodes().is_empty());
            assert_eq!(graph.edges().size(), 0);
            assert_eq!(graph.inline_matrix().size(), 0);
        }

        #[rstest]
        #[case::empty(vec![])]
        #[case::multiple(vec![10, 20, 30])]
        fn from_roots_initializes_nodes_and_matrices(#[case] roots: Vec<u32>) {
            let graph = DiGraph::from_roots(roots.clone());
            let size = roots.len();

            assert_eq!(graph.nodes(), roots);
            assert_eq!(graph.edges(), &SquareMatrix::new_with_size_default(size));
            assert_eq!(
                graph.inline_matrix(),
                &SquareMatrix::new_with_size_default(size)
            );
        }

        #[test]
        fn from_roots_skips_duplicate_entries() {
            let graph = DiGraph::from_roots(vec![10, 20, 10, 30, 20]);

            assert_eq!(graph.nodes(), &[10, 20, 30]);
            assert_eq!(graph.edges(), &SquareMatrix::new_with_size_default(3));
            assert_eq!(
                graph.inline_matrix(),
                &SquareMatrix::new_with_size_default(3)
            );
        }

        #[test]
        fn add_node_to_empty_graph_creates_one_by_one_matrices() {
            let mut graph = DiGraph::<u32>::new();

            assert_eq!(graph.add_node(10), 0);
            assert_eq!(graph.nodes(), &[10]);
            assert_eq!(graph.edges(), &SquareMatrix::new_with_size_default(1));
            assert_eq!(
                graph.inline_matrix(),
                &SquareMatrix::new_with_size_default(1)
            );
        }

        #[rstest]
        fn add_node_adds_only_new_nodes_and_grows_matrices_without_losing_edges(
            mut graph: DiGraph<u32>,
        ) {
            graph.add_edge(0, 1, Edge::Ref);
            graph.add_edge(1, 2, Edge::Inline);

            assert_eq!(graph.add_node(40), 3);
            assert_eq!(graph.add_node(20), 1);
            assert_eq!(graph.nodes(), &[10, 20, 30, 40]);
            assert_eq!(graph.edges().size(), 4);
            assert_eq!(graph.inline_matrix().size(), 4);
            assert_eq!(graph.get_edge(0, 1), Some(Edge::Ref));
            assert_eq!(graph.get_edge(3, 0), None);
            assert!(graph.inline_path_exists(1, 2));
            assert!(!graph.inline_path_exists(3, 0));
        }

        #[rstest]
        #[case::first(0, Some(10))]
        #[case::last(2, Some(30))]
        #[case::out_of_bounds(3, None)]
        fn get_node(graph: DiGraph<u32>, #[case] index: usize, #[case] expected: Option<u32>) {
            assert_eq!(graph.get_node(index), expected);
        }

        #[rstest]
        #[case::first(10, Some(0))]
        #[case::last(30, Some(2))]
        #[case::missing(40, None)]
        fn lookup_node(graph: DiGraph<u32>, #[case] node: u32, #[case] expected: Option<usize>) {
            assert_eq!(graph.lookup_node(node), expected);
        }

        #[rstest]
        fn add_edge_sets_directed_edge_and_returns_previous_value(mut graph: DiGraph<u32>) {
            assert_eq!(graph.get_edge(0, 1), None);
            assert_eq!(graph.add_edge(0, 1, Edge::Ref), None);
            assert_eq!(graph.get_edge(0, 1), Some(Edge::Ref));
            assert_eq!(graph.get_edge(1, 0), None);

            assert_eq!(graph.add_edge(0, 1, Edge::Inline), Some(Edge::Ref));
            assert_eq!(graph.get_edge(0, 1), Some(Edge::Inline));
            assert_eq!(graph.add_edge(0, 1, Edge::Inline), Some(Edge::Inline));
            assert_eq!(
                graph.edges(),
                &SquareMatrix {
                    matrix: vec![
                        vec![None, Some(Edge::Inline), None],
                        vec![None; 3],
                        vec![None; 3],
                    ],
                }
            );
        }

        #[rstest]
        fn replacing_ref_edge_with_inline_adds_inline_reachability(mut graph: DiGraph<u32>) {
            graph.add_edge(0, 1, Edge::Ref);

            assert_eq!(graph.add_edge(0, 1, Edge::Inline), Some(Edge::Ref));

            assert!(graph.inline_path_exists(0, 1));
        }

        #[rstest]
        #[case::x(3, 0)]
        #[case::y(0, 3)]
        #[should_panic]
        fn get_edge_panics_when_an_index_is_out_of_bounds(
            graph: DiGraph<u32>,
            #[case] x: usize,
            #[case] y: usize,
        ) {
            graph.get_edge(x, y);
        }

        #[rstest]
        #[case::x(3, 0)]
        #[case::y(0, 3)]
        #[should_panic]
        fn add_edge_panics_when_an_index_is_out_of_bounds(
            mut graph: DiGraph<u32>,
            #[case] x: usize,
            #[case] y: usize,
        ) {
            graph.add_edge(x, y, Edge::Ref);
        }

        #[rstest]
        #[case::x(3, 0)]
        #[case::y(0, 3)]
        #[should_panic]
        fn inline_path_exists_panics_when_an_index_is_out_of_bounds(
            graph: DiGraph<u32>,
            #[case] x: usize,
            #[case] y: usize,
        ) {
            graph.inline_path_exists(x, y);
        }

        #[test]
        fn inline_reachability_is_transitive_and_order_independent() {
            let expected = [
                [false, true, true],
                [false, false, true],
                [false, false, false],
            ];
            let mut forward = DiGraph::from_roots(vec![10_u32, 20, 30]);
            let mut reverse = forward.clone();

            forward.add_edge(0, 1, Edge::Inline);
            forward.add_edge(1, 2, Edge::Inline);
            reverse.add_edge(1, 2, Edge::Inline);
            reverse.add_edge(0, 1, Edge::Inline);

            assert_inline_paths(&forward, expected);
            assert_inline_paths(&reverse, expected);
        }

        #[test]
        fn inline_edge_connects_all_predecessors_to_all_successors() {
            let mut graph = DiGraph::from_roots(vec![10_u32, 20, 30, 40, 50]);
            graph.add_edge(0, 2, Edge::Inline);
            graph.add_edge(1, 2, Edge::Inline);
            graph.add_edge(3, 4, Edge::Inline);

            graph.add_edge(2, 3, Edge::Inline);

            assert_inline_paths(
                &graph,
                [
                    [false, false, true, true, true],
                    [false, false, true, true, true],
                    [false, false, false, true, true],
                    [false, false, false, false, true],
                    [false; 5],
                ],
            );
        }

        #[test]
        fn inline_cycle_marks_every_node_in_cycle_as_self_referring() {
            let mut graph = DiGraph::from_roots(vec![10_u32, 20, 30]);
            graph.add_edge(0, 1, Edge::Inline);
            graph.add_edge(1, 2, Edge::Inline);
            graph.add_edge(2, 0, Edge::Inline);

            assert_inline_paths(&graph, [[true; 3]; 3]);
        }

        #[test]
        fn inline_self_edge_marks_node_as_self_referring() {
            let mut graph = DiGraph::from_roots(vec![10_u32, 20]);

            graph.add_edge(1, 1, Edge::Inline);

            assert_inline_paths(&graph, [[false, false], [false, true]]);
        }

        #[test]
        fn ref_edges_do_not_create_inline_reachability() {
            let mut graph = DiGraph::from_roots(vec![10_u32, 20]);

            graph.add_edge(0, 1, Edge::Ref);
            graph.add_edge(1, 1, Edge::Ref);

            assert_inline_paths(&graph, [[false; 2]; 2]);
        }

        #[test]
        fn ref_edge_interrupts_inline_path() {
            let mut graph = DiGraph::from_roots(vec![10_u32, 20, 30, 40]);
            graph.add_edge(0, 1, Edge::Inline);
            graph.add_edge(1, 2, Edge::Ref);
            graph.add_edge(2, 3, Edge::Inline);

            assert_inline_paths(
                &graph,
                [
                    [false, true, false, false],
                    [false; 4],
                    [false, false, false, true],
                    [false; 4],
                ],
            );
        }

        #[test]
        fn replacing_inline_edge_recomputes_reachability() {
            let mut graph = DiGraph::from_roots(vec![10_u32, 20, 30]);
            graph.add_edge(0, 1, Edge::Inline);
            graph.add_edge(1, 2, Edge::Inline);
            assert!(graph.inline_path_exists(0, 2));

            assert_eq!(graph.add_edge(1, 2, Edge::Ref), Some(Edge::Inline));

            assert_inline_paths(&graph, [[false, true, false], [false; 3], [false; 3]]);
        }

        #[test]
        fn replacing_inline_edge_preserves_alternate_inline_path() {
            let mut graph = DiGraph::from_roots(vec![10_u32, 20, 30]);
            graph.add_edge(0, 1, Edge::Inline);
            graph.add_edge(0, 2, Edge::Inline);
            graph.add_edge(2, 1, Edge::Inline);

            graph.add_edge(0, 1, Edge::Ref);

            assert_inline_paths(
                &graph,
                [[false, true, true], [false; 3], [false, true, false]],
            );
        }
    }
}

#[cfg(feature = "v2")]
use daml_lf::package::VersionedSealedPackage;

use crate::{
    generator::GenCtx,
    ir::{Definition, Edge, GenGraph},
};

#[derive(Clone, Copy, Debug)]
pub struct GenGraphBuilder<'a> {
    ctx: &'a GenCtx<'a>,
}

impl<'a> GenGraphBuilder<'a> {
    pub fn build(ctx: &'a GenCtx<'a>) -> GenGraph<'a> {
        let builder = Self { ctx };
        let roots = builder.discover_templates_and_interfaces();
        builder.build_from_roots(roots)
    }

    fn build_from_roots(&self, roots: Vec<Definition<'a>>) -> GenGraph<'a> {
        let mut queue = roots.clone();
        let mut graph = GenGraph::from_roots(roots);

        while let Some(node) = queue.pop() {
            // This node is supposed to already be in the graph, so this will simply get an index
            let node_idx = graph.add_node(node);

            let dependencies = self.discover_dependencies(node);
            for (dep_node, edge) in dependencies {
                // Add dependency to the graph
                let dep_idx = graph.add_node(dep_node);

                // If we've seen definition in dep node before, don't queue it again
                if let Some(existing_edge) = graph.get_edge(node_idx, dep_idx) {
                    // If new edge is stronger, update it
                    if existing_edge < edge {
                        graph.add_edge(node_idx, dep_idx, edge);
                    }
                } else {
                    // Add unseen dependency to graph and queue it
                    graph.add_edge(node_idx, dep_idx, edge);
                    queue.push(dep_node);
                }
            }
        }

        graph
    }

    fn discover_templates_and_interfaces(&self) -> Vec<Definition<'a>> {
        let mut nodes = Vec::new();
        for (package_id, package) in self.ctx.packages() {
            let package_nodes = match package.package.versioned() {
                #[cfg(feature = "v2")]
                VersionedSealedPackage::V2(package_v2) => {
                    use crate::v2::deps_resolver::DepsResolver as DepsResolverV2;
                    use crate::v2::ir::Definition as NodeV2;

                    DepsResolverV2::discover_templates_and_interfaces(package_v2)
                        .into_iter()
                        .map(|definition| Definition::V2(NodeV2::new(package_id, definition)))
                }
            };
            nodes.extend(package_nodes);
        }
        nodes
    }

    fn discover_dependencies(&self, node: Definition<'a>) -> Vec<(Definition<'a>, Edge)> {
        match node {
            #[cfg(feature = "v2")]
            Definition::V2(node) => {
                use crate::v2::deps_resolver::DepsResolver as DepsResolverV2;

                DepsResolverV2::discover_dependencies(self.ctx, node)
            }
        }
    }
}

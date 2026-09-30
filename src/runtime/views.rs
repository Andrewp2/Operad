//! Explicit input dependencies for reusable, document-authored view sections.

#[cfg(test)]
mod tests;

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::panic::Location;

use crate::core::document::view_fragment::ViewFragment;
use crate::{UiDocument, UiDocumentScale, UiNodeId, UiSize};

/// Work performed while building the latest application description.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ViewBuildStats {
    pub rebuilt: usize,
    pub reused: usize,
}

trait ViewInputs {
    fn matches(&self, other: &dyn Any) -> bool;
}

impl<T: PartialEq + 'static> ViewInputs for T {
    fn matches(&self, other: &dyn Any) -> bool {
        other.downcast_ref::<T>().is_some_and(|other| self == other)
    }
}

struct ViewEntry {
    inputs: Box<dyn ViewInputs>,
    builder_type: &'static str,
    call_site: &'static Location<'static>,
    fragment: ViewFragment,
    children: ViewCache,
}

#[derive(Default)]
pub(crate) struct ViewCache {
    entries: HashMap<Vec<String>, ViewEntry>,
    environment: Option<(UiSize, UiDocumentScale)>,
    pub stats: ViewBuildStats,
}

impl std::fmt::Debug for ViewCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ViewCache")
            .field("sections", &self.entries.len())
            .field("stats", &self.stats)
            .finish()
    }
}

impl ViewCache {
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn build(
        &mut self,
        viewport: UiSize,
        scale: UiDocumentScale,
        build: impl FnOnce(UiSize, &mut ViewContext<'_>) -> UiDocument,
    ) -> UiDocument {
        if self.environment != Some((viewport, scale)) {
            self.clear();
            self.environment = Some((viewport, scale));
        }
        let mut stats = ViewBuildStats::default();
        let mut context = ViewContext {
            cache: self,
            seen: HashSet::new(),
            stats: &mut stats,
        };
        let document = build(viewport, &mut context);
        context.finish();
        self.stats = stats;
        document
    }
}

/// Session-owned view construction, supplied to native, web, and custom hosts.
///
/// Use [`Self::section`] around expensive panels. Every application value read
/// by its builder (including theme, selection, and child inputs) must be part of
/// `inputs`. A revision counter is also valid if the application updates it for
/// every relevant change. Builders must not perform application side effects.
/// Viewport and scale changes invalidate all sections automatically.
pub struct ViewContext<'a> {
    cache: &'a mut ViewCache,
    seen: HashSet<Vec<String>>,
    stats: &'a mut ViewBuildStats,
}

impl ViewContext<'_> {
    /// Append a named section, reusing it when its inputs and call site match.
    ///
    /// The builder returns a document whose root becomes the named section.
    /// Node IDs inside that document are local; the returned ID belongs to the
    /// destination document. Accessibility relations, layout constraints, and
    /// stacking references are remapped. Named portals are local to the section;
    /// app-overlay portals retain their viewport scope.
    ///
    /// Names must be unique among siblings. Moving a section to another parent
    /// starts a new lifetime. Removed sections are evicted on the next build.
    /// Nested sections can reuse their previous inputs when their parent rebuilds;
    /// an unchanged parent reuses its entire authored subtree.
    /// Different calls to `section`, such as branches that describe alternative
    /// panels, rebuild even with equal inputs. A wrapper can use `#[track_caller]`
    /// to forward its caller's location. If one call site selects builders at
    /// runtime, include that selection in `inputs` along with captured values.
    #[track_caller]
    pub fn section<I, F>(
        &mut self,
        document: &mut UiDocument,
        parent: UiNodeId,
        name: impl Into<String>,
        inputs: &I,
        build: F,
    ) -> UiNodeId
    where
        I: Clone + PartialEq + 'static,
        F: FnOnce(&I, &mut ViewContext<'_>) -> UiDocument,
    {
        let name = name.into();
        let mut path = vec![name.clone()];
        let mut ancestor = Some(parent);
        while let Some(id) = ancestor {
            let node = document.node(id);
            path.push(node.name().to_owned());
            ancestor = node.parent();
        }
        path.reverse();
        assert!(
            self.seen.insert(path.clone()),
            "duplicate view section: {path:?}"
        );
        assert!(
            !document
                .node(parent)
                .children()
                .iter()
                .any(|id| document.node(*id).name() == name),
            "view section names must be unique among siblings: {path:?}"
        );
        let builder_type = std::any::type_name::<F>();
        // Diagnostic type names do not distinguish all closure types. The call
        // site keeps alternative section definitions from sharing stale content.
        let call_site = Location::caller();
        if let Some(entry) = self.cache.entries.get(&path) {
            if entry.call_site == call_site
                && entry.builder_type == builder_type
                && entry.inputs.matches(inputs)
            {
                self.stats.reused += 1;
                return document.append_view_fragment(parent, name, &entry.fragment, true);
            }
        }

        let mut children = self
            .cache
            .entries
            .remove(&path)
            .map(|entry| entry.children)
            .unwrap_or_default();
        let mut nested = ViewContext {
            cache: &mut children,
            seen: HashSet::new(),
            stats: self.stats,
        };
        let mut authored = build(inputs, &mut nested);
        nested.finish();
        self.stats.rebuilt += 1;
        let uploads = authored.take_resource_updates();
        let fragment = ViewFragment::from_document(authored);
        let root = document.append_view_fragment(parent, name, &fragment, false);
        document.append_resource_updates(uploads);
        self.cache.entries.insert(
            path,
            ViewEntry {
                inputs: Box::new(inputs.clone()),
                builder_type,
                call_site,
                fragment,
                children,
            },
        );
        root
    }

    fn finish(self) {
        self.cache
            .entries
            .retain(|path, _| self.seen.contains(path));
    }
}

use crate::{concentration::ElementsConcentrationsWithAliases, elements::KnownElements};
use dyn_clone::DynClone;

/// A generic representation of the fertilizer, must return components percentage for the fertilizer
pub trait Fertilizer : DynClone {
	fn components_percentage(&self, known_elts: &KnownElements) -> Vec<ElementsConcentrationsWithAliases>;
	fn name(&self) -> &str;
}

dyn_clone::clone_trait_object!(Fertilizer);

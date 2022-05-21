use crate::{concentration::ElementsConcentrationsWithAliases, elements::KnownElements};

/// A generic representation of the fertilizer, must return components percentage for the fertilizer
pub trait Fertilizer {
	fn components_percentage(&self, known_elts: &KnownElements) -> Vec<ElementsConcentrationsWithAliases>;
	fn name(&self) -> &str;
}

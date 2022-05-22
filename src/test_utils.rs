use crate::{elements::KnownElements, tank::Tank};
use std::path::Path;

#[macro_export]
macro_rules! assert_delta_eq {
	($x:expr, $y:expr, $d:expr) => {
		assert!(
			($x >= $y && $x - $y < $d) || ($x < $y && $y - $x < $d),
			"assert_delta_eq!({}, {}); {:?} != {:?}",
			stringify!($x),
			stringify!($y),
			$x,
			$y
		)
	};
}

pub const MOLAR_MASS_EPSILON: f64 = 0.001;

pub fn load_known_elements() -> KnownElements {
	KnownElements::new_with_db(Path::new("./elements.json")).unwrap()
}

pub fn sample_tank() -> Tank {
	Tank::new_from_json(
		r#"{
		"volume": 200
		}"#,
	)
	.unwrap()
}

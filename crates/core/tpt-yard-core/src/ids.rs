//! Newtype identifiers shared across the shipyard domain.

/// Declares a lightweight, copyable newtype id wrapping a `u64`.
///
/// The ids serialize as plain JSON numbers and display with a short prefix
/// (e.g. `P3`, `B12`).
#[macro_export]
macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident, $prefix:expr) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
        )]
        pub struct $name(pub u64);

        impl $name {
            /// The prefix used by [`std::fmt::Display`].
            pub const PREFIX: &'static str = $prefix;
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}{}", $prefix, self.0)
            }
        }

        impl From<u64> for $name {
            fn from(v: u64) -> Self {
                Self(v)
            }
        }

        impl From<$name> for u64 {
            fn from(id: $name) -> u64 {
                id.0
            }
        }
    };
}

define_id!(
    /// Identifier of a vessel project.
    ProjectId,
    "P"
);
define_id!(
    /// Identifier of a build phase within a project.
    PhaseId,
    "PH"
);
define_id!(
    /// Identifier of a weight item.
    ItemId,
    "W"
);
define_id!(
    /// Identifier of a hull block.
    BlockId,
    "B"
);
define_id!(
    /// Identifier of a robotic arm.
    RobotId,
    "R"
);
define_id!(
    /// Identifier of a structural component in an orbital assembly.
    ComponentId,
    "C"
);
define_id!(
    /// Identifier of an assembly step in an orbital assembly sequence.
    StepId,
    "S"
);
define_id!(
    /// Identifier of a weld or weld seam.
    WeldId,
    "WL"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_display_and_convert() {
        let p = ProjectId(7);
        assert_eq!(p.to_string(), "P7");
        assert_eq!(ProjectId::PREFIX, "P");
        assert_eq!(u64::from(p), 7);
        assert_eq!(ProjectId::from(7u64), p);
        assert_eq!(BlockId::default(), BlockId(0));
        assert!(PhaseId(2) > PhaseId(1));
    }
}

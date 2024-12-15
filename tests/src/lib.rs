// Unit tests for the Commune Ai blockchain
// ========================================
//
// ! Commune does not write unit tests at the pallet level as you see.
// ! We try to preserve a simple clean architecture.
// ! To avoid redundancy and repetetion in mock modules, as well as circular dependencies.
#[cfg(test)]
pub mod encryption;
#[cfg(test)]
pub mod governance;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
pub mod offworker;
#[cfg(test)]
pub mod root;
#[cfg(test)]
pub mod subnet_emission;
#[cfg(test)]
pub mod subspace;

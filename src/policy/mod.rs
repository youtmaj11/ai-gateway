// OPA client and policy loader

pub mod opa;

pub use opa::{PolicyEnforcer, PolicyError, UserContext};

pub fn initialize() {
    // placeholder for policy initialization
}

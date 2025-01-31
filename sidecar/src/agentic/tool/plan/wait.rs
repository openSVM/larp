//! The wait tool allows the LLM to wait ... taking a step back when required
//! and focussing attention on something, all new llms are used to doing this internall
//! this can be used to also reduce the context from exploding

use crate::repo::types::RepoRef;

pub struct WaitInputPartial {
    repo_ref: RepoRef,
}

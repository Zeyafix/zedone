use serde::{Serialize, de::DeserializeOwned};
use utoipa::ToSchema;

use crate::error::CommandError;

pub trait Command: DeserializeOwned + Serialize + ToSchema {
    const NAME: &'static str;
    type Output: Serialize + ToSchema;
}

pub type CommandResult<T> = Result<T, CommandError>;

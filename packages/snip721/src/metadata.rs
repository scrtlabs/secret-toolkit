use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// The response of both NftINfo and PrivateMetadata queries are Metadata
//

/// token metadata
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug)]
pub struct Metadata {
    /// optional indentifier
    pub name: Option<String>,
    /// optional description
    pub description: Option<String>,
    /// optional uri to contain an image, additional data fields, etc...
    pub image: Option<String>,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct BlockDefinitionsFile {
    #[serde(rename = "block")]
    pub blocks: Vec<BlockDefinition>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BlockDefinition {
    pub id: u8,
    pub name: String,
    #[serde(default)]
    pub flags: Vec<String>,
    #[serde(default)]
    pub ticks: Vec<String>,
    #[serde(default)]
    pub values: Vec<String>, // or Vec<f32> if numeric
    #[serde(default)]
    pub fields: Vec<String>,
    #[serde(default)]
    pub colors: Vec<String>,
    #[serde(default)]
    pub gradients: Vec<String>,
    #[serde(default)]
    pub vectors: Vec<String>,
}
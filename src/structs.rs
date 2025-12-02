#[derive(Debug, Default)]
/// Represents an entire assembled structure.
/// 
/// A `Building` is composed of one or more roots (rigid bodies) and a flat list
/// of all blocks that appear in the structure. Each root acts as a physically
/// independent object, while blocks describe individual elements attached to
/// those roots.
pub struct Building {
    /// All physical roots connected to the building.  
    /// Each root behaves as a separate rigid body.
    pub roots: Vec<Root>,

    /// All blocks that belong to the building.  
    /// Blocks reference their parent root by index.
    pub blocks: Vec<Block>,
}

#[derive(Debug, Default)]
/// A physically independent part of a building.
/// 
/// A `Root` is a rigid body that can contain multiple blocks.  
/// Its global transform (position + rotation) defines how the entire root is
/// placed in the world. This transform does **not** directly modify block-world
/// coordinates; blocks store their own world-space transforms.
pub struct Root {
    /// World-space position of the root.  
    /// Used for placing the root on bearings, shock absorbers, or supports.
    pub position: [f32; 3],

    /// World-space rotation of the root, same semantics as `position`.
    pub rotation: [f32; 3],
}

#[derive(Clone, Debug, Default)]
/// A single element in a building.
///
/// Every `Block` is **always part of a `Root`**, and its `root` field
/// references the index of that root in the building's `root`'s vector.
///
/// Blocks store their own world-space position and rotation independently
/// of the root, but the root defines the physical grouping and overall
/// transform of all blocks attached to it.
pub struct Block {
    /// World-space position of the block.
    pub position: [f32; 3],

    /// World-space rotation of the block.
    pub rotation: [f32; 3],

    /// Numeric block‐type identifier.
    pub id: u8,

    /// Index of the root that this block belongs to.
    /// Every block must be attached to a root.
    pub root: u16,

    /// Additional block settings and data.
    pub metadata: Option<Metadata>,

    /// Human-readable block name (VLC-encoded in serialized formats).
    pub name: String,

    /// A user-facing adjustable value (e.g., slider output, button state, etc.).
    pub enable_state: f32,

    /// Internal state used for various reasons.  
    /// Often represents the "current" or "interpolated" enable state.
    pub enable_state_current: f32,

    /// Connections to other blocks, stored as block indices.
    pub connections: Vec<u16>,

    /// Index of a block from another root that is mechanically attached  
    /// to this one (used for bearings, shock absorbers, etc.).
    pub load: Option<u16>,

    /// Block color.  
    /// In versions above 0, this is serialized in RGB565 format.
    pub color: Option<[u8; 4]>,
}

#[derive(Clone, Debug)]
/// A color gradient consisting of color and alpha keys.
/// 
/// Each gradient is defined by color values over normalized time and alpha
/// (opacity) over normalized time. The vectors define the full keyed curve.
pub struct Gradient {
    pub color_keys: Vec<[u8; 4]>,
    pub color_time_keys: Vec<f32>,
    pub alpha_keys: Vec<f32>,
    pub alpha_time_keys: Vec<f32>,
}

#[derive(Clone, Debug, Default)]
/// All per-block editable settings.
/// 
/// `Metadata` contains a variety of UI-driven values used by different block
/// types: toggles, numeric values, colors, gradients, and custom type settings.
pub struct Metadata {
    /// Boolean toggle values (e.g., switches).
    pub toggles: Vec<bool>,

    /// Floating-point numeric parameters.
    pub values: Vec<f32>,

    /// Defines in which slots outcomming connections are placed (used in blocks like seat, tank chasis controller, etc).
    pub fields: Vec<Vec<u16>>,

    /// Values for dropdown-style settings.
    pub dropdowns: Vec<u8>,

    /// RGBA color fields.
    pub colors: Vec<[u8; 4]>,

    /// Gradient definitions.
    pub gradients: Vec<Gradient>,

    /// 3D vector settings.
    pub vectors: Vec<[f32; 3]>,

    /// Optional advanced settings that depend on the block type.
    pub type_settings: TypeSettings,
}

#[derive(Clone, Debug)]
/// Additional metadata specific to certain block types.
///
/// `TypeSettings` defines extra configuration for a block based on its type (`id`).
/// If a block receives a `TypeSettings` variant that doesn’t match its type,
/// default parameters are used instead. This ensures invalid or mismatched
/// configurations do not break anything.
pub enum TypeSettings {
    /// No advanced settings.
    None,

    /// Settings for math block, defining the computation and the placement of
    /// connected input blocks into specific slots.
    ///
    /// Each connected block is assigned to a slot using a pair of vectors:
    /// - `incoming_connections_order[i]` is the index of the i-th connected block.
    /// - `slots[i]` is the slot that this block should occupy.
    /// 
    /// Together, each `(incoming_connections_order[i], slots[i])` defines a
    /// connection-slot assignment.
    MathBlock {
        /// The math expression to evaluate.
        function: String,

        /// Indices of connected blocks in the building. Each element pairs
        /// with the same-index element in `slots`.
        incoming_connections_order: Vec<u8>,

        /// Slots for the connected blocks. Each element pairs with the same-index
        /// element in `incoming_connections_order`.
        slots: Vec<u8>,
    }
}

impl Default for TypeSettings {
    fn default() -> Self {
        TypeSettings::None
    }
}

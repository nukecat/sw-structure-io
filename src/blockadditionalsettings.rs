use std::rc::Weak;
use crate::block::Block;
use crate::additionaldatatypes::Gradient;

pub struct BlockAdditionalSettings {
    pub ticks: Vec<bool>,
    pub values: Vec<f32>,
    pub fields: Vec<Vec<Weak<Block>>>,
    pub colors: Vec<[f32; 4]>,
    pub gradients: Vec<Gradient>,
    pub vectors: Vec<[f32; 3]>
}
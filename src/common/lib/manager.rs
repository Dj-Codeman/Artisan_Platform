use std::fmt;

use dusa_collection_utils::stringy::Stringy;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct NetworkRequest {
    pub request_type: NetworkRequestType,
    pub data: Option<Stringy>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum NetworkRequestType {
    QUERYSYSTEM,
    QUERYSTATUS,
    QUERYGITREPO,
    UPDATEGITREPO,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkResponse {
    pub status: Stringy,
    pub data: Option<Stringy>,
}

impl fmt::Display for NetworkResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Status: {}", self.status)?;
        if let Some(ref data) = self.data {
            let parsed_data: serde_json::Value = serde_json::from_str(data).unwrap();
            writeln!(f, "Data: {:#}", parsed_data)?;
        } else {
            writeln!(f, "Data: None")?;
        }
        Ok(())
    }
}

// let response_data = response.data.unwrap();
// let warning_applications: Vec<_> = response_data
//     .lines()
//     .filter(|line| line.contains("Warning"))
//     .collect();
//

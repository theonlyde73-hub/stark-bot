use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Types of associations between memories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssociationType {
    Related,
    CausedBy,
    Contradicts,
    Supersedes,
    PartOf,
    References,
    Temporal,
    AgentSubtype,
    Custom(String),
}

impl fmt::Display for AssociationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssociationType::Related => write!(f, "related"),
            AssociationType::CausedBy => write!(f, "caused_by"),
            AssociationType::Contradicts => write!(f, "contradicts"),
            AssociationType::Supersedes => write!(f, "supersedes"),
            AssociationType::PartOf => write!(f, "part_of"),
            AssociationType::References => write!(f, "references"),
            AssociationType::Temporal => write!(f, "temporal"),
            AssociationType::AgentSubtype => write!(f, "agent_subtype"),
            AssociationType::Custom(value) => write!(f, "custom:{}", value),
        }
    }
}

impl FromStr for AssociationType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "related" => Ok(AssociationType::Related),
            "caused_by" => Ok(AssociationType::CausedBy),
            "contradicts" => Ok(AssociationType::Contradicts),
            "supersedes" => Ok(AssociationType::Supersedes),
            "part_of" => Ok(AssociationType::PartOf),
            "references" => Ok(AssociationType::References),
            "temporal" => Ok(AssociationType::Temporal),
            "agent_subtype" => Ok(AssociationType::AgentSubtype),
            other => {
                if let Some(value) = other.strip_prefix("custom:") {
                    Ok(AssociationType::Custom(value.to_string()))
                } else {
                    Err(format!("Unknown association type: {}", other))
                }
            }
        }
    }
}

impl Serialize for AssociationType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for AssociationType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        AssociationType::from_str(&s).map_err(serde::de::Error::custom)
    }
}

/// A directed association between two memories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAssociation {
    pub id: i64,
    pub source_memory_id: i64,
    pub target_memory_id: i64,
    pub association_type: AssociationType,
    pub strength: f32,
    pub created_at: String,
    pub metadata: Option<String>,
}

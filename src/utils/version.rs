use anyhow::{anyhow, Result};
use reqwest::get;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer};

fn compare_versions(a: &str, b: &str) -> Result<i32> {
    let a_int = a.replace("v", "").replace(".", "").parse::<i32>()?;
    let b_int = b.replace("v", "").replace(".", "").parse::<i32>()?;

    if a_int < b_int {
        Ok(-1)
    } else if a_int > b_int {
        Ok(1)
    } else {
        Ok(0)
    }
}


fn deserialize_flex_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    struct FlexBoolVisitor;

    impl<'de> Visitor<'de> for FlexBoolVisitor {
        type Value = bool;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string representable as a boolean")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match value {
                "1" | "true" | "yes" | "t" | "armed" | "active" | "enabled" | "ready" | "up"
                | "ok" => Ok(true),
                _ =>{
                    if value.len() > 0 {
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                }
            }
        }

        fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(value)
        }
    }

    deserializer.deserialize_any(FlexBoolVisitor)
}

#[derive(Debug, Deserialize)]
struct VersionInfo {
    version: String,
    #[serde(deserialize_with = "deserialize_flex_bool")]
    lts: bool,
}

pub async fn find_node_js_lts_version() -> Result<String> {
    let response = get("https://nodejs.org/dist/index.json").await?;
    if !response.status().is_success() {
        return Err(anyhow!("Request failed with status: {}", response.status()));
    }

    let versions: Vec<VersionInfo> = response.json().await?;

    let mut versions_slice = Vec::new();

    for archive in versions {
        if archive.lts {
            versions_slice.push(archive.version);
            break;
        }
    }

    let mut latest_version = String::new();

    for version in versions_slice {
        if latest_version.is_empty() {
            latest_version = version;
        } else {
            if compare_versions(&version, &latest_version)? > 0 {
                latest_version = version;
            }
        }
    }

    Ok(latest_version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        match find_node_js_lts_version().await {
            Ok(version) => println!("Latest LTS version is: {}", version),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
}

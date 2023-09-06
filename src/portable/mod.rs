use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
};

use reqwest::Client;
use zip::read::ZipArchive;

use crate::utils;

pub struct NodeOptions {
    pub home: String,
    pub version: String,
}

pub struct Node {
    options: NodeOptions,
    path: PathBuf,
}

impl Default for NodeOptions {
    fn default() -> Self {
        NodeOptions {
            home: String::from("./bin/node"),
            version: String::from("lts"),
        }
    }
}

impl Default for Node {
    fn default() -> Self {
        Node::new(NodeOptions::default()).unwrap()
    }
}

impl Node {
    pub fn new(options: NodeOptions) -> Result<Self, Box<dyn std::error::Error>> {
        let node_binary = if cfg!(target_os = "windows") {
            "node.exe"
        } else {
            "node"
        };
        let node_path = format!("{}/{}", options.home, node_binary);
        let path = PathBuf::from(node_path);
        Ok(Node { options, path })
    }

    pub async fn ensure(&self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.path.exists() {
            self.download_latest_node_lts(&self.options.home)
                .await
                .unwrap();
        }
        Ok(())
    }

    pub async fn get_node_lts_download_uri() -> String {
        let version = utils::version::find_node_js_lts_version().await.unwrap();
        let so_name = utils::soutils::get_so_name();
        let arch = utils::soutils::get_arch();
        format!(
            "https://nodejs.org/dist/{}/node-{}-{}-{}.zip",
            version, version, so_name, arch
        )
    }

    async fn download_latest_node_lts(
        &self,
        dir_path_to_extract: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Downloading latest node lts");
        let uri = Self::get_node_lts_download_uri().await;

        let client = Client::new();
        let response = client.get(&uri).send().await.unwrap();
        if !response.status().is_success() {
            panic!("Failed to download Node LTS");
        }

        println!("Downloaded latest node lts");

        fs::create_dir_all("temp")?;

        let mut file = fs::File::create("temp/node.zip")?;

        let content = response.bytes().await.unwrap();

        file.write_all(&content).unwrap();

        file.sync_all()?;

        self.extract_node_lts(Path::new("temp/node.zip"), dir_path_to_extract)
            .await
            .unwrap();

        fs::remove_file(Path::new("temp/node.zip"))?;

        Ok(())
    }

    async fn extract_node_lts(
        &self,
        file_path: &Path,
        dir_path_to_extract: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Extracting node lts");
        let version = utils::version::find_node_js_lts_version().await.unwrap();
        let so_name = utils::soutils::get_so_name();
        let arch = utils::soutils::get_arch();

        let first_path_to_remove = format!("node-{}-{}-{}", version, so_name, arch);

        fs::create_dir_all(dir_path_to_extract)?;

        let file = fs::File::open(file_path)?;
        let mut archive = ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = file.mangled_name();

            let outpath = match outpath.strip_prefix(&first_path_to_remove) {
                Ok(outpath) => outpath,
                Err(_) => {
                    println!("Skipping file: {:?}", outpath);
                    continue;
                }
            };

            let outpath = PathBuf::from(dir_path_to_extract).join(outpath);

            if (&*file.name()).ends_with('/') {
                // println!("File {} extracted to \"{}\"", i, outpath.display());
                fs::create_dir_all(&outpath)?;
            } else {
                // println!(
                //     "File {} extracted to \"{}\" ({} bytes)",
                //     i,
                //     outpath.display(),
                //     file.size()
                // );
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(&p)?;
                    }
                }
                let mut outfile = fs::File::create(&outpath)?;
                io::copy(&mut file, &mut outfile)?;
            }
        }

        Ok(())
    }

    pub fn eval(&self, code: &str) -> Result<(), Box<dyn std::error::Error>> {
        let output = Command::new(&self.path)
            .args(&["-e", code])
            .envs(std::env::vars())
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .output()?;

        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr);
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                error_message.to_string(),
            )));
        }

        Ok(())
    }

    pub fn node_module(&self, name: &str) -> NodeModule {
        NodeModule::new(name)
    }
}

pub struct NodeModule {
    name: String,
    path: PathBuf,
}

impl NodeModule {
    pub fn new(path_str: &str) -> Self {
        let path = PathBuf::from(format!("{}", path_str));
        let last_name = path
            .components()
            .last()
            .unwrap()
            .as_os_str()
            .to_str()
            .unwrap();
        NodeModule {
            name: String::from(last_name),
            path,
        }
    }

    pub fn ensure(&self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.path.exists() {
            self.install()?;
        }
        Ok(())
    }

    pub fn install(&self) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(target_os = "windows")]
        let bin_path = Path::new("./bin/node/npm.cmd");
        #[cfg(not(target_os = "windows"))]
        let bin_path = Path::new("./bin/node/npm");

        let output = Command::new(bin_path)
            .current_dir(self.path.as_path())
            .args(&["install", &self.name])
            .envs(std::env::vars())
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .output()?;

        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr);
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                error_message.to_string(),
            )));
        }

        Ok(())
    }

    pub fn run(&self, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
        let bin_path = Path::new("./bin/node/node");

        let output = Command::new(bin_path)
            .current_dir(self.path.as_path())
            .args(&[".", &args.join(" ")])
            .envs(std::env::vars())
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .output()?;

        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr);
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                error_message.to_string(),
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn it_works() {
        let node = super::Node::default();
        node.ensure().await.unwrap();
        node.eval("console.log('Hello World')").unwrap();
    }
}

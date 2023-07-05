use anyhow::anyhow;
use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use reqwest::{header, Client};
use std::fs;
use std::io::Write;
use std::{
    env,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct CliApp {
    config: Config,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    versionlist_proxy_url: String,
    download_proxy_url: String,
    set_godot_bin: bool,
    set_godot4_bin: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            versionlist_proxy_url: String::from("https://github.com/"),
            download_proxy_url: String::from("https://downloads.tuxfamily.org/godotengine/"),
            set_godot_bin: true,
            set_godot4_bin: true,
        }
    }
}

pub mod godot {
    use std::{collections::HashMap, env, fmt::Display};

    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct VersionList {
        versions: HashMap<Version, String>,
    }

    #[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
    pub enum Platform {
        Win32,
        Win64,
        Linux32,
        Linux64,
        Macos,
    }

    #[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
    pub enum Suffix {
        Stable,
        Alpha(u8),
        Beta(u8),
        Rc(u8),
    }
    #[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
    pub struct Version {
        pub major: u8,
        pub minor: u8,
        pub patch: u8,
        pub suffix: Suffix,
        pub is_mono: bool,
        pub platform: Platform,
    }

    impl Display for Version {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let suffix_str = match self.suffix {
                Suffix::Alpha(x) => format!("-alpha{}", x),
                Suffix::Beta(x) => format!("-beta{}", x),
                Suffix::Rc(x) => format!("-rc{}", x),
                Suffix::Stable => format!("-stable"),
            };
            let mono_str = if self.is_mono { "_mono" } else { "_stable" };
            write!(
                f,
                "Godot_v{}.{}.{}{}{}",
                self.major, self.minor, self.patch, suffix_str, mono_str
            )
        }
    }

    impl Version {
        pub fn versnum_to_str(&self) -> String {
            format!("{}.{}.{}", self.major, self.minor, self.patch)
        }
        pub fn to_filename(&self) -> String {
            format!("{}{}.zip", self, get_platform_suffix())
        }
    }

    fn get_platform_suffix() -> String {
        match env::consts::OS {
            "linux" => format!("linux.{}", get_arch()),
            "windows" => match get_arch() {
                "x86_32" => "win32.exe",
                "x86_64" => "win64.exe",
                _ => unreachable!(),
            }
            .to_string(),
            _ => {
                unimplemented!("godotup is not available in your system currently.")
            }
        }
    }

    const fn get_arch() -> &'static str {
        let _result = "unknown";
        #[cfg(target_arch = "x86")]
        let _result = "x86_32";
        #[cfg(target_arch = "x86_64")]
        let _result = "x86_64";
        _result
    }

    #[test]
    fn test_display_version() {
        let vcs = Version {
            major: 4,
            minor: 0,
            patch: 3,
            suffix: Suffix::Stable,
            is_mono: false,
            platform: Platform::Win32,
        };
        assert_eq!(vcs.to_string(), "Godot_v4.0.3_stable");
        let vcs = Version {
            suffix: Suffix::Rc(3),
            ..vcs
        };
        assert_eq!(vcs.to_string(), "Godot_v4.0.3-rc3_stable");
        let vcs = Version {
            suffix: Suffix::Stable,
            is_mono: true,
            ..vcs
        };
        assert_eq!(vcs.to_string(), "Godot_v4.0.3_mono");
        let vcs = Version {
            suffix: Suffix::Alpha(11),
            ..vcs
        };
        assert_eq!(vcs.to_string(), "Godot_v4.0.3-alpha11_mono");
    }

    #[test]
    fn test_serde_versionlist() {
        let mut versions = HashMap::new();
        versions.insert(
            Version {
                major: 4,
                minor: 0,
                patch: 3,
                suffix: Suffix::Stable,
                is_mono: false,
                platform: Platform::Linux32,
            },
            format!("https:sss"),
        );
        versions.insert(
            Version {
                major: 4,
                minor: 0,
                patch: 0,
                suffix: Suffix::Alpha(8),
                is_mono: false,
                platform: Platform::Linux64,
            },
            format!("https:sss"),
        );
        let list = VersionList { versions };
        println!("{}", serde_yaml::to_string(&list).unwrap());
    }
}

impl CliApp {
    pub fn install_godot(&self, version: godot::Version) {
        todo!()
    }
    // async fn download_godot_package(&self, version: godot::Version) -> Result<PathBuf> {
    //     let url = self.get_url(version);
    //     let path = dirs::cache_dir()
    //         .context("cache dir not found")?
    //         .join(format!("{}.zip", version));
    //     println!("下载包 {} 到 {:?}", &url, &path);
    //     let client = Client::new();
    //     let total_size = {
    //         let resp = client.head(&url).send().await?;
    //         if resp.status().is_success() {
    //             resp.headers()
    //                 .get(header::CONTENT_LENGTH)
    //                 .and_then(|ct_len| ct_len.to_str().ok())
    //                 .and_then(|ct_len| ct_len.parse().ok())
    //                 .unwrap_or(0)
    //         } else {
    //             return Err(anyhow!(
    //                 "Couldn't download URL: {}. Error: {:?}",
    //                 &url,
    //                 resp.status(),
    //             ));
    //         }
    //     };
    //     let client = Client::new();
    //     let mut request = client.get(&url);
    //     let pb = ProgressBar::new(total_size);
    //     pb.set_style(ProgressStyle::default_bar()
    //         .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
    //         .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| {
    //             write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap();
    //         })
    //         .progress_chars("#>-"));

    //     if path.exists() {
    //         let size = path.metadata()?.len().saturating_sub(1);
    //         request = request.header(header::RANGE, format!("bytes={}-", size));
    //         pb.inc(size);
    //     }
    //     let mut source = request.send().await?;
    //     let mut dest = fs::OpenOptions::new()
    //         .create(true)
    //         .append(true)
    //         .open(&path)?;
    //     while let Some(chunk) = source.chunk().await? {
    //         dest.write_all(&chunk)?;
    //         pb.inc(chunk.len() as u64);
    //     }
    //     println!("下载完成");
    //     Ok(path)
    // }
}

// #[cfg(test)]
// mod test {
//     use crate::{godot, CliApp};

//     #[test]
//     fn test_download() {
//         let app = CliApp::default();
//         let r = tokio_test::block_on(app.download_godot_package(godot::Version {
//             major: 4,
//             minor: 0,
//             patch: 3,
//             suffix: None,
//             is_mono: false,
//         }));
//         println!("{:?}", r);
//         assert!(false)
//     }
// }

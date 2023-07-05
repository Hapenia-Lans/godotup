use anyhow::anyhow;
use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use reqwest::{header, Client};
use std::io::{Read, Write};
use std::{
    env,
    path::{Path, PathBuf},
};
use std::{fs, io};

use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct CliApp {
    config: Config,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    version_list_proxy_url: String,
    download_proxy_url: String,
    set_godot_bin: bool,
    set_godot4_bin: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version_list_proxy_url: String::from(
                "https://raw.githubusercontent.com/Hapenia-Lans/godotup/main/versions.yml",
            ),
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

    impl VersionList {
        pub fn find_url(&self, vers: &Version) -> Option<&String> {
            self.versions.get(vers)
        }
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
    #[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
            let mono_str = if self.is_mono { "_mono" } else { "" };
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
        assert_eq!(vcs.to_string(), "Godot_v4.0.3-stable");
        let vcs = Version {
            suffix: Suffix::Rc(3),
            ..vcs
        };
        assert_eq!(vcs.to_string(), "Godot_v4.0.3-rc3");
        let vcs = Version {
            suffix: Suffix::Stable,
            is_mono: true,
            ..vcs
        };
        assert_eq!(vcs.to_string(), "Godot_v4.0.3-stable_mono");
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
    pub async fn update_version_list(&self) -> Result<()> {
        let version_list = version_list_path()?;
        if version_list.exists() {
            println!("Removing old version list..");
            fs::remove_file(&version_list)?;
        }
        download_from_url(&self.config.download_proxy_url, &version_list).await?;
        Ok(())
    }

    pub async fn install_godot(&self, version: &godot::Version) -> Result<()> {
        let vcs_list = load_version_list()?;
        let url = vcs_list
            .find_url(&version)
            .context(format!("Version {} not found", &version))?;
        let tmp_path = env::temp_dir().join(&format!("{}.zip", version));
        download_from_url(url, &tmp_path).await?;
        unzip(&tmp_path, &godot_version_dir(&version))?;
        Ok(())
    }

    pub fn switch(&self, version: &godot::Version) -> Result<()> {
        // find godot executable position
        // set GODOT_HOME
        // set GODOT_BIN
        // set GODOT4_BIN (optional)
        // add shortcuts
        todo!()
    }
}

fn appdata_dir() -> Result<PathBuf> {
    let dir = dirs::data_local_dir()
        .context("Data dir not found")?
        .join("godotup");
    // fs::create_dir_all(&dir)?;
    if !dir.exists() {
        fs::create_dir(&dir)?;
    }
    Ok(dir)
}

async fn download_from_url(url: &str, path: &Path) -> Result<()> {
    println!("Downloading {} to {:?}...", url, path);
    let client = Client::new();
    let total_size = {
        let resp = client.head(url).send().await?;
        if resp.status().is_success() {
            resp.headers()
                .get(header::CONTENT_LENGTH)
                .and_then(|ct_len| ct_len.to_str().ok())
                .and_then(|ct_len| ct_len.parse().ok())
                .unwrap_or(0)
        } else {
            return Err(anyhow!(
                "Couldn't download URL: {}. Error: {:?}",
                url,
                resp.status(),
            ));
        }
    };
    let client = Client::new();
    let mut request = client.get(url);
    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
        .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| {
            write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap();
        })
        .progress_chars("#>-"));

    if path.exists() {
        let size = path.metadata()?.len().saturating_sub(1);
        request = request.header(header::RANGE, format!("bytes={}-", size));
        pb.inc(size);
    }
    let mut source = request.send().await?;
    let mut dest = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    while let Some(chunk) = source.chunk().await? {
        dest.write_all(&chunk)?;
        pb.inc(chunk.len() as u64);
    }
    println!("Completed!");
    Ok(())
}

fn version_list_path() -> Result<PathBuf> {
    Ok(appdata_dir()?.join("versions.yml"))
}

fn load_version_list() -> Result<godot::VersionList> {
    let path = version_list_path()?;
    let mut file = fs::File::open(&path)?;
    let mut str = String::new();
    file.read_to_string(&mut str)?;
    Ok(serde_yaml::from_str::<godot::VersionList>(&str)?)
}

fn godot_version_dir(vcs: &godot::Version) -> PathBuf {
    dirs::home_dir()
        .unwrap()
        .join(".godotup")
        .join(&format!("{}", vcs))
}

use zip;

fn unzip(from: &Path, to: &Path) -> Result<()> {
    let file = fs::File::open(from)?;
    let mut archive = zip::ZipArchive::new(file).unwrap();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = match file.enclosed_name() {
            Some(path) => to.join(path),
            None => continue,
        };

        {
            let comment = file.comment();
            if !comment.is_empty() {
                println!("File {i} comment: {comment}");
            }
        }

        if (*file.name()).ends_with('/') {
            println!("File {} extracted to \"{}\"", i, outpath.display());
            fs::create_dir_all(&outpath).unwrap();
        } else {
            println!(
                "File {} extracted to \"{}\" ({} bytes)",
                i,
                outpath.display(),
                file.size()
            );
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).unwrap();
                }
            }
            let mut outfile = fs::File::create(&outpath).unwrap();
            io::copy(&mut file, &mut outfile).unwrap();
        }

        // Get and Set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).unwrap();
            }
        }
    }
    Ok(())
}

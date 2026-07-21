use std::sync::Mutex;

pub use dbx_core::update::UpdateInfo;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

const OFFICIAL_UPDATE_ENDPOINTS: [&str; 2] = [
    "https://dl.dbxio.com/releases/latest/latest.json",
    "https://github.com/t8y2/dbx/releases/latest/download/latest.json",
];
const R2_LATEST_RELEASE_DOWNLOAD_PREFIX: &str = "https://dl.dbxio.com/releases/latest/";
const CNB_RELEASE_DOWNLOAD_PREFIX: &str = "https://cnb.cool/dbxio.com/dbx/-/releases/download/";
const GITHUB_RELEASE_DOWNLOAD_PREFIX: &str = "https://github.com/t8y2/dbx/releases/download/";
const UPDATE_DOWNLOAD_PROGRESS_EVENT: &str = "update-download-progress";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateDownloadSource {
    Official,
    Cnb,
}

#[derive(Clone, Debug, Serialize)]
pub struct UpdateDownloadProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

enum PendingUpdate {
    Downloading,
    Ready { bytes: Vec<u8> },
}

#[derive(Default)]
pub struct PendingUpdateState {
    pending: Mutex<Option<PendingUpdate>>,
}

impl PendingUpdateState {
    fn begin_download(&self) -> Result<(), String> {
        let mut pending = self.pending.lock().map_err(|_| "Update state is unavailable.".to_string())?;
        if pending.is_some() {
            return Err("An update is already downloading or ready to install.".to_string());
        }
        *pending = Some(PendingUpdate::Downloading);
        Ok(())
    }

    fn finish_download(&self, bytes: Vec<u8>) -> Result<(), String> {
        let mut pending = self.pending.lock().map_err(|_| "Update state is unavailable.".to_string())?;
        *pending = Some(PendingUpdate::Ready { bytes });
        Ok(())
    }

    fn cancel_download(&self) {
        if let Ok(mut pending) = self.pending.lock() {
            if matches!(pending.as_ref(), Some(PendingUpdate::Downloading)) {
                *pending = None;
            }
        }
    }
}

impl UpdateDownloadSource {
    fn label(&self) -> &'static str {
        match self {
            Self::Official => "official",
            Self::Cnb => "cnb",
        }
    }

    fn endpoints(&self, latest_version: Option<&str>) -> Result<Vec<String>, String> {
        match self {
            Self::Official => Ok(OFFICIAL_UPDATE_ENDPOINTS.iter().map(|endpoint| endpoint.to_string()).collect()),
            Self::Cnb => {
                let version =
                    latest_version.ok_or_else(|| "Latest version is required for CNB updates.".to_string())?;
                Ok(vec![
                    format!("{CNB_RELEASE_DOWNLOAD_PREFIX}{}/latest.json", tag_version(version)),
                    OFFICIAL_UPDATE_ENDPOINTS[0].to_string(),
                ])
            }
        }
    }

    fn rewrite_download_url(&self, url: &str) -> Result<Option<String>, String> {
        let Some(target_prefix) = self.mirror_download_prefix() else { return Ok(None) };

        if url.starts_with(target_prefix) {
            return Ok(None);
        }

        // Mirror latest.json files still contain GitHub asset URLs, so rewrite only that known release prefix.
        let rewritten = url
            .strip_prefix(GITHUB_RELEASE_DOWNLOAD_PREFIX)
            .map(|path| format!("{target_prefix}{path}"))
            .ok_or_else(|| format!("Unsupported update download URL for {} source: {url}", self.label()))?;
        Ok(Some(rewritten))
    }

    fn mirror_download_prefix(&self) -> Option<&'static str> {
        match self {
            Self::Cnb => Some(CNB_RELEASE_DOWNLOAD_PREFIX),
            Self::Official => None,
        }
    }

    fn r2_fallback_url(&self, url: &str) -> Result<Option<String>, String> {
        if matches!(self, Self::Official) || url.starts_with(R2_LATEST_RELEASE_DOWNLOAD_PREFIX) {
            return Ok(None);
        }
        let filename = url
            .rsplit('/')
            .next()
            .filter(|name| !name.is_empty())
            .ok_or_else(|| format!("Unsupported update download URL for {} source: {url}", self.label()))?;
        Ok(Some(format!("{R2_LATEST_RELEASE_DOWNLOAD_PREFIX}{filename}")))
    }
}

fn tag_version(version: &str) -> String {
    let version = version.trim();
    if version.starts_with('v') {
        version.to_string()
    } else {
        format!("v{version}")
    }
}

#[tauri::command]
pub async fn check_for_updates(locale: Option<String>) -> Result<UpdateInfo, String> {
    let locale = locale.unwrap_or_else(|| "zh-CN".to_string());
    let release = dbx_core::update::fetch_latest_release(&locale).await?;
    let current_version = env!("CARGO_PKG_VERSION");
    let mut info = dbx_core::update::build_update_info(release, current_version);
    info.portable_mode = crate::data_dir::is_portable_mode();
    Ok(info)
}

#[tauri::command]
pub async fn fetch_changelog(lang: Option<String>) -> Result<dbx_core::changelog::ChangelogData, String> {
    let lang = lang.unwrap_or_else(|| "en".to_string());
    dbx_core::changelog::fetch_changelog(&lang).await
}

#[tauri::command]
pub async fn get_system_proxy_url() -> Option<String> {
    tauri::async_runtime::spawn_blocking(dbx_core::update::system_proxy_url).await.ok().flatten()
}

#[tauri::command]
pub async fn download_update(
    _app: AppHandle,
    _state: tauri::State<'_, PendingUpdateState>,
    _source: UpdateDownloadSource,
    _latest_version: Option<String>,
) -> Result<(), String> {
    Err("Auto-update is disabled in this build.".into())
}

#[tauri::command]
pub fn install_downloaded_update(_state: tauri::State<'_, PendingUpdateState>) -> Result<(), String> {
    Err("Auto-update is disabled in this build.".into())
}

#[cfg(test)]
mod tests {
    use super::{
        tag_version, UpdateDownloadSource, CNB_RELEASE_DOWNLOAD_PREFIX, OFFICIAL_UPDATE_ENDPOINTS,
        R2_LATEST_RELEASE_DOWNLOAD_PREFIX,
    };

    #[test]
    fn normalizes_update_tag_versions() {
        assert_eq!(tag_version("0.5.39"), "v0.5.39");
        assert_eq!(tag_version("v0.5.39"), "v0.5.39");
    }

    #[test]
    fn builds_official_update_endpoints() {
        let endpoints = UpdateDownloadSource::Official.endpoints(None).unwrap();
        assert_eq!(endpoints, OFFICIAL_UPDATE_ENDPOINTS);
    }

    #[test]
    fn builds_cnb_update_endpoint_for_tag() {
        let endpoints = UpdateDownloadSource::Cnb.endpoints(Some("0.5.39")).unwrap();
        assert_eq!(
            endpoints,
            vec![format!("{CNB_RELEASE_DOWNLOAD_PREFIX}v0.5.39/latest.json"), OFFICIAL_UPDATE_ENDPOINTS[0].to_string()]
        );
    }

    #[test]
    fn rewrites_github_asset_url_to_cnb() {
        let download_url = UpdateDownloadSource::Cnb
            .rewrite_download_url("https://github.com/t8y2/dbx/releases/download/v0.5.39/DBX_0.5.39_aarch64.dmg")
            .unwrap()
            .unwrap();
        assert_eq!(download_url, "https://cnb.cool/dbxio.com/dbx/-/releases/download/v0.5.39/DBX_0.5.39_aarch64.dmg");
    }

    #[test]
    fn accepts_existing_cnb_asset_url() {
        let download_url = UpdateDownloadSource::Cnb
            .rewrite_download_url("https://cnb.cool/dbxio.com/dbx/-/releases/download/v0.5.39/DBX_0.5.39_aarch64.dmg")
            .unwrap();
        assert_eq!(download_url, None);
    }

    #[test]
    fn builds_r2_fallback_for_mirror_asset() {
        let fallback = UpdateDownloadSource::Cnb
            .r2_fallback_url("https://cnb.cool/dbxio.com/dbx/-/releases/download/v0.5.44/DBX_0.5.44_x64.dmg")
            .unwrap();
        assert_eq!(fallback, Some(format!("{R2_LATEST_RELEASE_DOWNLOAD_PREFIX}DBX_0.5.44_x64.dmg")));
    }
}

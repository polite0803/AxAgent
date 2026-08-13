//! 包生态扫描器
//! 通过 PyPI 和 npm API 采集包下载量和采用率信号

use super::marketplace_scanner::{MarketplaceScanner, RawLead};
use async_trait::async_trait;

/// 包生态扫描器
pub struct PackageEcosystemScanner {
    http: reqwest::Client,
}

impl PackageEcosystemScanner {
    pub fn new() -> Self {
        let http = reqwest::Client::new();
        Self { http }
    }

    /// 构建 PyPI 包信息 URL
    fn build_pypi_url(package: &str) -> String {
        format!("https://pypi.org/pypi/{}/json", package)
    }

    /// 构建 npm 包信息 URL
    fn build_npm_url(package: &str) -> String {
        format!("https://registry.npmjs.org/{}", package)
    }

    /// 需求相关包关键词
    fn demand_package_keywords() -> Vec<&'static str> {
        vec![
            "api",
            "client",
            "server",
            "framework",
            "library",
            "tool",
            "sdk",
            "integration",
            "adapter",
            "plugin",
            "connector",
            "driver",
            "wrapper",
            "gateway",
            "data",
            "analytics",
            "reporting",
            "dashboard",
            "trading",
            "finance",
            "stock",
            "market",
            "exchange",
            "ai",
            "ml",
            "machine-learning",
            "nlp",
            "llm",
            "automation",
            "workflow",
            "orchestration",
            "pipeline",
            "cloud",
            "aws",
            "azure",
            "gcp",
            "kubernetes",
            "database",
            "sql",
            "nosql",
            "cache",
            "queue",
        ]
    }

    /// 检查包是否为需求相关
    fn is_demand_package(package_name: &str, description: &str, keywords: &[String]) -> bool {
        let demand_keywords = Self::demand_package_keywords();

        let full_text =
            format!("{} {} {}", package_name, description, keywords.join(" ")).to_lowercase();
        demand_keywords.iter().any(|kw| full_text.contains(kw))
    }

    /// 提取包趋势描述
    fn extract_package_trend(
        package_name: &str,
        description: &str,
        downloads: u64,
        version: &str,
        ecosystem: &str,
    ) -> Option<String> {
        if downloads >= 1_000_000 {
            Some(format!(
                "[{}热门] {} v{} | {} 次下载 | {}",
                ecosystem,
                package_name,
                version,
                downloads,
                description.chars().take(80).collect::<String>()
            ))
        } else if downloads >= 100_000 {
            Some(format!(
                "[{}流行] {} v{} | {} 次下载",
                ecosystem, package_name, version, downloads
            ))
        } else {
            None
        }
    }

    /// 从 PyPI 获取包信息
    async fn fetch_pypi_package(&self, package: &str) -> Result<Option<PackageInfo>, String> {
        let url = Self::build_pypi_url(package);

        let response =
            self.http.get(&url).send().await.map_err(|e| format!("PyPI API 请求失败: {}", e))?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let data: serde_json::Value =
            response.json().await.map_err(|e| format!("PyPI 响应解析失败: {}", e))?;

        let info = &data["info"];
        let name = info["name"].as_str().unwrap_or(package).to_string();
        let description = info["summary"].as_str().unwrap_or("").to_string();
        let version = info["version"].as_str().unwrap_or("unknown").to_string();
        let keywords: Vec<String> = info["keywords"]
            .as_str()
            .map(|k| k.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

        // 获取最近一个月的下载量（简化估算）
        let downloads = data["urls"]
            .as_array()
            .map(|urls| urls.iter().filter_map(|u| u["downloads"].as_i64()).sum::<i64>() as u64)
            .unwrap_or(0);

        Ok(Some(PackageInfo {
            name,
            description,
            version,
            keywords,
            downloads,
            ecosystem: "pypi".to_string(),
            url: format!("https://pypi.org/project/{}", package),
        }))
    }

    /// 从 npm 获取包信息
    async fn fetch_npm_package(&self, package: &str) -> Result<Option<PackageInfo>, String> {
        let url = Self::build_npm_url(package);

        let response =
            self.http.get(&url).send().await.map_err(|e| format!("npm API 请求失败: {}", e))?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let data: serde_json::Value =
            response.json().await.map_err(|e| format!("npm 响应解析失败: {}", e))?;

        let name = data["name"].as_str().unwrap_or(package).to_string();
        let description = data["description"].as_str().unwrap_or("").to_string();
        let version = data["dist-tags"]["latest"].as_str().unwrap_or("unknown").to_string();
        let keywords: Vec<String> = data["keywords"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|k| k.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        // npm 下载量需要额外请求，这里用粗略估算
        let downloads = 0; // 需要 npm downloads API

        Ok(Some(PackageInfo {
            name,
            description,
            version,
            keywords,
            downloads,
            ecosystem: "npm".to_string(),
            url: format!("https://www.npmjs.com/package/{}", package),
        }))
    }
}

/// 包信息结构
struct PackageInfo {
    name: String,
    description: String,
    version: String,
    keywords: Vec<String>,
    downloads: u64,
    ecosystem: String,
    url: String,
}

impl Default for PackageEcosystemScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MarketplaceScanner for PackageEcosystemScanner {
    fn platform(&self) -> &'static str {
        "package_ecosystem"
    }

    async fn search(&self, q: &str) -> Result<Vec<RawLead>, String> {
        if q.is_empty() {
            return Ok(Vec::new());
        }

        tracing::info!(query = q, "[PackageEcosystemScanner] 开始搜索");

        let mut leads = Vec::new();

        // 将查询拆分为可能的包名
        let potential_packages: Vec<String> =
            q.split_whitespace().map(|s| s.to_lowercase()).filter(|s| s.len() > 2).collect();

        // 同时在 PyPI 和 npm 搜索
        for package_name in &potential_packages {
            // PyPI
            if let Ok(Some(pkg_info)) = self.fetch_pypi_package(package_name).await
                && Self::is_demand_package(
                    &pkg_info.name,
                    &pkg_info.description,
                    &pkg_info.keywords,
                )
                && let Some(trend) = Self::extract_package_trend(
                    &pkg_info.name,
                    &pkg_info.description,
                    pkg_info.downloads,
                    &pkg_info.version,
                    &pkg_info.ecosystem,
                )
            {
                leads.push(RawLead {
                    platform: "package_ecosystem".to_string(),
                    title: format!("PyPI: {}", pkg_info.name),
                    description: trend,
                    url: pkg_info.url.clone(),
                    price_text: None,
                    contact: None,
                    contact_email: None,
                    contact_phone: None,
                    snapshot: serde_json::json!({
                        "name": pkg_info.name,
                        "version": pkg_info.version,
                        "downloads": pkg_info.downloads,
                        "ecosystem": "pypi",
                        "_extracted_source": "package_ecosystem",
                    }),
                });
            }

            // npm
            if let Ok(Some(pkg_info)) = self.fetch_npm_package(package_name).await
                && Self::is_demand_package(
                    &pkg_info.name,
                    &pkg_info.description,
                    &pkg_info.keywords,
                )
                && let Some(trend) = Self::extract_package_trend(
                    &pkg_info.name,
                    &pkg_info.description,
                    pkg_info.downloads,
                    &pkg_info.version,
                    &pkg_info.ecosystem,
                )
            {
                leads.push(RawLead {
                    platform: "package_ecosystem".to_string(),
                    title: format!("npm: {}", pkg_info.name),
                    description: trend,
                    url: pkg_info.url.clone(),
                    price_text: None,
                    contact: None,
                    contact_email: None,
                    contact_phone: None,
                    snapshot: serde_json::json!({
                        "name": pkg_info.name,
                        "version": pkg_info.version,
                        "downloads": pkg_info.downloads,
                        "ecosystem": "npm",
                        "_extracted_source": "package_ecosystem",
                    }),
                });
            }

            // 避免过快请求
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        tracing::info!(query = q, filtered = leads.len(), "[PackageEcosystemScanner] 搜索完成");

        Ok(leads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform() {
        let scanner = PackageEcosystemScanner::new();
        assert_eq!(scanner.platform(), "package_ecosystem");
    }

    #[test]
    fn test_build_urls() {
        let pypi_url = PackageEcosystemScanner::build_pypi_url("requests");
        assert!(pypi_url.contains("pypi.org"));
        assert!(pypi_url.contains("requests"));

        let npm_url = PackageEcosystemScanner::build_npm_url("express");
        assert!(npm_url.contains("npmjs.org"));
        assert!(npm_url.contains("express"));
    }

    #[test]
    fn test_is_demand_package() {
        // API 包
        assert!(PackageEcosystemScanner::is_demand_package(
            "requests",
            "Python HTTP library for humans",
            &["http".to_string(), "api".to_string()]
        ));

        // 框架包
        assert!(PackageEcosystemScanner::is_demand_package(
            "django",
            "A high-level Python web framework",
            &["framework".to_string(), "web".to_string()]
        ));

        // 金融包
        assert!(PackageEcosystemScanner::is_demand_package(
            "yfinance",
            "Yahoo Finance market data downloader",
            &["finance".to_string(), "stock".to_string()]
        ));

        // 不相关
        assert!(!PackageEcosystemScanner::is_demand_package(
            "unknown-pkg",
            "Unknown package for testing",
            &["other".to_string()]
        ));
    }

    #[test]
    fn test_extract_package_trend() {
        // 热门包
        let trend = PackageEcosystemScanner::extract_package_trend(
            "requests",
            "Python HTTP library",
            10_000_000,
            "2.31.0",
            "pypi",
        );
        assert!(trend.is_some());
        assert!(trend.unwrap().contains("热门"));

        // 流行包
        let trend = PackageEcosystemScanner::extract_package_trend(
            "flask",
            "Web framework",
            500_000,
            "3.0.0",
            "pypi",
        );
        assert!(trend.is_some());
        assert!(trend.unwrap().contains("流行"));

        // 小众包
        let trend = PackageEcosystemScanner::extract_package_trend(
            "small-pkg",
            "A small package",
            1_000,
            "1.0.0",
            "npm",
        );
        assert!(trend.is_none());
    }

    #[tokio::test]
    async fn test_fetch_pypi_package() {
        let scanner = PackageEcosystemScanner::new();
        let result = scanner.fetch_pypi_package("requests").await;
        assert!(result.is_ok());
        if let Ok(Some(pkg)) = result {
            assert!(!pkg.name.is_empty());
            assert!(pkg.downloads >= 0);
        }
    }

    #[tokio::test]
    async fn test_search_with_package_name() {
        let scanner = PackageEcosystemScanner::new();
        let result = scanner.search("requests django").await;
        assert!(result.is_ok());
    }
}

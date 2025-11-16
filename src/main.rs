use clap::Parser;
use dotenv::dotenv;
use http::StatusCode;
use indicatif::{ProgressBar, ProgressStyle};
use octocrab::{models::Repository, Octocrab, Page};
use std::env;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tokio::{
    io::AsyncWriteExt,
    sync::Semaphore,
    task::JoinSet,
    time::{sleep, Duration},
};
use tracing::{debug, error, info, warn};
use url::Url;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Lists organization forks of a given public GitHub repository."
)]
struct Args {
    /// Full GitHub repository URL (e.g., https://github.com/kubernetes/kubernetes)
    repo_url: String,

    /// Optionally specify a GitHub token directly via CLI
    #[arg(short, long)]
    token: Option<String>,

    /// Override output filename (default: "reports/<repo>_forks.md")
    #[arg(short, long)]
    output: Option<String>,

    /// Number of concurrent requests (default: 10)
    #[arg(short, long, default_value = "10")]
    concurrency: usize,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, Error)]
enum ForkliftError {
    #[error("No GitHub token found. Please set GITHUB_TOKEN in .env or environment variable, or pass --token=<TOKEN> on CLI.")]
    MissingGithubToken,

    #[error("Failed to parse repository URL: {0}")]
    InvalidUrl(String),

    #[error("Expected a 'github.com' domain, but got: {0}")]
    InvalidDomain(String),

    #[error("Expected the URL path format to be /OWNER/REPO, but got: {0:?}")]
    InvalidPathSegments(Vec<String>),

    #[error(transparent)]
    OctocrabError(#[from] octocrab::Error),

    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

/// Holds the extracted repository info
#[derive(Debug)]
struct RepoInfo {
    owner: String,
    name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments first to check for verbose flag
    let args = Args::parse();

    // Initialize tracing
    let log_level = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        )
        .with_target(false)
        .compact()
        .init();

    // Load .env if present
    dotenv().ok();

    // Determine final GitHub token
    let github_token = match args.token {
        Some(cli_token) => cli_token,
        None => env::var("GITHUB_TOKEN").map_err(|_| ForkliftError::MissingGithubToken)?,
    };

    // Parse the provided GitHub URL
    let RepoInfo { owner, name: repo } = parse_github_url(&args.repo_url)?;
    info!("Analyzing forks for {}/{}", owner, repo);

    // Build an Octocrab client
    let octocrab = Octocrab::builder().personal_token(github_token).build()?;

    // Fetch first page to determine total pages
    debug!("Fetching initial page to determine fork count");
    let mut current_page: Page<Repository> = octocrab
        .repos(&owner, &repo)
        .list_forks()
        .per_page(100)
        .send()
        .await?;

    let mut all_forks: Vec<Repository> = Vec::new();
    all_forks.extend(current_page.take_items());

    // Process remaining pages in parallel if there are more
    if let Some(total_pages) = current_page.number_of_pages() {
        info!("Found {} pages of forks to fetch", total_pages);

        // Create progress bar
        let progress = ProgressBar::new(total_pages as u64 - 1);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} pages ({eta})")
                .expect("Invalid progress bar template")
                .progress_chars("#>-")
        );

        let mut tasks = JoinSet::new();
        let semaphore = Arc::new(Semaphore::new(args.concurrency));
        let completed = Arc::new(AtomicUsize::new(0));

        for page in 2..=total_pages {
            let octo = octocrab.clone();
            let owner_clone = owner.clone();
            let repo_clone = repo.clone();
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let completed_clone = completed.clone();
            let progress_clone = progress.clone();

            tasks.spawn(async move {
                let _permit = permit;
                let result = fetch_page_with_retry(octo, owner_clone, repo_clone, page).await;

                // Update progress
                let count = completed_clone.fetch_add(1, Ordering::Relaxed) + 1;
                progress_clone.set_position(count as u64);

                result
            });
        }

        // Collect results as they come in
        while let Some(res) = tasks.join_next().await {
            match res {
                Ok(Ok(items)) => {
                    debug!("Fetched {} forks from page", items.len());
                    all_forks.extend(items);
                }
                Ok(Err(e)) => {
                    error!("Failed to fetch page: {}", e);
                    return Err(e.into());
                }
                Err(e) => {
                    error!("Task join error: {}", e);
                    return Err(e.into());
                }
            }
        }

        progress.finish_with_message("All pages fetched");
    } else {
        info!("Only one page of forks found");
    }

    // Filter organization forks
    let org_forks: Vec<_> = all_forks
        .into_iter()
        .filter_map(|fork| {
            fork.owner.and_then(|owner| {
                if owner.r#type.eq("Organization") {
                    Some((
                        owner.login,
                        fork.name,
                        fork.html_url.map(|u| u.to_string()).unwrap_or_default(),
                    ))
                } else {
                    None
                }
            })
        })
        .collect();

    info!("Found {} organization-owned forks", org_forks.len());

    // Determine output path
    let final_output = if let Some(path) = args.output {
        path
    } else {
        fs::create_dir_all("reports")?;
        format!("reports/{}_forks.md", repo)
    };

    // Write results asynchronously
    debug!("Writing results to {}", final_output);
    write_results(&final_output, &owner, &repo, &org_forks).await?;

    info!("✓ Analysis completed. Results written to: {}", final_output);
    Ok(())
}

/// Parse a GitHub URL of the form:
///   https://github.com/OWNER/REPO
///   http://github.com/OWNER/REPO
///   github.com/OWNER/REPO
/// Returns RepoInfo { owner, name } on success, or ForkliftError otherwise.
fn parse_github_url(raw_url: &str) -> Result<RepoInfo, ForkliftError> {
    // If missing scheme, prepend "https://"
    let url_with_scheme = if raw_url.starts_with("http://") || raw_url.starts_with("https://") {
        raw_url.to_string()
    } else {
        format!("https://{}", raw_url)
    };

    let parsed =
        Url::parse(&url_with_scheme).map_err(|_| ForkliftError::InvalidUrl(raw_url.to_string()))?;

    if parsed.domain() != Some("github.com") {
        return Err(ForkliftError::InvalidDomain(
            parsed.domain().unwrap_or_default().to_string(),
        ));
    }

    let mut segments: Vec<String> = parsed
        .path_segments()
        .map(|c| c.map(|s| s.to_string()).collect())
        .unwrap_or_default();
    segments.retain(|s| !s.is_empty());

    if segments.len() < 2 {
        return Err(ForkliftError::InvalidPathSegments(segments));
    }

    let owner = segments[0].clone();
    let name = segments[1].clone();

    Ok(RepoInfo { owner, name })
}

/// Fetch a single fork page and retry if GitHub's secondary rate limit is hit.
async fn fetch_page_with_retry(
    octocrab: Octocrab,
    owner: String,
    repo: String,
    page: u32,
) -> Result<Vec<Repository>, octocrab::Error> {
    let mut attempts = 0;
    const MAX_RETRIES: u32 = 3;

    loop {
        match octocrab
            .repos(&owner, &repo)
            .list_forks()
            .per_page(100)
            .page(page)
            .send()
            .await
        {
            Ok(mut p) => {
                if attempts > 0 {
                    debug!(
                        "Successfully fetched page {} after {} retries",
                        page, attempts
                    );
                }
                return Ok(p.take_items());
            }
            Err(err) => {
                let should_retry = match &err {
                    octocrab::Error::GitHub { source, .. } => {
                        source.status_code == StatusCode::FORBIDDEN
                            && source.message.to_ascii_lowercase().contains("rate limit")
                            && attempts < MAX_RETRIES
                    }
                    _ => false,
                };

                if should_retry {
                    attempts += 1;
                    // More reasonable exponential backoff: 2s, 4s, 8s
                    let wait = 2u64.pow(attempts);
                    warn!(
                        "Rate limit hit on page {}, retrying in {}s (attempt {}/{})",
                        page, wait, attempts, MAX_RETRIES
                    );
                    sleep(Duration::from_secs(wait)).await;
                    continue;
                }

                return Err(err);
            }
        }
    }
}

/// Write results to markdown file asynchronously
async fn write_results(
    path: &str,
    owner: &str,
    repo: &str,
    forks: &[(String, String, String)],
) -> Result<(), std::io::Error> {
    let mut file = tokio::fs::File::create(path).await?;

    file.write_all(format!("# Organization-owned forks for {}/{}\n\n", owner, repo).as_bytes())
        .await?;
    file.write_all(b"| Organization | Fork Name | URL |\n")
        .await?;
    file.write_all(b"|--------------|----------|-----|\n")
        .await?;

    for (org_name, fork_name, fork_url) in forks {
        file.write_all(format!("| {} | {} | {} |\n", org_name, fork_name, fork_url).as_bytes())
            .await?;
    }

    file.flush().await?;
    Ok(())
}

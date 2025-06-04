use clap::Parser;
use dotenv::dotenv;
use octocrab::{models::Repository, Octocrab, Page};
use tokio::task::JoinSet;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use thiserror::Error;
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
}

/// Holds the extracted repository info
#[derive(Debug)]
struct RepoInfo {
    owner: String,
    name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Parse CLI arguments
    let args = Args::parse();

    // 2. Load .env if present (this will populate the environment)
    dotenv().ok();

    // 3. Determine final GitHub token (priority: CLI token > .env/env variable)
    let github_token = match args.token {
        Some(cli_token) => cli_token,
        None => env::var("GITHUB_TOKEN").map_err(|_| ForkliftError::MissingGithubToken)?,
    };

    // 4. Parse the provided GitHub URL to extract (owner, repo)
    let RepoInfo { owner, name: repo } = parse_github_url(&args.repo_url)?;

    // 5. Build an Octocrab client
    let octocrab = Octocrab::builder().personal_token(github_token).build()?;

    // 6. Fetch all pages of forks
    let mut all_forks: Vec<Repository> = Vec::new();

    // Fetch first page and capture total page count
    let mut current_page: Page<Repository> = octocrab
        .repos(&owner, &repo)
        .list_forks()
        .per_page(100)
        .send()
        .await?;

    all_forks.extend(current_page.take_items());

    if let Some(total_pages) = current_page.number_of_pages() {
        let mut tasks = JoinSet::new();
        for page in 2..=total_pages {
            let octo = octocrab.clone();
            let owner = owner.clone();
            let repo = repo.clone();
            tasks.spawn(async move {
                let mut page = octo
                    .repos(&owner, &repo)
                    .list_forks()
                    .per_page(100)
                    .page(page)
                    .send()
                    .await?;
                Ok::<_, octocrab::Error>(page.take_items())
            });
        }

        while let Some(res) = tasks.join_next().await {
            let items = res??;
            all_forks.extend(items);
        }
    }

    // 7. Determine the final output path
    // If user gave an --output, use that; otherwise default to "reports/<repo>_forks.md"
    let final_output = if let Some(path) = args.output {
        path
    } else {
        // Ensure "reports" dir exists
        fs::create_dir_all("reports")?;
        format!("reports/{}_forks.md", repo)
    };

    // 8. Write results to the chosen Markdown file
    let mut file = File::create(&final_output)?;

    writeln!(file, "# Organization-owned forks for {}/{}", owner, repo)?;
    writeln!(file)?;
    writeln!(file, "| Organization | Fork Name | URL |")?;
    writeln!(file, "|--------------|----------|-----|")?;

    for fork in all_forks {
        if let Some(fork_owner) = fork.owner {
            if fork_owner.r#type.eq("Organization") {
                let org_name = fork_owner.login;
                let fork_name = fork.name;
                let fork_url = fork.html_url.map(|u| u.to_string()).unwrap_or_default();

                writeln!(file, "| {} | {} | {} |", org_name, fork_name, fork_url)?;
            }
        }
    }

    println!("Analysis completed. Results written to: {}", final_output);
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

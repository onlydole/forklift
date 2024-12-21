# Forklift

A Rust CLI tool that analyzes and reports on organization-owned forks of public GitHub repositories.

## Features

- Lists all organization-owned forks of any public GitHub repository
- Generates a clean Markdown report with organization names and fork URLs
- Supports authentication via environment variables or CLI arguments
- Handles pagination automatically to fetch all forks
- Customizable output file location

## Installation

### Prerequisites

- Rust 2021 edition or later
- A GitHub personal access token (for API authentication)

### Building from Source

Clone the repository:

```shell
git clone https://github.com/yourusername/forklift.git
cd forklift
```

Build the project:

```shell
cargo build --release
```

The compiled binary will be available at `target/release/forklift`.

## Usage

Basic usage:

```shell
forklift https://github.com/kubernetes/kubernetes
```

With explicit token:

```shell
forklift --token YOUR_GITHUB_TOKEN https://github.com/kubernetes/kubernetes
```

Custom output location:

```shell
forklift --output custom_report.md https://github.com/kubernetes/kubernetes
```

### Authentication

Forklift requires a GitHub personal access token. You can provide it in one of three ways:

1. Environment variable:

   ```shell
   export GITHUB_TOKEN=your_token_here
   ```

2. `.env` file in the project directory:

   ```env
   GITHUB_TOKEN=your_token_here
   ```

3. Command-line argument:

   ```shell
   forklift --token your_token_here REPO_URL
   ```

### Output

By default, Forklift generates a Markdown report in the `reports/` directory with the name pattern `{repo}_forks.md`. The report includes:

- Organization name
- Fork repository name
- Fork URL

Example output structure:

```markdown
| Organization | Fork Name | URL |
|--------------|-----------|-----|
| google | kubernetes | https://github.com/google/kubernetes |
| microsoft | kubernetes | https://github.com/microsoft/kubernetes |
```

## Error Handling

Forklift provides clear error messages for common issues:

- Missing GitHub token
- Invalid repository URLs
- Network/API errors
- Invalid file paths

## Dependencies

- `octocrab`: GitHub API client for Rust
- `tokio`: Async runtime
- `clap`: Command-line argument parsing
- `dotenv`: Environment variable management
- `url`: URL parsing
- `thiserror`: Error handling

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

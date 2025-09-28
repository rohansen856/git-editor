use crate::utils::types::Result;
use colored::*;
use std::fs;

pub fn execute_docs_operation() -> Result<()> {
    println!("{}", "📚 Opening Git Editor Documentation...".cyan().bold());

    let docs_html = generate_comprehensive_docs()?;

    // Create a temporary HTML file
    let temp_dir = std::env::temp_dir();
    let docs_file = temp_dir.join("git-editor-docs.html");

    fs::write(&docs_file, docs_html)?;

    // Open the file in the default browser
    match open_in_browser(&docs_file) {
        Ok(_) => {
            println!(
                "{}",
                "✅ Documentation opened in your default browser!"
                    .green()
                    .bold()
            );
            println!(
                "{}",
                format!("📍 File location: {}", docs_file.display()).dimmed()
            );
        }
        Err(_) => {
            println!(
                "{}",
                "⚠️  Could not open browser automatically.".yellow().bold()
            );
            println!(
                "{}",
                format!("📁 Documentation saved at: {}", docs_file.display()).cyan()
            );
            println!(
                "{}",
                "💡 You can manually open this file in your browser.".dimmed()
            );
            // Don't return an error - the docs were still successfully generated
        }
    }

    Ok(())
}

fn open_in_browser(file_path: &std::path::Path) -> Result<()> {
    // Skip browser opening if NO_BROWSER environment variable is set or if running in test context
    if std::env::var("NO_BROWSER").is_ok()
        || std::env::var("GIT_EDITOR_NO_BROWSER").is_ok()
        || cfg!(test)
    {
        return Err("Browser opening disabled".into());
    }

    let file_url = format!("file://{}", file_path.display());
    open::that(file_url)?;
    Ok(())
}

fn generate_comprehensive_docs() -> Result<String> {
    // Load the HTML template from the embedded file
    let template = include_str!("../docs/template.html");

    let version = env!("CARGO_PKG_VERSION");
    let current_date = chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S UTC")
        .to_string();

    // Replace placeholders in the template
    let html = template
        .replace("{{VERSION}}", version)
        .replace("{{DATE}}", &current_date);

    Ok(html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_comprehensive_docs() {
        let result = generate_comprehensive_docs();
        assert!(result.is_ok());

        let html = result.unwrap();

        // Check that HTML template is loaded
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Git Editor - Comprehensive Documentation"));

        // Check that placeholders are replaced
        assert!(html.contains(&format!("Git Editor v{}", env!("CARGO_PKG_VERSION"))));
        assert!(!html.contains("{{VERSION}}"));
        assert!(!html.contains("{{DATE}}"));

        // Check for key sections
        assert!(html.contains("Table of Contents"));
        assert!(html.contains("Overview"));
        assert!(html.contains("Installation"));
        assert!(html.contains("Operation Modes"));
        assert!(html.contains("Command Reference"));
        assert!(html.contains("Usage Examples"));
        assert!(html.contains("Advanced Features"));
        assert!(html.contains("Development"));
        assert!(html.contains("Troubleshooting"));

        // Check for docs command reference
        assert!(html.contains("--docs"));
        assert!(html.contains("Open comprehensive documentation"));
    }

    #[test]
    fn test_docs_html_contains_expected_sections() {
        let html = generate_comprehensive_docs().unwrap();

        // Test that all main sections are present
        let expected_sections = [
            "#overview",
            "#installation",
            "#operation-modes",
            "#command-reference",
            "#examples",
            "#advanced-features",
            "#development",
            "#troubleshooting",
        ];

        for section in expected_sections {
            assert!(html.contains(section), "Missing section: {section}");
        }
    }

    #[test]
    fn test_docs_html_contains_operation_modes() {
        let html = generate_comprehensive_docs().unwrap();

        // Test that all operation modes are documented
        assert!(html.contains("Full Rewrite"));
        assert!(html.contains("Show History"));
        assert!(html.contains("Pick Specific"));
        assert!(html.contains("Range Edit"));
        assert!(html.contains("Simulation"));
        assert!(html.contains("Documentation"));

        // Test that CLI flags are documented
        assert!(html.contains("--show-history"));
        assert!(html.contains("--pick-specific-commits"));
        assert!(html.contains("--range"));
        assert!(html.contains("--simulate"));
        assert!(html.contains("--docs"));
    }

    #[test]
    fn test_docs_html_contains_examples() {
        let html = generate_comprehensive_docs().unwrap();

        // Test that usage examples are present
        assert!(html.contains("git-editor --docs"));
        assert!(html.contains("git-editor -s"));
        assert!(html.contains("git-editor --simulate"));
        assert!(html.contains("Opens this comprehensive documentation"));
    }

    #[test]
    fn test_version_and_date_replacement() {
        let html = generate_comprehensive_docs().unwrap();

        // Test version replacement
        let expected_version = env!("CARGO_PKG_VERSION");
        assert!(html.contains(&format!("Git Editor v{expected_version}")));

        // Test that date is properly formatted (should contain UTC)
        assert!(html.contains("Generated on"));
        assert!(html.contains("UTC"));

        // Test that placeholders are completely replaced
        assert!(!html.contains("{{VERSION}}"));
        assert!(!html.contains("{{DATE}}"));
    }

    #[test]
    fn test_html_structure_validity() {
        let html = generate_comprehensive_docs().unwrap();

        // Test basic HTML structure
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<html lang=\"en\">"));
        assert!(html.contains("<head>"));
        assert!(html.contains("<body>"));
        assert!(html.contains("</body>"));
        assert!(html.contains("</html>"));

        // Test that CSS is included
        assert!(html.contains("<style>"));
        assert!(html.contains(".container"));
        assert!(html.contains(".badge"));

        // Test that the main container is present
        assert!(html.contains("<div class=\"container\">"));
    }
}

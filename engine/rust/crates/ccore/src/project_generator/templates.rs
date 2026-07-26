/// `.gitignore` content for a freshly generated project.
pub const GITIGNORE: &str = include_str!("templates/gitignore.tmpl");

const README_TEMPLATE: &str = include_str!("templates/readme.md.tmpl");

/// Fills in the `README.md` template. `description` is meant to later hold
/// whatever intro text the "create project" flow collects from the user;
/// it's optional for now, so an empty one renders a placeholder line.
pub fn render_readme(project_name: &str, description: &str, engine_version: &str) -> String {
    let description = if description.trim().is_empty() {
        "_No description yet._"
    } else {
        description
    };
    README_TEMPLATE
        .replace("{{project_name}}", project_name)
        .replace("{{description}}", description)
        .replace("{{engine_version}}", engine_version)
}
